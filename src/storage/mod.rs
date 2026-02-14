pub mod kv;

pub use kv::FileStorage;

use crate::error::Result;
use crate::state::State;
use crate::tx::SignedTx;

/// Storage abstraction for append-only transaction logs and state snapshots.
///
/// Implementations must preserve:
/// - Append-only semantics for transaction logs
/// - Atomic snapshot writes (crash-safe)
/// - Deterministic replay from transaction log
pub trait Storage {
    /// Append a transaction to the log (append-only, fsync before ack)
    fn append_tx(&mut self, tx: &SignedTx) -> Result<()>;

    /// Load the latest state snapshot with the next tx to apply (0-indexed position)
    ///
    /// Returns `None` if no snapshot exists (genesis state).
    fn load_state(&self) -> Result<Option<(State, u64)>>;

    /// Persist state snapshot atomically (write to temp file, fsync, rename)
    ///
    /// `next_tx_id` is the 0-indexed position of the next transaction to apply.
    fn persist_state(&mut self, state: &State, next_tx_id: u64) -> Result<()>;

    /// Load transactions from the log starting from `from_tx_id` (inclusive; 0-indexed position)
    ///
    /// Transaction IDs are sequential (0, 1, 2, ...).
    /// Returns all transactions from `from_tx_id` to the end of the log.
    fn load_txs_from(&self, from_tx_id: u64) -> Result<Vec<SignedTx>>;
}
