# Naming Conventions

For consistency with Phase 4 Settlement/Dispute context.

## Commands (Transactions)

- **Pattern**: VerbNoun or Verb (e.g. `OpenMeter`, `CloseMeter`, `Mint`, `Consume`)
- Phase 4: `ProposeSettlement`, `FinalizeSettlement`, `SubmitClaim`, `PayClaim`, `OpenDispute`, `ResolveDispute`

## Events (Phase 4)

- **Pattern**: NounPastParticiple (e.g. `SettlementProposed`, `SettlementFinalized`)
- Phase 4: `SettlementProposed`, `SettlementFinalized`, `ClaimSubmitted`, `ClaimPaid`, `DisputeOpened`, `DisputeResolved`

## State Labels (UI)

- Use deterministic language; avoid fuzzy finance wording.
- See Phase 4 UX story `State Language` section.
