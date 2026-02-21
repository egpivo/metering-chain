//! Replay service: load state from storage and replay transaction log to tip.
//!
//! G4: replay_tx_slice and replay_slice_to_summary for evidence-backed ResolveDispute.

use crate::error::Result;
use crate::evidence::{ReplaySummary, tx_slice_hash};
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
    match storage.load_state()? {
        Some((snapshot_state, next_tx_id)) => {
            let txs_to_apply = storage.load_txs_from(next_tx_id)?;
            let mut current_state = snapshot_state;
            let mut next_id = next_tx_id;
            for tx in txs_to_apply {
                if tx.signature.is_some() {
                    wallet::verify_signature(&tx)?;
                }
                let ctx = ValidationContext::replay_for_tx(next_id);
                current_state = apply(&current_state, &tx, &ctx, None)?;
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
                let ctx = ValidationContext::replay_for_tx(next_id);
                current_state = apply(&current_state, &tx, &ctx, None)?;
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
    let to_apply: Vec<_> = all_txs.into_iter().take(up_to_tx_id as usize).collect();
    let mut state = State::new();
    for (i, tx) in to_apply.into_iter().enumerate() {
        if tx.signature.is_some() {
            wallet::verify_signature(&tx)?;
        }
        let ctx = ValidationContext::replay_for_tx(i as u64);
        state = apply(&state, &tx, &ctx, None)?;
    }
    Ok(state)
}

/// Replay a slice of txs starting from given state; first tx has index start_tx_id (G4).
pub fn replay_tx_slice(
    state: &State,
    txs: &[SignedTx],
    start_tx_id: u64,
) -> Result<State> {
    let mut s = state.clone();
    for (i, tx) in txs.iter().enumerate() {
        if tx.signature.is_some() {
            wallet::verify_signature(tx)?;
        }
        let ctx = ValidationContext::replay_for_tx(start_tx_id.saturating_add(i as u64));
        s = apply(&s, tx, &ctx, None)?;
    }
    Ok(s)
}

/// Replay the settlement window from storage and build ReplaySummary (G4).
///
/// **Replay-derived**: `gross_spent` (from meter delta), `from_tx_id`, `to_tx_id`, `tx_count`, and
/// the returned `evidence_hash` (hash of the tx slice). **Taken from settlement for comparison**:
/// `operator_share`, `protocol_fee`, `reserve_locked` — these are not recomputed from policy here;
/// the resolver passes the settlement's recorded split so the summary can be compared to the
/// settlement in ResolveDispute. Validation enforces that the summary's from/to match the
/// settlement's window and that evidence_hash in the tx matches the settlement.
///
/// Returns (ReplaySummary, evidence_hash of the tx slice) for the window [from_tx_id, to_tx_id).
pub fn replay_slice_to_summary<S: Storage>(
    storage: &S,
    from_tx_id: u64,
    to_tx_id: u64,
    owner: &str,
    service_id: &str,
    operator_share: u64,
    protocol_fee: u64,
    reserve_locked: u64,
) -> Result<(ReplaySummary, String)> {
    let state_from = replay_up_to(storage, from_tx_id)?;
    let txs = storage.load_txs_from(from_tx_id)?;
    let window_txs: Vec<_> = txs.into_iter().take((to_tx_id.saturating_sub(from_tx_id)) as usize).collect();
    let tx_count = window_txs.len() as u64;
    let state_to = replay_tx_slice(&state_from, &window_txs, from_tx_id)?;
    let spent_from = state_from
        .get_meter(owner, service_id)
        .map(|m| m.total_spent())
        .unwrap_or(0);
    let spent_to = state_to
        .get_meter(owner, service_id)
        .map(|m| m.total_spent())
        .unwrap_or(0);
    let gross_spent = spent_to.saturating_sub(spent_from);
    let evidence_hash = tx_slice_hash(&window_txs);
    let summary = ReplaySummary::new(
        from_tx_id,
        to_tx_id,
        tx_count,
        gross_spent,
        operator_share,
        protocol_fee,
        reserve_locked,
    );
    Ok((summary, evidence_hash))
}
