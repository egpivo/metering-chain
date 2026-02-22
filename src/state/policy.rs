//! Phase 4C (G3): Policy aggregate and resolution.
//! See .local/phase4_spec.md Phase 4C.

use serde::{Deserialize, Serialize};

const BPS_MAX: u16 = 10_000;

/// Policy scope; precedence OwnerService > Owner > Global.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyScope {
    Global,
    Owner { owner: String },
    OwnerService { owner: String, service_id: String },
}

impl PolicyScope {
    /// Stable key for storage and resolution ordering.
    pub fn scope_key(&self) -> String {
        match self {
            PolicyScope::Global => "global".to_string(),
            PolicyScope::Owner { owner } => format!("owner:{}", owner),
            PolicyScope::OwnerService { owner, service_id } => {
                format!("owner_service:{}:{}", owner, service_id)
            }
        }
    }

    /// Precedence order for resolution: higher index = higher precedence.
    /// Chain for (owner, service_id): [OwnerService, Owner, Global].
    pub fn scope_chain(owner: &str, service_id: &str) -> Vec<PolicyScope> {
        vec![
            PolicyScope::OwnerService {
                owner: owner.to_string(),
                service_id: service_id.to_string(),
            },
            PolicyScope::Owner {
                owner: owner.to_string(),
            },
            PolicyScope::Global,
        ]
    }
}

/// Fee split in basis points (10_000 = 100%). operator_share_bps + protocol_fee_bps == 10_000.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeePolicy {
    pub operator_share_bps: u16,
    pub protocol_fee_bps: u16,
}

impl FeePolicy {
    pub fn validate(&self) -> bool {
        self.operator_share_bps
            .saturating_add(self.protocol_fee_bps)
            == BPS_MAX
    }

    /// Compute operator_share and protocol_fee from gross_spent (integer division).
    pub fn split(&self, gross_spent: u64) -> (u64, u64) {
        let op = (gross_spent * self.operator_share_bps as u64) / BPS_MAX as u64;
        let proto = (gross_spent * self.protocol_fee_bps as u64) / BPS_MAX as u64;
        (op, proto)
    }
}

/// Reserve policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReservePolicy {
    None,
    Fixed { amount: u64 },
    Bps { reserve_bps: u16 },
}

/// Dispute policy: challenge window in seconds.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DisputePolicy {
    pub dispute_window_secs: u64,
}

/// Full policy config (G3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyConfig {
    pub fee_policy: FeePolicy,
    pub reserve_policy: ReservePolicy,
    pub dispute_policy: DisputePolicy,
}

impl PolicyConfig {
    pub fn validate(&self) -> bool {
        self.fee_policy.validate() && self.dispute_policy.dispute_window_secs > 0
    }

    /// Compute reserve_locked from gross_spent for this config.
    pub fn reserve_from_gross(&self, gross_spent: u64) -> u64 {
        match &self.reserve_policy {
            ReservePolicy::None => 0,
            ReservePolicy::Fixed { amount } => *amount,
            ReservePolicy::Bps { reserve_bps } => {
                (gross_spent * *reserve_bps as u64) / BPS_MAX as u64
            }
        }
    }
}

/// Policy version status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyVersionStatus {
    Draft,
    Published,
    Superseded,
}

/// Policy version aggregate identity: (scope_key, version).
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyVersionId {
    pub scope_key: String,
    pub version: u64,
}

impl PolicyVersionId {
    pub fn key(&self) -> String {
        format!("{}:{}", self.scope_key, self.version)
    }
}

/// Policy version aggregate (G3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyVersion {
    /// Schema version for this record; reader must support <= current.
    #[serde(default)]
    pub schema_version: u16,
    pub id: PolicyVersionId,
    pub scope: PolicyScope,
    pub effective_from_tx_id: u64,
    pub published_by: String,
    pub published_at: u64,
    pub config: PolicyConfig,
    pub status: PolicyVersionStatus,
}

impl PolicyVersion {
    pub fn is_published(&self) -> bool {
        self.status == PolicyVersionStatus::Published
    }

    pub fn is_effective_at(&self, current_tx_id: u64) -> bool {
        self.is_published() && self.effective_from_tx_id <= current_tx_id
    }
}
