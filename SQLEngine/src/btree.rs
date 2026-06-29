use crate::page::{Page, PageHeader, PageType};
use crate::pager::Pager;
use crate::record::User;

pub struct BTree<'a> {
    pager: &'a mut Pager,
    verbose: bool,
}

/// Returned when a child page splits — the parent must insert this key + new page.
struct SplitResult {
    pushed_up_key: u64,
    new_page_id: u32,
}

impl<'a> BTree<'a> {
    pub fn new(pager: &'a mut Pager) -> Self {
        BTree { pager, verbose: false }
    }

    pub fn new_verbose(pager: &'a mut Pager) -> Self {
        BTree { pager, verbose: true }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    pub fn insert(&mut self, user: &User) -> std::io::Result<()> {
        let root_id = self.pager.root_page_id;
        let split = self.insert_recursive(root_id, user)?;

        if let Some(s) = split {
            // Root split — create a new root internal node
            let new_root_id = self.pager.allocate_page()?;
            let mut new_root = Page::new();
            new_root.write_header(&PageHeader::new(PageType::Internal));
            new_root.internal_set_leftmost_child(root_id);
            new_root.internal_insert_sorted(s.pushed_up_key, s.new_page_id);
            self.pager.write_page(new_root_id, &new_root)?;
            self.pager.root_page_id = new_root_id;
            self.pager.flush_meta()?;
        }

        Ok(())
    }

    /// Delete a record by id. Returns false if not found.
    pub fn delete(&mut self, id: u64) -> std::io::Result<bool> {
        let root_id = self.pager.root_page_id;

        if self.verbose {
            println!("  [delete] searching for id={}", id);
        }

        // Traverse to the leaf page
        let leaf_id = self.find_leaf_page(root_id, id)?;
        if leaf_id == u32::MAX {
            return Ok(false);
        }

        let mut page = self.pager.read_page(leaf_id)?;
        let hdr = page.header();
        let n = hdr.num_slots as usize;

        if self.verbose {
            // Show full leaf state before delete
            let first = if n > 0 { page.leaf_read(0).id } else { 0 };
            let last  = if n > 0 { page.leaf_read(n as u32 - 1).id } else { 0 };
            println!("  [delete] arrived at page {} [LEAF, {} records, ids {}..{}]",
                leaf_id, n, first, last);
            println!("  [delete] page {} slot layout BEFORE delete:", leaf_id);
            for i in 0..n {
                let rec  = page.leaf_read(i as u32);
                let off  = crate::config::PAGE_HEADER_SIZE + i * crate::config::RECORD_SIZE;
                let mark = if rec.id == id { " ← DELETE THIS" } else { "" };
                println!("           slot[{:2}]  offset={:<5}  id={}{}", i, off, rec.id, mark);
            }
        }

        // Perform the delete — returns the slot index that was removed
        let result = page.leaf_delete(id);

        match result {
            None => {
                if self.verbose {
                    println!("  [delete] id={} not found in page {}", id, leaf_id);
                }
                Ok(false)
            }
            Some(deleted_slot) => {
                if self.verbose {
                    let new_n = page.header().num_slots as usize;
                    println!("  [delete] removed slot[{}] at offset {} ({}+{}×{})",
                        deleted_slot,
                        crate::config::PAGE_HEADER_SIZE + deleted_slot * crate::config::RECORD_SIZE,
                        crate::config::PAGE_HEADER_SIZE, deleted_slot, crate::config::RECORD_SIZE);

                    if deleted_slot < new_n {
                        println!("  [delete] shifting slots {}..{} one position LEFT:",
                            deleted_slot + 1, new_n);
                        for i in deleted_slot..new_n {
                            let rec      = page.leaf_read(i as u32);
                            let old_off  = crate::config::PAGE_HEADER_SIZE + (i + 1) * crate::config::RECORD_SIZE;
                            let new_off  = crate::config::PAGE_HEADER_SIZE + i * crate::config::RECORD_SIZE;
                            println!("           id={:<6}  offset {} → {}  (slot[{}] → slot[{}])",
                                rec.id, old_off, new_off, i + 1, i);
                        }
                    }

                    println!("  [delete] slot[{}] at offset {} zeroed (free space)",
                        new_n,
                        crate::config::PAGE_HEADER_SIZE + new_n * crate::config::RECORD_SIZE);
                    println!("  [delete] num_slots: {} → {}", new_n + 1, new_n);

                    // Show full page after delete
                    println!("  [delete] page {} slot layout AFTER delete:", leaf_id);
                    for i in 0..new_n {
                        let rec = page.leaf_read(i as u32);
                        let off = crate::config::PAGE_HEADER_SIZE + i * crate::config::RECORD_SIZE;
                        println!("           slot[{:2}]  offset={:<5}  id={}", i, off, rec.id);
                    }
                    let empty_off = crate::config::PAGE_HEADER_SIZE + new_n * crate::config::RECORD_SIZE;
                    println!("           slot[{:2}]  offset={:<5}  [empty / zeroed]", new_n, empty_off);

                    println!("  [delete] writing page {} to disk", leaf_id);
                }

                self.pager.write_page(leaf_id, &page)?;
                Ok(true)
            }
        }
    }

    /// Insert, rejecting duplicate keys. Returns Err if id already exists.
    pub fn insert_unique(&mut self, user: &User) -> std::io::Result<Result<(), String>> {
        let root_id = self.pager.root_page_id;
        if self.find_recursive(root_id, user.id)?.is_some() {
            return Ok(Err(format!("id={} already exists. Use UPDATE to modify it.", user.id)));
        }
        self.insert(user)?;
        Ok(Ok(()))
    }

    /// Update non-key fields of an existing record in-place.
    /// Returns false if no record with that id exists.
    pub fn update(&mut self, user: &User) -> std::io::Result<bool> {
        let root_id = self.pager.root_page_id;
        let leaf_id = self.find_leaf_page(root_id, user.id)?;
        if leaf_id == u32::MAX {
            return Ok(false);
        }
        let mut page = self.pager.read_page(leaf_id)?;
        if page.leaf_update(user) {
            if self.verbose {
                println!("  [update] overwrote record in-place at page {}", leaf_id);
            }
            self.pager.write_page(leaf_id, &page)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Point lookup by id.
    pub fn find(&mut self, id: u64) -> std::io::Result<Option<User>> {
        let root_id = self.pager.root_page_id;
        if self.verbose {
            println!("  [traversal] starting at root page {}", root_id);
        }
        self.find_recursive(root_id, id)
    }

    /// Range query [start_id, end_id] inclusive.
    pub fn range(&mut self, start_id: u64, end_id: u64) -> std::io::Result<Vec<User>> {
        let root_id = self.pager.root_page_id;
        if self.verbose {
            println!("  [traversal] locating start leaf for id={}", start_id);
        }
        let leaf_id = self.find_leaf_page(root_id, start_id)?;

        let mut results = Vec::new();
        let mut current_leaf_id = leaf_id;
        let mut leaf_num = 0usize;

        loop {
            if current_leaf_id == u32::MAX {
                break;
            }
            let page = self.pager.read_page(current_leaf_id)?;
            let hdr = page.header();
            let mut done = false;
            let mut collected = 0usize;

            for i in 0..hdr.num_slots {
                let rec = page.leaf_read(i);
                if rec.id > end_id {
                    done = true;
                    break;
                }
                if rec.id >= start_id {
                    results.push(rec);
                    collected += 1;
                }
            }

            if self.verbose {
                let first = if hdr.num_slots > 0 { page.leaf_read(0).id } else { 0 };
                let last  = if hdr.num_slots > 0 { page.leaf_read(hdr.num_slots - 1).id } else { 0 };
                let next  = hdr.next_page_id;
                let next_str = if next == u32::MAX { "none (end of chain)".to_string() } else { format!("page {}", next) };
                println!(
                    "  [leaf {}] page {:4} │ {:3} records (id {}..{}) │ collected {} │ next → {}{}",
                    leaf_num, current_leaf_id, hdr.num_slots, first, last,
                    collected, next_str,
                    if done { " │ RANGE END" } else { "" }
                );
            }

            leaf_num += 1;
            if done { break; }
            current_leaf_id = hdr.next_page_id;
        }

        Ok(results)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn insert_recursive(&mut self, page_id: u32, user: &User) -> std::io::Result<Option<SplitResult>> {
        let page = self.pager.read_page(page_id)?;
        let hdr = page.header();

        match hdr.page_type {
            PageType::Leaf => self.leaf_insert(page_id, page, user),
            PageType::Internal => self.internal_insert(page_id, page, user),
            PageType::Meta => unreachable!("traversed into meta page"),
        }
    }

    fn leaf_insert(&mut self, page_id: u32, mut page: Page, user: &User) -> std::io::Result<Option<SplitResult>> {
        if !page.leaf_is_full() {
            page.leaf_insert_sorted(user);
            self.pager.write_page(page_id, &page)?;
            return Ok(None);
        }

        // Leaf is full — split
        let new_leaf_id = self.pager.allocate_page()?;

        // Collect all records + new one, sorted
        let n = page.header().num_slots as usize;
        let mut records: Vec<User> = (0..n as u32).map(|i| page.leaf_read(i)).collect();
        let pos = records.partition_point(|r| r.id < user.id);
        records.insert(pos, user.clone());

        let mid = records.len() / 2;
        let pushed_up_key = records[mid].id;

        // Left leaf keeps [0..mid)
        let mut left = Page::new();
        let old_next = page.header().next_page_id;
        let mut left_hdr = PageHeader::new(PageType::Leaf);
        left_hdr.next_page_id = new_leaf_id;
        left.write_header(&left_hdr);
        for (i, rec) in records[..mid].iter().enumerate() {
            left.leaf_write(i as u32, rec);
        }
        {
            let mut h = left.header();
            h.num_slots = mid as u32;
            left.write_header(&h);
        }

        // Right leaf keeps [mid..]
        let mut right = Page::new();
        let mut right_hdr = PageHeader::new(PageType::Leaf);
        right_hdr.next_page_id = old_next;
        right.write_header(&right_hdr);
        for (i, rec) in records[mid..].iter().enumerate() {
            right.leaf_write(i as u32, rec);
        }
        {
            let mut h = right.header();
            h.num_slots = (records.len() - mid) as u32;
            right.write_header(&h);
        }

        self.pager.write_page(page_id, &left)?;
        self.pager.write_page(new_leaf_id, &right)?;

        Ok(Some(SplitResult { pushed_up_key, new_page_id: new_leaf_id }))
    }

    fn internal_insert(&mut self, page_id: u32, page: Page, user: &User) -> std::io::Result<Option<SplitResult>> {
        let child_id = page.internal_find_child(user.id);
        let split = self.insert_recursive(child_id, user)?;

        let Some(s) = split else {
            return Ok(None);
        };

        // Re-read page (child writes may have changed pager state but not this page)
        let mut page = self.pager.read_page(page_id)?;

        if !page.internal_is_full() {
            page.internal_insert_sorted(s.pushed_up_key, s.new_page_id);
            self.pager.write_page(page_id, &page)?;
            return Ok(None);
        }

        // Internal node is full — split it
        let new_internal_id = self.pager.allocate_page()?;

        let n = page.header().num_slots as usize;
        let mut entries: Vec<(u64, u32)> = (0..n as u32).map(|i| page.internal_entry(i)).collect();
        let leftmost = page.internal_leftmost_child();

        // Insert the new separator
        let pos = entries.partition_point(|(k, _)| *k < s.pushed_up_key);
        entries.insert(pos, (s.pushed_up_key, s.new_page_id));

        let mid = entries.len() / 2;
        let mid_key = entries[mid].0;

        // Left internal: leftmost child + entries[0..mid)
        let mut left = Page::new();
        left.write_header(&PageHeader::new(PageType::Internal));
        left.internal_set_leftmost_child(leftmost);
        for (i, &(k, c)) in entries[..mid].iter().enumerate() {
            left.internal_set_entry(i as u32, k, c);
        }
        {
            let mut h = left.header();
            h.num_slots = mid as u32;
            left.write_header(&h);
        }

        // Right internal: entries[mid].1 becomes leftmost child, entries[mid+1..] are keys
        let right_leftmost = entries[mid].1;
        let mut right = Page::new();
        right.write_header(&PageHeader::new(PageType::Internal));
        right.internal_set_leftmost_child(right_leftmost);
        for (i, &(k, c)) in entries[mid + 1..].iter().enumerate() {
            right.internal_set_entry(i as u32, k, c);
        }
        {
            let mut h = right.header();
            h.num_slots = (entries.len() - mid - 1) as u32;
            right.write_header(&h);
        }

        self.pager.write_page(page_id, &left)?;
        self.pager.write_page(new_internal_id, &right)?;

        Ok(Some(SplitResult { pushed_up_key: mid_key, new_page_id: new_internal_id }))
    }

    fn find_recursive(&mut self, page_id: u32, id: u64) -> std::io::Result<Option<User>> {
        let page = self.pager.read_page(page_id)?;
        let hdr = page.header();
        match hdr.page_type {
            PageType::Leaf => {
                if self.verbose {
                    let first = if hdr.num_slots > 0 { page.leaf_read(0).id } else { 0 };
                    let last  = if hdr.num_slots > 0 { page.leaf_read(hdr.num_slots - 1).id } else { 0 };
                    println!(
                        "  [traversal] page {:4} [LEAF,     {:3} records, ids {}..{}] → binary search for id={}",
                        page_id, hdr.num_slots, first, last, id
                    );
                }
                let result = page.leaf_search(id);
                if self.verbose {
                    match &result {
                        Some(_) => println!("  [traversal] → FOUND at page {}", page_id),
                        None    => println!("  [traversal] → NOT FOUND"),
                    }
                }
                Ok(result)
            }
            PageType::Internal => {
                let (child_id, reason) = page.internal_find_child_logged(id);
                if self.verbose {
                    println!(
                        "  [traversal] page {:4} [INTERNAL, {:3} keys  ] → {}",
                        page_id, hdr.num_slots, reason
                    );
                }
                self.find_recursive(child_id, id)
            }
            PageType::Meta => Ok(None),
        }
    }

    fn find_leaf_page(&mut self, page_id: u32, id: u64) -> std::io::Result<u32> {
        let page = self.pager.read_page(page_id)?;
        let hdr = page.header();
        match hdr.page_type {
            PageType::Leaf => {
                if self.verbose {
                    let first = if hdr.num_slots > 0 { page.leaf_read(0).id } else { 0 };
                    let last  = if hdr.num_slots > 0 { page.leaf_read(hdr.num_slots - 1).id } else { 0 };
                    println!(
                        "  [traversal] page {:4} [LEAF,     {:3} records, ids {}..{}] → start scanning here",
                        page_id, hdr.num_slots, first, last
                    );
                }
                Ok(page_id)
            }
            PageType::Internal => {
                let (child_id, reason) = page.internal_find_child_logged(id);
                if self.verbose {
                    println!(
                        "  [traversal] page {:4} [INTERNAL, {:3} keys  ] → {}",
                        page_id, hdr.num_slots, reason
                    );
                }
                self.find_leaf_page(child_id, id)
            }
            PageType::Meta => Ok(u32::MAX),
        }
    }
}
