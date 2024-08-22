pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: String,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Credentials {
        Credentials { username, password }
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}
