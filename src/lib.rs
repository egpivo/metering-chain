pub mod chain;
pub mod tx;
pub mod state;
pub mod storage;
pub mod error;
pub mod logger;
pub mod config;

use sha2::{Sha256, Digest};

/// Get current Unix timestamp
pub fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Calculate SHA256 digest
pub fn sha256_digest(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}
