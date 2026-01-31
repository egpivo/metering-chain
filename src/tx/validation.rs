use crate::error::{Error, Result};
use crate::state::State;
use crate::tx::{Pricing, SignedTx, Transaction};

pub fn compute_cost(units: u64, pricing: &Pricing) -> Result<u64> {
    match pricing {
        Pricing::UnitPrice(unit_price) => {
            let cost = units.checked_mul(*unit_price).ok_or_else(|| {
                Error::InvalidTransaction(format!(
                    "Cost computation overflow: {} units × {} price",
                    units, unit_price
                ))
            })?;
            Ok(cost)
        }
        Pricing::FixedCost(cost) => Ok(*cost),
    }
}

pub fn validate_mint(
    _state: &State,
    tx: &SignedTx,
    authorized_minters: Option<&std::collections::HashSet<String>>,
) -> Result<()> {
    let Transaction::Mint { to: _to, amount } = &tx.kind else {
        return Err(Error::InvalidTransaction(
            "Expected Mint transaction".to_string(),
        ));
    };

    if let Some(minters) = authorized_minters {
        if !minters.contains(&tx.signer) {
            return Err(Error::InvalidTransaction(format!(
                "Mint authorization failed: {} is not an authorized minter",
                tx.signer
            )));
        }
    }

    if *amount == 0 {
        return Err(Error::InvalidTransaction(
            "Mint amount must be greater than zero".to_string(),
        ));
    }

    Ok(())
}

pub fn validate_open_meter(state: &State, tx: &SignedTx) -> Result<()> {
    let Transaction::OpenMeter {
        owner,
        service_id,
        deposit,
    } = &tx.kind
    else {
        return Err(Error::InvalidTransaction(
            "Expected OpenMeter transaction".to_string(),
        ));
    };

    if tx.signer != *owner {
        return Err(Error::InvalidTransaction(format!(
            "Signer {} does not match owner {}",
            tx.signer, owner
        )));
    }

    let account = state.get_account(&tx.signer).ok_or_else(|| {
        Error::InvalidTransaction(format!("Account {} does not exist", tx.signer))
    })?;

    if account.nonce() != tx.nonce {
        return Err(Error::InvalidTransaction(format!(
            "Nonce mismatch: expected {}, got {}",
            account.nonce(),
            tx.nonce
        )));
    }

    if *deposit == 0 {
        return Err(Error::InvalidTransaction(
            "Deposit must be greater than zero".to_string(),
        ));
    }

    if !account.has_sufficient_balance(*deposit) {
        return Err(Error::InvalidTransaction(format!(
            "Insufficient balance for deposit: have {}, need {}",
            account.balance(),
            deposit
        )));
    }

    if state.has_active_meter(owner, service_id) {
        return Err(Error::InvalidTransaction(format!(
            "Active meter already exists for owner {} and service {}",
            owner, service_id
        )));
    }

    Ok(())
}

/// Validate a Consume transaction
///
/// Checks:
/// - INV-4: Ownership Authorization (signer == owner)
/// - INV-9: Nonce Monotonicity
/// - INV-6: Active Meter Requirement
/// - INV-13: Positive Units
/// - INV-12: Valid Pricing
/// - INV-14: Overflow Protection
/// - INV-11: Sufficient Balance for Consumption
pub fn validate_consume(state: &State, tx: &SignedTx) -> Result<u64> {
    let Transaction::Consume {
        owner,
        service_id,
        units,
        pricing,
    } = &tx.kind
    else {
        return Err(Error::InvalidTransaction(
            "Expected Consume transaction".to_string(),
        ));
    };

    if tx.signer != *owner {
        return Err(Error::InvalidTransaction(format!(
            "Signer {} does not match owner {}",
            tx.signer, owner
        )));
    }

    let account = state.get_account(&tx.signer).ok_or_else(|| {
        Error::InvalidTransaction(format!("Account {} does not exist", tx.signer))
    })?;

    if account.nonce() != tx.nonce {
        return Err(Error::InvalidTransaction(format!(
            "Nonce mismatch: expected {}, got {}",
            account.nonce(),
            tx.nonce
        )));
    }

    let meter = state.get_meter(owner, service_id).ok_or_else(|| {
        Error::InvalidTransaction(format!(
            "Meter does not exist for owner {} and service {}",
            owner, service_id
        ))
    })?;

    if !meter.is_active() {
        return Err(Error::InvalidTransaction(format!(
            "Meter is not active for owner {} and service {}",
            owner, service_id
        )));
    }

    if *units == 0 {
        return Err(Error::InvalidTransaction(
            "Units must be greater than zero".to_string(),
        ));
    }

    match pricing {
        Pricing::UnitPrice(price) => {
            if *price == 0 {
                return Err(Error::InvalidTransaction(
                    "UnitPrice must be greater than zero".to_string(),
                ));
            }
        }
        Pricing::FixedCost(cost) => {
            if *cost == 0 {
                return Err(Error::InvalidTransaction(
                    "FixedCost must be greater than zero".to_string(),
                ));
            }
        }
    }

    let cost = compute_cost(*units, pricing)?;

    if !account.has_sufficient_balance(cost) {
        return Err(Error::InvalidTransaction(format!(
            "Insufficient balance for consumption: have {}, need {}",
            account.balance(),
            cost
        )));
    }

    Ok(cost)
}

/// Validate a CloseMeter transaction
///
/// Checks:
/// - INV-4: Ownership Authorization (signer == owner)
/// - INV-9: Nonce Monotonicity
/// - INV-6: Active Meter Requirement
pub fn validate_close_meter(state: &State, tx: &SignedTx) -> Result<()> {
    let Transaction::CloseMeter { owner, service_id } = &tx.kind else {
        return Err(Error::InvalidTransaction(
            "Expected CloseMeter transaction".to_string(),
        ));
    };

    if tx.signer != *owner {
        return Err(Error::InvalidTransaction(format!(
            "Signer {} does not match owner {}",
            tx.signer, owner
        )));
    }

    let account = state.get_account(&tx.signer).ok_or_else(|| {
        Error::InvalidTransaction(format!("Account {} does not exist", tx.signer))
    })?;

    if account.nonce() != tx.nonce {
        return Err(Error::InvalidTransaction(format!(
            "Nonce mismatch: expected {}, got {}",
            account.nonce(),
            tx.nonce
        )));
    }

    let meter = state.get_meter(owner, service_id).ok_or_else(|| {
        Error::InvalidTransaction(format!(
            "Meter does not exist for owner {} and service {}",
            owner, service_id
        ))
    })?;

    if !meter.is_active() {
        return Err(Error::InvalidTransaction(format!(
            "Meter is not active for owner {} and service {}",
            owner, service_id
        )));
    }

    Ok(())
}

/// Main validation function that dispatches to specific validators
///
/// Returns Ok(()) for Mint, OpenMeter, CloseMeter
/// Returns Ok(cost) for Consume (computed cost)
pub fn validate(
    state: &State,
    tx: &SignedTx,
    authorized_minters: Option<&std::collections::HashSet<String>>,
) -> Result<Option<u64>> {
    match &tx.kind {
        Transaction::Mint { .. } => {
            validate_mint(state, tx, authorized_minters)?;
            Ok(None)
        }
        Transaction::OpenMeter { .. } => {
            validate_open_meter(state, tx)?;
            Ok(None)
        }
        Transaction::Consume { .. } => {
            let cost = validate_consume(state, tx)?;
            Ok(Some(cost))
        }
        Transaction::CloseMeter { .. } => {
            validate_close_meter(state, tx)?;
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Account, Meter, State};
    use std::collections::HashSet;

    fn create_test_state() -> State {
        let mut state = State::new();
        let account = Account::with_balance(1000);
        state.accounts.insert("alice".to_string(), account);
        state
    }

    fn create_authorized_minters() -> HashSet<String> {
        let mut minters = HashSet::new();
        minters.insert("authority".to_string());
        minters
    }

    #[test]
    fn test_compute_cost_unit_price() {
        let pricing = Pricing::UnitPrice(10);
        let cost = compute_cost(5, &pricing).unwrap();
        assert_eq!(cost, 50);
    }

    #[test]
    fn test_compute_cost_fixed_cost() {
        let pricing = Pricing::FixedCost(100);
        let cost = compute_cost(5, &pricing).unwrap();
        assert_eq!(cost, 100);
    }

    #[test]
    fn test_compute_cost_overflow() {
        let pricing = Pricing::UnitPrice(u64::MAX);
        let result = compute_cost(2, &pricing);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_mint_success() {
        let state = State::new();
        let minters = create_authorized_minters();
        let tx = SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 100,
            },
        );
        assert!(validate_mint(&state, &tx, Some(&minters)).is_ok());
    }

    #[test]
    fn test_validate_mint_unauthorized() {
        let state = State::new();
        let minters = create_authorized_minters();
        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Mint {
                to: "bob".to_string(),
                amount: 100,
            },
        );
        assert!(validate_mint(&state, &tx, Some(&minters)).is_err());
    }

    #[test]
    fn test_validate_mint_zero_amount() {
        let state = State::new();
        let minters = create_authorized_minters();
        let tx = SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 0,
            },
        );
        assert!(validate_mint(&state, &tx, Some(&minters)).is_err());
    }

    #[test]
    fn test_validate_open_meter_success() {
        let state = create_test_state();
        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        );
        assert!(validate_open_meter(&state, &tx).is_ok());
    }

    #[test]
    fn test_validate_open_meter_wrong_signer() {
        let state = create_test_state();
        let tx = SignedTx::new(
            "bob".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        );
        assert!(validate_open_meter(&state, &tx).is_err());
    }

    #[test]
    fn test_validate_open_meter_insufficient_balance() {
        let state = create_test_state();
        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 2000,
            },
        );
        assert!(validate_open_meter(&state, &tx).is_err());
    }

    #[test]
    fn test_validate_open_meter_existing_active_meter() {
        let mut state = create_test_state();
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 50);
        state.insert_meter(meter);

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        );
        assert!(validate_open_meter(&state, &tx).is_err());
    }

    #[test]
    fn test_validate_consume_success() {
        let mut state = create_test_state();
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        state.insert_meter(meter);

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        );
        let result = validate_consume(&state, &tx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50);
    }

    #[test]
    fn test_validate_consume_inactive_meter() {
        let mut state = create_test_state();
        let mut meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        meter.close();
        state.insert_meter(meter);

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        );
        assert!(validate_consume(&state, &tx).is_err());
    }

    #[test]
    fn test_validate_consume_zero_units() {
        let mut state = create_test_state();
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        state.insert_meter(meter);

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 0,
                pricing: Pricing::UnitPrice(5),
            },
        );
        assert!(validate_consume(&state, &tx).is_err());
    }

    #[test]
    fn test_validate_close_meter_success() {
        let mut state = create_test_state();
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        state.insert_meter(meter);

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::CloseMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
            },
        );
        assert!(validate_close_meter(&state, &tx).is_ok());
    }

    #[test]
    fn test_validate_close_meter_inactive() {
        let mut state = create_test_state();
        let mut meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        meter.close();
        state.insert_meter(meter);

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::CloseMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
            },
        );
        assert!(validate_close_meter(&state, &tx).is_err());
    }
}
