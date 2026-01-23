use serde::{Deserialize, Serialize};

/// Meter aggregate: represents a usage ledger for a specific service owned by an account.
///
/// Identity: `(owner, service_id)`
///
/// Invariants:
/// - Only the owner may operate the meter
/// - At most one active meter per `(owner, service_id)`
/// - `total_units` and `total_spent` are monotonic
/// - `locked_deposit` represents committed funds
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Meter {
    /// Account that owns this meter
    pub owner: String,
    
    /// Service identifier (e.g., "storage", "api_calls")
    pub service_id: String,
    
    /// Cumulative usage units
    pub total_units: u64,
    
    /// Cumulative cost paid
    pub total_spent: u64,
    
    /// Whether the meter accepts consumption
    pub active: bool,
    
    /// Committed funds (refunded on closure)
    pub locked_deposit: u64,
}

impl Meter {
    pub fn new(owner: String, service_id: String, deposit: u64) -> Self {
        Meter {
            owner,
            service_id,
            total_units: 0,
            total_spent: 0,
            active: true,
            locked_deposit: deposit,
        }
    }

    pub fn inactive(owner: String, service_id: String, total_units: u64, total_spent: u64) -> Self {
        Meter {
            owner,
            service_id,
            total_units,
            total_spent,
            active: false,
            locked_deposit: 0,
        }
    }

    pub fn reactivate(&mut self, deposit: u64) {
        self.active = true;
        self.locked_deposit = deposit;
    }

    pub fn close(&mut self) -> u64 {
        self.active = false;
        let deposit = self.locked_deposit;
        self.locked_deposit = 0;
        deposit
    }

    pub fn record_consumption(&mut self, units: u64, cost: u64) {
        self.total_units = self.total_units.saturating_add(units);
        self.total_spent = self.total_spent.saturating_add(cost);
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn identity(&self) -> (&str, &str) {
        (&self.owner, &self.service_id)
    }

    pub fn total_units(&self) -> u64 {
        self.total_units
    }

    pub fn total_spent(&self) -> u64 {
        self.total_spent
    }

    pub fn locked_deposit(&self) -> u64 {
        self.locked_deposit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_creation() {
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        assert_eq!(meter.owner, "alice");
        assert_eq!(meter.service_id, "storage");
        assert_eq!(meter.total_units, 0);
        assert_eq!(meter.total_spent, 0);
        assert!(meter.active);
        assert_eq!(meter.locked_deposit, 100);
    }

    #[test]
    fn test_meter_close() {
        let mut meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        let deposit = meter.close();
        assert_eq!(deposit, 100);
        assert!(!meter.active);
        assert_eq!(meter.locked_deposit, 0);
    }

    #[test]
    fn test_meter_reactivate() {
        let mut meter = Meter::inactive(
            "alice".to_string(),
            "storage".to_string(),
            50,
            200,
        );
        meter.reactivate(150);
        assert!(meter.active);
        assert_eq!(meter.locked_deposit, 150);
        assert_eq!(meter.total_units, 50); // Preserved
        assert_eq!(meter.total_spent, 200); // Preserved
    }

    #[test]
    fn test_record_consumption() {
        let mut meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        meter.record_consumption(10, 50);
        assert_eq!(meter.total_units, 10);
        assert_eq!(meter.total_spent, 50);
        
        meter.record_consumption(5, 25);
        assert_eq!(meter.total_units, 15);
        assert_eq!(meter.total_spent, 75);
    }
}
