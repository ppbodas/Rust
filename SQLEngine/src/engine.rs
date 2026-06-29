use crate::btree::BTree;
use crate::pager::Pager;
use crate::record::User;

/// Public façade over the B+ tree and file pager.
///
/// `Engine` owns the [`Pager`] (and therefore the open file handle). All operations
/// are forwarded to a short-lived [`BTree`] that borrows the pager for a single call.
/// The `verbose` flag on each method controls whether step-by-step traversal logs
/// are printed to stdout.
pub struct Engine {
    pager: Pager,
}

impl Engine {
    /// Open or create the database file at `path` and return a ready-to-use `Engine`.
    pub fn open(path: &str) -> std::io::Result<Self> {
        let pager = Pager::open(path)?;
        Ok(Engine { pager })
    }

    /// Insert `user` without checking for duplicate ids. Used by the seed command for speed.
    /// For normal inserts from the REPL, use [`insert_unique`](Self::insert_unique) instead.
    pub fn insert(&mut self, user: &User) -> std::io::Result<()> {
        BTree::new(&mut self.pager).insert(user)
    }

    /// Insert `user`, rejecting the operation if a record with the same id already exists.
    ///
    /// Returns `Ok(Ok(()))` on success, `Ok(Err(msg))` if the id is a duplicate.
    /// Set `verbose = true` to print traversal and slot-shift logs to stdout.
    pub fn insert_unique(&mut self, user: &User, verbose: bool) -> std::io::Result<Result<(), String>> {
        if verbose {
            BTree::new_verbose(&mut self.pager).insert_unique(user)
        } else {
            BTree::new(&mut self.pager).insert_unique(user)
        }
    }

    /// Overwrite the non-key fields (name, age, phone, address) of an existing record in-place.
    ///
    /// Returns `true` if the record was found and updated, `false` if no record with that id exists.
    /// Set `verbose = true` to print a field-by-field old-vs-new diff and the exact byte offset written.
    pub fn update(&mut self, user: &User, verbose: bool) -> std::io::Result<bool> {
        if verbose {
            BTree::new_verbose(&mut self.pager).update(user)
        } else {
            BTree::new(&mut self.pager).update(user)
        }
    }

    /// Delete the record with the given `id`. Compacts the leaf page by shifting
    /// subsequent slots left.
    ///
    /// Returns `true` if deleted, `false` if no record with that id exists.
    /// Set `verbose = true` to print before/after slot layouts and shift details.
    pub fn delete(&mut self, id: u64, verbose: bool) -> std::io::Result<bool> {
        if verbose {
            BTree::new_verbose(&mut self.pager).delete(id)
        } else {
            BTree::new(&mut self.pager).delete(id)
        }
    }

    /// Point lookup: find the record with the given `id` using B+ tree traversal.
    ///
    /// Returns `Some(User)` if found, `None` otherwise.
    /// Set `verbose = true` to print each internal/leaf page visited during traversal.
    pub fn find_by_id(&mut self, id: u64, verbose: bool) -> std::io::Result<Option<User>> {
        if verbose {
            BTree::new_verbose(&mut self.pager).find(id)
        } else {
            BTree::new(&mut self.pager).find(id)
        }
    }

    /// Range scan: return all records where `start_id <= id <= end_id`.
    ///
    /// Finds the start leaf via B+ tree traversal, then follows the leaf linked list
    /// until all matching records are collected or the end of the range is exceeded.
    /// Set `verbose = true` to print traversal path and per-leaf collection stats.
    pub fn range_query(&mut self, start_id: u64, end_id: u64, verbose: bool) -> std::io::Result<Vec<User>> {
        if verbose {
            BTree::new_verbose(&mut self.pager).range(start_id, end_id)
        } else {
            BTree::new(&mut self.pager).range(start_id, end_id)
        }
    }

    /// Flush the current `root_page_id` and `num_pages` to the metadata page and close
    /// the database. Must be called before the process exits to keep the file consistent.
    pub fn close(&mut self) -> std::io::Result<()> {
        self.pager.flush_meta()
    }
}
