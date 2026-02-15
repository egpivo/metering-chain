use crate::error::{Error, Result};
use crate::state::{ClaimId, SettlementId, State};
use crate::tx::{Pricing, SignedTx, Transaction};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
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

/// Canonical ability name for Consume (delegation scope). Proof scoped to this may only be used for Consume.
pub const ABILITY_CONSUME: &str = "consume";

/// Minimal delegation proof claims. Must be signed by owner (issuer); see SignedDelegationProof.
/// Scope: proof is valid only for (owner, service_id, ability) matching the transaction.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct DelegationProofMinimal {
    pub iat: u64,
    pub exp: u64,
    pub issuer: String,
    pub audience: String,
    /// Resource scope: meter (owner, service_id). Must match Consume tx's service_id (issuer = owner).
    pub service_id: String,
    /// Ability scope: e.g. "consume". If present, must equal ABILITY_CONSUME for Consume tx.
    #[serde(default)]
    pub ability: Option<String>,
    /// Optional caveat: max units for this capability (consumed_units + this_tx <= limit).
    #[serde(default)]
    pub max_units: Option<u64>,
    /// Optional caveat: max cost for this capability (consumed_cost + this_tx <= limit).
    #[serde(default)]
    pub max_cost: Option<u64>,
}

/// Deterministic capability ID per M2: sha256(canonical_proof_bytes), lowercase hex.
/// canonical_proof_bytes = exact bytes from delegation_proof field in tx.
pub fn capability_id(proof_bytes: &[u8]) -> String {
    hex::encode(crate::sha256_digest(proof_bytes)).to_lowercase()
}

/// Signed delegation proof: owner (issuer) signs canonical bincode(claims). Prevents forgery.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct SignedDelegationProof {
    pub claims: DelegationProofMinimal,
    /// Ed25519 signature by issuer (owner) over bincode(claims).
    pub signature: Vec<u8>,
}

/// Build signed proof bytes. Owner must sign canonical bincode(claims); pass signature from wallet.sign_bytes(bincode(claims)).
pub fn build_signed_proof(claims: &DelegationProofMinimal, signature: Vec<u8>) -> Vec<u8> {
    bincode::serialize(&SignedDelegationProof {
        claims: claims.clone(),
        signature,
    })
    .unwrap()
}

/// Canonical message the owner must sign to create a valid proof.
pub fn delegation_claims_to_sign(claims: &DelegationProofMinimal) -> Vec<u8> {
    bincode::serialize(claims).unwrap()
}

/// Multicodec varint for Ed25519 public key (0xed = 237): encoded as two bytes 0xed, 0x01.
const MULTICODEC_ED25519_HEADER: [u8; 2] = [0xed, 0x01];

/// Extract 32-byte Ed25519 public key from a principal. Used for proof issuer verification (supports did:key).
/// Accepted: (1) 0x + 64 hex chars; (2) did:key with Ed25519 (z6Mk...).
pub fn principal_to_public_key(principal: &str) -> Result<[u8; 32]> {
    let s = principal.trim();

    // did:key:z<base58btc>
    if let Some(mb_value) = s.strip_prefix("did:key:") {
        let mb = mb_value.trim();
        let multibase_body = mb.strip_prefix('z').ok_or_else(|| {
            Error::PrincipalBindingFailed(
                "did:key multibase value must start with 'z' (base58-btc)".to_string(),
            )
        })?;
        let decoded = bs58::decode(multibase_body).into_vec().map_err(|e| {
            Error::PrincipalBindingFailed(format!("did:key base58 decode failed: {}", e))
        })?;
        // Canonical length only: exactly 2 (multicodec) + 32 (key); reject trailing bytes
        if decoded.len() != 34
            || decoded[0] != MULTICODEC_ED25519_HEADER[0]
            || decoded[1] != MULTICODEC_ED25519_HEADER[1]
        {
            return Err(Error::PrincipalBindingFailed(
                "did:key only supports Ed25519 (multicodec 0xed); wrong header or length"
                    .to_string(),
            ));
        }
        let arr: [u8; 32] = decoded[2..34].try_into().map_err(|_| {
            Error::PrincipalBindingFailed("did:key Ed25519 key must be 32 bytes".to_string())
        })?;
        return Ok(arr);
    }

    // 0x + 64 hex chars
    let hex_part = match s.strip_prefix("0x") {
        Some(h) => h,
        None => {
            return Err(Error::PrincipalBindingFailed(
                "Principal must be 0x+hex (32-byte) or did:key (Ed25519)".to_string(),
            ));
        }
    };
    let hex_lower = hex_part.to_lowercase();
    let bytes = hex::decode(&hex_lower)
        .map_err(|e| Error::PrincipalBindingFailed(format!("Invalid hex: {}", e)))?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| {
        Error::PrincipalBindingFailed("Expected 32-byte pubkey (64 hex chars)".to_string())
    })?;
    Ok(arr)
}

/// Convert principal to chain address. Rejects unconvertible principals per M2.
/// Accepted: (1) 0x + 64 hex chars (32-byte Ed25519 pubkey); (2) did:key with Ed25519 (z6Mk...).
pub fn principal_to_chain_address(principal: &str) -> Result<String> {
    let arr = principal_to_public_key(principal)?;
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

/// Metering Context: shared Consume rules (meter, units, pricing, cost).
/// Authorization is handled separately by validate_consume_delegation / validate_consume_owner.
fn validate_consume_metering(
    state: &State,
    owner: &str,
    service_id: &str,
    units: u64,
    pricing: &Pricing,
) -> Result<u64> {
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
    if units == 0 {
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
    compute_cost(units, pricing)
}

/// Authorization Context: delegated Consume (proof, time, scope, caveats, nonce, balance).
fn validate_consume_delegation(
    state: &State,
    tx: &SignedTx,
    ctx: &ValidationContext,
    owner: &str,
    service_id: &str,
    units: u64,
    cost: u64,
) -> Result<()> {
    // Hard gate: delegated consume must use payload_version=2. Enforced here so --allow-unsigned cannot bypass.
    if tx.effective_payload_version() != crate::tx::transaction::PAYLOAD_VERSION_V2 {
        return Err(Error::DelegatedConsumeRequiresV2);
    }
    let proof_bytes = tx
        .delegation_proof
        .as_ref()
        .ok_or(Error::DelegationProofMissing)?;
    let valid_at = tx.valid_at.ok_or(Error::ValidAtMissing)?;
    let nonce_account = tx
        .nonce_account
        .as_ref()
        .filter(|a| a.as_str() == owner)
        .ok_or(Error::NonceAccountMissingOrInvalid)?;

    if ctx.mode == ValidationMode::Live {
        let now = ctx
            .now
            .ok_or(Error::InvalidValidationContextLiveNowMissing)?;
        let max_age = ctx
            .max_age
            .ok_or(Error::InvalidValidationContextLiveMaxAgeMissing)?;
        if valid_at > now {
            return Err(Error::ReferenceTimeFuture);
        }
        if now.saturating_sub(valid_at) > max_age {
            return Err(Error::ReferenceTimeTooOld);
        }
    }

    let signed_proof: SignedDelegationProof =
        bincode::deserialize(proof_bytes).map_err(|_| Error::DelegationExpiredOrNotYetValid)?;
    let proof = &signed_proof.claims;

    // Verify owner (issuer) signed the claims — supports 0x and did:key (principal_to_public_key)
    let issuer_pubkey = principal_to_public_key(&proof.issuer).map_err(|e| match e {
        Error::PrincipalBindingFailed(msg) => Error::PrincipalBindingFailed(format!(
            "Issuer not a valid principal (0x or did:key): {}",
            msg
        )),
        other => other,
    })?;
    let message = delegation_claims_to_sign(proof);
    let sig_bytes: [u8; 64] = signed_proof
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| Error::DelegationExpiredOrNotYetValid)?;
    let sig = Signature::from_bytes(&sig_bytes);
    let verifying_key = VerifyingKey::from_bytes(&issuer_pubkey)
        .map_err(|_| Error::DelegationExpiredOrNotYetValid)?;
    verifying_key
        .verify(&message, &sig)
        .map_err(|_| Error::DelegationExpiredOrNotYetValid)?;

    if proof.iat > valid_at || valid_at >= proof.exp {
        return Err(Error::DelegationExpiredOrNotYetValid);
    }

    let issuer_addr = principal_to_chain_address(&proof.issuer)?;
    let audience_addr = principal_to_chain_address(&proof.audience)?;
    let owner_addr = principal_to_chain_address(owner)?;
    let signer_addr = principal_to_chain_address(&tx.signer)?;
    if owner_addr != issuer_addr {
        return Err(Error::DelegationIssuerOwnerMismatch);
    }
    if signer_addr != audience_addr {
        return Err(Error::DelegationAudienceSignerMismatch);
    }

    // Scope: proof must be for this (owner, service_id, ability)
    if proof.service_id != *service_id {
        return Err(Error::DelegationScopeMismatch);
    }
    if let Some(ref ab) = proof.ability {
        if ab.as_str() != ABILITY_CONSUME {
            return Err(Error::DelegationScopeMismatch);
        }
    }

    // Caveat limits: consumed + this_tx <= limit (per capability_id)
    let cap_id = capability_id(proof_bytes);
    if state.is_capability_revoked(&cap_id) {
        return Err(Error::DelegationRevoked);
    }
    let (consumed_units, consumed_cost) = state.get_capability_consumption(&cap_id);
    if let Some(limit) = proof.max_units {
        if consumed_units.saturating_add(units) > limit {
            return Err(Error::CapabilityLimitExceeded);
        }
    }
    if let Some(limit) = proof.max_cost {
        if consumed_cost.saturating_add(cost) > limit {
            return Err(Error::CapabilityLimitExceeded);
        }
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
    let balance_acc = state
        .get_account(owner)
        .ok_or_else(|| Error::InvalidTransaction(format!("Account {} does not exist", owner)))?;
    if !balance_acc.has_sufficient_balance(cost) {
        return Err(Error::InvalidTransaction(format!(
            "Insufficient balance for consumption: have {}, need {}",
            balance_acc.balance(),
            cost
        )));
    }
    Ok(())
}

/// Authorization Context: owner-signed Consume (signer == owner, nonce from signer, balance).
fn validate_consume_owner(state: &State, tx: &SignedTx, owner: &str, cost: u64) -> Result<()> {
    if tx.signer != *owner {
        return Err(Error::InvalidTransaction(format!(
            "Signer {} does not match owner {}",
            tx.signer, owner
        )));
    }
    // Owner-signed consume: nonce_account must be None or Some(signer). Forbids incrementing another account's nonce.
    if let Some(ref na) = tx.nonce_account {
        if na != owner {
            return Err(Error::NonceAccountMissingOrInvalid);
        }
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
    Ok(())
}

/// Validate a Consume transaction
///
/// Owner-signed: signer == owner, nonce from signer. Delegated: signer != owner, nonce from nonce_account (owner);
/// delegation proof and valid_at required; time rules per ValidationContext (Live: now/max_age, Replay: no wall clock).
/// Metering (meter, units, pricing, cost) is isolated from Authorization (delegation vs owner-signed).
pub fn validate_consume(state: &State, tx: &SignedTx, ctx: &ValidationContext) -> Result<u64> {
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

    let cost = validate_consume_metering(state, owner, service_id, *units, pricing)?;
    let is_delegated = tx.signer != *owner || tx.delegation_proof.is_some();

    if is_delegated {
        validate_consume_delegation(state, tx, ctx, owner, service_id, *units, cost)?;
    } else {
        validate_consume_owner(state, tx, owner, cost)?;
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

/// Validate a RevokeDelegation transaction.
///
/// Checks: signer == owner, owner account exists, nonce matches.
pub fn validate_revoke_delegation(state: &State, tx: &SignedTx) -> Result<()> {
    let Transaction::RevokeDelegation {
        owner,
        capability_id: _,
    } = &tx.kind
    else {
        return Err(Error::InvalidTransaction(
            "Expected RevokeDelegation transaction".to_string(),
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

    Ok(())
}

/// Main validation function that dispatches to specific validators
///
/// Returns Ok(()) for Mint, OpenMeter, CloseMeter, RevokeDelegation
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
        Transaction::RevokeDelegation { .. } => {
            validate_revoke_delegation(state, tx)?;
            Ok(None)
        }
        Transaction::ProposeSettlement { .. } => {
            validate_propose_settlement(state, tx, authorized_minters)?;
            Ok(None)
        }
        Transaction::FinalizeSettlement { .. } => {
            validate_finalize_settlement(state, tx, authorized_minters)?;
            Ok(None)
        }
        Transaction::SubmitClaim { .. } => {
            validate_submit_claim(state, tx)?;
            Ok(None)
        }
        Transaction::PayClaim { .. } => {
            validate_pay_claim(state, tx, authorized_minters)?;
            Ok(None)
        }
    }
}

fn validate_propose_settlement(
    state: &State,
    tx: &SignedTx,
    authorized_minters: Option<&std::collections::HashSet<String>>,
) -> Result<()> {
    let Transaction::ProposeSettlement {
        owner,
        service_id,
        window_id,
        gross_spent,
        operator_share,
        protocol_fee,
        reserve_locked,
        evidence_hash,
        ..
    } = &tx.kind
    else {
        unreachable!()
    };
    if let Some(minters) = authorized_minters {
        if !minters.contains(&tx.signer) {
            return Err(Error::InvalidTransaction(format!(
                "ProposeSettlement: signer {} must be authorized minter/admin",
                tx.signer
            )));
        }
    }
    let expected_nonce = state
        .get_account(&tx.signer)
        .map(|a| a.nonce())
        .unwrap_or(0);
    if tx.nonce != expected_nonce {
        return Err(Error::InvalidTransaction(format!(
            "ProposeSettlement: Nonce mismatch for signer {}: expected {}, got {}",
            tx.signer, expected_nonce, tx.nonce
        )));
    }
    let id = SettlementId::new(owner.clone(), service_id.clone(), window_id.clone());
    if state.get_settlement(&id).is_some() {
        return Err(Error::DuplicateSettlementWindow);
    }
    if *gross_spent
        != operator_share
            .saturating_add(*protocol_fee)
            .saturating_add(*reserve_locked)
    {
        return Err(Error::SettlementConservationViolation);
    }
    if evidence_hash.is_empty() {
        return Err(Error::InvalidTransaction(
            "evidence_hash required for ProposeSettlement".to_string(),
        ));
    }
    Ok(())
}

fn validate_finalize_settlement(
    state: &State,
    tx: &SignedTx,
    authorized_minters: Option<&std::collections::HashSet<String>>,
) -> Result<()> {
    let Transaction::FinalizeSettlement {
        owner,
        service_id,
        window_id,
    } = &tx.kind
    else {
        unreachable!()
    };
    if let Some(minters) = authorized_minters {
        if !minters.contains(&tx.signer) {
            return Err(Error::InvalidTransaction(format!(
                "FinalizeSettlement: signer {} must be authorized minter/admin",
                tx.signer
            )));
        }
    }
    let expected_nonce = state
        .get_account(&tx.signer)
        .map(|a| a.nonce())
        .unwrap_or(0);
    if tx.nonce != expected_nonce {
        return Err(Error::InvalidTransaction(format!(
            "FinalizeSettlement: Nonce mismatch for signer {}: expected {}, got {}",
            tx.signer, expected_nonce, tx.nonce
        )));
    }
    let id = SettlementId::new(owner.clone(), service_id.clone(), window_id.clone());
    let s = state
        .get_settlement(&id)
        .ok_or(Error::SettlementNotFound)?;
    if s.status != crate::state::SettlementStatus::Proposed {
        return Err(Error::SettlementNotProposed);
    }
    if s.is_disputed() {
        return Err(Error::InvalidTransaction(
            "Cannot finalize: settlement has open dispute".to_string(),
        ));
    }
    Ok(())
}

fn validate_submit_claim(state: &State, tx: &SignedTx) -> Result<()> {
    let Transaction::SubmitClaim {
        operator,
        owner,
        service_id,
        window_id,
        claim_amount,
    } = &tx.kind
    else {
        unreachable!()
    };
    if tx.signer != *operator {
        return Err(Error::InvalidTransaction(format!(
            "SubmitClaim: signer {} must equal operator {}",
            tx.signer, operator
        )));
    }
    let expected_nonce = state
        .get_account(operator)
        .map(|a| a.nonce())
        .unwrap_or(0);
    if tx.nonce != expected_nonce {
        return Err(Error::InvalidTransaction(format!(
            "SubmitClaim: Nonce mismatch for operator {}: expected {}, got {}",
            operator, expected_nonce, tx.nonce
        )));
    }
    let sid = SettlementId::new(owner.clone(), service_id.clone(), window_id.clone());
    let s = state
        .get_settlement(&sid)
        .ok_or(Error::SettlementNotFound)?;
    if !s.is_finalized() {
        return Err(Error::SettlementNotFinalized);
    }
    let payable = s.payable();
    if *claim_amount > payable {
        return Err(Error::ClaimAmountExceedsPayable);
    }
    if *claim_amount == 0 {
        return Err(Error::InvalidTransaction(
            "claim_amount must be positive".to_string(),
        ));
    }
    let cid = ClaimId::new(operator.clone(), &sid);
    if state.get_claim(&cid).is_some() {
        return Err(Error::InvalidTransaction(
            "Claim already exists for this operator/settlement".to_string(),
        ));
    }
    Ok(())
}

fn validate_pay_claim(
    state: &State,
    tx: &SignedTx,
    authorized_minters: Option<&std::collections::HashSet<String>>,
) -> Result<()> {
    let Transaction::PayClaim {
        operator,
        owner,
        service_id,
        window_id,
    } = &tx.kind
    else {
        unreachable!()
    };
    if let Some(minters) = authorized_minters {
        if !minters.contains(&tx.signer) {
            return Err(Error::InvalidTransaction(format!(
                "PayClaim: signer {} must be authorized minter/admin",
                tx.signer
            )));
        }
    }
    let expected_nonce = state
        .get_account(&tx.signer)
        .map(|a| a.nonce())
        .unwrap_or(0);
    if tx.nonce != expected_nonce {
        return Err(Error::InvalidTransaction(format!(
            "PayClaim: Nonce mismatch for signer {}: expected {}, got {}",
            tx.signer, expected_nonce, tx.nonce
        )));
    }
    let sid = SettlementId::new(owner.clone(), service_id.clone(), window_id.clone());
    let cid = ClaimId::new(operator.clone(), &sid);
    let c = state.get_claim(&cid).ok_or(Error::ClaimNotPending)?;
    if !c.is_pending() {
        return Err(Error::ClaimNotPending);
    }
    let s = state
        .get_settlement(&sid)
        .ok_or(Error::SettlementNotFound)?;
    if s.is_disputed() {
        return Err(Error::InvalidTransaction(
            "Payout frozen by dispute".to_string(),
        ));
    }
    Ok(())
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
    fn test_principal_to_chain_address_ok() {
        let hex32 = "0x".to_string() + &"a".repeat(64);
        let out = principal_to_chain_address(&hex32).unwrap();
        assert_eq!(out, "0x".to_string() + &"a".repeat(64));
        let hex_upper = "0x".to_string() + &"Ab".repeat(32);
        let out = principal_to_chain_address(&hex_upper).unwrap();
        assert_eq!(out, "0x".to_string() + &"ab".repeat(32));
    }

    #[test]
    fn test_principal_to_chain_address_reject_non_0x() {
        let err = principal_to_chain_address("alice").unwrap_err();
        match &err {
            Error::PrincipalBindingFailed(msg) => {
                assert!(msg.contains("0x") || msg.contains("did:key"));
            }
            _ => panic!("expected PrincipalBindingFailed, got {:?}", err),
        }
    }

    #[test]
    fn test_principal_to_chain_address_did_key_roundtrip() {
        use crate::wallet::Wallet;
        let wallet = Wallet::new_random();
        let address = wallet.address().to_string();
        let hex_body = address.strip_prefix("0x").unwrap();
        let key_bytes: [u8; 32] = hex::decode(hex_body).unwrap().try_into().unwrap();
        let mut payload = vec![0xed, 0x01];
        payload.extend_from_slice(&key_bytes);
        let multibase = "did:key:z".to_string() + &bs58::encode(payload).into_string();
        let out = principal_to_chain_address(&multibase).unwrap();
        assert_eq!(out, address, "did:key round-trip must match wallet address");
    }

    #[test]
    fn test_principal_to_chain_address_did_key_spec_vector() {
        // W3C did:key spec Ed25519 example
        let did = "did:key:z6Mkf5rGMoatrSj1f4CyvuHBeXJELe9RPdzo2PKGNCKVtZxP";
        let out = principal_to_chain_address(did).unwrap();
        assert!(out.starts_with("0x") && out.len() == 66);
        assert!(out.chars().skip(2).all(|c| c.is_ascii_hexdigit()));
        // Same input must always produce same output (determinism)
        assert_eq!(principal_to_chain_address(did).unwrap(), out);
    }

    #[test]
    fn test_principal_to_chain_address_did_key_reject_wrong_multicodec() {
        // Build a did:key with wrong multicodec (e.g. 0xec 0x01 for x25519) — decode will succeed but we reject non-Ed25519
        let key_bytes = [0u8; 32];
        let mut payload = vec![0xec, 0x01]; // x25519-pub
        payload.extend_from_slice(&key_bytes);
        let multibase = "did:key:z".to_string() + &bs58::encode(payload).into_string();
        let err = principal_to_chain_address(&multibase).unwrap_err();
        match &err {
            Error::PrincipalBindingFailed(msg) => {
                assert!(msg.contains("Ed25519") || msg.contains("0xed"))
            }
            _ => panic!("expected PrincipalBindingFailed, got {:?}", err),
        }
    }

    #[test]
    fn test_principal_to_public_key_did_key() {
        use crate::wallet::Wallet;
        // principal_to_public_key(issuer) is used for proof verification; did:key issuer must yield same key as 0x.
        let wallet = Wallet::new_random();
        let address = wallet.address().to_string();
        let hex_body = address.strip_prefix("0x").unwrap();
        let key_bytes: [u8; 32] = hex::decode(hex_body).unwrap().try_into().unwrap();
        let mut payload = vec![0xed, 0x01];
        payload.extend_from_slice(&key_bytes);
        let did_key = "did:key:z".to_string() + &bs58::encode(payload).into_string();
        let from_did = principal_to_public_key(&did_key).unwrap();
        assert_eq!(
            from_did, key_bytes,
            "principal_to_public_key(did:key) must match 0x pubkey for proof verification"
        );
    }

    #[test]
    fn test_principal_to_public_key_did_key_reject_non_canonical_length() {
        // did:key with decoded length > 34 (canonical 2+32) must be rejected (no trailing bytes).
        let key_bytes = [0u8; 32];
        let mut payload = vec![0xed, 0x01];
        payload.extend_from_slice(&key_bytes);
        payload.push(0); // 35 bytes total
        let multibase = "did:key:z".to_string() + &bs58::encode(payload).into_string();
        let err = principal_to_public_key(&multibase).unwrap_err();
        match &err {
            Error::PrincipalBindingFailed(msg) => {
                assert!(
                    msg.contains("length") || msg.contains("0xed"),
                    "expected length/header message: {}",
                    msg
                );
            }
            _ => panic!("expected PrincipalBindingFailed, got {:?}", err),
        }
    }
}
