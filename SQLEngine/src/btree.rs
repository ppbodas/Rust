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

        if self.verbose {
            println!("  [insert] starting at root page {}", root_id);
        }

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
                    println!("           id        {:<20} {} (unchanged — key field)",
                        old.id, user.id);
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

        if self.verbose {
            match hdr.page_type {
                PageType::Internal => {
                    let (child_id, reason) = page.internal_find_child_logged(user.id);
                    println!("  [insert] page {:4} [INTERNAL, {:3} keys  ] → {}",
                        page_id, hdr.num_slots, reason);
                    // recurse after logging
                    let _ = child_id; // already computed inside internal_insert
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

    fn leaf_insert(&mut self, page_id: u32, mut page: Page, user: &User) -> std::io::Result<Option<SplitResult>> {
        if !page.leaf_is_full() {
            let n = page.header().num_slots as usize;

            // Find insertion slot index before mutating
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

        // Leaf is full — split
        let n = page.header().num_slots as usize;

        if self.verbose {
            println!("  [insert] page {} is FULL ({}/{}) — SPLIT required",
                page_id, n, crate::config::LEAF_CAPACITY);
        }

        let new_leaf_id = self.pager.allocate_page()?;

        // Collect all records + new one, sorted
        let mut records: Vec<User> = (0..n as u32).map(|i| page.leaf_read(i)).collect();
        let pos = records.partition_point(|r| r.id < user.id);
        records.insert(pos, user.clone());

        let mid = records.len() / 2;
        let pushed_up_key = records[mid].id;

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

        if self.verbose {
            println!("  [insert] writing left  half to page {} (next_leaf=page_{})",
                page_id, new_leaf_id);
            println!("  [insert] writing right half to page {} (next_leaf=page_{})",
                new_leaf_id,
                if old_next == u32::MAX { "none".to_string() } else { old_next.to_string() });
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
            if self.verbose {
                let n = page.header().num_slots as usize;
                println!("  [insert] propagating split: page {} [INTERNAL, {} keys] ← inserting key={} right_child=page_{}",
                    page_id, n, s.pushed_up_key, s.new_page_id);
            }
            page.internal_insert_sorted(s.pushed_up_key, s.new_page_id);
            self.pager.write_page(page_id, &page)?;
            return Ok(None);
        }

        // Internal node is full — split it
        if self.verbose {
            println!("  [insert] internal page {} is FULL — splitting internal node", page_id);
        }

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
