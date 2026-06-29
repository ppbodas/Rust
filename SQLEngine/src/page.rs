use crate::config::{
    PAGE_SIZE, PAGE_HEADER_SIZE, RECORD_SIZE, INTERNAL_ENTRY_SIZE,
    LEAF_CAPACITY, INTERNAL_CAPACITY,
};
use crate::record::User;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PageType {
    Meta     = 0,
    Internal = 1,
    Leaf     = 2,
}

impl PageType {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => PageType::Internal,
            2 => PageType::Leaf,
            _ => PageType::Meta,
        }
    }
}

/// Page header layout (24 bytes):
///   [0]     page_type: u8
///   [1..3]  padding
///   [4..8]  num_slots: u32       — number of used entries
///   [8..12] next_page_id: u32    — for leaf: next leaf in chain; for internal: rightmost child
///   [12..24] reserved
#[derive(Debug, Clone)]
pub struct PageHeader {
    pub page_type: PageType,
    pub num_slots: u32,
    pub next_page_id: u32,   // leaf: next sibling; internal: rightmost child ptr
}

impl PageHeader {
    pub fn new(page_type: PageType) -> Self {
        PageHeader { page_type, num_slots: 0, next_page_id: u32::MAX }
    }

    pub fn to_bytes(&self) -> [u8; PAGE_HEADER_SIZE] {
        let mut buf = [0u8; PAGE_HEADER_SIZE];
        buf[0] = self.page_type as u8;
        buf[4..8].copy_from_slice(&self.num_slots.to_le_bytes());
        buf[8..12].copy_from_slice(&self.next_page_id.to_le_bytes());
        buf
    }

    pub fn from_bytes(buf: &[u8; PAGE_HEADER_SIZE]) -> Self {
        let page_type = PageType::from_u8(buf[0]);
        let num_slots = u32::from_le_bytes(buf[4..8].try_into().unwrap());
        let next_page_id = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        PageHeader { page_type, num_slots, next_page_id }
    }
}

/// Raw page buffer — always PAGE_SIZE bytes.
pub struct Page {
    pub data: [u8; PAGE_SIZE],
}

impl Page {
    pub fn new() -> Self {
        Page { data: [0u8; PAGE_SIZE] }
    }

    pub fn from_bytes(bytes: [u8; PAGE_SIZE]) -> Self {
        Page { data: bytes }
    }

    pub fn header(&self) -> PageHeader {
        let hdr: [u8; PAGE_HEADER_SIZE] = self.data[..PAGE_HEADER_SIZE].try_into().unwrap();
        PageHeader::from_bytes(&hdr)
    }

    pub fn write_header(&mut self, hdr: &PageHeader) {
        self.data[..PAGE_HEADER_SIZE].copy_from_slice(&hdr.to_bytes());
    }

    // ── Leaf page helpers ────────────────────────────────────────────────────

    /// Read the i-th record from a leaf page.
    pub fn leaf_read(&self, slot: u32) -> User {
        let offset = PAGE_HEADER_SIZE + (slot as usize) * RECORD_SIZE;
        let buf: &[u8; RECORD_SIZE] = self.data[offset..offset + RECORD_SIZE].try_into().unwrap();
        User::from_bytes(buf)
    }

    /// Append a record to a leaf page (does NOT check capacity — caller must).
    pub fn leaf_write(&mut self, slot: u32, user: &User) {
        let offset = PAGE_HEADER_SIZE + (slot as usize) * RECORD_SIZE;
        self.data[offset..offset + RECORD_SIZE].copy_from_slice(&user.to_bytes());
    }

    pub fn leaf_is_full(&self) -> bool {
        self.header().num_slots >= LEAF_CAPACITY as u32
    }

    /// Insert a record in sorted order by id. Returns false if full.
    pub fn leaf_insert_sorted(&mut self, user: &User) -> bool {
        let mut hdr = self.header();
        if hdr.num_slots >= LEAF_CAPACITY as u32 {
            return false;
        }
        // Find insertion point
        let n = hdr.num_slots as usize;
        let mut pos = n;
        for i in 0..n {
            if self.leaf_read(i as u32).id > user.id {
                pos = i;
                break;
            }
        }
        // Shift right
        for i in (pos..n).rev() {
            let rec = self.leaf_read(i as u32);
            self.leaf_write(i as u32 + 1, &rec);
        }
        self.leaf_write(pos as u32, user);
        hdr.num_slots += 1;
        self.write_header(&hdr);
        true
    }

    /// Delete a record by id. Shifts subsequent slots left. Returns the slot index if found.
    /// Caller is responsible for logging; this method just mutates bytes.
    pub fn leaf_delete(&mut self, id: u64) -> Option<usize> {
        let mut hdr = self.header();
        let n = hdr.num_slots as usize;

        // Binary search for the slot
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

        // Shift everything after `slot` one position to the left
        for i in slot..n - 1 {
            let rec = self.leaf_read(i as u32 + 1);
            self.leaf_write(i as u32, &rec);
        }
        // Zero out the last (now-vacated) slot
        let last_off = PAGE_HEADER_SIZE + (n - 1) * RECORD_SIZE;
        self.data[last_off..last_off + RECORD_SIZE].fill(0);

        hdr.num_slots -= 1;
        self.write_header(&hdr);
        Some(slot)
    }

    /// Overwrite an existing record in-place by id. Returns true if found.
    pub fn leaf_update(&mut self, user: &User) -> bool {
        let n = self.header().num_slots as usize;
        let mut lo = 0usize;
        let mut hi = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let rec = self.leaf_read(mid as u32);
            match rec.id.cmp(&user.id) {
                std::cmp::Ordering::Equal => {
                    self.leaf_write(mid as u32, user);
                    return true;
                }
                std::cmp::Ordering::Less    => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        false
    }

    /// Binary search for a key in a leaf. Returns Some(User) if found.
    pub fn leaf_search(&self, id: u64) -> Option<User> {
        let n = self.header().num_slots as usize;
        let mut lo = 0usize;
        let mut hi = n;
        while lo < hi {
            let mid = (lo + hi) / 2;
            let rec = self.leaf_read(mid as u32);
            match rec.id.cmp(&id) {
                std::cmp::Ordering::Equal => return Some(rec),
                std::cmp::Ordering::Less  => lo = mid + 1,
                std::cmp::Ordering::Greater => hi = mid,
            }
        }
        None
    }

    // ── Internal page helpers ────────────────────────────────────────────────
    //
    // Internal node layout in body:
    //   [child_0][key_0][child_1][key_1]...[key_{n-1}][child_n]
    //
    // Stored as: first child (4 bytes), then n × (key 8B + right_child 4B)
    // num_slots = number of keys (= number of children - 1, except rightmost stored in header.next_page_id)
    //
    // We store:
    //   body[0..4]              = leftmost child page_id (u32)
    //   body[4 + i*12 .. +8]   = key[i] (u64)
    //   body[4 + i*12 + 8..+4] = right child page_id[i] (u32)

    fn internal_body_offset() -> usize {
        PAGE_HEADER_SIZE
    }

    pub fn internal_leftmost_child(&self) -> u32 {
        let off = Self::internal_body_offset();
        u32::from_le_bytes(self.data[off..off + 4].try_into().unwrap())
    }

    pub fn internal_set_leftmost_child(&mut self, page_id: u32) {
        let off = Self::internal_body_offset();
        self.data[off..off + 4].copy_from_slice(&page_id.to_le_bytes());
    }

    /// Get (key, right_child) for entry i.
    pub fn internal_entry(&self, i: u32) -> (u64, u32) {
        let off = Self::internal_body_offset() + 4 + (i as usize) * INTERNAL_ENTRY_SIZE;
        let key = u64::from_le_bytes(self.data[off..off + 8].try_into().unwrap());
        let child = u32::from_le_bytes(self.data[off + 8..off + 12].try_into().unwrap());
        (key, child)
    }

    pub fn internal_set_entry(&mut self, i: u32, key: u64, right_child: u32) {
        let off = Self::internal_body_offset() + 4 + (i as usize) * INTERNAL_ENTRY_SIZE;
        self.data[off..off + 8].copy_from_slice(&key.to_le_bytes());
        self.data[off + 8..off + 12].copy_from_slice(&right_child.to_le_bytes());
    }

    pub fn internal_is_full(&self) -> bool {
        self.header().num_slots >= INTERNAL_CAPACITY as u32
    }

    /// Find the child page_id to follow for a given search key.
    pub fn internal_find_child(&self, key: u64) -> u32 {
        self.internal_find_child_logged(key).0
    }

    /// Same as internal_find_child but also returns a human-readable description of the decision.
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
                    let (prev_k, child) = self.internal_entry(i as u32 - 1);
                    return (child, format!(
                        "key[{}]={} ≤ id {} < key[{}]={} → page {}",
                        i - 1, prev_k, key, i, k, child
                    ));
                }
            }
        }
        // key >= all keys → rightmost child
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

    /// Insert (key, right_child) into an internal node in sorted order.
    /// The left child of key is implicitly the previous rightmost child.
    pub fn internal_insert_sorted(&mut self, key: u64, right_child: u32) -> bool {
        let mut hdr = self.header();
        if hdr.num_slots >= INTERNAL_CAPACITY as u32 {
            return false;
        }
        let n = hdr.num_slots as usize;
        let mut pos = n;
        for i in 0..n {
            if self.internal_entry(i as u32).0 > key {
                pos = i;
                break;
            }
        }
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
