use thiserror::Error;

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
