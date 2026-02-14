# Error Code Taxonomy

Deterministic, non-ambiguous error codes for UI mapping. Phase 4 UX story maps these to user-facing messages and suggested actions.

## Phase 1–3 (Current)

| Code | Variant | Suggested UI action |
|------|---------|---------------------|
| `MINING_EXHAUSTED` | MiningExhausted | Retry or increase nonce range |
| `INVALID_TRANSACTION` | InvalidTransaction | Check tx payload; see message for details |
| `STATE_ERROR` | StateError | Check state consistency; replay if needed |
| `SIGNATURE_VERIFICATION_FAILED` | SignatureVerification | Verify signer and payload |
| `DELEGATED_CONSUME_REQUIRES_V2` | DelegatedConsumeRequiresV2 | Use payload_version=2 for delegated Consume |
| `DELEGATION_PROOF_MISSING` | DelegationProofMissing | Include delegation_proof in tx |
| `VALID_AT_MISSING` | ValidAtMissing | Include valid_at for delegated Consume |
| `NONCE_ACCOUNT_MISSING_OR_INVALID` | NonceAccountMissingOrInvalid | Set nonce_account to owner for delegated Consume |
| `VALIDATION_CONTEXT_LIVE_NOW_MISSING` | InvalidValidationContextLiveNowMissing | Provide now in Live context |
| `VALIDATION_CONTEXT_LIVE_MAX_AGE_MISSING` | InvalidValidationContextLiveMaxAgeMissing | Provide max_age in Live context |
| `REFERENCE_TIME_FUTURE` | ReferenceTimeFuture | valid_at must not be in future |
| `REFERENCE_TIME_TOO_OLD` | ReferenceTimeTooOld | valid_at exceeds max_age |
| `DELEGATION_EXPIRED_OR_NOT_YET_VALID` | DelegationExpiredOrNotYetValid | Check proof iat/exp and valid_at |
| `PRINCIPAL_BINDING_FAILED` | PrincipalBindingFailed | Use 0x+hex or did:key (Ed25519) |
| `DELEGATION_ISSUER_OWNER_MISMATCH` | DelegationIssuerOwnerMismatch | Proof issuer must match meter owner |
| `DELEGATION_AUDIENCE_SIGNER_MISMATCH` | DelegationAudienceSignerMismatch | Proof audience must match tx signer |
| `CAPABILITY_LIMIT_EXCEEDED` | CapabilityLimitExceeded | Check max_units/max_cost caveat |
| `DELEGATION_REVOKED` | DelegationRevoked | Capability was revoked; obtain new proof |
| `DELEGATION_SCOPE_MISMATCH` | DelegationScopeMismatch | Proof service_id/ability must match tx |

## Phase 4 (Planned)

Extension points for Settlement and Dispute contexts. See `.local/phase4_spec.md` and `phase4_ux_story.md`.
