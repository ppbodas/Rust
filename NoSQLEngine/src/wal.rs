use crc32fast::Hasher;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

// WAL entry layout (all little-endian):
//   key_len:   u32  (4 bytes)
//   key:       [u8; key_len]
//   value_len: u32  (4 bytes)
//   value:     [u8; value_len]
//   crc32:     u32  (4 bytes) — checksum of key bytes + value bytes

pub struct Wal {
    writer: BufWriter<File>,
    path: PathBuf,
}

impl Wal {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path.as_ref())?;
        Ok(Wal {
            writer: BufWriter::new(file),
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn append(&mut self, key: &str, value: &[u8]) -> io::Result<()> {
        let key_bytes = key.as_bytes();

        let mut hasher = Hasher::new();
        hasher.update(key_bytes);
        hasher.update(value);
        let crc = hasher.finalize();

        self.writer.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
        self.writer.write_all(key_bytes)?;
        self.writer.write_all(&(value.len() as u32).to_le_bytes())?;
        self.writer.write_all(value)?;
        self.writer.write_all(&crc.to_le_bytes())?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn recover(path: impl AsRef<Path>) -> io::Result<Vec<(String, Vec<u8>)>> {
        let file = match File::open(path.as_ref()) {
            Ok(f) => f,
            Err(_) => return Ok(vec![]),
        };
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();

        loop {
            let mut buf4 = [0u8; 4];

            match reader.read_exact(&mut buf4) {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
            let key_len = u32::from_le_bytes(buf4) as usize;

            let mut key_buf = vec![0u8; key_len];
            reader.read_exact(&mut key_buf)?;
            let key = String::from_utf8(key_buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            reader.read_exact(&mut buf4)?;
            let value_len = u32::from_le_bytes(buf4) as usize;

            let mut value = vec![0u8; value_len];
            reader.read_exact(&mut value)?;

            reader.read_exact(&mut buf4)?;
            let stored_crc = u32::from_le_bytes(buf4);

            let mut hasher = Hasher::new();
            hasher.update(key.as_bytes());
            hasher.update(&value);
            if hasher.finalize() != stored_crc {
                eprintln!("[WAL] CRC mismatch for key '{}' — skipping corrupted entry", key);
                continue;
            }

            entries.push((key, value));
        }

        Ok(entries)
    }

    // Called after a successful MemTable flush to SSTable — old WAL data is no longer needed
    pub fn clear(&mut self) -> io::Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.path)?;
        self.writer = BufWriter::new(file);
        Ok(())
    }
}
