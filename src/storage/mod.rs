pub mod kv;

pub use kv::FileStorage;

use crate::state::State;
use crate::tx::SignedTx;
use crate::error::Result;

/// Storage abstraction for append-only transaction logs and state snapshots.
///
/// Implementations must preserve:
/// - Append-only semantics for transaction logs
/// - Atomic snapshot writes (crash-safe)
/// - Deterministic replay from transaction log
pub trait Storage {
    /// Append a transaction to the log (append-only, fsync before ack)
    fn append_tx(&mut self, tx: &SignedTx) -> Result<()>;

    /// Load the latest state snapshot with the last applied transaction ID
    ///
    /// Returns `None` if no snapshot exists (genesis state).
    fn load_state(&self) -> Result<Option<(State, u64)>>;

    /// Persist state snapshot atomically (write to temp file, fsync, rename)
    ///
    /// `last_tx_id` is the sequential ID of the last transaction applied to this state.
    fn persist_state(&mut self, state: &State, last_tx_id: u64) -> Result<()>;

    /// Load transactions from the log starting from `from_tx_id` (inclusive)
    ///
    /// Transaction IDs are sequential (0, 1, 2, ...).
    /// Returns all transactions from `from_tx_id` to the end of the log.
    fn load_txs_from(&self, from_tx_id: u64) -> Result<Vec<SignedTx>>;
}
