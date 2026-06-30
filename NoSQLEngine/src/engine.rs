use crate::config::DbConfig;
use crate::memtable::{MemTable, TOMBSTONE};
use crate::sstable::{SsTableReader, SsTableWriter};
use crate::wal::Wal;
use std::fs;
use std::io;
use std::path::PathBuf;

pub struct Engine {
    config: DbConfig,
    memtable: MemTable,
    wal: Wal,
    sstables: Vec<SsTableReader>,
    next_sst_id: u32,
}

impl Engine {
    pub fn open(config: DbConfig) -> io::Result<Self> {
        fs::create_dir_all(&config.data_dir)?;

        let wal_path = config.data_dir.join("wal.log");
        let mut memtable = MemTable::new(config.memtable_capacity);

        // Replay WAL into MemTable before opening WAL for appending
        let entries = Wal::recover(&wal_path)?;
        let recovered = entries.len();
        for (key, value) in entries {
            memtable.put(key, value);
        }
        if recovered > 0 {
            println!("[Engine] Recovered {} entries from WAL", recovered);
        }

        let wal = Wal::open(&wal_path)?;

        // Load existing SSTable files in creation order
        let mut sst_paths: Vec<PathBuf> = fs::read_dir(&config.data_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "sst").unwrap_or(false))
            .collect();
        sst_paths.sort();

        let mut next_sst_id = 0u32;
        let mut sstables = Vec::new();
        for path in sst_paths {
            if let Some(id) = sst_id_from_path(&path) {
                next_sst_id = next_sst_id.max(id + 1);
            }
            sstables.push(SsTableReader::open(path)?);
        }

        Ok(Engine { config, memtable, wal, sstables, next_sst_id })
    }

    pub fn put(&mut self, key: String, value: Vec<u8>) -> io::Result<()> {
        println!(
            "[PUT] key={} | WAL append → MemTable ({}/{})",
            key,
            self.memtable.len() + 1,
            self.config.memtable_capacity
        );
        self.wal.append(&key, &value)?;
        self.memtable.put(key, value);
        if self.memtable.is_full() {
            self.flush()?;
        }
        Ok(())
    }

    pub fn get(&self, key: &str) -> io::Result<Option<Vec<u8>>> {
        // MemTable is always checked first (most recent writes)
        if let Some(value) = self.memtable.get(key) {
            if value == TOMBSTONE {
                println!("[GET] key={} | found tombstone in MemTable → None", key);
                return Ok(None);
            }
            println!("[GET] key={} | hit MemTable", key);
            return Ok(Some(value.clone()));
        }

        // SSTables newest → oldest
        for (i, sst) in self.sstables.iter().enumerate().rev() {
            let sst_name = sst
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");

            if !sst.bloom_may_contain(key) {
                println!("[GET] key={} | bloom filter rejected {}", key, sst_name);
                continue;
            }

            match sst.get(key)? {
                Some(value) => {
                    println!("[GET] key={} | hit {} (SSTable #{})", key, sst_name, i);
                    return Ok(Some(value));
                }
                None => {
                    println!("[GET] key={} | not found in {} (tombstone or absent)", key, sst_name);
                }
            }
        }

        println!("[GET] key={} | not found anywhere", key);
        Ok(None)
    }

    pub fn delete(&mut self, key: String) -> io::Result<()> {
        println!("[DELETE] key={} | writing tombstone", key);
        self.put(key, TOMBSTONE.to_vec())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.memtable.is_empty() {
            println!("[Engine] MemTable is empty — nothing to flush");
            return Ok(());
        }
        let path = self
            .config
            .data_dir
            .join(format!("sst_{:04}.sst", self.next_sst_id));
        println!(
            "[Engine] Flushing {} records → {}",
            self.memtable.len(),
            path.display()
        );

        SsTableWriter::flush(&self.memtable, &path)?;
        self.sstables.push(SsTableReader::open(&path)?);
        self.next_sst_id += 1;

        self.memtable.clear();
        self.wal.clear()?;
        Ok(())
    }

    // Returns all live keys visible from MemTable + SSTables (deduped, tombstones excluded)
    pub fn list_keys(&self) -> io::Result<Vec<String>> {
        use std::collections::{BTreeMap, HashSet};

        // Collect every key we know about (MemTable wins over SSTables)
        let mut seen: HashSet<String> = HashSet::new();
        let mut live: BTreeMap<String, bool> = BTreeMap::new(); // true = alive

        // MemTable has the most recent state
        for (key, value) in self.memtable.iter() {
            seen.insert(key.clone());
            live.insert(key.clone(), value != crate::memtable::TOMBSTONE);
        }

        // SSTables newest → oldest; skip keys already resolved by MemTable
        for sst in self.sstables.iter().rev() {
            for (key, _offset) in sst.index_iter() {
                if seen.contains(key.as_str()) {
                    continue;
                }
                seen.insert(key.clone());
                match sst.get(key)? {
                    Some(_) => { live.insert(key.clone(), true); }
                    None => { live.insert(key.clone(), false); }
                }
            }
        }

        Ok(live.into_iter().filter_map(|(k, alive)| alive.then_some(k)).collect())
    }

    pub fn purge(&mut self) -> io::Result<()> {
        // Delete all SSTable files from disk
        for sst in &self.sstables {
            let path = sst.path();
            println!("[PURGE] Deleting {}", path.display());
            fs::remove_file(path)?;
        }
        let sst_count = self.sstables.len();
        self.sstables.clear();
        self.next_sst_id = 0;

        // Clear MemTable and WAL
        let mem_count = self.memtable.len();
        self.memtable.clear();
        self.wal.clear()?;

        println!(
            "[PURGE] Done — removed {} SSTable file(s) and {} MemTable record(s)",
            sst_count, mem_count
        );
        Ok(())
    }

    pub fn stats(&self) {
        println!(
            "[Stats] MemTable: {}/{} records | SSTables on disk: {}",
            self.memtable.len(),
            self.config.memtable_capacity,
            self.sstables.len()
        );
    }
}

fn sst_id_from_path(path: &std::path::Path) -> Option<u32> {
    path.file_stem()?
        .to_str()?
        .strip_prefix("sst_")?
        .parse()
        .ok()
}
