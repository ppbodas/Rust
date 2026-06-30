use std::collections::BTreeMap;

pub const TOMBSTONE: &[u8] = b"__TOMBSTONE__";

pub struct MemTable {
    data: BTreeMap<String, Vec<u8>>,
    capacity: usize,
}

impl MemTable {
    pub fn new(capacity: usize) -> Self {
        MemTable {
            data: BTreeMap::new(),
            capacity,
        }
    }

    pub fn put(&mut self, key: String, value: Vec<u8>) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&Vec<u8>> {
        self.data.get(key)
    }

    pub fn delete(&mut self, key: String) {
        self.data.insert(key, TOMBSTONE.to_vec());
    }

    pub fn is_full(&self) -> bool {
        self.data.len() >= self.capacity
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // BTreeMap iterates in sorted key order — required for SSTable index correctness
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Vec<u8>)> {
        self.data.iter()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}
