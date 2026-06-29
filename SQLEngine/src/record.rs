use crate::config::RECORD_SIZE;

/// Fixed 128-byte user record layout:
///   [0..8]    id: u64
///   [8..40]   name: [u8; 32]
///   [40]      age: u8
///   [41..57]  phone: [u8; 16]
///   [57..120] address: [u8; 63]
///   [120..128] padding
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub age: u8,
    pub phone: String,
    pub address: String,
}

impl User {
    pub fn new(id: u64, name: &str, age: u8, phone: &str, address: &str) -> Self {
        User {
            id,
            name: name.to_string(),
            age,
            phone: phone.to_string(),
            address: address.to_string(),
        }
    }

    pub fn to_bytes(&self) -> [u8; RECORD_SIZE] {
        let mut buf = [0u8; RECORD_SIZE];

        buf[0..8].copy_from_slice(&self.id.to_le_bytes());

        let name_bytes = self.name.as_bytes();
        let name_len = name_bytes.len().min(32);
        buf[8..8 + name_len].copy_from_slice(&name_bytes[..name_len]);

        buf[40] = self.age;

        let phone_bytes = self.phone.as_bytes();
        let phone_len = phone_bytes.len().min(16);
        buf[41..41 + phone_len].copy_from_slice(&phone_bytes[..phone_len]);

        let addr_bytes = self.address.as_bytes();
        let addr_len = addr_bytes.len().min(63);
        buf[57..57 + addr_len].copy_from_slice(&addr_bytes[..addr_len]);

        buf
    }

    pub fn from_bytes(buf: &[u8; RECORD_SIZE]) -> Self {
        let id = u64::from_le_bytes(buf[0..8].try_into().unwrap());

        let name_end = buf[8..40].iter().position(|&b| b == 0).unwrap_or(32);
        let name = String::from_utf8_lossy(&buf[8..8 + name_end]).to_string();

        let age = buf[40];

        let phone_end = buf[41..57].iter().position(|&b| b == 0).unwrap_or(16);
        let phone = String::from_utf8_lossy(&buf[41..41 + phone_end]).to_string();

        let addr_end = buf[57..120].iter().position(|&b| b == 0).unwrap_or(63);
        let address = String::from_utf8_lossy(&buf[57..57 + addr_end]).to_string();

        User { id, name, age, phone, address }
    }
}
