# Naming Conventions

For consistency with Phase 4 Settlement/Dispute/Policy context.

## Commands (Transactions)

- **Pattern**: VerbNoun or Verb (e.g. `OpenMeter`, `CloseMeter`, `Mint`, `Consume`)
- Phase 4 Settlement/Dispute: `ProposeSettlement`, `FinalizeSettlement`, `SubmitClaim`, `PayClaim`, `OpenDispute`, `ResolveDispute`
- Phase 4 Policy (G3): `PublishPolicyVersion`, `SupersedePolicyVersion`

## Events (Phase 4)

- **Pattern**: NounPastParticiple (e.g. `SettlementProposed`, `SettlementFinalized`)
- Settlement/Dispute: `SettlementProposed`, `SettlementFinalized`, `ClaimSubmitted`, `ClaimPaid`, `DisputeOpened`, `DisputeResolved`
- Policy: `PolicyVersionPublished`, `PolicyVersionSuperseded`

## Policy domain (G3)

- **Scopes**: `PolicyScope::Global`, `PolicyScope::Owner`, `PolicyScope::OwnerService`
- **Scope keys** (storage/audit): `global`, `owner:{owner}`, `owner_service:{owner}:{service_id}`
- **Tx names**: `PublishPolicyVersion`, `SupersedePolicyVersion` (no "PolicyVersionPublish" etc.)

## State Labels (UI)

- Use deterministic language; avoid fuzzy finance wording.
- See Phase 4 UX story `State Language` section.
