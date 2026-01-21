use crate::tx::SignedTx;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub timestamp: i64,
    pub pre_block_hash: String,
    pub hash: String,
    pub transactions: Vec<SignedTx>,
    pub nonce: i64,
    pub height: usize,
}

impl Block {
    pub fn new_block(
        pre_block_hash: String,
        transactions: &[SignedTx],
        height: usize,
    ) -> Block {
        Block {
            timestamp: crate::current_timestamp(),
            pre_block_hash,
            hash: String::new(),
            transactions: transactions.to_vec(),
            nonce: 0,
            height,
        }
    }

    /// Serialize block into compact bytes (for storage/network).
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize block from bytes (for storage/network).
    pub fn from_bytes(data: &[u8]) -> Result<Block, bincode::Error> {
        bincode::deserialize(data)
    }
}
