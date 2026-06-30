mod book;
mod library;

fn main() {
    println!("Hello, world!");


    // Create book object
    let book = book::Book::new("The Great Gatsby",
                               "F. Scott Fitzgerald");

    println!("Book: {}", book.get_name());
    println!("Author: {}", book.get_author());

    // Create library object
    let mut library = library::Library::new();
    library.add_book(book);

    // Add one more book
    let book2 = book::Book::new("The Catcher in the Rye",
                                "J. D. Salinger");
    library.add_book(book2);

    // print library books
    library.print_books();
}
