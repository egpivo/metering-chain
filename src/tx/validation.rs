use crate::error::{Error, Result};
use crate::state::State;
use crate::tx::{Pricing, SignedTx, Transaction};
use serde::Deserialize;

/// Live = use wall clock (now, max_age). Replay = no wall clock, only signed reference_time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    Live,
    Replay,
}

/// Context for validation: Live requires now and max_age; Replay forbids wall clock.
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub mode: ValidationMode,
    /// Required when mode is Live; must be None in Replay.
    pub now: Option<u64>,
    /// Required when mode is Live for delegated consume; unused in Replay.
    pub max_age: Option<u64>,
}

impl ValidationContext {
    pub fn live(now: u64, max_age: u64) -> Self {
        ValidationContext {
            mode: ValidationMode::Live,
            now: Some(now),
            max_age: Some(max_age),
        }
    }

    pub fn replay() -> Self {
        ValidationContext {
            mode: ValidationMode::Replay,
            now: None,
            max_age: None,
        }
    }
}

/// Minimal delegation proof claims (v1: bincode). Full UCAN/JWT can be added later.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DelegationProofMinimal {
    pub iat: u64,
    pub exp: u64,
    pub issuer: String,
    pub audience: String,
}

/// Build proof bytes for testing (bincode serialized DelegationProofMinimal).
pub fn make_minimal_proof_bytes(iat: u64, exp: u64, issuer: &str, audience: &str) -> Vec<u8> {
    let p = DelegationProofMinimal {
        iat,
        exp,
        issuer: issuer.to_string(),
        audience: audience.to_string(),
    };
    bincode::serialize(&p).unwrap()
}

/// Normalize principal to chain address. If 0x+hex (32 bytes), return 0x+lowercase hex; else return lowercase for comparison.
pub fn principal_to_chain_address(principal: &str) -> Result<String> {
    let s = principal.trim();
    let hex_part = match s.strip_prefix("0x") {
        Some(h) => h,
        None => return Ok(s.to_lowercase()),
    };
    let hex_lower = hex_part.to_lowercase();
    let bytes = hex::decode(&hex_lower).map_err(|e| {
        Error::PrincipalBindingFailed(format!("Invalid hex: {}", e))
    })?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| {
        Error::PrincipalBindingFailed("Expected 32-byte pubkey".to_string())
    })?;
    Ok(format!("0x{}", hex::encode(arr)))
}

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
/// Owner-signed: signer == owner, nonce from signer. Delegated: signer != owner, nonce from nonce_account (owner);
/// delegation proof and valid_at required; time rules per ValidationContext (Live: now/max_age, Replay: no wall clock).
pub fn validate_consume(
    state: &State,
    tx: &SignedTx,
    ctx: &ValidationContext,
) -> Result<u64> {
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

    let is_delegated = tx.signer != *owner || tx.delegation_proof.is_some();

    if is_delegated {
        let proof_bytes = tx.delegation_proof.as_ref().ok_or(Error::DelegationProofMissing)?;
        let valid_at = tx.valid_at.ok_or(Error::ValidAtMissing)?;
        let nonce_account = tx
            .nonce_account
            .as_ref()
            .filter(|a| a.as_str() == owner.as_str())
            .ok_or(Error::NonceAccountMissingOrInvalid)?;

        if ctx.mode == ValidationMode::Live {
            let now = ctx.now.ok_or(Error::InvalidValidationContextLiveNowMissing)?;
            let max_age = ctx.max_age.ok_or(Error::InvalidValidationContextLiveMaxAgeMissing)?;
            if valid_at > now {
                return Err(Error::ReferenceTimeFuture);
            }
            if now.saturating_sub(valid_at) > max_age {
                return Err(Error::ReferenceTimeTooOld);
            }
        }

        let proof: DelegationProofMinimal = bincode::deserialize(proof_bytes)
            .map_err(|_| Error::DelegationExpiredOrNotYetValid)?;
        if proof.iat > valid_at || valid_at >= proof.exp {
            return Err(Error::DelegationExpiredOrNotYetValid);
        }

        let issuer_addr = principal_to_chain_address(&proof.issuer)?;
        let audience_addr = principal_to_chain_address(&proof.audience)?;
        if normalize_address(owner) != issuer_addr {
            return Err(Error::DelegationIssuerOwnerMismatch);
        }
        if normalize_address(&tx.signer) != audience_addr {
            return Err(Error::DelegationAudienceSignerMismatch);
        }

        let nonce_acc = state.get_account(nonce_account).ok_or_else(|| {
            Error::InvalidTransaction(format!("Account {} does not exist", nonce_account))
        })?;
        if nonce_acc.nonce() != tx.nonce {
            return Err(Error::InvalidTransaction(format!(
                "Nonce mismatch: expected {}, got {}",
                nonce_acc.nonce(),
                tx.nonce
            )));
        }
        let balance_acc = state.get_account(owner).ok_or_else(|| {
            Error::InvalidTransaction(format!("Account {} does not exist", owner))
        })?;
        if !balance_acc.has_sufficient_balance(cost) {
            return Err(Error::InvalidTransaction(format!(
                "Insufficient balance for consumption: have {}, need {}",
                balance_acc.balance(),
                cost
            )));
        }
    } else {
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
        if !account.has_sufficient_balance(cost) {
            return Err(Error::InvalidTransaction(format!(
                "Insufficient balance for consumption: have {}, need {}",
                account.balance(),
                cost
            )));
        }
    }

    Ok(cost)
}

fn normalize_address(addr: &str) -> String {
    principal_to_chain_address(addr).unwrap_or_else(|_| addr.to_lowercase())
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
/// Returns Ok(cost) for Consume (computed cost). ctx is used for Consume (Live/Replay time rules).
pub fn validate(
    state: &State,
    tx: &SignedTx,
    ctx: &ValidationContext,
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
            let cost = validate_consume(state, tx, ctx)?;
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

    fn replay_ctx() -> ValidationContext {
        ValidationContext::replay()
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
        let result = validate_consume(&state, &tx, &replay_ctx());
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
        assert!(validate_consume(&state, &tx, &replay_ctx()).is_err());
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
        assert!(validate_consume(&state, &tx, &replay_ctx()).is_err());
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

    #[test]
    fn test_principal_to_chain_address_normalize() {
        let hex32 = "0x".to_string() + &"a".repeat(64);
        let out = principal_to_chain_address(&hex32).unwrap();
        assert_eq!(out, "0x".to_string() + &"a".repeat(64));
        let out = principal_to_chain_address("alice").unwrap();
        assert_eq!(out, "alice");
    }
}
