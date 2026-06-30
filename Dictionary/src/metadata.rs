use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use crate::error::DictError;
use crate::format::{FIELD_SEP, FOOTER_READ_WINDOW};
use crate::lookup;

pub struct DictionaryMetadata {
    pub file: File,
    pub table: HashMap<String, u64>,
}

impl DictionaryMetadata {
    pub fn open(path: &str) -> Result<Self, DictError> {
        let mut file = File::open(path)?;
        let table_start = read_footer(&mut file)?;
        let table = load_offset_table(&mut file, table_start)?;
        Ok(Self { file, table })
    }

    pub fn lookup(&mut self, word: &str) -> Result<String, DictError> {
        let normalized = word.to_lowercase();
        let data_offset = self.table
            .get(&normalized)
            .copied()
            .ok_or_else(|| DictError::WordNotFound(word.to_string()))?;
        lookup::read_entry_at(&mut self.file, data_offset)
    }
}

fn read_footer(file: &mut File) -> Result<u64, DictError> {
    let file_size = file.seek(SeekFrom::End(0))?;
    eprintln!("[read_footer] file size: {file_size} bytes");
    if file_size == 0 {
        return Err(DictError::InvalidFormat("empty file".into()));
    }

    let read_start = file_size.saturating_sub(FOOTER_READ_WINDOW);
    eprintln!("[read_footer] reading tail from byte {read_start}");
    file.seek(SeekFrom::Start(read_start))?;
    let mut tail = String::new();
    BufReader::new(file).read_to_string(&mut tail)?;

    let last_line = tail.lines().last()
        .ok_or_else(|| DictError::InvalidFormat("file has no content".into()))?;
    eprintln!("[read_footer] footer line: {last_line:?}");

    let offset = last_line
        .parse::<u64>()
        .map_err(|_| DictError::InvalidFormat(format!("footer is not a valid offset: {last_line:?}")))?;
    eprintln!("[read_footer] offset table starts at byte {offset}");
    Ok(offset)
}

fn load_offset_table(file: &mut File, table_start: u64) -> Result<HashMap<String, u64>, DictError> {
    file.seek(SeekFrom::Start(table_start))?;
    let reader = BufReader::new(&*file);

    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<_, _>>()?;

    let table_lines = if lines.is_empty() { &lines[..] } else { &lines[..lines.len() - 1] };

    let mut map = HashMap::with_capacity(table_lines.len());
    for line in table_lines {
        let line = line.trim_end_matches('\r');
        let (word, offset_str) = line
            .splitn(2, FIELD_SEP)
            .collect::<Vec<_>>()
            .split_first()
            .and_then(|(w, rest)| rest.first().map(|o| (*w, *o)))
            .ok_or_else(|| DictError::InvalidFormat(format!("malformed offset entry: {line:?}")))?;

        let offset = offset_str
            .parse::<u64>()
            .map_err(|_| DictError::InvalidFormat(format!("invalid offset value: {offset_str:?}")))?;

        map.insert(word.to_string(), offset);
    }

    Ok(map)
}
