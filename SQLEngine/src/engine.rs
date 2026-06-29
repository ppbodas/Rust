use crate::btree::BTree;
use crate::pager::Pager;
use crate::record::User;

pub struct Engine {
    pager: Pager,
}

impl Engine {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let pager = Pager::open(path)?;
        Ok(Engine { pager })
    }

    pub fn insert(&mut self, user: &User) -> std::io::Result<()> {
        BTree::new(&mut self.pager).insert(user)
    }

    /// Delete a record by id. Returns false if not found.
    pub fn delete(&mut self, id: u64, verbose: bool) -> std::io::Result<bool> {
        if verbose {
            BTree::new_verbose(&mut self.pager).delete(id)
        } else {
            BTree::new(&mut self.pager).delete(id)
        }
    }

    /// Insert rejecting duplicates. Returns Err string if id already exists.
    pub fn insert_unique(&mut self, user: &User, verbose: bool) -> std::io::Result<Result<(), String>> {
        if verbose {
            BTree::new_verbose(&mut self.pager).insert_unique(user)
        } else {
            BTree::new(&mut self.pager).insert_unique(user)
        }
    }

    /// Update non-key fields in-place. Returns false if id not found.
    pub fn update(&mut self, user: &User, verbose: bool) -> std::io::Result<bool> {
        if verbose {
            BTree::new_verbose(&mut self.pager).update(user)
        } else {
            BTree::new(&mut self.pager).update(user)
        }
    }

    pub fn find_by_id(&mut self, id: u64, verbose: bool) -> std::io::Result<Option<User>> {
        if verbose {
            BTree::new_verbose(&mut self.pager).find(id)
        } else {
            BTree::new(&mut self.pager).find(id)
        }
    }

    pub fn range_query(&mut self, start_id: u64, end_id: u64, verbose: bool) -> std::io::Result<Vec<User>> {
        if verbose {
            BTree::new_verbose(&mut self.pager).range(start_id, end_id)
        } else {
            BTree::new(&mut self.pager).range(start_id, end_id)
        }
    }

    pub fn close(&mut self) -> std::io::Result<()> {
        self.pager.flush_meta()
    }
}
