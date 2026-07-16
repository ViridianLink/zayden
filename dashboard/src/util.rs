use std::fmt::Write as _;

use sha2::{Digest, Sha256};

pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

pub fn email_hash(email: &str) -> String {
    hex_encode(&Sha256::digest(email.trim().to_lowercase()))
}
