# Phase 4 Extension Points

Documented extension points for Settlement and Dispute contexts after Pre-Phase 4 refactoring.

## Validation

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

## Storage

- `Storage::load_txs_from(from_tx_id)` already supports tx-slice loading for replay.
- Phase 4 EvidenceBundle will reference `(from_tx_id, to_tx_id)` or similar.
