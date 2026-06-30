use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub phone: String,
    pub address: String,
}

impl User {
    pub fn new(id: &str, name: &str, phone: &str, address: &str) -> Self {
        User {
            id: id.to_string(),
            name: name.to_string(),
            phone: phone.to_string(),
            address: address.to_string(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("User serialization failed")
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).expect("User deserialization failed")
    }
}
