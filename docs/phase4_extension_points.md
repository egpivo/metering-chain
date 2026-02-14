# Phase 4 Extension Points

Documented extension points for Settlement and Dispute contexts after Pre-Phase 4 refactoring.

## Validation (WS-R2)

- **Flow**: auth checks → domain checks → replay/evidence checks (see `docs/validation_flow.md`)
- **Error matrix**: per-tx error codes in `docs/error_codes.md`
- **Authorization vs Metering** (isolated):
  - `validate_consume_metering`: meter, units, pricing, cost (shared)
  - `validate_consume_delegation`: proof, time, scope, caveats, nonce, balance
  - `validate_consume_owner`: signer == owner, nonce, balance

## Error Taxonomy

- See `docs/error_codes.md` for deterministic codes for UI mapping.
- `Error::error_code()` returns stable strings (e.g. `DELEGATION_REVOKED`).

## Evidence / Replay

- `src/evidence.rs`:
  - `evidence_hash(data)`: SHA256 hex for hashing
  - `tx_slice_hash(txs)`: hash of tx slice (Phase 4 will define canonical serialization)
  - `ReplaySummary`: placeholder for Phase 4 replay summary

## Naming

- See `docs/naming_conventions.md` for Commands (VerbNoun) and Events (NounPastParticiple).

## Replay Service (WS-R3)

- `src/replay.rs`:
  - `replay_to_tip(storage)`: load state from storage by replaying tx log to tip; used by CLI and tests
  - `load_tx_slice(storage, from_tx_id)`: load tx slice for evidence bundle (Phase 4)

## Storage

- `Storage::load_txs_from(from_tx_id)` supports tx-slice loading for replay.
- Phase 4 EvidenceBundle will reference `(from_tx_id, to_tx_id)` or similar.
