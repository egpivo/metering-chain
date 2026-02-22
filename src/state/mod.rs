pub mod account;
pub mod apply;
pub mod hook;
pub mod meter;
pub mod policy;
pub mod settlement;

pub use account::Account;
pub use apply::{apply, StateMachine};
pub use hook::{Hook, NoOpHook};
pub use meter::Meter;
pub use policy::{
    DisputePolicy, FeePolicy, PolicyConfig, PolicyScope, PolicyVersion, PolicyVersionId,
    PolicyVersionStatus, ReservePolicy,
};
pub use settlement::{
    Claim, ClaimId, ClaimStatus, Dispute, DisputeId, DisputeStatus, ResolutionAudit, Settlement,
    SettlementId, SettlementStatus,
};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Meter key: composite key for identifying meters by (owner, service_id)
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeterKey {
    pub owner: String,
    pub service_id: String,
}

impl MeterKey {
    pub fn new(owner: String, service_id: String) -> Self {
        MeterKey { owner, service_id }
    }
}

/// Per-capability cumulative consumption for caveat checks (delegated consume).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityConsumption {
    pub consumed_units: u64,
    pub consumed_cost: u64,
}

/// Core domain state: aggregates all accounts, meters, settlements, claims.
///
/// State is fully reconstructible by replaying transactions from genesis.
/// All state transitions are deterministic and side-effect free.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    /// All accounts indexed by account address/identifier
    pub accounts: HashMap<String, Account>,

    /// All meters indexed by (owner, service_id)
    pub meters: HashMap<MeterKey, Meter>,

    /// Consumed units/cost per capability_id (lowercase hex) for caveat limits.
    #[serde(default)]
    pub capability_consumption: HashMap<String, CapabilityConsumption>,

    /// Revoked capability IDs (owner-signed RevokeDelegation). Delegated Consume with this capability_id is rejected.
    #[serde(default)]
    pub revoked_capability_ids: HashSet<String>,

    /// Phase 4A: Settlements indexed by (owner, service_id, window_id)
    #[serde(default)]
    pub settlements: HashMap<String, Settlement>,

    /// Phase 4A: Claims indexed by (operator, settlement_key)
    #[serde(default)]
    pub claims: HashMap<String, Claim>,

    /// Phase 4B: Disputes indexed by settlement_key (one open dispute per settlement)
    #[serde(default)]
    pub disputes: HashMap<String, Dispute>,

    /// Phase 4C (G3): Policy versions indexed by (scope_key:version)
    #[serde(default)]
    pub policy_versions: HashMap<String, PolicyVersion>,
}

impl State {
    /// Create empty genesis state
    pub fn new() -> Self {
        State {
            accounts: HashMap::new(),
            meters: HashMap::new(),
            capability_consumption: HashMap::new(),
            revoked_capability_ids: HashSet::new(),
            settlements: HashMap::new(),
            claims: HashMap::new(),
            disputes: HashMap::new(),
            policy_versions: HashMap::new(),
        }
    }

    /// Returns true if the capability has been revoked (RevokeDelegation applied).
    pub fn is_capability_revoked(&self, capability_id: &str) -> bool {
        self.revoked_capability_ids.contains(capability_id)
    }

    /// Mark a capability as revoked (used by apply RevokeDelegation).
    pub fn revoke_capability(&mut self, capability_id: String) {
        self.revoked_capability_ids.insert(capability_id);
    }

    /// Get cumulative consumption for a capability_id (0,0 if unknown).
    pub fn get_capability_consumption(&self, capability_id: &str) -> (u64, u64) {
        self.capability_consumption
            .get(capability_id)
            .map(|c| (c.consumed_units, c.consumed_cost))
            .unwrap_or((0, 0))
    }

    /// Record consumption for a capability (add units and cost to cumulative).
    pub fn record_capability_consumption(&mut self, capability_id: String, units: u64, cost: u64) {
        let entry = self
            .capability_consumption
            .entry(capability_id)
            .or_default();
        entry.consumed_units = entry.consumed_units.saturating_add(units);
        entry.consumed_cost = entry.consumed_cost.saturating_add(cost);
    }

    /// Get or create an account (returns mutable reference)
    pub fn get_or_create_account(&mut self, address: &str) -> &mut Account {
        self.accounts.entry(address.to_string()).or_default()
    }

    /// Get account (returns Option)
    pub fn get_account(&self, address: &str) -> Option<&Account> {
        self.accounts.get(address)
    }

    /// Get account mutably (returns Option)
    pub fn get_account_mut(&mut self, address: &str) -> Option<&mut Account> {
        self.accounts.get_mut(address)
    }

    /// Get meter by owner and service_id
    pub fn get_meter(&self, owner: &str, service_id: &str) -> Option<&Meter> {
        let key = MeterKey::new(owner.to_string(), service_id.to_string());
        self.meters.get(&key)
    }

    /// Get meter mutably
    pub fn get_meter_mut(&mut self, owner: &str, service_id: &str) -> Option<&mut Meter> {
        let key = MeterKey::new(owner.to_string(), service_id.to_string());
        self.meters.get_mut(&key)
    }

    /// Insert or update a meter
    pub fn insert_meter(&mut self, meter: Meter) {
        let key = MeterKey::new(meter.owner.clone(), meter.service_id.clone());
        self.meters.insert(key, meter);
    }

    /// Check if an active meter exists for (owner, service_id)
    ///
    /// Used to enforce INV-5: Meter Uniqueness
    pub fn has_active_meter(&self, owner: &str, service_id: &str) -> bool {
        if let Some(meter) = self.get_meter(owner, service_id) {
            meter.is_active()
        } else {
            false
        }
    }

    /// Get all meters for a given owner
    pub fn get_owner_meters(&self, owner: &str) -> Vec<&Meter> {
        self.meters.values().filter(|m| m.owner == owner).collect()
    }

    /// Get all active meters for a given owner
    pub fn get_owner_active_meters(&self, owner: &str) -> Vec<&Meter> {
        self.meters
            .values()
            .filter(|m| m.owner == owner && m.active)
            .collect()
    }

    /// Get settlement by id (owner, service_id, window_id).
    pub fn get_settlement(&self, id: &SettlementId) -> Option<&Settlement> {
        self.settlements.get(&id.key())
    }

    /// Get settlement mutably.
    pub fn get_settlement_mut(&mut self, id: &SettlementId) -> Option<&mut Settlement> {
        self.settlements.get_mut(&id.key())
    }

    /// Insert settlement.
    pub fn insert_settlement(&mut self, s: Settlement) {
        let key = s.id.key();
        self.settlements.insert(key, s);
    }

    /// Get claim by id.
    pub fn get_claim(&self, id: &ClaimId) -> Option<&Claim> {
        self.claims.get(&id.key())
    }

    /// Get claim mutably.
    pub fn get_claim_mut(&mut self, id: &ClaimId) -> Option<&mut Claim> {
        self.claims.get_mut(&id.key())
    }

    /// Insert claim.
    pub fn insert_claim(&mut self, c: Claim) {
        let key = c.id.key().to_string();
        self.claims.insert(key, c);
    }

    /// Get dispute by id (settlement_key).
    pub fn get_dispute(&self, id: &DisputeId) -> Option<&Dispute> {
        self.disputes.get(id.key())
    }

    /// Get dispute mutably.
    pub fn get_dispute_mut(&mut self, id: &DisputeId) -> Option<&mut Dispute> {
        self.disputes.get_mut(id.key())
    }

    /// Insert or replace dispute.
    pub fn insert_dispute(&mut self, d: Dispute) {
        let key = d.id.key().to_string();
        self.disputes.insert(key, d);
    }

    /// Get policy version by id.
    pub fn get_policy_version(&self, id: &PolicyVersionId) -> Option<&PolicyVersion> {
        self.policy_versions.get(&id.key())
    }

    /// Get policy version mutably.
    pub fn get_policy_version_mut(&mut self, id: &PolicyVersionId) -> Option<&mut PolicyVersion> {
        self.policy_versions.get_mut(&id.key())
    }

    /// Insert or replace policy version.
    pub fn insert_policy_version(&mut self, p: PolicyVersion) {
        let key = p.id.key();
        self.policy_versions.insert(key, p);
    }

    /// Resolve active policy for (owner, service_id) at current_tx_id.
    /// Precedence: OwnerService > Owner > Global; highest version with effective_from_tx_id <= current_tx_id.
    pub fn resolve_policy(
        &self,
        owner: &str,
        service_id: &str,
        current_tx_id: u64,
    ) -> Option<&PolicyVersion> {
        for scope in PolicyScope::scope_chain(owner, service_id) {
            let scope_key = scope.scope_key();
            let best = self
                .policy_versions
                .values()
                .filter(|pv| pv.scope.scope_key() == scope_key)
                .filter(|pv| pv.is_published())
                .filter(|pv| pv.effective_from_tx_id <= current_tx_id)
                .max_by_key(|pv| pv.id.version);
            if let Some(pv) = best {
                return Some(pv);
            }
        }
        None
    }

    /// Return the scope chain for (owner, service_id) in precedence order for debugging/audit.
    /// Order: OwnerService, Owner, Global.
    pub fn resolve_scope_chain(&self, owner: &str, service_id: &str) -> Vec<PolicyScope> {
        PolicyScope::scope_chain(owner, service_id)
    }

    /// Latest version number for a scope (max version with this scope_key). None if no versions.
    pub fn latest_policy_version_for_scope(&self, scope_key: &str) -> Option<u64> {
        self.policy_versions
            .values()
            .filter(|pv| pv.id.scope_key == scope_key)
            .map(|pv| pv.id.version)
            .max()
    }

    /// G4: resolution audit for a dispute (if resolved with evidence).
    pub fn get_dispute_resolution_audit(
        &self,
        settlement_id: &SettlementId,
    ) -> Option<&ResolutionAudit> {
        let did = DisputeId::new(settlement_id);
        self.get_dispute(&did)?.resolution_audit.as_ref()
    }

    /// G4: build evidence bundle for a settlement from settlement + resolution audit (if resolved).
    pub fn get_evidence_bundle(
        &self,
        settlement_id: &SettlementId,
    ) -> Option<crate::evidence::EvidenceBundle> {
        let s = self.get_settlement(settlement_id)?;
        let audit = self.get_dispute_resolution_audit(settlement_id)?;
        Some(crate::evidence::EvidenceBundle {
            schema_version: crate::evidence::CURRENT_EVIDENCE_SCHEMA_VERSION,
            replay_protocol_version: crate::evidence::REPLAY_PROTOCOL_VERSION,
            settlement_key: settlement_id.key(),
            from_tx_id: s.from_tx_id,
            to_tx_id: s.to_tx_id,
            evidence_hash: s.evidence_hash.clone(),
            replay_hash: audit.replay_hash.clone(),
            replay_summary: audit.replay_summary.clone(),
        })
    }
}

impl Default for State {
    fn default() -> Self {
        State::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_creation() {
        let state = State::new();
        assert!(state.accounts.is_empty());
        assert!(state.meters.is_empty());
    }

    #[test]
    fn test_get_or_create_account() {
        let mut state = State::new();
        let account = state.get_or_create_account("alice");
        assert_eq!(account.balance, 0);
        assert_eq!(account.nonce, 0);
    }

    #[test]
    fn test_insert_meter() {
        let mut state = State::new();
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        state.insert_meter(meter);

        let retrieved = state.get_meter("alice", "storage");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().service_id, "storage");
    }

    #[test]
    fn test_has_active_meter() {
        let mut state = State::new();
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        state.insert_meter(meter);

        assert!(state.has_active_meter("alice", "storage"));
        assert!(!state.has_active_meter("alice", "api_calls"));
        assert!(!state.has_active_meter("bob", "storage"));
    }

    #[test]
    fn test_has_active_meter_inactive() {
        let mut state = State::new();
        let mut meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        meter.close();
        state.insert_meter(meter);

        assert!(!state.has_active_meter("alice", "storage"));
    }
}
