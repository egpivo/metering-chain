# Metering Chain

A **deterministic metering engine** for on-chain service usage and billing, written in Rust.

Designed for **DePIN / Pay-As-You-Go / usage-based protocols** where usage, cost, and balance must be:

* deterministic
* auditable
* reproducible

---

## What it does

Metering Chain models service consumption as a **pure state machine**:

```
User Action → Meter Update → Cost Accumulation → Deterministic Receipt
```

It supports:

* Usage-based billing
* Balance tracking
* Deterministic replay
* On-chain–friendly logic (no hidden state)

Think of it as:

> **"Stripe billing logic, but deterministic and blockchain-native."**

---

## Quick Demo

```bash
cargo run --bin metering-chain -- init

cat examples/tx/01_mint_alice.json | cargo run --bin metering-chain -- apply
cat examples/tx/02_open_storage.json | cargo run --bin metering-chain -- apply
cat examples/tx/03_consume_storage_unit_price.json | cargo run --bin metering-chain -- apply
cat examples/tx/05_close_storage.json | cargo run --bin metering-chain -- apply
```

Inspect state:

```bash
cargo run --bin metering-chain -- account 0x...A11
cargo run --bin metering-chain -- meters  0x...A11
cargo run --bin metering-chain -- report  0x...A11
```

---

## Real-World Motivation (DePIN)

This project simulates how **DePIN protocols** can:

* track real resource usage
* charge per unit (storage / compute / bandwidth)
* produce verifiable billing records
* remain chain-agnostic

Example flow:

```
Dune / On-chain metrics
        ↓
   Consume events
        ↓
 Metering Chain
        ↓
 Deterministic receipt
```

---

## Design Philosophy

* **Deterministic** – same input → same state
* **Auditable** – every charge explainable
* **Composable** – usable inside smart contracts or off-chain indexers
* **Minimal** – no database, no side effects

---

## Documentation

* `docs/domain_spec.md` – domain model
* `docs/state_transitions.md` – state machine
* `docs/invariants.md` – safety rules
* `docs/architecture.md` – system design

---

## Why this exists

Most Web3 billing systems are:

* ad-hoc
* off-chain
* hard to audit

Metering Chain shows how **usage-based pricing can be deterministic, verifiable, and composable**, suitable for DePIN and agent-based protocols.

---

## License

MIT
