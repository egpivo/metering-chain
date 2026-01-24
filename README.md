# Metering Chain

Deterministic metering engine for on-chain service usage and billing (Rust).

---

## What it is

A pure state machine that tracks:

* usage
* cost
* balance

with **deterministic and reproducible results**.

Designed for pay-as-you-go and DePIN-style systems.

---

## Installation

```bash
git clone <repo-url>
cd metering-chain
cargo build --release
```

---

## Quick Start

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

## Usage

### Apply a transaction

```bash
# From JSON string
echo '{"signer":"alice","nonce":0,"kind":{"Mint":{"to":"bob","amount":1000}}}' | \
  cargo run --bin metering-chain -- apply

# From file
cargo run --bin metering-chain -- apply --file examples/tx/01_mint_alice.json

# Dry-run (validate without applying)
cargo run --bin metering-chain -- apply --file tx.json --dry-run
```

### Query state

```bash
# Account info
cargo run --bin metering-chain -- account <address>

# Meters for account
cargo run --bin metering-chain -- meters <address>

# Usage report
cargo run --bin metering-chain -- report [<address>]

# JSON output
cargo run --bin metering-chain -- --format json account <address>
```

---

## Features

* Deterministic state transitions
* Usage-based billing
* Auditable receipts
* No database / no side effects

---

## Docs

* `docs/domain_spec.md`
* `docs/state_transitions.md`
* `docs/invariants.md`
* `docs/architecture.md`

---

## License

MIT
