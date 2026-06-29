use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use crate::config::{PAGE_SIZE, META_PAGE_ID, META_ROOT_PAGE_OFFSET, META_NUM_PAGES_OFFSET};
use crate::page::{Page, PageHeader, PageType};

pub struct Pager {
    file: File,
    pub num_pages: u32,
    pub root_page_id: u32,
}

impl Pager {
    pub fn open(path: &str) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let file_len = file.metadata()?.len();

        if file_len == 0 {
            // Fresh database — write metadata page and an empty root leaf page
            let mut pager = Pager { file, num_pages: 0, root_page_id: 1 };

            // Page 0: metadata
            let meta = Page::new();
            pager.write_raw_page(META_PAGE_ID, &meta)?;
            pager.num_pages = 1;

            // Page 1: initial root (leaf)
            let mut root = Page::new();
            root.write_header(&PageHeader::new(PageType::Leaf));
            pager.write_raw_page(1, &root)?;
            pager.num_pages = 2;

            pager.flush_meta()?;
            Ok(pager)
        } else {
            // Existing database — read metadata from page 0
            let mut pager = Pager { file, num_pages: 0, root_page_id: 1 };
            let meta = pager.read_raw_page(META_PAGE_ID)?;
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

    /// Allocate a new page and return its id.
    pub fn allocate_page(&mut self) -> std::io::Result<u32> {
        let page_id = self.num_pages;
        let blank = Page::new();
        self.write_raw_page(page_id, &blank)?;
        self.num_pages += 1;
        Ok(page_id)
    }

    pub fn read_page(&mut self, page_id: u32) -> std::io::Result<Page> {
        self.read_raw_page(page_id)
    }

    pub fn write_page(&mut self, page_id: u32, page: &Page) -> std::io::Result<()> {
        self.write_raw_page(page_id, page)
    }

    pub fn flush_meta(&mut self) -> std::io::Result<()> {
        let mut meta = Page::new();
        meta.data[META_ROOT_PAGE_OFFSET..META_ROOT_PAGE_OFFSET + 4]
            .copy_from_slice(&self.root_page_id.to_le_bytes());
        meta.data[META_NUM_PAGES_OFFSET..META_NUM_PAGES_OFFSET + 4]
            .copy_from_slice(&self.num_pages.to_le_bytes());
        self.write_raw_page(META_PAGE_ID, &meta)
    }

    fn read_raw_page(&mut self, page_id: u32) -> std::io::Result<Page> {
        let offset = (page_id as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(offset))?;
        let mut buf = [0u8; PAGE_SIZE];
        self.file.read_exact(&mut buf)?;
        Ok(Page::from_bytes(buf))
    }

    fn write_raw_page(&mut self, page_id: u32, page: &Page) -> std::io::Result<()> {
        let offset = (page_id as u64) * (PAGE_SIZE as u64);
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(&page.data)?;
        Ok(())
    }
}
