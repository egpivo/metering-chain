# Compatibility fixtures

This directory holds versioned fixtures for schema/tx/event compatibility tests.

## Current

- **EvidenceBundle (v0):** No on-disk file yet; v0 behavior is tested in `src/evidence.rs` (`test_evidence_bundle_v0_protocol_rejected`). Old serialized bundles (without `schema_version` / `replay_protocol_version`) deserialize with default 0 and are rejected with `ReplayProtocolMismatch`.

## When adding v2 schema

- Add fixtures for **v1** (current) schema: e.g. `evidence_bundle_v1.bin`, settlement/dispute/policy samples.
- Add tests: N (current binary) replays N-1 fixture → same result or explicit reject.
- Add deterministic upcasters (v1 → v2) if needed; test idempotence.

See `.local/phase4_versioning_coverage_policy.md` and `.local/final_hardening_task_list.md` (F2).
