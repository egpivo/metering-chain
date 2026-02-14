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
}

pub type Result<T> = std::result::Result<T, Error>;
