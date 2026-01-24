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

/// Signed transaction wrapper with signer and nonce
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedTx {
    pub signer: String,
    pub nonce: u64,
    pub kind: Transaction,
    // TODO: Add signature field when implementing cryptographic signatures
    // pub signature: Vec<u8>,
}

impl SignedTx {
    pub fn new(signer: String, nonce: u64, kind: Transaction) -> Self {
        SignedTx {
            signer,
            nonce,
            kind,
        }
    }
}
