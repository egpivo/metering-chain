# Final demo: run from docs only

A new contributor can run this sequence to verify the binary and replay/audit path without reading code.

## Prerequisites

- Rust toolchain (`cargo --version`)
- Clone repo, `cargo build --release` (optional; `cargo run` works without release)

## 1. Init and basic flow

```bash
cd metering-chain
cargo run --bin metering-chain -- init
cat examples/tx/01_mint_alice.json | cargo run --bin metering-chain -- apply --allow-unsigned
cat examples/tx/02_open_storage.json | cargo run --bin metering-chain -- apply --allow-unsigned
cat examples/tx/03_consume_storage_unit_price.json | cargo run --bin metering-chain -- apply --allow-unsigned
```

**Expected:** No error. State has one meter and consumption recorded.

## 2. Query state (replay/audit surface)

```bash
# Replace 0x...A11 with the address from 01_mint_alice.json (e.g. alice's address in your example)
cargo run --bin metering-chain -- account <address>
cargo run --bin metering-chain -- meters <address>
```

**Expected:** Account shows balance; meters list the open meter and usage.

## 3. Settlement + replay demo (Phase 4)

```bash
cargo run --example settlement_demo
```

**Expected:** Console output: propose → finalize → claim → pay. No panic.

## 4. Signed end-to-end (optional)

```bash
./examples/signed/run_signed_demo.sh
```

**Expected:** Script runs init, mint, open, consume, close with signed tx. See `examples/signed/README.md` if env (e.g. `METERING_CHAIN_MINTERS`) is needed.

## 5. Replay and audit checks (Phase 4+)

After any run that creates settlements (e.g. `settlement_demo` or CLI `settlement propose` …):

```bash
cargo run --bin metering-chain -- settlement list
cargo run --bin metering-chain -- settlement show <owner> <service_id> <window_id> --format json
cargo run --bin metering-chain -- settlement dispute show <owner> <service_id> <window_id> --format json
cargo run --bin metering-chain -- settlement evidence show <owner> <service_id> <window_id> --format json
```

**Expected:** `settlement show` includes `schema_version`; if dispute was resolved, `dispute show` and `evidence show` include `replay_protocol_version`, `replay_hash`, `replay_summary`. See `docs/replay_audit_runbook.md`.

## Determinism check (optional)

Run the same apply sequence twice (e.g. from a clean `init`), then compare state or hashes; they should match. See `docs/invariants.md` and `tests/basic_flow.rs` for regression coverage.
