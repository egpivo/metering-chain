//! Evidence and replay interfaces for Phase 4 Settlement/Dispute (G4).
//!
//! See `.local/phase4_spec.md` and `.local/phase4_g4_tasks.md` for EvidenceBundle schema.
//! Versioning: schema_version and replay_protocol_version for deterministic reject paths (Phase 4+ hardening).

use crate::error::{Error, Result};

/// Current evidence bundle schema version. Readers must support <= this.
pub const CURRENT_EVIDENCE_SCHEMA_VERSION: u16 = 1;
/// Replay hash/serialization contract version. Mismatch => ReplayProtocolMismatch.
pub const REPLAY_PROTOCOL_VERSION: u16 = 1;
use crate::sha256_digest;
use crate::tx::SignedTx;
use serde::{Deserialize, Serialize};

/// SHA256 hash of data, lowercase hex. Used for evidence hashing and capability IDs.
pub fn evidence_hash(data: &[u8]) -> String {
    hex::encode(sha256_digest(data)).to_lowercase()
}

/// Hash of a tx slice for evidence bundle (canonical bincode serialization).
pub fn tx_slice_hash(txs: &[SignedTx]) -> String {
    let bytes: Vec<u8> = txs
        .iter()
        .flat_map(|tx| bincode::serialize(tx).unwrap_or_default())
        .collect();
    evidence_hash(&bytes)
}

/// Replay summary for a settlement window: deterministic totals from replay (G4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaySummary {
    pub from_tx_id: u64,
    pub to_tx_id: u64,
    pub tx_count: u64,
    pub gross_spent: u64,
    pub operator_share: u64,
    pub protocol_fee: u64,
    pub reserve_locked: u64,
}

impl ReplaySummary {
    pub fn new(
        from_tx_id: u64,
        to_tx_id: u64,
        tx_count: u64,
        gross_spent: u64,
        operator_share: u64,
        protocol_fee: u64,
        reserve_locked: u64,
    ) -> Self {
        ReplaySummary {
            from_tx_id,
            to_tx_id,
            tx_count,
            gross_spent,
            operator_share,
            protocol_fee,
            reserve_locked,
        }
    }

    /// Canonical bytes for deterministic replay_hash.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deterministic hash of this summary (G4 replay proof).
    pub fn replay_hash(&self) -> String {
        evidence_hash(&self.canonical_bytes())
    }
}

/// Evidence bundle for a settlement window (G4): reference data for replay-justified resolve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    /// Schema version for this record; reader must support <= CURRENT_EVIDENCE_SCHEMA_VERSION.
    #[serde(default)]
    pub schema_version: u16,
    /// Replay protocol version (hash/canonicalization contract). Mismatch => ReplayProtocolMismatch.
    #[serde(default)]
    pub replay_protocol_version: u16,
    pub settlement_key: String,
    pub from_tx_id: u64,
    pub to_tx_id: u64,
    pub evidence_hash: String,
    pub replay_hash: String,
    pub replay_summary: ReplaySummary,
}

impl EvidenceBundle {
    /// Canonical bytes for bundle hash.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap_or_default()
    }

    /// Deterministic bundle hash.
    pub fn bundle_hash(&self) -> String {
        evidence_hash(&self.canonical_bytes())
    }

    /// Validate shape and sanity (required fields, from_tx_id < to_tx_id, tx_count consistent).
    /// Rejects unsupported schema or replay protocol version with deterministic error codes.
    pub fn validate_shape(&self) -> Result<()> {
        if self.schema_version > CURRENT_EVIDENCE_SCHEMA_VERSION {
            return Err(Error::UnsupportedSchemaVersion);
        }
        if self.replay_protocol_version != REPLAY_PROTOCOL_VERSION {
            return Err(Error::ReplayProtocolMismatch);
        }
        if self.settlement_key.is_empty() {
            return Err(Error::InvalidEvidenceBundle);
        }
        if self.from_tx_id >= self.to_tx_id {
            return Err(Error::InvalidEvidenceBundle);
        }
        if self.evidence_hash.is_empty() || self.replay_hash.is_empty() {
            return Err(Error::InvalidEvidenceBundle);
        }
        let expected_count = self.to_tx_id.saturating_sub(self.from_tx_id);
        if self.replay_summary.tx_count != expected_count {
            return Err(Error::InvalidEvidenceBundle);
        }
        if self.replay_summary.from_tx_id != self.from_tx_id
            || self.replay_summary.to_tx_id != self.to_tx_id
        {
            return Err(Error::InvalidEvidenceBundle);
        }
        Ok(())
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

    #[test]
    fn test_replay_summary_replay_hash_deterministic() {
        let s = ReplaySummary::new(0, 3, 3, 50, 45, 5, 0);
        assert_eq!(s.replay_hash(), s.replay_hash());
    }

    #[test]
    fn test_evidence_bundle_validate_shape() {
        let summary = ReplaySummary::new(0, 2, 2, 10, 9, 1, 0);
        let bundle = EvidenceBundle {
            schema_version: CURRENT_EVIDENCE_SCHEMA_VERSION,
            replay_protocol_version: REPLAY_PROTOCOL_VERSION,
            settlement_key: "a:b:w".to_string(),
            from_tx_id: 0,
            to_tx_id: 2,
            evidence_hash: "eh".to_string(),
            replay_hash: summary.replay_hash(),
            replay_summary: summary,
        };
        assert!(bundle.validate_shape().is_ok());
        let bad = EvidenceBundle {
            from_tx_id: 1,
            to_tx_id: 1,
            ..bundle.clone()
        };
        assert!(bad.validate_shape().is_err());
    }

    #[test]
    fn test_evidence_bundle_unsupported_schema_version_rejected() {
        let summary = ReplaySummary::new(0, 2, 2, 10, 9, 1, 0);
        let bundle = EvidenceBundle {
            schema_version: 99,
            replay_protocol_version: REPLAY_PROTOCOL_VERSION,
            settlement_key: "a:b:w".to_string(),
            from_tx_id: 0,
            to_tx_id: 2,
            evidence_hash: "eh".to_string(),
            replay_hash: summary.replay_hash(),
            replay_summary: summary,
        };
        let err = bundle.validate_shape().unwrap_err();
        assert!(matches!(err, Error::UnsupportedSchemaVersion));
    }

    #[test]
    fn test_evidence_bundle_replay_protocol_mismatch_rejected() {
        let summary = ReplaySummary::new(0, 2, 2, 10, 9, 1, 0);
        let bundle = EvidenceBundle {
            schema_version: CURRENT_EVIDENCE_SCHEMA_VERSION,
            replay_protocol_version: 999,
            settlement_key: "a:b:w".to_string(),
            from_tx_id: 0,
            to_tx_id: 2,
            evidence_hash: "eh".to_string(),
            replay_hash: summary.replay_hash(),
            replay_summary: summary,
        };
        let err = bundle.validate_shape().unwrap_err();
        assert!(matches!(err, Error::ReplayProtocolMismatch));
    }

    /// Old serialized EvidenceBundle (no schema_version/replay_protocol_version) deserializes with 0,0 and is rejected by current reader.
    #[test]
    fn test_evidence_bundle_v0_protocol_rejected() {
        let summary = ReplaySummary::new(0, 2, 2, 10, 9, 1, 0);
        let bundle = EvidenceBundle {
            schema_version: 0,
            replay_protocol_version: 0,
            settlement_key: "a:b:w".to_string(),
            from_tx_id: 0,
            to_tx_id: 2,
            evidence_hash: "eh".to_string(),
            replay_hash: summary.replay_hash(),
            replay_summary: summary,
        };
        let err = bundle.validate_shape().unwrap_err();
        assert!(matches!(err, Error::ReplayProtocolMismatch));
    }
}
