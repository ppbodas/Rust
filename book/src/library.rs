// Create struct Library which holds vector of Book

use crate::book::Book;

pub(crate) struct Library<'a,'b> {
    books: Vec<Book<'a,'b>>,
}

// Implement methods for Library
impl<'a,'b> Library<'a,'b> {
    pub(crate) fn new() -> Library<'a,'b> {
        Library {
            books: Vec::new(),
        }
    }

    pub(crate) fn add_book(&mut self, book: Book<'a,'b>) {
        self.books.push(book);
    }

    pub fn print_books(&self) {
        for book in &self.books {
            println!("{}", book);
        }
    }
}

