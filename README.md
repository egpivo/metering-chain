# Metering Chain

[![CI](https://github.com/egpivo/metering-chain/actions/workflows/ci.yml/badge.svg)](https://github.com/egpivo/metering-chain/actions/workflows/ci.yml)

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

# Phase 2 default: signed tx required. For legacy unsigned examples, pass --allow-unsigned.
cat examples/tx/01_mint_alice.json | cargo run --bin metering-chain -- apply --allow-unsigned
cat examples/tx/02_open_storage.json | cargo run --bin metering-chain -- apply --allow-unsigned
cat examples/tx/03_consume_storage_unit_price.json | cargo run --bin metering-chain -- apply --allow-unsigned
cat examples/tx/05_close_storage.json | cargo run --bin metering-chain -- apply --allow-unsigned

cargo run --bin metering-chain -- account 0x...A11
cargo run --bin metering-chain -- meters 0x...A11
```

### Phase 2 signed demo (default: signed tx required)

Strict flow with real signatures (no `--allow-unsigned`):

```bash
./examples/signed/run_signed_demo.sh
```

The script runs `init`, creates two wallets (authority + user), sets `METERING_CHAIN_MINTERS`, then Mint, OpenMeter, Consume, CloseMeter with signed tx. See `docs/phase2_signed_demo.md` and `examples/signed/README.md` for manual steps and kind templates.

### Phase 3 delegation demo

Delegation flow with `signer != owner`: no-proof reject, with-proof accept, revoke then reject.

```bash
./examples/phase3_demo/run_phase3_demo.sh
```

See `examples/phase3_demo/README.md` for expected scenes and manual steps.

---

## Usage

### Apply a transaction

```bash
# From JSON string
echo '{"signer":"alice","nonce":0,"kind":{"Mint":{"to":"bob","amount":1000}}}' | \
  cargo run --bin metering-chain -- apply --allow-unsigned

# From file
cargo run --bin metering-chain -- apply --file examples/tx/01_mint_alice.json --allow-unsigned

# Dry-run (validate without applying; add --allow-unsigned for unsigned tx)
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
* `docs/error_codes.md` – Error taxonomy for UI mapping
* `docs/validation_flow.md` – Validation pipeline (auth → domain → replay/evidence)
* `docs/naming_conventions.md` – Tx/event naming (Phase 4 ready)
* `docs/phase4_extension_points.md` – Pre-Phase 4 refactoring; extension points for Settlement/Dispute
* `docs/phase2_signed_demo.md` – Phase 2 signed flow (wallet create, sign, apply)
* `docs/phase_ii_implementation.md` – Phase II implementation plan and status
* `examples/phase3_demo/README.md` – Phase 3 delegation demo (scenes, commands, expected outcomes)

## Architecture

![Metering Chain architecture](docs/arch.gif)

---

## License

MIT
