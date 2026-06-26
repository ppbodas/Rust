use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};

use crate::error::DictError;
use crate::format::FIELD_SEP;

struct CountingWriter<W: Write> {
    inner: W,
    pub offset: u64,
}

impl<W: Write> CountingWriter<W> {
    fn new(inner: W) -> Self {
        Self { inner, offset: 0 }
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.offset += n as u64;
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

fn parse_input_line(line: &str, line_num: usize) -> Result<Option<(String, String)>, DictError> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(None);
    }
    match trimmed.split_once(": ") {
        Some((word, meaning)) => Ok(Some((word.trim().to_lowercase(), meaning.trim().to_string()))),
        None => Err(DictError::MalformedInput {
            line_num,
            content: trimmed.to_string(),
        }),
    }
}

pub fn build(input_path: &str, output_path: &str) -> Result<usize, DictError> {
    let input = File::open(input_path)?;
    let reader = BufReader::new(input);

    let mut entries: Vec<(String, String)> = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        match parse_input_line(&line, i + 1) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {}
            Err(e) => eprintln!("Warning: {e}"),
        }
    }

    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    entries.dedup_by(|(a_word, _), (b_word, _)| a_word == b_word);

    let output = File::create(output_path)?;
    let mut writer = CountingWriter::new(BufWriter::new(output));
    let mut offset_table: Vec<(String, u64)> = Vec::with_capacity(entries.len());

    for (word, meaning) in &entries {
        let data_offset = writer.offset;
        write!(writer, "{word}{FIELD_SEP}{meaning}\n")?;
        offset_table.push((word.clone(), data_offset));
    }

    let table_start = writer.offset;

    for (word, data_offset) in &offset_table {
        write!(writer, "{word}{FIELD_SEP}{data_offset}\n")?;
    }

    write!(writer, "{table_start}\n")?;
    writer.flush()?;

    Ok(entries.len())
}
