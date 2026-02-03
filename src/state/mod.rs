pub mod account;
pub mod apply;
pub mod meter;

pub use account::Account;
pub use apply::apply;
pub use meter::Meter;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Core domain state: aggregates all accounts and meters
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
}

impl State {
    /// Create empty genesis state
    pub fn new() -> Self {
        State {
            accounts: HashMap::new(),
            meters: HashMap::new(),
            capability_consumption: HashMap::new(),
        }
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
