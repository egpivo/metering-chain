//! Replay service: load state from storage and replay transaction log to tip.
//!
//! Used by CLI and tests. Phase 4 Settlement/Dispute will use tx-slice loading for evidence.

use crate::error::Result;
use crate::state::{apply, State};
use crate::storage::Storage;
use crate::tx::validation::ValidationContext;
use crate::tx::SignedTx;
use crate::wallet;

/// Replay transaction log from storage to current tip.
///
/// - If a snapshot exists: load it, then replay txs from `last_tx_id + 1`.
/// - If no snapshot: replay from genesis (tx_id 0).
/// - Uses `ValidationContext::replay()` and `None` for minters (replay mode).
/// - Verifies signatures for signed txs to ensure log integrity.
pub fn replay_to_tip<S: Storage>(storage: &S) -> Result<(State, u64)> {
    let replay_ctx = ValidationContext::replay();
    match storage.load_state()? {
        Some((snapshot_state, snapshot_tx_id)) => {
            let txs_after_snapshot = storage.load_txs_from(snapshot_tx_id)?;
            let mut current_state = snapshot_state;
            let mut current_tx_id = snapshot_tx_id;
            for tx in txs_after_snapshot {
                if tx.signature.is_some() {
                    wallet::verify_signature(&tx)?;
                }
                current_state = apply(&current_state, &tx, &replay_ctx, None)?;
                current_tx_id += 1;
            }
            Ok((current_state, current_tx_id))
        }
        None => {
            let all_txs = storage.load_txs_from(0)?;
            let mut current_state = State::new();
            let mut current_tx_id = 0u64;
            for tx in all_txs {
                if tx.signature.is_some() {
                    wallet::verify_signature(&tx)?;
                }
                current_state = apply(&current_state, &tx, &replay_ctx, None)?;
                current_tx_id += 1;
            }
            Ok((current_state, current_tx_id))
        }
    }
}

/// Load a tx slice from storage for evidence bundle (Phase 4).
pub fn load_tx_slice<S: Storage>(storage: &S, from_tx_id: u64) -> Result<Vec<SignedTx>> {
    storage.load_txs_from(from_tx_id)
}
