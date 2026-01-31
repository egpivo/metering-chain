use serde::{Deserialize, Serialize};

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
}

/// Payload used for canonical signing (signer + nonce + kind, no signature).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignablePayload {
    pub signer: String,
    pub nonce: u64,
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

impl From<SignedTxLegacy> for SignedTx {
    fn from(l: SignedTxLegacy) -> Self {
        SignedTx {
            signer: l.signer,
            nonce: l.nonce,
            kind: l.kind,
            signature: None,
        }
    }
}

/// Signed transaction wrapper with signer, nonce, and optional signature (Phase 2).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedTx {
    pub signer: String,
    pub nonce: u64,
    pub kind: Transaction,
    /// Ed25519 signature over canonical payload. None = legacy/unsigned (Phase 1 replay).
    #[serde(default)]
    pub signature: Option<Vec<u8>>,
}

/// Deserialize SignedTx from bincode; accepts Phase 1 layout (no signature) for backward compatibility.
pub fn deserialize_signed_tx_bincode(bytes: &[u8]) -> crate::error::Result<SignedTx> {
    match bincode::deserialize::<SignedTx>(bytes) {
        Ok(tx) => Ok(tx),
        Err(_) => {
            let legacy =
                bincode::deserialize::<SignedTxLegacy>(bytes).map_err(|e| {
                    crate::error::Error::StateError(format!("Failed to deserialize tx: {}", e))
                })?;
            Ok(legacy.into())
        }
    }
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

impl SignedTx {
    pub fn new(signer: String, nonce: u64, kind: Transaction) -> Self {
        SignedTx {
            signer,
            nonce,
            kind,
            signature: None,
        }
    }

    /// Canonical bytes to sign (bincode of signer, nonce, kind). Verification must use the same format.
    pub fn message_to_sign(&self) -> crate::error::Result<Vec<u8>> {
        let payload = SignablePayload {
            signer: self.signer.clone(),
            nonce: self.nonce,
            kind: self.kind.clone(),
        };
        bincode::serialize(&payload)
            .map_err(|e| crate::error::Error::InvalidTransaction(e.to_string()))
    }
}
