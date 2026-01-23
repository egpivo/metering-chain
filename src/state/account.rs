use serde::{Deserialize, Serialize};

/// Account aggregate: represents a payer with balance and transaction ordering.
///
/// Invariants:
/// - Balance never becomes negative
/// - Nonce is strictly increasing per account
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    /// Spendable funds held by the account
    pub balance: u64,
    
    /// Per-account sequence number for ordering and replay protection
    pub nonce: u64,
}

impl Account {
    /// Create a new account with zero balance and nonce
    pub fn new() -> Self {
        Account {
            balance: 0,
            nonce: 0,
        }
    }

    /// Create an account with initial balance
    pub fn with_balance(balance: u64) -> Self {
        Account {
            balance,
            nonce: 0,
        }
    }

    /// Increment nonce (used after accepting a transaction)
    ///
    /// This enforces INV-2: Monotonic Nonce
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }

    /// Add to balance (e.g., from Mint or CloseMeter deposit return)
    ///
    /// Returns the new balance
    pub fn add_balance(&mut self, amount: u64) -> u64 {
        self.balance = self.balance.saturating_add(amount);
        self.balance
    }

    /// Subtract from balance (e.g., for deposits or consumption)
    ///
    /// Returns Ok(new_balance) if sufficient funds, Err if insufficient
    /// This enforces INV-1: No Negative Balances
    pub fn subtract_balance(&mut self, amount: u64) -> Result<u64, String> {
        if self.balance < amount {
            return Err(format!(
                "Insufficient balance: have {}, need {}",
                self.balance, amount
            ));
        }
        self.balance -= amount;
        Ok(self.balance)
    }

    /// Check if account has sufficient balance
    pub fn has_sufficient_balance(&self, amount: u64) -> bool {
        self.balance >= amount
    }

    /// Get current balance
    pub fn balance(&self) -> u64 {
        self.balance
    }

    /// Get current nonce
    pub fn nonce(&self) -> u64 {
        self.nonce
    }
}

impl Default for Account {
    fn default() -> Self {
        Account::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_creation() {
        let account = Account::new();
        assert_eq!(account.balance, 0);
        assert_eq!(account.nonce, 0);
    }

    #[test]
    fn test_account_with_balance() {
        let account = Account::with_balance(100);
        assert_eq!(account.balance, 100);
        assert_eq!(account.nonce, 0);
    }

    #[test]
    fn test_add_balance() {
        let mut account = Account::new();
        account.add_balance(50);
        assert_eq!(account.balance, 50);
    }

    #[test]
    fn test_subtract_balance_success() {
        let mut account = Account::with_balance(100);
        let result = account.subtract_balance(30);
        assert!(result.is_ok());
        assert_eq!(account.balance, 70);
    }

    #[test]
    fn test_subtract_balance_insufficient() {
        let mut account = Account::with_balance(50);
        let result = account.subtract_balance(100);
        assert!(result.is_err());
        assert_eq!(account.balance, 50); // Balance unchanged
    }

    #[test]
    fn test_increment_nonce() {
        let mut account = Account::new();
        account.increment_nonce();
        assert_eq!(account.nonce, 1);
        account.increment_nonce();
        assert_eq!(account.nonce, 2);
    }
}
