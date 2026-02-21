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

## Validation Error Code Matrix (by Tx Type)

| Tx Type | Possible Error Codes |
|---------|----------------------|
| Mint | `INVALID_TRANSACTION`, `MINT_*` (via InvalidTransaction message) |
| OpenMeter | `INVALID_TRANSACTION` (signer/owner, nonce, balance, meter) |
| Consume (owner) | `INVALID_TRANSACTION` (meter, units, pricing, signer, nonce, balance) |
| Consume (delegated) | `DELEGATED_CONSUME_REQUIRES_V2`, `DELEGATION_PROOF_MISSING`, `VALID_AT_MISSING`, `NONCE_ACCOUNT_MISSING_OR_INVALID`, `VALIDATION_CONTEXT_LIVE_*`, `REFERENCE_TIME_*`, `DELEGATION_*`, `CAPABILITY_LIMIT_EXCEEDED`, `INVALID_TRANSACTION` |
| CloseMeter | `INVALID_TRANSACTION` (signer, nonce, meter) |
| RevokeDelegation | `INVALID_TRANSACTION` (signer, nonce) |

Use `Error::error_code()` for deterministic UI mapping.

## Phase 4 (Settlement, Dispute, Policy)

| Code | Variant | Suggested UI action |
|------|---------|---------------------|
| `DUPLICATE_SETTLEMENT_WINDOW` | DuplicateSettlementWindow | Use a different window_id for this owner/service |
| `SETTLEMENT_NOT_FOUND` | SettlementNotFound | Propose settlement first |
| `SETTLEMENT_NOT_PROPOSED` | SettlementNotProposed | Settlement must be in Proposed state |
| `SETTLEMENT_NOT_FINALIZED` | SettlementNotFinalized | Finalize settlement before claim |
| `CLAIM_AMOUNT_EXCEEDS_PAYABLE` | ClaimAmountExceedsPayable | claim_amount ≤ remaining payable |
| `CLAIM_NOT_PENDING` | ClaimNotPending | Claim already paid or rejected |
| `SETTLEMENT_CONSERVATION_VIOLATION` | SettlementConservationViolation | gross_spent = operator_share + protocol_fee + reserve_locked |
| `DISPUTE_ALREADY_OPEN` | DisputeAlreadyOpen | Resolve existing dispute first |
| `DISPUTE_NOT_FOUND` | DisputeNotFound | Invalid dispute id |
| `DISPUTE_NOT_OPEN` | DisputeNotOpen | Target dispute must be Open |
| `INVALID_POLICY_PARAMETERS` | InvalidPolicyParameters | operator_share_bps + protocol_fee_bps = 10000; dispute_window_secs > 0 |
| `POLICY_VERSION_CONFLICT` | PolicyVersionConflict | Duplicate (scope, version) or non-monotonic version |
| `POLICY_NOT_FOUND` | PolicyNotFound | No policy for scope (e.g. Supersede target) |
| `POLICY_NOT_EFFECTIVE` | PolicyNotEffective | effective_from_tx_id > current_tx_id |
| `RETROACTIVE_POLICY_FORBIDDEN` | RetroactivePolicyForbidden | effective_from_tx_id must be >= next_tx_id |
| `INVALID_EVIDENCE_BUNDLE` | InvalidEvidenceBundle | Evidence bundle shape invalid or replay_hash empty |
| `REPLAY_MISMATCH` | ReplayMismatch | Replay result does not match settlement totals or replay_hash |
| `EVIDENCE_NOT_FOUND` | EvidenceNotFound | Evidence or bundle not found (optional storage) |

### Phase 4 Validation Error Matrix (by Tx Type)

| Tx Type | Possible Error Codes |
|---------|----------------------|
| ProposeSettlement | `INVALID_TRANSACTION`, `SETTLEMENT_CONSERVATION_VIOLATION`, `DUPLICATE_SETTLEMENT_WINDOW` |
| FinalizeSettlement | `INVALID_TRANSACTION`, `SETTLEMENT_NOT_FOUND`, `SETTLEMENT_NOT_PROPOSED` |
| SubmitClaim | `INVALID_TRANSACTION`, `SETTLEMENT_NOT_FOUND`, `SETTLEMENT_NOT_FINALIZED`, `CLAIM_AMOUNT_EXCEEDS_PAYABLE` |
| PayClaim | `INVALID_TRANSACTION`, `CLAIM_NOT_PENDING`, `CLAIM_AMOUNT_EXCEEDS_PAYABLE` |
| OpenDispute | `INVALID_TRANSACTION`, `SETTLEMENT_NOT_FOUND`, `SETTLEMENT_NOT_FINALIZED`, `DISPUTE_ALREADY_OPEN` (and window check from bound policy) |
| ResolveDispute | `INVALID_TRANSACTION`, `DISPUTE_NOT_FOUND`, `DISPUTE_NOT_OPEN`, `INVALID_EVIDENCE_BUNDLE`, `REPLAY_MISMATCH`, `SETTLEMENT_NOT_FOUND` |
| PublishPolicyVersion | `INVALID_TRANSACTION`, `INVALID_POLICY_PARAMETERS`, `POLICY_VERSION_CONFLICT`, `RETROACTIVE_POLICY_FORBIDDEN` |
| SupersedePolicyVersion | `INVALID_TRANSACTION`, `POLICY_NOT_FOUND` (target not Published) |

Use `Error::error_code()` for deterministic UI mapping. See `.local/phase4_spec.md` and Phase 4 UX story for flows.
