# Replay and audit troubleshooting runbook

For operators and support: how to inspect evidence and replay state when debugging settlement/dispute flows.

## One-command audit chain

After a settlement is finalized and (optionally) a dispute resolved, you can show the full evidence and replay audit chain:

```bash
# 1. Settlement (includes schema_version; replay_hash/replay_summary if dispute was resolved)
cargo run --bin metering-chain -- settlement show <owner> <service_id> <window_id> --format json

# 2. Dispute (if any): status and resolution_audit (replay_protocol_version, replay_hash, replay_summary)
cargo run --bin metering-chain -- settlement dispute show <owner> <service_id> <window_id> --format json

# 3. Evidence bundle (schema_version, replay_protocol_version, hashes, replay_summary)
cargo run --bin metering-chain -- settlement evidence show <owner> <service_id> <window_id> --format json
```

Use `--format json` for machine-readable output; omit for human-readable.

## Version fields

| Field | Meaning |
|-------|--------|
| `schema_version` | Version of the persisted record (Settlement, Dispute, EvidenceBundle). Reader must support this version. |
| `replay_protocol_version` | Replay hash/canonicalization contract. Mismatch with current binary ⇒ `REPLAY_PROTOCOL_MISMATCH`. |

If you see `UNSUPPORTED_SCHEMA_VERSION` or `REPLAY_PROTOCOL_MISMATCH`, the data was produced with a different schema or replay protocol; upgrade the binary or run a migration path (see versioning policy).

## Common errors

| Code | What to do |
|------|------------|
| `REPLAY_MISMATCH` | Replay result does not match settlement totals or replay_hash. Re-run replay for the window and compare; check tx log integrity. |
| `EVIDENCE_NOT_FOUND` | No resolution audit for this settlement (dispute not resolved, or evidence not stored). |
| `INVALID_EVIDENCE_BUNDLE` | Bundle shape invalid (e.g. from_tx_id ≥ to_tx_id, or tx_count mismatch). Fix evidence input. |

## Determinism checks

Same tx log ⇒ same final state and same replay hash. To verify:

1. Run `apply` with the same log twice; state hashes should match.
2. `replay_slice_to_summary` for the same window yields the same `replay_hash`.

See `docs/invariants.md` and Phase 4 G4 tests in `tests/basic_flow.rs`.
