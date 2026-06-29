pub const PAGE_SIZE: usize = 4096;

// User record is fixed 128 bytes
pub const RECORD_SIZE: usize = 128;

// Page 0 is always the metadata page
pub const META_PAGE_ID: u32 = 0;

// Internal node entry: 8 (key) + 4 (child_page_id) = 12 bytes
pub const INTERNAL_ENTRY_SIZE: usize = 12;

// Page header size in bytes
pub const PAGE_HEADER_SIZE: usize = 24;

// Usable bytes per page after header
pub const PAGE_BODY_SIZE: usize = PAGE_SIZE - PAGE_HEADER_SIZE;

// Max records a leaf page can hold
pub const LEAF_CAPACITY: usize = PAGE_BODY_SIZE / RECORD_SIZE; // ~31 with 4096 page

// Max entries an internal node can hold (key + child pairs)
// +1 because an internal node with N keys has N+1 children
pub const INTERNAL_CAPACITY: usize = (PAGE_BODY_SIZE - 4) / INTERNAL_ENTRY_SIZE; // ~337

// Metadata page layout offsets
pub const META_ROOT_PAGE_OFFSET: usize = 0;   // u32
pub const META_NUM_PAGES_OFFSET: usize = 4;   // u32
