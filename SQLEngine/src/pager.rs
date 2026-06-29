use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use crate::config::{PAGE_SIZE, META_PAGE_ID, META_ROOT_PAGE_OFFSET, META_NUM_PAGES_OFFSET};
use crate::page::{Page, PageHeader, PageType};

/// Low-level file I/O manager for the database file.
///
/// The file is divided into fixed-size pages. Every page is exactly [`PAGE_SIZE`] bytes,
/// so the byte offset of any page is simply `page_id × PAGE_SIZE`. Page 0 is always the
/// metadata page; page 1 is the initial root leaf created on a fresh database.
///
/// There is no in-memory buffer pool — every [`read_page`](Pager::read_page) and
/// [`write_page`](Pager::write_page) call issues a seek + syscall directly to the file.
pub struct Pager {
    file: File,
    /// Total number of pages currently allocated (file length / PAGE_SIZE).
    pub num_pages: u32,
    /// Page id of the current B+ tree root. Updated in memory after a root split
    /// and persisted by [`flush_meta`](Pager::flush_meta).
    pub root_page_id: u32,
}

impl Pager {
    /// Open or create the database file at `path`.
    ///
    /// **New file (zero length):** writes a blank metadata page (page 0) and an empty
    /// leaf root page (page 1), then flushes the metadata.
    ///
    /// **Existing file:** reads `root_page_id` and `num_pages` from the metadata page
    /// so the engine resumes exactly where it left off.
    pub fn open(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let file_len = file.metadata()?.len();

        if file_len == 0 {
            let mut pager = Pager { file, num_pages: 0, root_page_id: 1 };

            // Page 0: blank metadata placeholder (real values written by flush_meta below)
            let meta = Page::new();
            pager.write_raw_page(META_PAGE_ID, &meta)?;
            pager.num_pages = 1;

            // Page 1: empty leaf, becomes the initial tree root
            let mut root = Page::new();
            root.write_header(&PageHeader::new(PageType::Leaf));
            pager.write_raw_page(1, &root)?;
            pager.num_pages = 2;

            pager.flush_meta()?;
            Ok(pager)
        } else {
            let mut pager = Pager { file, num_pages: 0, root_page_id: 1 };
            let meta = pager.read_raw_page(META_PAGE_ID)?;

            // Decode the two u32 fields stored at the start of page 0
            let root_page_id = u32::from_le_bytes(
                meta.data[META_ROOT_PAGE_OFFSET..META_ROOT_PAGE_OFFSET + 4].try_into().unwrap(),
            );
            let num_pages = u32::from_le_bytes(
                meta.data[META_NUM_PAGES_OFFSET..META_NUM_PAGES_OFFSET + 4].try_into().unwrap(),
            );

            pager.root_page_id = root_page_id;
            pager.num_pages = num_pages;
            Ok(pager)
        }
    }

    /// Append a new blank page to the end of the file and return its page id.
    ///
    /// Writes all-zero bytes to reserve space. The caller is responsible for writing
    /// meaningful content (header + records or keys) before the page is first used.
    pub fn allocate_page(&mut self) -> std::io::Result<u32> {
        let page_id = self.num_pages;
        let blank = Page::new();
        self.write_raw_page(page_id, &blank)?;
        self.num_pages += 1;
        Ok(page_id)
    }

    /// Read the page with the given `page_id` from disk.
    /// Issues one seek and one `read_exact` syscall per call — no caching.
    pub fn read_page(&mut self, page_id: u32) -> std::io::Result<Page> {
        self.read_raw_page(page_id)
    }

    /// Write `page` to disk at the position corresponding to `page_id`.
    /// Issues one seek and one `write_all` syscall per call.
    pub fn write_page(&mut self, page_id: u32, page: &Page) -> std::io::Result<()> {
        self.write_raw_page(page_id, page)
    }

    /// Persist the current `root_page_id` and `num_pages` to the metadata page (page 0).
    ///
    /// Must be called after any root split or at database close to ensure the next
    /// `open` resumes with the correct tree root and page count.
    pub fn flush_meta(&mut self) -> std::io::Result<()> {
        let mut meta = Page::new();
        meta.data[META_ROOT_PAGE_OFFSET..META_ROOT_PAGE_OFFSET + 4]
            .copy_from_slice(&self.root_page_id.to_le_bytes());
        meta.data[META_NUM_PAGES_OFFSET..META_NUM_PAGES_OFFSET + 4]
            .copy_from_slice(&self.num_pages.to_le_bytes());
        self.write_raw_page(META_PAGE_ID, &meta)
    }

    /// Seek to `page_id × PAGE_SIZE` and read exactly `PAGE_SIZE` bytes into a [`Page`].
    fn read_raw_page(&mut self, page_id: u32) -> std::io::Result<Page> {
        let offset = (page_id as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(offset))?;
        let mut buf = [0u8; PAGE_SIZE];
        self.file.read_exact(&mut buf)?;
        Ok(Page::from_bytes(buf))
    }

    /// Seek to `page_id × PAGE_SIZE` and write exactly `PAGE_SIZE` bytes from `page.data`.
    fn write_raw_page(&mut self, page_id: u32, page: &Page) -> std::io::Result<()> {
        let offset = (page_id as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&page.data)?;
        Ok(())
    }
}
