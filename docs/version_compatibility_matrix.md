# Version Compatibility Matrix (first pass)

Date: 2026-03-27  
Owner: `metering-chain`  
Scope: tx log, snapshot/state, evidence, CLI JSON, frontend payloads

---

## Legend

- **Supported**: expected to load/replay successfully.
- **Deterministic reject**: expected to fail with stable `error_code`.
- **Migration required**: currently no runtime upgrader; manual migration or newer path needed.

## Compatibility policy

- No silent reinterpretation of persisted artifacts.
- Unsupported or inconsistent artifacts should fail closed with deterministic behavior.
- In v1, CLI/frontend contracts allow additive-only evolution.
- Breaking persisted-format changes require either explicit migration or a new version gate.

This matrix is intentionally fail-closed:
compatibility here does not mean "best effort parsing".
It means explicit acceptance or explicit deterministic rejection.

## Contract tiers

- Persistence contract: tx log, snapshot/state, evidence bundle
  These affect replay, recovery, and long-lived stored artifacts; compatibility should be fail-closed.
- Interface contract: CLI JSON, frontend payloads
  These are consumer-facing contracts where additive-only evolution is allowed in v1, subject to stable required fields.

---

## Artifact matrix

| Artifact | Producer format | Consumer binary | Expected behavior | Notes |
|---|---:|---:|---|---|
| tx log (`tx.log`, length-prefixed bincode `SignedTx`) | tx-format v1 | current runtime (v1 line) | Supported within the current `SignedTx` schema and runtime line | Verified by replay tests and fixture-backed compatibility path; not yet a long-term stable interchange contract across structural tx schema changes. |
| tx log with malformed/truncated payload | malformed payload | current runtime (v1 line) | Deterministic reject (`STATE_ERROR`) | Covered in `tests/replay_recovery.rs`. |
| snapshot (`state.bin`) + valid cursor | snapshot-format v1 | current runtime (v1 line) | Supported | Replay tail from `next_tx_id`. |
| snapshot with corrupted bytes | corrupted snapshot bytes | current runtime (v1 line) | Deterministic reject (`STATE_ERROR`) | Covered in `tests/replay_recovery.rs`. |
| snapshot cursor > tx log tip | inconsistent snapshot metadata | current runtime (v1 line) | Deterministic reject (`STATE_ERROR`) | Explicit guard in `replay_to_tip`; fail closed to avoid silent tx skipping. |
| EvidenceBundle schema_version=1 + replay_protocol_version=1 | schema v1 + replay protocol v1 | current runtime (v1 line) | Supported | Fixture: `tests/fixtures/evidence_bundle_v1.json`. |
| EvidenceBundle schema_version > supported | future schema | current runtime (v1 line) | Deterministic reject (`UNSUPPORTED_SCHEMA_VERSION`) | Covered in compatibility/evidence tests. |
| EvidenceBundle replay_protocol_version mismatch | other replay protocol | current runtime (v1 line) | Deterministic reject (`REPLAY_PROTOCOL_MISMATCH`) | Covered in compatibility/evidence tests. |
| CLI JSON (settlement/dispute/evidence show) | CLI JSON contract v1 | current consumer of CLI JSON v1 | Supported with additive fields | Existing required keys must retain name, meaning, and type; removal, rename, semantic, or type changes are breaking in v1. |
| Frontend snapshot payload (`phase4_snapshot.json`) | frontend payload contract v1 | current frontend adapter for payload v1 | Supported with additive fields | Existing required fields must retain name, meaning, and type; removal, rename, semantic, or type changes are breaking in v1. |

---

## Error-code contract (compatibility failures)

Expected stable machine-readable codes:

- `STATE_ERROR` (corrupted/truncated storage artifacts)
- `UNSUPPORTED_SCHEMA_VERSION`
- `REPLAY_PROTOCOL_MISMATCH`
- Reserved for future tx/event envelope versioning:
  - `UNSUPPORTED_TX_VERSION`
  - `UNSUPPORTED_EVENT_VERSION`
  - `MIGRATION_REQUIRED`

Note: tx/event version runtime paths are not implemented yet; codes are defined as forward-compatibility contract.
`STATE_ERROR` is currently a coarse storage/compatibility failure bucket. Future refinements may split this into narrower codes, but unsupported or corrupted artifacts should continue to fail deterministically.

---

## Rollback / forward-fix expectations

### Rollback-safe (generally safe)

- CI/test-only changes with no persisted-format changes.
- Frontend-only additive UI changes.
- Doc-only and checklist/runbook updates.

### Forward-fix preferred (rollback risky or low-value)

- Evidence schema/protocol guard behavior changes after data has been produced.
- Storage/replay semantics changes (`replay_to_tip`, cursor handling).
- Changes affecting deterministic replay outputs or error-code surfaces.

### One-way / migration-required candidates

- Future introduction of tx/event envelope versions (`tx_version`, `event_version`).
- Future schema bump that cannot be represented by `serde(default)` compatibility.
- Replay protocol hash/canonicalization change (requires explicit protocol bump and upgrade path).
- Future snapshot format v2 that requires an explicit upgrader before load.
- Future tx envelope v2 produced by a newer runtime but unsupported by an older binary.

---

## Release rehearsal checks

Run before release when version-affecting changes exist:

1. replay old fixture(s) under new binary:
   - success where supported
   - deterministic reject where unsupported
2. verify no silent reinterpretation:
   - schema/protocol mismatch must not be auto-accepted
3. verify operator diagnostics:
   - error_code stable and documented
   - runbook path can explain failure

---

## Follow-ups (next iteration)

- Add concrete tx-log fixture(s) generated by prior release artifacts (not hand-authored only).
- Add explicit CLI JSON schema snapshot tests for downstream consumers.
- When tx/event versioning is implemented:
  - add positive and negative runtime tests for `UNSUPPORTED_TX_VERSION`, `UNSUPPORTED_EVENT_VERSION`, and `MIGRATION_REQUIRED`.
