use crate::page::{Page, PageHeader, PageType};
use crate::pager::Pager;
use crate::record::User;

/// B+ tree over the page file managed by [`Pager`].
///
/// `BTree` is created per-operation — it borrows the pager for the duration of one
/// call and is then dropped. The `verbose` flag causes every method to print
/// step-by-step logs of page reads, routing decisions, slot shifts, and splits.
pub struct BTree<'a> {
    pager: &'a mut Pager,
    verbose: bool,
}

/// Returned by a recursive insert when a node split occurs.
/// The parent must absorb `pushed_up_key` as a new separator pointing to `new_page_id`.
struct SplitResult {
    pushed_up_key: u64,
    new_page_id: u32,
}

impl<'a> BTree<'a> {
    /// Create a BTree that operates silently (no stdout logs).
    pub fn new(pager: &'a mut Pager) -> Self {
        BTree { pager, verbose: false }
    }

    /// Create a BTree that prints step-by-step traversal and mutation logs to stdout.
    pub fn new_verbose(pager: &'a mut Pager) -> Self {
        BTree { pager, verbose: true }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Insert `user` into the B+ tree without checking for duplicates.
    ///
    /// Used by the seed command for bulk-load speed. If the root leaf splits,
    /// a new root internal node is created and `pager.root_page_id` is updated.
    pub fn insert(&mut self, user: &User) -> std::io::Result<()> {
        let root_id = self.pager.root_page_id;

        if self.verbose {
            println!("  [insert] starting at root page {}", root_id);
        }

        let split = self.insert_recursive(root_id, user)?;

        if let Some(s) = split {
            // The old root split — promote a new root internal node one level up
            let new_root_id = self.pager.allocate_page()?;
            let mut new_root = Page::new();
            new_root.write_header(&PageHeader::new(PageType::Internal));
            new_root.internal_set_leftmost_child(root_id);        // old root becomes left child
            new_root.internal_insert_sorted(s.pushed_up_key, s.new_page_id);
            self.pager.write_page(new_root_id, &new_root)?;
            self.pager.root_page_id = new_root_id;
            self.pager.flush_meta()?; // durably record the new root

            if self.verbose {
                println!("  [insert] ROOT SPLIT — new root page {} created",  new_root_id);
                println!("           left subtree  = old root page {}", root_id);
                println!("           right subtree = page {}", s.new_page_id);
                println!("           separator key = {}", s.pushed_up_key);
                println!("           tree height increased by 1");
            }
        }

        Ok(())
    }

    /// Delete the record with `id` from the tree.
    ///
    /// Traverses to the correct leaf, then removes the slot and shifts subsequent
    /// slots left to compact the page. No underflow merging — partially full pages
    /// remain as-is (a real engine would merge or redistribute; we skip that here).
    ///
    /// Returns `true` if deleted, `false` if no record with that id was found.
    pub fn delete(&mut self, id: u64) -> std::io::Result<bool> {
        let root_id = self.pager.root_page_id;

        if self.verbose {
            println!("  [delete] searching for id={}", id);
        }

        let leaf_id = self.find_leaf_page(root_id, id)?;
        if leaf_id == u32::MAX {
            return Ok(false);
        }

        let mut page = self.pager.read_page(leaf_id)?;
        let hdr = page.header();
        let n = hdr.num_slots as usize;

        if self.verbose {
            let first = if n > 0 { page.leaf_read(0).id } else { 0 };
            let last  = if n > 0 { page.leaf_read(n as u32 - 1).id } else { 0 };
            println!("  [delete] arrived at page {} [LEAF, {} records, ids {}..{}]",
                leaf_id, n, first, last);

            // Print the full slot map so the user can see exactly which slot is removed
            println!("  [delete] page {} slot layout BEFORE delete:", leaf_id);
            for i in 0..n {
                let rec  = page.leaf_read(i as u32);
                let off  = crate::config::PAGE_HEADER_SIZE + i * crate::config::RECORD_SIZE;
                let mark = if rec.id == id { " ← DELETE THIS" } else { "" };
                println!("           slot[{:2}]  offset={:<5}  id={}{}", i, off, rec.id, mark);
            }
        }

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

                    // Print the full slot map after the delete for comparison
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

    /// Insert `user`, rejecting the operation if a record with the same id already exists.
    ///
    /// Does a point lookup first (`find`), then delegates to `insert` only if the id is new.
    /// Returns `Ok(Err(msg))` if the id is a duplicate so the REPL can print the message.
    pub fn insert_unique(&mut self, user: &User) -> std::io::Result<Result<(), String>> {
        let root_id = self.pager.root_page_id;
        if self.find_recursive(root_id, user.id)?.is_some() {
            return Ok(Err(format!("id={} already exists. Use UPDATE to modify it.", user.id)));
        }
        self.insert(user)?;
        Ok(Ok(()))
    }

    /// Overwrite the non-key fields (name, age, phone, address) of an existing record.
    ///
    /// Traverses to the correct leaf and writes 128 bytes at the exact slot offset.
    /// No slots shift — the id (which determines sort order) is unchanged.
    ///
    /// Returns `true` if found and updated, `false` if no record with that id exists.
    pub fn update(&mut self, user: &User) -> std::io::Result<bool> {
        let root_id = self.pager.root_page_id;

        if self.verbose {
            println!("  [update] searching for id={}", user.id);
        }

        let leaf_id = self.find_leaf_page(root_id, user.id)?;
        if leaf_id == u32::MAX {
            return Ok(false);
        }

        let mut page = self.pager.read_page(leaf_id)?;
        let n = page.header().num_slots as usize;

        if self.verbose {
            let first = if n > 0 { page.leaf_read(0).id } else { 0 };
            let last  = if n > 0 { page.leaf_read(n as u32 - 1).id } else { 0 };
            println!("  [update] arrived at page {} [LEAF, {} records, ids {}..{}]",
                leaf_id, n, first, last);
        }

        match page.leaf_update(user) {
            None => Ok(false),
            Some((slot, old)) => {
                if self.verbose {
                    let off = crate::config::PAGE_HEADER_SIZE + slot * crate::config::RECORD_SIZE;
                    println!("  [update] found id={} at slot[{}]  offset={} ({}+{}×{})",
                        user.id, slot, off,
                        crate::config::PAGE_HEADER_SIZE, slot, crate::config::RECORD_SIZE);
                    println!("  [update] overwriting in-place (no slot shift needed):");
                    println!("           field     OLD value            NEW value");
                    println!("           ─────────────────────────────────────────────────────");
                    println!("           id        {:<20} {} (unchanged — key field)", old.id, user.id);

                    // Only highlight fields that actually changed
                    if old.name != user.name {
                        println!("           name      {:<20} {}", old.name, user.name);
                    } else {
                        println!("           name      {:<20} (unchanged)", old.name);
                    }
                    if old.age != user.age {
                        println!("           age       {:<20} {}", old.age, user.age);
                    } else {
                        println!("           age       {:<20} (unchanged)", old.age);
                    }
                    if old.phone != user.phone {
                        println!("           phone     {:<20} {}", old.phone, user.phone);
                    } else {
                        println!("           phone     {:<20} (unchanged)", old.phone);
                    }
                    if old.address != user.address {
                        println!("           address   {:<20} {}", old.address, user.address);
                    } else {
                        println!("           address   {:<20} (unchanged)", old.address);
                    }

                    println!("  [update] 128 bytes written at offset {}..{} in page {}",
                        off, off + crate::config::RECORD_SIZE, leaf_id);
                    println!("  [update] no other slots moved — all offsets unchanged");
                    println!("  [update] writing page {} to disk", leaf_id);
                }
                self.pager.write_page(leaf_id, &page)?;
                Ok(true)
            }
        }
    }

    /// Point lookup: find and return the record with `id`, or `None` if not found.
    /// Traverses internal nodes top-down, then binary-searches the leaf.
    pub fn find(&mut self, id: u64) -> std::io::Result<Option<User>> {
        let root_id = self.pager.root_page_id;
        if self.verbose {
            println!("  [traversal] starting at root page {}", root_id);
        }
        self.find_recursive(root_id, id)
    }

    /// Range scan: return all records where `start_id <= id <= end_id`, in ascending order.
    ///
    /// Locates the first qualifying leaf via tree traversal, then follows the leaf linked
    /// list (via `next_page_id`) until a record's id exceeds `end_id` or the chain ends.
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
                    done = true; // no more records can qualify in this or later leaves
                    break;
                }
                if rec.id >= start_id {
                    results.push(rec);
                    collected += 1;
                }
            }

            if self.verbose {
                let first    = if hdr.num_slots > 0 { page.leaf_read(0).id } else { 0 };
                let last     = if hdr.num_slots > 0 { page.leaf_read(hdr.num_slots - 1).id } else { 0 };
                let next     = hdr.next_page_id;
                let next_str = if next == u32::MAX {
                    "none (end of chain)".to_string()
                } else {
                    format!("page {}", next)
                };
                println!(
                    "  [leaf {}] page {:4} │ {:3} records (id {}..{}) │ collected {} │ next → {}{}",
                    leaf_num, current_leaf_id, hdr.num_slots, first, last,
                    collected, next_str,
                    if done { " │ RANGE END" } else { "" }
                );
            }

            leaf_num += 1;
            if done { break; }
            current_leaf_id = hdr.next_page_id; // follow the leaf chain
        }

        Ok(results)
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Recursively descend the tree from `page_id` to insert `user`.
    ///
    /// On reaching a leaf, delegates to `leaf_insert`. On an internal node, routes to the
    /// correct child and, if that child split, absorbs the pushed-up key (`internal_insert`).
    /// Returns `Some(SplitResult)` if this page also needs to split, else `None`.
    fn insert_recursive(&mut self, page_id: u32, user: &User) -> std::io::Result<Option<SplitResult>> {
        let page = self.pager.read_page(page_id)?;
        let hdr = page.header();

        if self.verbose {
            match hdr.page_type {
                PageType::Internal => {
                    let (_, reason) = page.internal_find_child_logged(user.id);
                    println!("  [insert] page {:4} [INTERNAL, {:3} keys  ] → {}",
                        page_id, hdr.num_slots, reason);
                }
                PageType::Leaf => {
                    let n = hdr.num_slots as usize;
                    let first = if n > 0 { page.leaf_read(0).id } else { 0 };
                    let last  = if n > 0 { page.leaf_read(n as u32 - 1).id } else { 0 };
                    println!("  [insert] page {:4} [LEAF,     {:3} records, ids {}..{}] → inserting here",
                        page_id, n, first, last);
                }
                PageType::Meta => {}
            }
        }

        match hdr.page_type {
            PageType::Leaf     => self.leaf_insert(page_id, page, user),
            PageType::Internal => self.internal_insert(page_id, page, user),
            PageType::Meta     => unreachable!("traversed into meta page"),
        }
    }

    /// Insert `user` into the leaf at `page_id`.
    ///
    /// **No-split path**: there is room; find the sorted position, shift later records
    /// right, write the new record, update `num_slots`, and flush the page.
    ///
    /// **Split path**: the leaf is full (31 records). Collect all 31 existing records
    /// plus the new one (32 total), sort by id, split at midpoint 16. The left half
    /// stays on the original page; the right half goes to a freshly allocated page.
    /// The first id of the right half becomes the separator key pushed up to the parent.
    fn leaf_insert(&mut self, page_id: u32, mut page: Page, user: &User) -> std::io::Result<Option<SplitResult>> {
        if !page.leaf_is_full() {
            let n = page.header().num_slots as usize;

            // Compute insertion slot before mutating the page
            let insert_pos = (0..n).find(|&i| page.leaf_read(i as u32).id > user.id).unwrap_or(n);

            if self.verbose {
                let off = crate::config::PAGE_HEADER_SIZE + insert_pos * crate::config::RECORD_SIZE;
                println!("  [insert] page has room ({}/{}), no split needed", n, crate::config::LEAF_CAPACITY);
                println!("  [insert] inserting id={} at slot[{}]  offset={} ({}+{}×{})",
                    user.id, insert_pos, off,
                    crate::config::PAGE_HEADER_SIZE, insert_pos, crate::config::RECORD_SIZE);
                if insert_pos < n {
                    println!("  [insert] shifting slots {}..{} one position RIGHT to make room:",
                        insert_pos, n - 1);
                    for i in (insert_pos..n).rev() {
                        let rec     = page.leaf_read(i as u32);
                        let old_off = crate::config::PAGE_HEADER_SIZE + i * crate::config::RECORD_SIZE;
                        let new_off = crate::config::PAGE_HEADER_SIZE + (i + 1) * crate::config::RECORD_SIZE;
                        println!("           id={:<6}  offset {} → {}  (slot[{}] → slot[{}])",
                            rec.id, old_off, new_off, i, i + 1);
                    }
                }
            }

            page.leaf_insert_sorted(user);

            if self.verbose {
                let new_n = page.header().num_slots as usize;
                println!("  [insert] wrote id={} at slot[{}]  offset={}",
                    user.id, insert_pos,
                    crate::config::PAGE_HEADER_SIZE + insert_pos * crate::config::RECORD_SIZE);
                println!("  [insert] num_slots: {} → {}", n, new_n);
                println!("  [insert] writing page {} to disk", page_id);
            }

            self.pager.write_page(page_id, &page)?;
            return Ok(None);
        }

        // Leaf is full — must split
        let n = page.header().num_slots as usize;

        if self.verbose {
            println!("  [insert] page {} is FULL ({}/{}) — SPLIT required",
                page_id, n, crate::config::LEAF_CAPACITY);
        }

        let new_leaf_id = self.pager.allocate_page()?;

        // Merge existing records with the new one into one sorted vec
        let mut records: Vec<User> = (0..n as u32).map(|i| page.leaf_read(i)).collect();
        let pos = records.partition_point(|r| r.id < user.id);
        records.insert(pos, user.clone());

        let mid = records.len() / 2;
        let pushed_up_key = records[mid].id; // smallest id in the right half becomes separator

        if self.verbose {
            println!("  [insert] merging {} existing + 1 new = {} records, sorted",
                n, records.len());
            println!("  [insert] split at midpoint {}:", mid);
            println!("           LEFT  page {:4} ← ids {}..{} ({} records)",
                page_id, records[0].id, records[mid-1].id, mid);
            println!("           RIGHT page {:4} ← ids {}..{} ({} records)",
                new_leaf_id, records[mid].id, records.last().unwrap().id, records.len() - mid);
            println!("  [insert] pushed-up key={} → parent must add (key={}, right_child=page_{})",
                pushed_up_key, pushed_up_key, new_leaf_id);
        }

        // Build the left half: records[0..mid], next_leaf points to the new right page
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

        // Build the right half: records[mid..], next_leaf inherits the old right pointer
        let mut right = Page::new();
        let mut right_hdr = PageHeader::new(PageType::Leaf);
        right_hdr.next_page_id = old_next; // preserve the chain: right → whatever left used to point to
        right.write_header(&right_hdr);
        for (i, rec) in records[mid..].iter().enumerate() {
            right.leaf_write(i as u32, rec);
        }
        {
            let mut h = right.header();
            h.num_slots = (records.len() - mid) as u32;
            right.write_header(&h);
        }

        if self.verbose {
            println!("  [insert] writing left  half to page {} (next_leaf=page_{})",
                page_id, new_leaf_id);
            println!("  [insert] writing right half to page {} (next_leaf=page_{})",
                new_leaf_id,
                if old_next == u32::MAX { "none".to_string() } else { old_next.to_string() });
        }

        // Left half reuses the original page_id (keeps existing parent pointers valid)
        self.pager.write_page(page_id, &left)?;
        self.pager.write_page(new_leaf_id, &right)?;

        Ok(Some(SplitResult { pushed_up_key, new_page_id: new_leaf_id }))
    }

    /// Route the insert through an internal node, then absorb any split propagated up
    /// from the child. If this internal node also overflows, splits it and propagates
    /// the mid key further up.
    fn internal_insert(&mut self, page_id: u32, page: Page, user: &User) -> std::io::Result<Option<SplitResult>> {
        let child_id = page.internal_find_child(user.id);
        let split = self.insert_recursive(child_id, user)?;

        let Some(s) = split else {
            return Ok(None); // child did not split — nothing more to do
        };

        // Re-read this page; child writes do not mutate our copy but we need the latest header
        let mut page = self.pager.read_page(page_id)?;

        if !page.internal_is_full() {
            if self.verbose {
                let n = page.header().num_slots as usize;
                println!("  [insert] propagating split: page {} [INTERNAL, {} keys] ← inserting key={} right_child=page_{}",
                    page_id, n, s.pushed_up_key, s.new_page_id);
            }
            page.internal_insert_sorted(s.pushed_up_key, s.new_page_id);
            self.pager.write_page(page_id, &page)?;
            return Ok(None);
        }

        // Internal node is also full — split it and push the mid key further up
        if self.verbose {
            println!("  [insert] internal page {} is FULL — splitting internal node", page_id);
        }

        let new_internal_id = self.pager.allocate_page()?;

        let n = page.header().num_slots as usize;
        let mut entries: Vec<(u64, u32)> = (0..n as u32).map(|i| page.internal_entry(i)).collect();
        let leftmost = page.internal_leftmost_child();

        // Insert the new separator from the child split
        let pos = entries.partition_point(|(k, _)| *k < s.pushed_up_key);
        entries.insert(pos, (s.pushed_up_key, s.new_page_id));

        let mid = entries.len() / 2;
        let mid_key = entries[mid].0; // mid key is pushed up; it does NOT stay in either half

        // Left internal node: leftmost child + entries[0..mid)
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

        // Right internal node: entries[mid].1 becomes the leftmost child, entries[mid+1..] are keys
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

    /// Recursive top-down search for `id`. Returns `Some(User)` if found, else `None`.
    /// On internal nodes, follows the appropriate child based on separator keys.
    /// On leaf nodes, binary-searches the sorted slot array.
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

    /// Descend the tree to find the leaf page that should contain `id`.
    ///
    /// Unlike `find_recursive`, this does not search within the leaf — it just returns
    /// the leaf page id. Used by `delete`, `update`, and `range` which need the page id
    /// itself rather than the record value.
    ///
    /// Returns `u32::MAX` if the tree degenerates to a meta page (should not happen).
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
