use crate::config::{
    PAGE_SIZE, PAGE_HEADER_SIZE, RECORD_SIZE, INTERNAL_ENTRY_SIZE,
    LEAF_CAPACITY, INTERNAL_CAPACITY,
};
use crate::record::User;

/// Discriminates the three kinds of pages stored in the database file.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PageType {
    /// Page 0 — holds root_page_id and num_pages; never part of the B+ tree.
    Meta     = 0,
    /// B+ tree internal node — stores separator keys and child page ids.
    Internal = 1,
    /// B+ tree leaf node — stores full User records in sorted id order.
    Leaf     = 2,
}

impl PageType {
    /// Decode the page type from its stored byte value. Unknown values default to Meta.
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => PageType::Internal,
            2 => PageType::Leaf,
            _ => PageType::Meta,
        }
    }
}

/// 24-byte header stored at the very start of every page.
///
/// ```text
///   [0]      page_type   — u8 (0=Meta, 1=Internal, 2=Leaf)
///   [1..4]   padding     — unused, always zero
///   [4..8]   num_slots   — u32 LE: number of used records (leaf) or separator keys (internal)
///   [8..12]  next_page_id — u32 LE: leaf → id of next leaf in chain; internal → unused (u32::MAX)
///   [12..24] reserved    — zero
/// ```
#[derive(Debug, Clone)]
pub struct PageHeader {
    pub page_type: PageType,
    /// Number of valid entries in this page (records for leaf, keys for internal).
    pub num_slots: u32,
    /// For leaf pages: page id of the next leaf in the sorted linked list (`u32::MAX` = end).
    /// For internal pages: unused.
    pub next_page_id: u32,
}

impl PageHeader {
    /// Create a new header of the given type with no slots and no next-page link.
    pub fn new(page_type: PageType) -> Self {
        PageHeader { page_type, num_slots: 0, next_page_id: u32::MAX }
    }

    /// Encode this header into its 24-byte on-disk representation.
    pub fn to_bytes(&self) -> [u8; PAGE_HEADER_SIZE] {
        let mut buf = [0u8; PAGE_HEADER_SIZE];
        buf[0] = self.page_type as u8;
        buf[4..8].copy_from_slice(&self.num_slots.to_le_bytes());
        buf[8..12].copy_from_slice(&self.next_page_id.to_le_bytes());
        buf
    }

    /// Decode a header from its 24-byte on-disk representation.
    pub fn from_bytes(buf: &[u8; PAGE_HEADER_SIZE]) -> Self {
        let page_type = PageType::from_u8(buf[0]);
        let num_slots = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let next_page_id = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        PageHeader { page_type, num_slots, next_page_id }
    }
}

/// A raw database page — always exactly [`PAGE_SIZE`] bytes.
///
/// The first [`PAGE_HEADER_SIZE`] bytes hold the [`PageHeader`]; the rest is the body,
/// interpreted differently depending on page type:
/// - **Leaf**: `num_slots × RECORD_SIZE` bytes of packed [`User`] records.
/// - **Internal**: 4-byte leftmost child, then `num_slots × INTERNAL_ENTRY_SIZE` key+child pairs.
pub struct Page {
    pub data: [u8; PAGE_SIZE],
}

impl Page {
    /// Create a blank page (all bytes zero).
    pub fn new() -> Self {
        Page { data: [0u8; PAGE_SIZE] }
    }

    /// Wrap an existing raw byte array as a Page without copying.
    pub fn from_bytes(bytes: [u8; PAGE_SIZE]) -> Self {
        Page { data: bytes }
    }

    /// Decode and return this page's header. Allocates a small stack copy on each call.
    pub fn header(&self) -> PageHeader {
        let hdr: [u8; PAGE_HEADER_SIZE] = self.data[..PAGE_HEADER_SIZE].try_into().unwrap();
        PageHeader::from_bytes(&hdr)
    }

    /// Overwrite the first `PAGE_HEADER_SIZE` bytes with the encoded form of `hdr`.
    pub fn write_header(&mut self, hdr: &PageHeader) {
        self.data[..PAGE_HEADER_SIZE].copy_from_slice(&hdr.to_bytes());
    }

    // ── Leaf page helpers ────────────────────────────────────────────────────────
    //
    // Slot layout: slot i starts at PAGE_HEADER_SIZE + i × RECORD_SIZE.
    // No explicit slot directory — the offset is always recomputed from the index.

    /// Read the record at the given slot index. Does not bounds-check against num_slots.
    pub fn leaf_read(&self, slot: u32) -> User {
        let offset = PAGE_HEADER_SIZE + (slot as usize) * RECORD_SIZE;
        let buf: &[u8; RECORD_SIZE] = self.data[offset..offset + RECORD_SIZE].try_into().unwrap();
        User::from_bytes(buf)
    }

    /// Write `user` into the given slot. Does not update `num_slots` — caller must do that.
    pub fn leaf_write(&mut self, slot: u32, user: &User) {
        let offset = PAGE_HEADER_SIZE + (slot as usize) * RECORD_SIZE;
        self.data[offset..offset + RECORD_SIZE].copy_from_slice(&user.to_bytes());
    }

    /// Return true if `num_slots` has reached [`LEAF_CAPACITY`] (31 for 4096-byte pages).
    pub fn leaf_is_full(&self) -> bool {
        self.header().num_slots >= LEAF_CAPACITY as u32
    }

    /// Insert `user` into this leaf page in ascending id order, shifting later records right.
    ///
    /// Returns `false` without modifying the page if the page is already full.
    /// Updates `num_slots` in the header.
    pub fn leaf_insert_sorted(&mut self, user: &User) -> bool {
        let mut hdr = self.header();
        if hdr.num_slots >= LEAF_CAPACITY as u32 {
            return false;
        }

        let n = hdr.num_slots as usize;

        // Find the first slot whose id is greater than the new record's id
        let mut pos = n; // default: append at the end
        for i in 0..n {
            if self.leaf_read(i as u32).id > user.id {
                pos = i;
                break;
            }
        }

        // Shift records at [pos..n) one slot to the right to open a gap
        for i in (pos..n).rev() {
            let rec = self.leaf_read(i as u32);
            self.leaf_write(i as u32 + 1, &rec);
        }

        self.leaf_write(pos as u32, user);
        hdr.num_slots += 1;
        self.write_header(&hdr);
        true
    }

    /// Binary search for `id` in this leaf page.
    ///
    /// Returns `Some(User)` if found, `None` otherwise.
    /// Assumes slots are in ascending id order (guaranteed by [`leaf_insert_sorted`](Self::leaf_insert_sorted)).
    pub fn leaf_search(&self, id: u64) -> Option<User> {
        let n = self.header().num_slots as usize;
        let mut lo = 0usize;
        let mut hi = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let rec = self.leaf_read(mid as u32);
            match rec.id.cmp(&id) {
                std::cmp::Ordering::Equal   => return Some(rec),
                std::cmp::Ordering::Less    => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        None
    }

    /// Delete the record with `id` from this leaf page.
    ///
    /// Removes the slot, shifts all subsequent slots one position left to close the gap,
    /// and zeroes the vacated last slot. Updates `num_slots` in the header.
    ///
    /// Returns `Some(slot_index)` of the deleted record, or `None` if `id` was not found.
    /// The caller is responsible for verbose logging if needed.
    pub fn leaf_delete(&mut self, id: u64) -> Option<usize> {
        let mut hdr = self.header();
        let n = hdr.num_slots as usize;

        // Binary search for the target slot
        let mut lo = 0usize;
        let mut hi = n;
        let mut found_slot = None;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let rec = self.leaf_read(mid as u32);
            match rec.id.cmp(&id) {
                std::cmp::Ordering::Equal   => { found_slot = Some(mid); break; }
                std::cmp::Ordering::Less    => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }

        let slot = found_slot?;

        // Close the gap: shift [slot+1..n) one position left
        for i in slot..n - 1 {
            let rec = self.leaf_read(i as u32 + 1);
            self.leaf_write(i as u32, &rec);
        }

        // Zero the vacated last slot so no stale bytes remain
        let last_off = PAGE_HEADER_SIZE + (n - 1) * RECORD_SIZE;
        self.data[last_off..last_off + RECORD_SIZE].fill(0);

        hdr.num_slots -= 1;
        self.write_header(&hdr);
        Some(slot)
    }

    /// Overwrite the non-key fields of the record matching `user.id` in-place.
    ///
    /// Uses binary search to locate the slot, then writes 128 bytes at the exact same
    /// offset — no other slots move. This is why UPDATE is faster than DELETE + INSERT.
    ///
    /// Returns `Some((slot, old_record))` if found so the caller can log the diff,
    /// or `None` if no record with that id exists in this page.
    pub fn leaf_update(&mut self, user: &User) -> Option<(usize, User)> {
        let n = self.header().num_slots as usize;
        let mut lo = 0usize;
        let mut hi = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let rec = self.leaf_read(mid as u32);
            match rec.id.cmp(&user.id) {
                std::cmp::Ordering::Equal => {
                    let old = rec;
                    self.leaf_write(mid as u32, user); // overwrite at same slot, same offset
                    return Some((mid, old));
                }
                std::cmp::Ordering::Less    => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        None
    }

    // ── Internal page helpers ────────────────────────────────────────────────────
    //
    // Internal node body layout:
    //   body[0..4]                 = leftmost child page_id (u32 LE)
    //   body[4 + i×12 ..  +8]     = separator key[i] (u64 LE)
    //   body[4 + i×12 + 8 .. +4]  = right-child page_id[i] (u32 LE)
    //
    // An internal node with N keys has N+1 children:
    //   - Keys less than key[0]     → leftmost child
    //   - key[i-1] ≤ key < key[i]  → right-child of entry [i-1]
    //   - Keys ≥ key[N-1]           → right-child of entry [N-1]

    /// Return the fixed byte offset in `data` where the internal node body begins.
    fn internal_body_offset() -> usize {
        PAGE_HEADER_SIZE
    }

    /// Read the leftmost child page id (stored in the first 4 bytes of the body).
    pub fn internal_leftmost_child(&self) -> u32 {
        let off = Self::internal_body_offset();
        u32::from_le_bytes(self.data[off..off + 4].try_into().unwrap())
    }

    /// Write the leftmost child page id into the first 4 bytes of the body.
    pub fn internal_set_leftmost_child(&mut self, page_id: u32) {
        let off = Self::internal_body_offset();
        self.data[off..off + 4].copy_from_slice(&page_id.to_le_bytes());
    }

    /// Read the `(separator_key, right_child_page_id)` pair at entry index `i`.
    pub fn internal_entry(&self, i: u32) -> (u64, u32) {
        let off = Self::internal_body_offset() + 4 + (i as usize) * INTERNAL_ENTRY_SIZE;
        let key   = u64::from_le_bytes(self.data[off..off + 8].try_into().unwrap());
        let child = u32::from_le_bytes(self.data[off + 8..off + 12].try_into().unwrap());
        (key, child)
    }

    /// Write a `(key, right_child)` pair at entry index `i`.
    pub fn internal_set_entry(&mut self, i: u32, key: u64, right_child: u32) {
        let off = Self::internal_body_offset() + 4 + (i as usize) * INTERNAL_ENTRY_SIZE;
        self.data[off..off + 8].copy_from_slice(&key.to_le_bytes());
        self.data[off + 8..off + 12].copy_from_slice(&right_child.to_le_bytes());
    }

    /// Return true if `num_slots` has reached [`INTERNAL_CAPACITY`] (~337 for 4096-byte pages).
    pub fn internal_is_full(&self) -> bool {
        self.header().num_slots >= INTERNAL_CAPACITY as u32
    }

    /// Return the child page id to follow when searching for `key`.
    /// Thin wrapper around [`internal_find_child_logged`](Self::internal_find_child_logged)
    /// that discards the log string.
    pub fn internal_find_child(&self, key: u64) -> u32 {
        self.internal_find_child_logged(key).0
    }

    /// Find the child page id for `key` and also return a human-readable description of
    /// the routing decision, used by verbose traversal logs across all tree operations.
    ///
    /// Routing rules:
    /// - `key < key[0]`           → leftmost child
    /// - `key[i-1] ≤ key < key[i]` → right-child of entry [i-1]
    /// - `key ≥ key[N-1]`         → right-child of entry [N-1]  (rightmost)
    pub fn internal_find_child_logged(&self, key: u64) -> (u32, String) {
        let n = self.header().num_slots as usize;

        for i in 0..n {
            let (k, _) = self.internal_entry(i as u32);
            if key < k {
                if i == 0 {
                    let child = self.internal_leftmost_child();
                    return (child, format!(
                        "id {} < key[0]={} → leftmost child → page {}",
                        key, k, child
                    ));
                } else {
                    // key falls in the interval [key[i-1], key[i])
                    let (prev_k, child) = self.internal_entry(i as u32 - 1);
                    return (child, format!(
                        "key[{}]={} ≤ id {} < key[{}]={} → page {}",
                        i - 1, prev_k, key, i, k, child
                    ));
                }
            }
        }

        // key is ≥ every separator key → take the rightmost child
        if n == 0 {
            let child = self.internal_leftmost_child();
            (child, format!("no keys, only child → page {}", child))
        } else {
            let (last_k, child) = self.internal_entry(n as u32 - 1);
            (child, format!(
                "id {} ≥ key[{}]={} → rightmost child → page {}",
                key, n - 1, last_k, child
            ))
        }
    }

    /// Insert `(key, right_child)` into this internal node in ascending key order,
    /// shifting later entries right.
    ///
    /// Returns `false` without modifying the node if it is already full.
    /// Updates `num_slots` in the header.
    pub fn internal_insert_sorted(&mut self, key: u64, right_child: u32) -> bool {
        let mut hdr = self.header();
        if hdr.num_slots >= INTERNAL_CAPACITY as u32 {
            return false;
        }

        let n = hdr.num_slots as usize;

        // Find the insertion point: first existing key greater than the new key
        let mut pos = n;
        for i in 0..n {
            if self.internal_entry(i as u32).0 > key {
                pos = i;
                break;
            }
        }

        // Shift entries at [pos..n) one slot to the right
        for i in (pos..n).rev() {
            let (k, c) = self.internal_entry(i as u32);
            self.internal_set_entry(i as u32 + 1, k, c);
        }

        self.internal_set_entry(pos as u32, key, right_child);
        hdr.num_slots += 1;
        self.write_header(&hdr);
        true
    }
}
