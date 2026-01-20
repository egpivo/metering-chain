use crate::chain::block::Block;
use crate::logger::Logger;
use num_bigint::{BigInt, Sign};
use hex::encode as hex_encode;
use std::borrow::Borrow;

const MAX_NONCE: i64 = i64::MAX;

pub struct ProofOfWork {
    pub block: Block,
    pub target: BigInt,
}

impl ProofOfWork {
    pub fn new(block: Block, target: BigInt) -> Self {
        ProofOfWork { block, target }
    }

    pub fn prepare_data(&self, nonce: i64) -> Vec<u8> {
        // Serialize block data with nonce for hashing
        // Format: pre_block_hash + transactions_hash + timestamp + height + nonce
        let mut data = Vec::new();
        data.extend_from_slice(self.block.pre_block_hash.as_bytes());
        // TODO: Add transactions hash
        data.extend_from_slice(&self.block.timestamp.to_be_bytes());
        data.extend_from_slice(&self.block.height.to_be_bytes());
        data.extend_from_slice(&nonce.to_be_bytes());
        data
    }

    pub fn run(&self) -> (i64, String) {
        let mut nonce = 0;
        let mut hash = Vec::new();
        Logger::info("Mining the block");
        while nonce < MAX_NONCE {
            let data = self.prepare_data(nonce);
            hash = crate::sha256_digest(data.as_slice());
            let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());
            if hash_int.lt(self.target.borrow()) {
                Logger::info(&hex_encode(hash.as_slice()));
                break;
            } else {
                nonce += 1;
            }
        }
        (nonce, hex_encode(hash.as_slice()))
    }
}
