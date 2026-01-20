use crate::tx::Transaction;

#[derive(Debug, Clone)]
pub struct Block {
    pub timestamp: i64,
    pub pre_block_hash: String,
    pub hash: String,
    pub transactions: Vec<Transaction>,
    pub nonce: i64,
    pub height: usize,
}

impl Block {
    pub fn new_block(
        pre_block_hash: String,
        transactions: &[Transaction],
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
}
