# Metering Chain

Deterministic metering engine for on-chain service usage and billing (Rust).

For **DePIN / Pay-As-You-Go** protocols where usage, cost, and balance must be deterministic, auditable, and reproducible.

---

## What it does

Pure state machine for service consumption:

```
User Action → Meter Update → Cost Accumulation → Deterministic Receipt
```

Supports usage-based billing, balance tracking, deterministic replay, and on-chain–friendly logic.

---

## Quick Demo

```bash
cargo run --bin metering-chain -- init

cat examples/tx/01_mint_alice.json | cargo run --bin metering-chain -- apply
cat examples/tx/02_open_storage.json | cargo run --bin metering-chain -- apply
cat examples/tx/03_consume_storage_unit_price.json | cargo run --bin metering-chain -- apply
cat examples/tx/05_close_storage.json | cargo run --bin metering-chain -- apply

cargo run --bin metering-chain -- account 0x...A11
cargo run --bin metering-chain -- meters 0x...A11
```

---

## Real-World (DePIN)

Track resource usage, charge per unit, produce verifiable receipts.

See `examples/depin/README.md` for SIM Dune integration.

---

## Design

* **Deterministic** – same input → same state
* **Auditable** – every charge explainable
* **Composable** – usable in smart contracts or off-chain indexers
* **Minimal** – no database, no side effects

---

## Docs

* `docs/domain_spec.md` – domain model
* `docs/state_transitions.md` – state machine
* `docs/invariants.md` – safety rules
* `docs/architecture.md` – system design

---

## License

MIT
