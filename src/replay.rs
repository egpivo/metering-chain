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
/// - If a snapshot exists: load it, then replay txs from `next_tx_id` (0-indexed position of next tx).
/// - If no snapshot: replay from genesis (position 0).
/// - Uses `ValidationContext::replay()` and `None` for minters (replay mode).
/// - Verifies signatures for signed txs to ensure log integrity.
pub fn replay_to_tip<S: Storage>(storage: &S) -> Result<(State, u64)> {
    let replay_ctx = ValidationContext::replay();
    match storage.load_state()? {
        Some((snapshot_state, next_tx_id)) => {
            let txs_to_apply = storage.load_txs_from(next_tx_id)?;
            let mut current_state = snapshot_state;
            let mut next_id = next_tx_id;
            for tx in txs_to_apply {
                if tx.signature.is_some() {
                    wallet::verify_signature(&tx)?;
                }
                current_state = apply(&current_state, &tx, &replay_ctx, None)?;
                next_id += 1;
            }
            Ok((current_state, next_id))
        }
        None => {
            let all_txs = storage.load_txs_from(0)?;
            let mut current_state = State::new();
            let mut next_id = 0u64;
            for tx in all_txs {
                if tx.signature.is_some() {
                    wallet::verify_signature(&tx)?;
                }
                current_state = apply(&current_state, &tx, &replay_ctx, None)?;
                next_id += 1;
            }
            Ok((current_state, next_id))
        }
    }
}

/// Load a tx slice from storage for evidence bundle (Phase 4).
pub fn load_tx_slice<S: Storage>(storage: &S, from_tx_id: u64) -> Result<Vec<SignedTx>> {
    storage.load_txs_from(from_tx_id)
}

/// Replay txs from genesis up to index up_to_tx_id (exclusive).
/// Returns state after applying txs 0..up_to_tx_id-1. Used for settlement propose.
pub fn replay_up_to<S: Storage>(storage: &S, up_to_tx_id: u64) -> Result<State> {
    let all_txs = storage.load_txs_from(0)?;
    let to_apply: Vec<_> = all_txs
        .into_iter()
        .take(up_to_tx_id as usize)
        .collect();
    let replay_ctx = ValidationContext::replay();
    let mut state = State::new();
    for tx in to_apply {
        if tx.signature.is_some() {
            wallet::verify_signature(&tx)?;
        }
        state = apply(&state, &tx, &replay_ctx, None)?;
    }
    Ok(state)
}
