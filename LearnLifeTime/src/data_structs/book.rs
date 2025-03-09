pub struct Book<'a> {
    title: &'a str
}

impl<'a> Book<'a> {
    pub fn new(title: &'a str) -> Book<'a> {
        Book { title }
    }

    pub fn get_title(&self) -> &str {
        self.title
    }
}
