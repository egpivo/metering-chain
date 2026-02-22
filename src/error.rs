use thiserror::Error;

/// Domain error codes for deterministic UI mapping.
/// See docs/error_codes.md for the full taxonomy.
impl Error {
    pub fn error_code(&self) -> &'static str {
        use Error::*;
        match self {
            MiningExhausted => "MINING_EXHAUSTED",
            InvalidTransaction(_) => "INVALID_TRANSACTION",
            StateError(_) => "STATE_ERROR",
            SignatureVerification(_) => "SIGNATURE_VERIFICATION_FAILED",
            DelegatedConsumeRequiresV2 => "DELEGATED_CONSUME_REQUIRES_V2",
            DelegationProofMissing => "DELEGATION_PROOF_MISSING",
            ValidAtMissing => "VALID_AT_MISSING",
            NonceAccountMissingOrInvalid => "NONCE_ACCOUNT_MISSING_OR_INVALID",
            InvalidValidationContextLiveNowMissing => "VALIDATION_CONTEXT_LIVE_NOW_MISSING",
            InvalidValidationContextLiveMaxAgeMissing => "VALIDATION_CONTEXT_LIVE_MAX_AGE_MISSING",
            ReferenceTimeFuture => "REFERENCE_TIME_FUTURE",
            ReferenceTimeTooOld => "REFERENCE_TIME_TOO_OLD",
            DelegationExpiredOrNotYetValid => "DELEGATION_EXPIRED_OR_NOT_YET_VALID",
            PrincipalBindingFailed(_) => "PRINCIPAL_BINDING_FAILED",
            DelegationIssuerOwnerMismatch => "DELEGATION_ISSUER_OWNER_MISMATCH",
            DelegationAudienceSignerMismatch => "DELEGATION_AUDIENCE_SIGNER_MISMATCH",
            CapabilityLimitExceeded => "CAPABILITY_LIMIT_EXCEEDED",
            DelegationRevoked => "DELEGATION_REVOKED",
            DelegationScopeMismatch => "DELEGATION_SCOPE_MISMATCH",
            DuplicateSettlementWindow => "DUPLICATE_SETTLEMENT_WINDOW",
            SettlementNotFound => "SETTLEMENT_NOT_FOUND",
            SettlementNotProposed => "SETTLEMENT_NOT_PROPOSED",
            SettlementNotFinalized => "SETTLEMENT_NOT_FINALIZED",
            ClaimAmountExceedsPayable => "CLAIM_AMOUNT_EXCEEDS_PAYABLE",
            ClaimNotPending => "CLAIM_NOT_PENDING",
            SettlementConservationViolation => "SETTLEMENT_CONSERVATION_VIOLATION",
            DisputeAlreadyOpen => "DISPUTE_ALREADY_OPEN",
            DisputeNotFound => "DISPUTE_NOT_FOUND",
            DisputeNotOpen => "DISPUTE_NOT_OPEN",
            InvalidPolicyParameters => "INVALID_POLICY_PARAMETERS",
            PolicyVersionConflict => "POLICY_VERSION_CONFLICT",
            PolicyNotFound => "POLICY_NOT_FOUND",
            PolicyNotEffective => "POLICY_NOT_EFFECTIVE",
            RetroactivePolicyForbidden => "RETROACTIVE_POLICY_FORBIDDEN",
            InvalidEvidenceBundle => "INVALID_EVIDENCE_BUNDLE",
            ReplayMismatch => "REPLAY_MISMATCH",
            EvidenceNotFound => "EVIDENCE_NOT_FOUND",
            UnsupportedSchemaVersion => "UNSUPPORTED_SCHEMA_VERSION",
            UnsupportedTxVersion => "UNSUPPORTED_TX_VERSION",
            UnsupportedEventVersion => "UNSUPPORTED_EVENT_VERSION",
            ReplayProtocolMismatch => "REPLAY_PROTOCOL_MISMATCH",
            MigrationRequired => "MIGRATION_REQUIRED",
        }
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Mining failed: exhausted nonce range without finding valid hash")]
    MiningExhausted,

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("State error: {0}")]
    StateError(String),

    #[error("Signature verification failed: {0}")]
    SignatureVerification(String),

    // Phase 3 delegation
    #[error("Delegated Consume requires payload_version=2")]
    DelegatedConsumeRequiresV2,

    #[error("Delegation proof missing")]
    DelegationProofMissing,

    #[error("valid_at (reference time) missing for delegated consume")]
    ValidAtMissing,

    #[error("nonce_account missing or invalid for delegated consume")]
    NonceAccountMissingOrInvalid,

    #[error("Live validation context requires now")]
    InvalidValidationContextLiveNowMissing,

    #[error("Live validation context requires max_age")]
    InvalidValidationContextLiveMaxAgeMissing,

    #[error("Reference time (valid_at) is in the future")]
    ReferenceTimeFuture,

    #[error("Reference time (valid_at) too old (exceeds max_age)")]
    ReferenceTimeTooOld,

    #[error("Delegation expired or not yet valid")]
    DelegationExpiredOrNotYetValid,

    #[error("Principal binding failed: {0}")]
    PrincipalBindingFailed(String),

    #[error("Delegation issuer does not match owner")]
    DelegationIssuerOwnerMismatch,

    #[error("Delegation audience does not match signer")]
    DelegationAudienceSignerMismatch,

    #[error("Capability limit exceeded")]
    CapabilityLimitExceeded,

    #[error("Delegation revoked")]
    DelegationRevoked,

    #[error("Delegation scope mismatch: proof service_id or ability does not match transaction")]
    DelegationScopeMismatch,

    // Phase 4A: Settlement
    #[error("Duplicate settlement window: same (owner, service_id, window_id) already exists")]
    DuplicateSettlementWindow,
    #[error("Settlement not found")]
    SettlementNotFound,
    #[error("Settlement not proposed or already finalized")]
    SettlementNotProposed,
    #[error("Settlement not finalized")]
    SettlementNotFinalized,
    #[error("Claim amount exceeds payable")]
    ClaimAmountExceedsPayable,
    #[error("Claim not found or not pending")]
    ClaimNotPending,
    #[error("Invalid settlement: gross_spent != operator_share + protocol_fee + reserve")]
    SettlementConservationViolation,

    // Phase 4B: Dispute
    #[error("Dispute already open for this settlement")]
    DisputeAlreadyOpen,
    #[error("Dispute not found")]
    DisputeNotFound,
    #[error("Dispute not open (already resolved)")]
    DisputeNotOpen,

    // Phase 4C (G3): Policy
    #[error("Invalid policy parameters (e.g. bps sum != 10000 or dispute_window_secs == 0)")]
    InvalidPolicyParameters,
    #[error("Policy version conflict (duplicate scope:version)")]
    PolicyVersionConflict,
    #[error("Policy not found")]
    PolicyNotFound,
    #[error("Policy not effective at this tx_id")]
    PolicyNotEffective,
    #[error("Retroactive policy forbidden (effective_from_tx_id < next_tx_id)")]
    RetroactivePolicyForbidden,

    // Phase 4 G4: Evidence / replay
    #[error("Invalid evidence bundle (shape or required fields)")]
    InvalidEvidenceBundle,
    #[error("Replay result does not match settlement totals or replay_hash")]
    ReplayMismatch,
    #[error("Evidence or bundle not found")]
    EvidenceNotFound,

    // Phase 4+ Versioning (final hardening)
    #[error("Unsupported schema version (reader does not support this record version)")]
    UnsupportedSchemaVersion,
    #[error("Unsupported transaction version")]
    UnsupportedTxVersion,
    #[error("Unsupported event version")]
    UnsupportedEventVersion,
    #[error("Replay protocol version mismatch (hash/serialization contract changed)")]
    ReplayProtocolMismatch,
    #[error("Migration required (data must be upgraded before use)")]
    MigrationRequired,
}

pub type Result<T> = std::result::Result<T, Error>;
