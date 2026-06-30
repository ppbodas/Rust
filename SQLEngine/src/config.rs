/// Total size of each page on disk, in bytes. Every page — meta, internal, leaf — is
/// exactly this size, which makes seeking to any page a simple multiply: `page_id × PAGE_SIZE`.
pub const PAGE_SIZE: usize = 4096;

/// Fixed size of a serialized [`User`](crate::record::User) record on disk, in bytes.
/// All leaf slots are this wide so slot offsets can be computed without a directory:
/// `PAGE_HEADER_SIZE + slot_index × RECORD_SIZE`.
pub const RECORD_SIZE: usize = 128;

/// Page id that is always reserved for database metadata. Written first on every open
/// so the file always starts with a valid root_page_id and num_pages.
pub const META_PAGE_ID: u32 = 0;

/// Bytes per internal-node entry: 8 bytes for the separator key (u64) + 4 bytes for the
/// right-child page id (u32). The leftmost child is stored separately before these entries.
pub const INTERNAL_ENTRY_SIZE: usize = 12;

/// Bytes consumed by the page header at the very start of every page.
/// Layout: [0]=page_type, [1..4]=padding, [4..8]=num_slots, [8..12]=next_page_id, [12..24]=reserved.
pub const PAGE_HEADER_SIZE: usize = 24;

/// Usable bytes in the page body (everything after the header). Leaf records and internal
/// entries are packed into this region.
pub const PAGE_BODY_SIZE: usize = PAGE_SIZE - PAGE_HEADER_SIZE;

/// Maximum number of [`User`](crate::record::User) records a leaf page can hold.
/// With 4096-byte pages and 128-byte records: (4096 - 24) / 128 = 31.
pub const LEAF_CAPACITY: usize = PAGE_BODY_SIZE / RECORD_SIZE;

/// Maximum number of separator keys an internal node can hold.
/// The first 4 bytes of the body store the leftmost child pointer, so the remaining space
/// holds `(PAGE_BODY_SIZE - 4) / 12` key+child pairs — about 337 for 4096-byte pages.
pub const INTERNAL_CAPACITY: usize = (PAGE_BODY_SIZE - 4) / INTERNAL_ENTRY_SIZE;

/// Byte offset within the metadata page where `root_page_id` (u32 LE) is stored.
pub const META_ROOT_PAGE_OFFSET: usize = 0;

/// Byte offset within the metadata page where `num_pages` (u32 LE) is stored.
pub const META_NUM_PAGES_OFFSET: usize = 4;
