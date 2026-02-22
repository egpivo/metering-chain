# Release notes (draft)

Use this template for the next release tag. Classify changes as **Breaking**, **Feature**, or **Fix**.

---

## [Unreleased]

### Breaking

- **Domain records now have `schema_version`.** Settlement, Dispute, EvidenceBundle, and PolicyVersion include a `schema_version` field (default 1 when omitted). Old persisted state without the field deserializes with default 0. EvidenceBundle validates version: unsupported schema → `UNSUPPORTED_SCHEMA_VERSION`, replay protocol mismatch → `REPLAY_PROTOCOL_MISMATCH`. If you have custom tooling that parses state or evidence, ensure it handles the new field.
- **CLI `settlement show` / `dispute show` / `evidence show` output** now include `schema_version` and (where applicable) `replay_protocol_version`. Scripts that parse this output may need to tolerate new keys.

### Feature

- **Versioning and auditability (Phase 4+ hardening).**
  - `EvidenceBundle` and resolution audit carry `replay_protocol_version`; replay contract version is fixed and rejected when mismatched.
  - New error codes: `UNSUPPORTED_SCHEMA_VERSION`, `UNSUPPORTED_TX_VERSION`, `UNSUPPORTED_EVENT_VERSION`, `REPLAY_PROTOCOL_MISMATCH`, `MIGRATION_REQUIRED` (see `docs/error_codes.md`).
  - CLI `settlement show`, `dispute show`, `evidence show` expose schema and replay protocol versions for troubleshooting.
- **Runbook:** `docs/replay_audit_runbook.md` for replay/audit troubleshooting and one-command audit chain.
- **CI:** Coverage job with `cargo llvm-cov`; gate at 50% line coverage (policy target 70%).
- **README:** Release and compatibility section; link to error codes and audit runbook.

### Fix

- (None in this draft.)

---

## Classification notes

- **Breaking:** Changes that can break existing callers or persisted data format.
- **Feature:** Backward-compatible additions (new fields with defaults, new commands, new docs).
- **Fix:** Bug fixes and behavior corrections without API/schema break.

When cutting the release, move the **[Unreleased]** block into a versioned section (e.g. `## [0.2.0] - 2026-02-XX`) and clear or update **[Unreleased]**.
