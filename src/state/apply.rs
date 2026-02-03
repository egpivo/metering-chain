use crate::error::{Error, Result};
use crate::state::{MeterKey, State};
use crate::tx::validation::{validate, ValidationContext};
use crate::tx::{SignedTx, Transaction};
use std::collections::HashSet;

/// When authorized_minters is None (replay), mint authorization is skipped for deterministic replay.
/// ctx must be ValidationContext::replay() when replaying from log; Live(now, max_age) when applying new tx.
pub fn apply(
    state: &State,
    tx: &SignedTx,
    ctx: &ValidationContext,
    authorized_minters: Option<&HashSet<String>>,
) -> Result<State> {
    let cost_opt = validate(state, tx, ctx, authorized_minters)?;
    let mut new_state = state.clone();
    match &tx.kind {
        Transaction::Mint { to, amount } => {
            apply_mint(&mut new_state, to, *amount)?;
        }
        Transaction::OpenMeter {
            owner,
            service_id,
            deposit,
        } => {
            apply_open_meter(&mut new_state, owner, service_id, *deposit, &tx.signer)?;
        }
        Transaction::Consume {
            owner,
            service_id,
            units,
            pricing: _,
        } => {
            let cost = cost_opt.expect("validate_consume should return cost");
            let nonce_account = tx.nonce_account.as_deref().unwrap_or(&tx.signer);
            apply_consume(&mut new_state, owner, service_id, *units, cost, nonce_account)?;
        }
        Transaction::CloseMeter { owner, service_id } => {
            apply_close_meter(&mut new_state, owner, service_id, &tx.signer)?;
        }
    }

    Ok(new_state)
}

fn apply_mint(state: &mut State, to: &str, amount: u64) -> Result<()> {
    let account = state.get_or_create_account(to);
    account.add_balance(amount);
    Ok(())
}

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
            return Err(Error::StateError(format!(
                "Active meter already exists for {}:{}",
                owner, service_id
            )));
        }
    } else {
        let meter = crate::state::Meter::new(owner.to_string(), service_id.to_string(), deposit);
        state.insert_meter(meter);
    }

    let account = state
        .get_account_mut(owner)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", owner)))?;
    account
        .subtract_balance(deposit)
        .map_err(Error::StateError)?;
    let signer_account = state
        .get_account_mut(signer)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", signer)))?;
    signer_account.increment_nonce();

    Ok(())
}

fn apply_consume(
    state: &mut State,
    owner: &str,
    service_id: &str,
    units: u64,
    cost: u64,
    signer: &str,
) -> Result<()> {
    let meter = state.get_meter_mut(owner, service_id).ok_or_else(|| {
        Error::StateError(format!("Meter not found for {}:{}", owner, service_id))
    })?;
    meter.record_consumption(units, cost);

    let account = state
        .get_account_mut(owner)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", owner)))?;
    account.subtract_balance(cost).map_err(Error::StateError)?;
    let signer_account = state
        .get_account_mut(signer)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", signer)))?;
    signer_account.increment_nonce();

    Ok(())
}

fn apply_close_meter(state: &mut State, owner: &str, service_id: &str, signer: &str) -> Result<()> {
    let meter = state.get_meter_mut(owner, service_id).ok_or_else(|| {
        Error::StateError(format!("Meter not found for {}:{}", owner, service_id))
    })?;
    let deposit = meter.close();

    let account = state
        .get_account_mut(owner)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", owner)))?;
    account.add_balance(deposit);
    let signer_account = state
        .get_account_mut(signer)
        .ok_or_else(|| Error::StateError(format!("Account {} not found", signer)))?;
    signer_account.increment_nonce();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Account, Meter};
    use crate::tx::validation::ValidationContext;
    use crate::tx::{Pricing, Transaction};

    fn replay_ctx() -> ValidationContext {
        ValidationContext::replay()
    }

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

        let new_state = apply(&state, &tx, &replay_ctx(), Some(&minters)).unwrap();
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 1000);
    }

    #[test]
    fn test_apply_open_meter() {
        let mut state = State::new();
        state
            .accounts
            .insert("alice".to_string(), Account::with_balance(1000));
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

        let new_state = apply(&state, &tx, &replay_ctx(), Some(&minters)).unwrap();

        // Check account balance decreased
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 900);
        assert_eq!(account.nonce(), 1);

        // Check meter created
        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert!(meter.is_active());
        assert_eq!(meter.locked_deposit(), 100);
        assert_eq!(meter.total_units(), 0);
        assert_eq!(meter.total_spent(), 0);
    }

    #[test]
    fn test_apply_open_meter_reactivate() {
        let mut state = State::new();
        state
            .accounts
            .insert("alice".to_string(), Account::with_balance(1000));

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

        let new_state = apply(&state, &tx, &replay_ctx(), Some(&minters)).unwrap();

        // Check meter reactivated with preserved totals
        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert!(meter.is_active());
        assert_eq!(meter.locked_deposit(), 100);
        assert_eq!(meter.total_units(), 10); // Preserved
        assert_eq!(meter.total_spent(), 25); // Preserved
    }

    #[test]
    fn test_apply_consume() {
        let mut state = State::new();
        state
            .accounts
            .insert("alice".to_string(), Account::with_balance(1000));
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

        let new_state = apply(&state, &tx, &replay_ctx(), Some(&minters)).unwrap();

        // Check account balance decreased
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 950); // 1000 - 50
        assert_eq!(account.nonce(), 1);

        // Check meter updated
        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert_eq!(meter.total_units(), 10);
        assert_eq!(meter.total_spent(), 50);
    }

    #[test]
    fn test_apply_close_meter() {
        let mut state = State::new();
        state
            .accounts
            .insert("alice".to_string(), Account::with_balance(1000));
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

        let new_state = apply(&state, &tx, &replay_ctx(), Some(&minters)).unwrap();

        // Check account balance increased (deposit returned)
        let account = new_state.get_account("alice").unwrap();
        assert_eq!(account.balance(), 1100); // 1000 + 100
        assert_eq!(account.nonce(), 1);

        // Check meter closed
        let meter = new_state.get_meter("alice", "storage").unwrap();
        assert!(!meter.is_active());
        assert_eq!(meter.locked_deposit(), 0);
    }

    #[test]
    fn test_apply_invalid_transaction() {
        let state = State::new();
        let minters = create_authorized_minters();

        // Try to mint with unauthorized signer
        let tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::Mint {
                to: "bob".to_string(),
                amount: 100,
            },
        );

        let result = apply(&state, &tx, &replay_ctx(), Some(&minters));
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_end_to_end_flow() {
        let mut state = State::new();
        let minters = create_authorized_minters();

        // 1. Mint to alice
        let tx1 = SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        );
        state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 1000);

        // 2. Open meter
        let tx2 = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        );
        state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 900);

        // 3. Consume
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
        state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 850);
        assert_eq!(
            state.get_meter("alice", "storage").unwrap().total_units(),
            10
        );

        // 4. Close meter
        let tx4 = SignedTx::new(
            "alice".to_string(),
            2,
            Transaction::CloseMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
            },
        );
        state = apply(&state, &tx4, &replay_ctx(), Some(&minters)).unwrap();
        assert_eq!(state.get_account("alice").unwrap().balance(), 950); // 850 + 100 deposit
        assert!(!state.get_meter("alice", "storage").unwrap().is_active());
    }
}
