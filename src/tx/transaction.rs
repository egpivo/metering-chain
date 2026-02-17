use serde::{Deserialize, Serialize};

/// Payload version: absent or 1 = v1 (legacy), 2 = v2 (Phase 3 delegation).
pub const PAYLOAD_VERSION_V1: u8 = 1;
pub const PAYLOAD_VERSION_V2: u8 = 2;

/// Pricing model for consumption transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Pricing {
    /// Cost per unit: total_cost = units × unit_price
    UnitPrice(u64),
    /// Fixed total cost regardless of units
    FixedCost(u64),
}

/// Domain command types (metering-first)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Transaction {
    /// Create new funds (authority-only)
    Mint { to: String, amount: u64 },
    /// Create a new meter for a service
    OpenMeter {
        owner: String,
        service_id: String,
        deposit: u64,
    },
    /// Record usage and deduct cost
    Consume {
        owner: String,
        service_id: String,
        units: u64,
        pricing: Pricing,
    },
    /// Close a meter and return locked deposit
    CloseMeter { owner: String, service_id: String },
    /// Revoke a delegation capability (owner-signed). Apply adds capability_id to revoked set.
    RevokeDelegation {
        owner: String,
        capability_id: String,
    },
    // --- Phase 4A: Settlement ---
    /// Propose settlement for a window (operator/protocol-signed).
    ProposeSettlement {
        owner: String,
        service_id: String,
        window_id: String,
        from_tx_id: u64,
        to_tx_id: u64,
        gross_spent: u64,
        operator_share: u64,
        protocol_fee: u64,
        reserve_locked: u64,
        evidence_hash: String,
    },
    /// Finalize settlement (challenge window elapsed; 4A: no window, immediate).
    FinalizeSettlement {
        owner: String,
        service_id: String,
        window_id: String,
    },
    /// Submit claim for operator payable (operator-signed).
    SubmitClaim {
        operator: String,
        owner: String,
        service_id: String,
        window_id: String,
        claim_amount: u64,
    },
    /// Pay a pending claim (protocol/admin-signed).
    PayClaim {
        operator: String,
        owner: String,
        service_id: String,
        window_id: String,
    },
    // --- Phase 4B: Dispute ---
    /// Open a dispute on a finalized settlement (freezes payouts).
    OpenDispute {
        owner: String,
        service_id: String,
        window_id: String,
        reason_code: String,
        evidence_hash: String,
    },
    /// Resolve an open dispute (verdict: Upheld or Dismissed).
    ResolveDispute {
        owner: String,
        service_id: String,
        window_id: String,
        verdict: DisputeVerdict,
    },
}

/// Verdict for ResolveDispute (Phase 4B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeVerdict {
    /// Challenger wins; settlement stays blocked.
    Upheld,
    /// Settlement upheld; payouts can resume.
    Dismissed,
}

/// Payload V1: canonical signing (signer + nonce + kind). Used for legacy and owner-signed tx.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignablePayload {
    pub signer: String,
    pub nonce: u64,
    #[serde(rename = "kind")]
    pub kind: Transaction,
}

/// Payload V2: includes delegation fields. Required for delegated Consume.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PayloadV2 {
    pub payload_version: u8,
    pub signer: String,
    pub nonce: u64,
    pub nonce_account: Option<String>,
    pub valid_at: Option<u64>,
    pub delegation_proof: Option<Vec<u8>>,
    #[serde(rename = "kind")]
    pub kind: Transaction,
}

/// Phase 1 tx.log layout (no signature field). Used only for bincode backward compatibility.
#[derive(Deserialize)]
struct SignedTxLegacy {
    pub signer: String,
    pub nonce: u64,
    pub kind: Transaction,
}

/// Phase 2 tx.log layout (signer, nonce, kind, signature). Used for backward compatibility.
#[derive(Deserialize)]
struct SignedTxPhase2 {
    pub signer: String,
    pub nonce: u64,
    pub kind: Transaction,
    #[serde(default)]
    pub signature: Option<Vec<u8>>,
}

impl From<SignedTxLegacy> for SignedTx {
    fn from(l: SignedTxLegacy) -> Self {
        SignedTx {
            payload_version: None,
            signer: l.signer,
            nonce: l.nonce,
            nonce_account: None,
            valid_at: None,
            delegation_proof: None,
            kind: l.kind,
            signature: None,
        }
    }
}

impl From<SignedTxPhase2> for SignedTx {
    fn from(p: SignedTxPhase2) -> Self {
        SignedTx {
            payload_version: None,
            signer: p.signer,
            nonce: p.nonce,
            nonce_account: None,
            valid_at: None,
            delegation_proof: None,
            kind: p.kind,
            signature: p.signature,
        }
    }
}

/// Signed transaction wrapper (Phase 2 + Phase 3 v2 fields).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedTx {
    /// Absent or 1 = v1 payload; 2 = v2. Delegated Consume must use 2.
    #[serde(default)]
    pub payload_version: Option<u8>,
    pub signer: String,
    pub nonce: u64,
    /// Some(owner) for delegated consume (owner-nonce).
    #[serde(default)]
    pub nonce_account: Option<String>,
    /// Reference time for delegation expiry (required for delegated consume).
    #[serde(default)]
    pub valid_at: Option<u64>,
    /// UCAN/ReCap proof bytes (required for delegated consume).
    #[serde(default)]
    pub delegation_proof: Option<Vec<u8>>,
    pub kind: Transaction,
    /// Ed25519 signature over canonical payload. None = legacy/unsigned (Phase 1 replay).
    #[serde(default)]
    pub signature: Option<Vec<u8>>,
}

impl SignedTx {
    pub fn new(signer: String, nonce: u64, kind: Transaction) -> Self {
        SignedTx {
            payload_version: None,
            signer,
            nonce,
            nonce_account: None,
            valid_at: None,
            delegation_proof: None,
            kind,
            signature: None,
        }
    }

    /// True if this is a delegated Consume (signer != owner or has delegation_proof).
    pub fn is_delegated_consume(&self) -> bool {
        match &self.kind {
            Transaction::Consume { owner, .. } => {
                self.signer != *owner || self.delegation_proof.is_some()
            }
            _ => false,
        }
    }

    /// Effective payload version: None/Some(1) => v1, Some(2) => v2.
    pub fn effective_payload_version(&self) -> u8 {
        match self.payload_version {
            Some(2) => PAYLOAD_VERSION_V2,
            _ => PAYLOAD_VERSION_V1,
        }
    }

    /// Canonical bytes to sign. Uses PayloadV1 or PayloadV2 per effective_payload_version.
    pub fn message_to_sign(&self) -> crate::error::Result<Vec<u8>> {
        if self.effective_payload_version() == PAYLOAD_VERSION_V2 {
            let payload = PayloadV2 {
                payload_version: PAYLOAD_VERSION_V2,
                signer: self.signer.clone(),
                nonce: self.nonce,
                nonce_account: self.nonce_account.clone(),
                valid_at: self.valid_at,
                delegation_proof: self.delegation_proof.clone(),
                kind: self.kind.clone(),
            };
            bincode::serialize(&payload)
                .map_err(|e| crate::error::Error::InvalidTransaction(e.to_string()))
        } else {
            let payload = SignablePayload {
                signer: self.signer.clone(),
                nonce: self.nonce,
                kind: self.kind.clone(),
            };
            bincode::serialize(&payload)
                .map_err(|e| crate::error::Error::InvalidTransaction(e.to_string()))
        }
    }
}

/// Deserialize SignedTx from bincode. Tries full SignedTx (Phase 3), then Phase 2 layout (signer, nonce, kind, signature), then Phase 1 (no signature).
pub fn deserialize_signed_tx_bincode(bytes: &[u8]) -> crate::error::Result<SignedTx> {
    if let Ok(tx) = bincode::deserialize::<SignedTx>(bytes) {
        return Ok(tx);
    }
    if let Ok(p2) = bincode::deserialize::<SignedTxPhase2>(bytes) {
        return Ok(p2.into());
    }
    let legacy = bincode::deserialize::<SignedTxLegacy>(bytes)
        .map_err(|e| crate::error::Error::StateError(format!("Failed to deserialize tx: {}", e)))?;
    Ok(legacy.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_legacy_bincode() {
        let payload = SignablePayload {
            signer: "alice".to_string(),
            nonce: 0,
            kind: Transaction::Mint {
                to: "bob".to_string(),
                amount: 100,
            },
        };
        let bytes = bincode::serialize(&payload).unwrap();
        let tx = deserialize_signed_tx_bincode(&bytes).unwrap();
        assert_eq!(tx.signer, "alice");
        assert_eq!(tx.nonce, 0);
        assert!(matches!(tx.kind, Transaction::Mint { .. }));
        assert!(tx.signature.is_none());
    }
}
