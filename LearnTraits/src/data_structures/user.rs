// Define User struct

#[derive(Default)]
pub struct User {
    username: String,
    email: String,
    active: bool,
}

impl User {
    pub fn new(username: &str, email: &str) -> User {
        User {
            username: username.to_string(),
            email: email.to_string(),
            active: true,
        }
    }
}

