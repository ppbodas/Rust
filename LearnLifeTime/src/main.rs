mod data_structs;

use crate::data_structs::book::Book;

fn main() {
    println!("Hello, world!");

    // Lifetime is useful in case of references
    let string1 = String::from("abcd");
    let string2 = "xyz";
    let result = longest(string1.as_str(), string2);

    println!("The longest string is {}", result);

    let book = Book::new("Book1");

    println!("{} ", book.get_title());
}

fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}