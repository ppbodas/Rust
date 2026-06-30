use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};

use crate::error::DictError;
use crate::format::FIELD_SEP;

pub(crate) fn read_entry_at(file: &mut File, data_offset: u64) -> Result<String, DictError> {
    file.seek(SeekFrom::Start(data_offset))?;
    let mut reader = BufReader::new(&*file);
    let mut line = String::new();
    reader.read_line(&mut line)?;

    let line = line.trim_end_matches('\n').trim_end_matches('\r');
    let meaning = line
        .splitn(2, FIELD_SEP)
        .nth(1)
        .ok_or_else(|| DictError::InvalidFormat(format!("malformed data entry at offset {data_offset}")))?;

    Ok(meaning.to_string())
}
