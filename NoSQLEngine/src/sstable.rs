use crate::bloom::BloomFilter;
use crate::memtable::{MemTable, TOMBSTONE};
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// SSTable file layout:
//
//   [Data section]
//     For each entry (sorted by key):
//       key_len:   u32
//       key:       [u8; key_len]
//       value_len: u32
//       value:     [u8; value_len]
//
//   [Index section]  — starts at index_offset
//     For each entry:
//       key_len: u32
//       key:     [u8; key_len]
//       offset:  u64  (byte offset of entry in data section)
//
//   [Bloom section]  — starts at bloom_offset
//     bloom filter bytes (see bloom.rs)
//
//   [Footer]  — last 20 bytes
//       index_offset: u64
//       bloom_offset: u64
//       num_entries:  u32

pub struct SsTableWriter;

impl SsTableWriter {
    pub fn flush(memtable: &MemTable, path: impl AsRef<Path>) -> io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path.as_ref())?;
        let mut writer = BufWriter::new(file);

        let mut bloom = BloomFilter::new(memtable.len().max(1), 0.01);
        let mut index: Vec<(String, u64)> = Vec::new();
        let mut offset: u64 = 0;

        for (key, value) in memtable.iter() {
            bloom.insert(key.as_bytes());
            index.push((key.clone(), offset));

            let key_bytes = key.as_bytes();
            writer.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(key_bytes)?;
            writer.write_all(&(value.len() as u32).to_le_bytes())?;
            writer.write_all(value)?;
            offset += 4 + key_bytes.len() as u64 + 4 + value.len() as u64;
        }

        let index_offset = offset;
        for (key, entry_offset) in &index {
            let key_bytes = key.as_bytes();
            writer.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(key_bytes)?;
            writer.write_all(&entry_offset.to_le_bytes())?;
        }

        let index_size: u64 = index
            .iter()
            .map(|(k, _)| 4 + k.len() as u64 + 8)
            .sum();
        let bloom_offset = index_offset + index_size;

        let bloom_bytes = bloom.to_bytes();
        writer.write_all(&bloom_bytes)?;

        writer.write_all(&index_offset.to_le_bytes())?;
        writer.write_all(&bloom_offset.to_le_bytes())?;
        writer.write_all(&(index.len() as u32).to_le_bytes())?;

        writer.flush()?;
        Ok(())
    }
}

pub struct SsTableReader {
    path: PathBuf,
    index: BTreeMap<String, u64>,
    bloom: BloomFilter,
}

impl SsTableReader {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = File::open(path.as_ref())?;

        // Footer is always the last 20 bytes
        file.seek(SeekFrom::End(-20))?;
        let mut footer = [0u8; 20];
        file.read_exact(&mut footer)?;
        let index_offset = u64::from_le_bytes(footer[0..8].try_into().unwrap());
        let bloom_offset = u64::from_le_bytes(footer[8..16].try_into().unwrap());
        let num_entries = u32::from_le_bytes(footer[16..20].try_into().unwrap()) as usize;

        // Read index
        file.seek(SeekFrom::Start(index_offset))?;
        let mut index = BTreeMap::new();
        for _ in 0..num_entries {
            let mut buf4 = [0u8; 4];
            file.read_exact(&mut buf4)?;
            let key_len = u32::from_le_bytes(buf4) as usize;

            let mut key_buf = vec![0u8; key_len];
            file.read_exact(&mut key_buf)?;
            let key = String::from_utf8(key_buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            let mut offset_buf = [0u8; 8];
            file.read_exact(&mut offset_buf)?;
            index.insert(key, u64::from_le_bytes(offset_buf));
        }

        // Read bloom filter (everything between bloom_offset and footer)
        let file_len = file.seek(SeekFrom::End(0))?;
        let bloom_size = (file_len - 20 - bloom_offset) as usize;
        file.seek(SeekFrom::Start(bloom_offset))?;
        let mut bloom_bytes = vec![0u8; bloom_size];
        file.read_exact(&mut bloom_bytes)?;
        let bloom = BloomFilter::from_bytes(&bloom_bytes);

        Ok(SsTableReader {
            path: path.as_ref().to_path_buf(),
            index,
            bloom,
        })
    }

    pub fn get(&self, key: &str) -> io::Result<Option<Vec<u8>>> {
        if !self.bloom.may_contain(key.as_bytes()) {
            return Ok(None);
        }

        let &offset = match self.index.get(key) {
            Some(o) => o,
            None => return Ok(None),
        };

        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;

        let mut buf4 = [0u8; 4];
        file.read_exact(&mut buf4)?;
        let key_len = u32::from_le_bytes(buf4) as usize;
        file.seek(SeekFrom::Current(key_len as i64))?;

        file.read_exact(&mut buf4)?;
        let value_len = u32::from_le_bytes(buf4) as usize;
        let mut value = vec![0u8; value_len];
        file.read_exact(&mut value)?;

        if value == TOMBSTONE {
            return Ok(None);
        }

        Ok(Some(value))
    }

    pub fn bloom_may_contain(&self, key: &str) -> bool {
        self.bloom.may_contain(key.as_bytes())
    }

    pub fn index_iter(&self) -> impl Iterator<Item = (&String, &u64)> {
        self.index.iter()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}
