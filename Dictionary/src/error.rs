use std::fmt;

pub enum DictError {
    Io(std::io::Error),
    InvalidFormat(String),
    WordNotFound(String),
    MalformedInput { line_num: usize, content: String },
}

impl fmt::Display for DictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DictError::Io(e) => write!(f, "IO error: {e}"),
            DictError::InvalidFormat(msg) => write!(f, "Invalid dictionary format: {msg}"),
            DictError::WordNotFound(w) => write!(f, "Word '{w}' not found"),
            DictError::MalformedInput { line_num, content } => {
                write!(f, "Malformed input at line {line_num}: {content:?}")
            }
        }
    }
}

impl fmt::Debug for DictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<std::io::Error> for DictError {
    fn from(e: std::io::Error) -> Self {
        DictError::Io(e)
    }
}

impl std::error::Error for DictError {}
