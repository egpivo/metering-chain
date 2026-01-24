use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Mining failed: exhausted nonce range without finding valid hash")]
    MiningExhausted,

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("State error: {0}")]
    StateError(String),
}

pub type Result<T> = std::result::Result<T, Error>;
