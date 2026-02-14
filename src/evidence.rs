//! Evidence and replay interfaces for Phase 4 Settlement/Dispute.
//!
//! Extension points for:
//! - Hashing tx slices and evidence bundles
//! - Loading tx slices from storage (Phase 4)
//! - Replay summary for dispute resolution (Phase 4)
//!
//! See `.local/phase4_spec.md` for the Phase 4 EvidenceBundle schema.

use crate::{sha256_digest, tx::SignedTx};

/// SHA256 hash of data, lowercase hex. Used for evidence hashing and capability IDs.
pub fn evidence_hash(data: &[u8]) -> String {
    hex::encode(sha256_digest(data)).to_lowercase()
}

/// Hash of a tx slice for evidence bundle. Phase 4 will define the canonical serialization.
pub fn tx_slice_hash(txs: &[SignedTx]) -> String {
    let bytes: Vec<u8> = txs
        .iter()
        .flat_map(|tx| bincode::serialize(tx).unwrap_or_default())
        .collect();
    evidence_hash(&bytes)
}

/// Placeholder for Phase 4: replay summary (tx count, totals, state hash).
/// Settlement and Dispute contexts will use this for deterministic evidence verification.
#[derive(Debug, Clone)]
pub struct ReplaySummary {
    pub tx_count: u64,
    pub from_tx_id: u64,
}

impl ReplaySummary {
    pub fn new(from_tx_id: u64, tx_count: u64) -> Self {
        ReplaySummary {
            tx_count,
            from_tx_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tx::{SignedTx, Transaction};

    #[test]
    fn test_evidence_hash_deterministic() {
        let data = b"hello";
        let h1 = evidence_hash(data);
        let h2 = evidence_hash(data);
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
    }

    #[test]
    fn test_tx_slice_hash_deterministic() {
        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Mint {
                to: "bob".to_string(),
                amount: 100,
            },
        );
        let txs = vec![tx.clone(), tx.clone()];
        let h1 = tx_slice_hash(&txs);
        let h2 = tx_slice_hash(&txs);
        assert_eq!(h1, h2);
    }
}
