use std::path::PathBuf;

pub struct DbConfig {
    pub memtable_capacity: usize,
    pub data_dir: PathBuf,
}

impl Default for DbConfig {
    fn default() -> Self {
        DbConfig {
            memtable_capacity: 20,
            data_dir: PathBuf::from("./data"),
        }
    }
}
