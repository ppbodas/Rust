use crate::config::RECORD_SIZE;

/// A single user record stored in the database, indexed by `id`.
///
/// On-disk layout — always exactly 128 bytes:
/// ```text
///   [0..8]     id      — u64, little-endian
///   [8..40]    name    — UTF-8, zero-padded to 32 bytes (max 32 chars)
///   [40]       age     — u8
///   [41..57]   phone   — UTF-8, zero-padded to 16 bytes (max 16 chars)
///   [57..120]  address — UTF-8, zero-padded to 63 bytes (max 63 chars)
///   [120..128] padding — reserved, always zero
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: u64,
    pub name: String,
    pub age: u8,
    pub phone: String,
    pub address: String,
}

impl User {
    /// Construct a new User from its constituent fields.
    ///
    /// Strings longer than their field limit (name: 32, phone: 16, address: 63) will be
    /// silently truncated when serialized. Callers should validate lengths beforehand.
    pub fn new(id: u64, name: &str, age: u8, phone: &str, address: &str) -> Self {
        User {
            id,
            name: name.to_string(),
            age,
            phone: phone.to_string(),
            address: address.to_string(),
        }
    }

    /// Serialize this record into a fixed 128-byte buffer suitable for writing to disk.
    ///
    /// Each string field is copied as raw UTF-8 bytes into its fixed window; the remainder
    /// of that window stays zero (null-terminated). Fields exceeding the limit are truncated.
    pub fn to_bytes(&self) -> [u8; RECORD_SIZE] {
        let mut buf = [0u8; RECORD_SIZE];

        // id: bytes 0..8, little-endian u64
        buf[0..8].copy_from_slice(&self.id.to_le_bytes());

        // name: bytes 8..40, up to 32 UTF-8 bytes, zero-padded
        let name_bytes = self.name.as_bytes();
        let name_len = name_bytes.len().min(32);
        buf[8..8 + name_len].copy_from_slice(&name_bytes[..name_len]);

        // age: single byte at offset 40
        buf[40] = self.age;

        // phone: bytes 41..57, up to 16 UTF-8 bytes, zero-padded
        let phone_bytes = self.phone.as_bytes();
        let phone_len = phone_bytes.len().min(16);
        buf[41..41 + phone_len].copy_from_slice(&phone_bytes[..phone_len]);

        // address: bytes 57..120, up to 63 UTF-8 bytes, zero-padded
        let addr_bytes = self.address.as_bytes();
        let addr_len = addr_bytes.len().min(63);
        buf[57..57 + addr_len].copy_from_slice(&addr_bytes[..addr_len]);

        buf
    }

    /// Deserialize a User from a 128-byte disk buffer produced by [`to_bytes`](Self::to_bytes).
    ///
    /// Each string field is read from its fixed byte window, stopping at the first null byte
    /// (the zero-padding written by `to_bytes`). Non-UTF-8 bytes are replaced with `U+FFFD`.
    pub fn from_bytes(buf: &[u8; RECORD_SIZE]) -> Self {
        let id = u64::from_le_bytes(buf[0..8].try_into().unwrap());

        // Read name up to the first null byte within its window
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
