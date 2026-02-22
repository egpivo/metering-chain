//! Phase 4A: Settlement aggregate and status.
//!
//! See .local/phase4_spec.md for the full domain model. G4: ResolutionAudit on Dispute.

use crate::evidence::ReplaySummary;
use serde::{Deserialize, Serialize};

/// Settlement status lifecycle (Phase 4A: Dispute is stub).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementStatus {
    /// Window computed, awaiting challenge period (4A: finalize immediately).
    Proposed,
    /// Economically final; payouts allowed.
    Finalized,
    /// At least one claim paid.
    Claimed,
    /// Dispute opened (4A: status only, no resolution flow).
    Disputed,
    /// Dispute closed (4B).
    Resolved,
}

/// Claim status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimStatus {
    /// Claim submitted, payment not executed.
    Pending,
    /// Payout executed and recorded.
    Paid,
    /// Claim rejected.
    Rejected,
}

/// Settlement aggregate identity: (owner, service_id, settlement_window_id).
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementId {
    pub owner: String,
    pub service_id: String,
    pub window_id: String,
}

impl SettlementId {
    pub fn new(owner: String, service_id: String, window_id: String) -> Self {
        SettlementId {
            owner,
            service_id,
            window_id,
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.owner, self.service_id, self.window_id)
    }
}

/// Settlement aggregate (Phase 4A). Phase 4C (G3): optional policy snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settlement {
    /// Schema version for this record; reader must support <= current.
    #[serde(default)]
    pub schema_version: u16,
    pub id: SettlementId,
    pub gross_spent: u64,
    pub operator_share: u64,
    pub protocol_fee: u64,
    pub reserve_locked: u64,
    pub status: SettlementStatus,
    pub evidence_hash: String,
    /// Tx range for evidence (from_tx_id inclusive, to_tx_id exclusive).
    pub from_tx_id: u64,
    pub to_tx_id: u64,
    /// Total amount already paid out via claims (cannot exceed operator_share).
    pub total_paid: u64,
    /// G3: bound policy snapshot (when resolved at propose time).
    #[serde(default)]
    pub policy_scope_key: Option<String>,
    #[serde(default)]
    pub policy_version: Option<u64>,
    #[serde(default)]
    pub dispute_window_secs: Option<u64>,
    /// G3: when settlement was finalized (epoch secs); used for dispute window.
    #[serde(default)]
    pub finalized_at: Option<u64>,
}

impl Settlement {
    #[allow(clippy::too_many_arguments)]
    pub fn proposed(
        id: SettlementId,
        gross_spent: u64,
        operator_share: u64,
        protocol_fee: u64,
        reserve_locked: u64,
        evidence_hash: String,
        from_tx_id: u64,
        to_tx_id: u64,
    ) -> Self {
        Settlement {
            schema_version: 1,
            id,
            gross_spent,
            operator_share,
            protocol_fee,
            reserve_locked,
            status: SettlementStatus::Proposed,
            evidence_hash,
            from_tx_id,
            to_tx_id,
            total_paid: 0,
            policy_scope_key: None,
            policy_version: None,
            dispute_window_secs: None,
            finalized_at: None,
        }
    }

    /// G3: set bound policy snapshot (called from apply when policy was resolved).
    pub fn set_bound_policy(&mut self, scope_key: String, version: u64, dispute_window_secs: u64) {
        self.policy_scope_key = Some(scope_key);
        self.policy_version = Some(version);
        self.dispute_window_secs = Some(dispute_window_secs);
    }

    /// G3: set finalized_at (epoch secs) when finalizing; used for dispute window.
    pub fn set_finalized_at(&mut self, at: u64) {
        self.finalized_at = Some(at);
    }

    pub fn payable(&self) -> u64 {
        self.operator_share.saturating_sub(self.total_paid)
    }

    pub fn finalize(&mut self) {
        self.status = SettlementStatus::Finalized;
    }

    pub fn mark_claimed(&mut self) {
        self.status = SettlementStatus::Claimed;
    }

    pub fn add_paid(&mut self, amount: u64) {
        let remaining = self.operator_share.saturating_sub(self.total_paid);
        let to_add = amount.min(remaining);
        self.total_paid = self.total_paid.saturating_add(to_add);
        if self.total_paid >= self.operator_share {
            self.status = SettlementStatus::Claimed;
        }
    }

    pub fn is_finalized(&self) -> bool {
        matches!(
            self.status,
            SettlementStatus::Finalized | SettlementStatus::Claimed
        )
    }

    pub fn is_disputed(&self) -> bool {
        self.status == SettlementStatus::Disputed
    }

    /// Revert to Finalized after dispute dismissed (G2: payouts can resume).
    pub fn reopen_after_dismissed(&mut self) {
        if self.status == SettlementStatus::Disputed {
            self.status = SettlementStatus::Finalized;
        }
    }

    /// Mark settlement as disputed (block payouts).
    pub fn mark_disputed(&mut self) {
        self.status = SettlementStatus::Disputed;
    }
}

/// Claim aggregate identity: (operator, settlement_key).
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimId {
    pub operator: String,
    pub settlement_key: String,
}

impl ClaimId {
    pub fn new(operator: String, settlement_id: &SettlementId) -> Self {
        ClaimId {
            operator,
            settlement_key: settlement_id.key(),
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.operator, self.settlement_key)
    }
}

/// Claim aggregate (Phase 4A).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claim {
    pub id: ClaimId,
    pub claim_amount: u64,
    pub status: ClaimStatus,
}

impl Claim {
    pub fn pending(id: ClaimId, claim_amount: u64) -> Self {
        Claim {
            id,
            claim_amount,
            status: ClaimStatus::Pending,
        }
    }

    pub fn pay(&mut self) {
        self.status = ClaimStatus::Paid;
    }

    pub fn reject(&mut self) {
        self.status = ClaimStatus::Rejected;
    }

    pub fn is_pending(&self) -> bool {
        self.status == ClaimStatus::Pending
    }
}

// --- Phase 4B: Dispute aggregate ---

/// Dispute status (Phase 4B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeStatus {
    /// Dispute opened; payouts frozen for target settlement.
    Open,
    /// Dispute closed in favor of challenger (settlement corrected or blocked).
    Upheld,
    /// Dispute closed in favor of settlement (payouts can resume).
    Dismissed,
}

/// Dispute aggregate identity: one open dispute per settlement (key = settlement_key).
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeId {
    pub settlement_key: String,
}

impl DisputeId {
    pub fn new(settlement_id: &SettlementId) -> Self {
        DisputeId {
            settlement_key: settlement_id.key(),
        }
    }

    pub fn key(&self) -> &str {
        &self.settlement_key
    }
}

/// Resolution audit for a dispute (G4): replay proof persisted when dispute is resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolutionAudit {
    /// Replay protocol version used for this resolution (audit trail).
    #[serde(default)]
    pub replay_protocol_version: u16,
    pub replay_hash: String,
    pub replay_summary: ReplaySummary,
}

/// Dispute aggregate (Phase 4B). G4: optional resolution_audit when resolved.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dispute {
    /// Schema version for this record; reader must support <= current.
    #[serde(default)]
    pub schema_version: u16,
    pub id: DisputeId,
    pub target_settlement_id: SettlementId,
    pub reason_code: String,
    pub evidence_hash: String,
    /// Epoch secs when dispute was opened (0 if not used).
    pub opened_at: u64,
    pub status: DisputeStatus,
    /// G4: set when dispute is resolved (replay-justified).
    #[serde(default)]
    pub resolution_audit: Option<ResolutionAudit>,
}

impl Dispute {
    pub fn open(
        target_settlement_id: SettlementId,
        reason_code: String,
        evidence_hash: String,
        opened_at: u64,
    ) -> Self {
        let id = DisputeId::new(&target_settlement_id);
        Dispute {
            schema_version: 1,
            id,
            target_settlement_id,
            reason_code,
            evidence_hash,
            opened_at,
            status: DisputeStatus::Open,
            resolution_audit: None,
        }
    }

    /// G4: set resolution audit when resolving with replay-justified evidence.
    pub fn set_resolution_audit(&mut self, audit: ResolutionAudit) {
        self.resolution_audit = Some(audit);
    }

    pub fn is_open(&self) -> bool {
        self.status == DisputeStatus::Open
    }

    pub fn resolve(&mut self, verdict: DisputeStatus) {
        debug_assert!(matches!(
            verdict,
            DisputeStatus::Upheld | DisputeStatus::Dismissed
        ));
        self.status = verdict;
    }
}
