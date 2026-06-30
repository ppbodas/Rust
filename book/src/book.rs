// Create book struct taking name and author as reference
pub struct Book<'a, 'b> {
    name: &'a str,
    author: &'b str,
}

impl<'a, 'b> Book<'a, 'b> {
    pub(crate) fn new(p0: &'a str, p1: &'b str) -> Book<'a, 'a>
    where 'b: 'a
    {
        Book {
            name: p0,
            author: p1,
        }
    }

    pub(crate) fn get_name(&self) -> &str {
        self.name
    }
    pub(crate) fn get_author(&self) -> &str {
        self.author
    }
}

// Implement the Display trait for Point
impl<'a, 'b> std::fmt::Display for Book<'a, 'b> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Book: {} by {}", self.name, self.author)
    }
}