use crate::state::{State, MeterKey};
use crate::tx::{SignedTx, Transaction};
use crate::tx::validation::validate;
use crate::error::{Error, Result};
use std::collections::HashSet;

/// Apply a transaction to state, producing a new state.
///
/// This is a pure function: given the same state and transaction,
/// it always produces the same result. This enables deterministic replay.
///
/// The function validates the transaction first, then applies the state changes.
/// If validation fails, an error is returned and state remains unchanged.
pub fn apply(
    state: &State,
    tx: &SignedTx,
    authorized_minters: &HashSet<String>,
) -> Result<State> {
    let cost_opt = validate(state, tx, authorized_minters)?;
    let mut new_state = state.clone();

    match &tx.kind {
        Transaction::Mint { to, amount } => {
            apply_mint(&mut new_state, to, *amount)?;
        }
        Transaction::OpenMeter { owner, service_id, deposit } => {
            apply_open_meter(&mut new_state, owner, service_id, *deposit, &tx.signer)?;
        }
        Transaction::Consume { owner, service_id, units, pricing: _ } => {
            let cost = cost_opt.expect("validate_consume should return cost");
            apply_consume(&mut new_state, owner, service_id, *units, cost, &tx.signer)?;
        }
        Transaction::CloseMeter { owner, service_id } => {
            apply_close_meter(&mut new_state, owner, service_id, &tx.signer)?;
        }
    }

    Ok(new_state)
}

/// Apply Mint transaction: add balance to target account
///
/// State Update:
/// - `accounts[to].balance += amount`
fn apply_mint(state: &mut State, to: &str, amount: u64) -> Result<()> {
    let account = state.get_or_create_account(to);
    account.add_balance(amount);
    Ok(())
}

/// Apply OpenMeter transaction: create or reactivate meter, deduct deposit
///
/// State Update:
/// - If meter does not exist: create with zero totals and deposit
/// - If meter exists but is inactive: reactivate, preserve totals, set new deposit
/// - `accounts[owner].balance -= deposit`
/// - `accounts[signer].nonce += 1`
fn apply_open_meter(
    state: &mut State,
    owner: &str,
    service_id: &str,
    deposit: u64,
    signer: &str,
) -> Result<()> {
    let key = MeterKey::new(owner.to_string(), service_id.to_string());

    if let Some(meter) = state.meters.get_mut(&key) {
        if !meter.is_active() {
            meter.reactivate(deposit);
        } else {
            return Err(Error::StateError(
                format!("Active meter already exists for {}:{}", owner, service_id)
            ));
        }
    } else {
        let meter = crate::state::Meter::new(owner.to_string(), service_id.to_string(), deposit);
        state.insert_meter(meter);
    }

    let account = state.get_account_mut(owner)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", owner)))?;
    account.subtract_balance(deposit)
        .map_err(|e| Error::StateError(e))?;

    let signer_account = state.get_account_mut(signer)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", signer)))?;
    signer_account.increment_nonce();

    Ok(())
}

/// Apply Consume transaction: record usage and deduct cost
///
/// State Update:
/// - `meters[(owner, service_id)].total_units += units`
/// - `meters[(owner, service_id)].total_spent += cost`
/// - `accounts[owner].balance -= cost`
/// - `accounts[signer].nonce += 1`
fn apply_consume(
    state: &mut State,
    owner: &str,
    service_id: &str,
    units: u64,
    cost: u64,
    signer: &str,
) -> Result<()> {
    let meter = state.get_meter_mut(owner, service_id)
        .ok_or_else(|| Error::StateError(
            format!("Meter not found for {}:{}", owner, service_id)
        ))?;
    meter.record_consumption(units, cost);

    let account = state.get_account_mut(owner)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", owner)))?;
    account.subtract_balance(cost)
        .map_err(|e| Error::StateError(e))?;

    let signer_account = state.get_account_mut(signer)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", signer)))?;
    signer_account.increment_nonce();

    Ok(())
}

/// Apply CloseMeter transaction: close meter and return deposit
///
/// State Update:
/// - `meters[(owner, service_id)].active = false`
/// - `accounts[owner].balance += meters[(owner, service_id)].locked_deposit`
/// - `meters[(owner, service_id)].locked_deposit = 0`
/// - `accounts[signer].nonce += 1`
fn apply_close_meter(
    state: &mut State,
    owner: &str,
    service_id: &str,
    signer: &str,
) -> Result<()> {
    let meter = state.get_meter_mut(owner, service_id)
        .ok_or_else(|| Error::StateError(
            format!("Meter not found for {}:{}", owner, service_id)
        ))?;
    let deposit = meter.close();

    let account = state.get_account_mut(owner)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", owner)))?;
    account.add_balance(deposit);

    let signer_account = state.get_account_mut(signer)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", signer)))?;
    signer_account.increment_nonce();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Account, Meter};
    use crate::tx::{Pricing, Transaction};

    fn create_authorized_minters() -> HashSet<String> {
        let mut minters = HashSet::new();
        minters.insert("authority".to_string());
        minters
    }

    #[test]
    fn test_apply_mint() {
        let state = State::new();
        let minters = create_authorized_minters();
        let tx = SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        );

        let new_state = apply(&state, &tx, &minters).unwrap();
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 1000);
    }

    #[test]
    fn test_apply_open_meter() {
        let mut state = State::new();
        state.accounts.insert("alice".to_string(), Account::with_balance(1000));
        let minters = create_authorized_minters();

        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        );

        let new_state = apply(&state, &tx, &minters).unwrap();
        
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 900);
        assert_eq!(account.nonce(), 1);

        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert!(meter.is_active());
        assert_eq!(meter.locked_deposit(), 100);
        assert_eq!(meter.total_units(), 0);
        assert_eq!(meter.total_spent(), 0);
    }

    #[test]
    fn test_apply_open_meter_reactivate() {
        let mut state = State::new();
        state.accounts.insert("alice".to_string(), Account::with_balance(1000));
        
        // Create inactive meter
        let mut meter = Meter::new("alice".to_string(), "storage".to_string(), 50);
        meter.record_consumption(10, 25);
        meter.close();
        state.insert_meter(meter);

        let minters = create_authorized_minters();
        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        );

        let new_state = apply(&state, &tx, &minters).unwrap();
        
        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert!(meter.is_active());
        assert_eq!(meter.locked_deposit(), 100);
        assert_eq!(meter.total_units(), 10); // Preserved
        assert_eq!(meter.total_spent(), 25); // Preserved
    }

    #[test]
    fn test_apply_consume() {
        let mut state = State::new();
        state.accounts.insert("alice".to_string(), Account::with_balance(1000));
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        state.insert_meter(meter);

        let minters = create_authorized_minters();
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

        let new_state = apply(&state, &tx, &minters).unwrap();
        
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 950);
        assert_eq!(account.nonce(), 1);

        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert_eq!(meter.total_units(), 10);
        assert_eq!(meter.total_spent(), 50);
    }

    #[test]
    fn test_apply_close_meter() {
        let mut state = State::new();
        state.accounts.insert("alice".to_string(), Account::with_balance(1000));
        let meter = Meter::new("alice".to_string(), "storage".to_string(), 100);
        state.insert_meter(meter);

        let minters = create_authorized_minters();
        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::CloseMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
            },
        );

        let new_state = apply(&state, &tx, &minters).unwrap();
        
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 1100);
        assert_eq!(account.nonce(), 1);

        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert!(!meter.is_active());
        assert_eq!(meter.locked_deposit(), 0);
    }

    #[test]
    fn test_apply_invalid_transaction() {
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

        let result = apply(&state, &tx, &minters);
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_end_to_end_flow() {
        let mut state = State::new();
        let minters = create_authorized_minters();

        let tx1 = SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        );
        state = apply(&state, &tx1, &minters).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 1000);

        let tx2 = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        );
        state = apply(&state, &tx2, &minters).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 900);

        let tx3 = SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        );
        state = apply(&state, &tx3, &minters).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 850);
        assert_eq!(state.get_meter("alice", "storage").unwrap().total_units(), 10);

        let tx4 = SignedTx::new(
            "alice".to_string(),
            2,
            Transaction::CloseMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
            },
        );
        state = apply(&state, &tx4, &minters).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 950);
        assert!(!state.get_meter("alice", "storage").unwrap().is_active());
    }
}
