# Metering Chain

A deterministic state machine for **service usage and billing**, built with Domain-Driven Design (DDD) principles in Rust.

## Why

Metering Chain models service consumption and billing as a **pure, replayable state machine**. Unlike traditional billing systems that mix business logic with infrastructure concerns, this project separates domain semantics from execution details, making metering correctness explicit, testable, and reproducible.

### Key Principles

- **Deterministic**: Replaying the same transactions always produces the same state
- **Domain-First**: Business rules are encoded as explicit invariants, not hidden in infrastructure
- **Replayable**: State can be fully reconstructed from the transaction log
- **Testable**: Pure functions enable comprehensive testing of all business rules

## How

The system uses a **Transaction Script** pattern with **Rich Domain Model** elements:

- **State** (`state/`): Core aggregates (Account, Meter) with lifecycle methods
- **Transactions** (`tx/`): Domain commands (Mint, OpenMeter, Consume, CloseMeter)
- **Apply** (`state/apply.rs`): Pure functions that transform state deterministically
- **Storage** (`storage/`): Append-only transaction log + atomic snapshots
- **CLI** (`cli.rs`): Command-line interface for interacting with the system

All business rules are documented as **invariants** (see `docs/invariants.md`) and enforced during transaction validation and application.

## Quickstart

### Build

```bash
cargo build
# or
make build
```

### Initialize

```bash
cargo run --bin metering-chain -- init
# or
make run-init
```

This creates a `.metering-chain/` directory in the current working directory.

### Apply Transactions

Transactions are applied as JSON. Here's the complete flow:

#### 1. Mint (Create funds)

```bash
echo '{"signer":"authority","nonce":0,"kind":{"Mint":{"to":"alice","amount":1000}}}' | \
  cargo run --bin metering-chain -- apply
```

#### 2. OpenMeter (Create a meter)

```bash
echo '{"signer":"alice","nonce":0,"kind":{"OpenMeter":{"owner":"alice","service_id":"storage","deposit":100}}}' | \
  cargo run --bin metering-chain -- apply
```

#### 3. Consume (Record usage)

```bash
# UnitPrice: cost = units × price
echo '{"signer":"alice","nonce":1,"kind":{"Consume":{"owner":"alice","service_id":"storage","units":10,"pricing":{"UnitPrice":5}}}}' | \
  cargo run --bin metering-chain -- apply

# FixedCost: fixed cost regardless of units
echo '{"signer":"alice","nonce":2,"kind":{"Consume":{"owner":"alice","service_id":"storage","units":5,"pricing":{"FixedCost":25}}}}' | \
  cargo run --bin metering-chain -- apply
```

#### 4. CloseMeter (Close meter and return deposit)

```bash
echo '{"signer":"alice","nonce":3,"kind":{"CloseMeter":{"owner":"alice","service_id":"storage"}}}' | \
  cargo run --bin metering-chain -- apply
```

### View State

#### Account Information

```bash
cargo run --bin metering-chain -- account alice
# or
make run-account ADDR=alice
```

#### Meters for Account

```bash
cargo run --bin metering-chain -- meters alice
# or
make run-meters ADDR=alice
```

#### Usage Report

```bash
# All accounts
cargo run --bin metering-chain -- report

# Specific account
cargo run --bin metering-chain -- report alice
# or
make run-report ADDR=alice
```

### JSON Output

Use `--format json` for machine-readable output:

```bash
cargo run --bin metering-chain -- --format json account alice
```

### Dry-Run

Validate transactions without applying them:

```bash
echo '{"signer":"alice","nonce":1,"kind":{"Consume":...}}' | \
  cargo run --bin metering-chain -- apply --dry-run
```

## Demo

Run the complete end-to-end flow automatically:

```bash
make demo
```

This executes: Mint → OpenMeter → Consume → Consume → CloseMeter and shows the final state.

## Project Structure

```
metering-chain/
├── src/
│   ├── state/          # Domain aggregates (Account, Meter, State)
│   ├── tx/             # Transactions and validation
│   ├── storage/        # Persistence abstraction
│   ├── cli.rs          # Command-line interface
│   └── lib.rs          # Library root
├── docs/               # Domain specifications
│   ├── domain_spec.md  # Core domain concepts
│   ├── invariants.md   # Business rules
│   ├── state_transitions.md  # State transition rules
│   └── architecture.md # System architecture
└── tests/
    └── basic_flow.rs   # Integration tests
```

## Domain Model

### Account
- Holds balance and nonce (transaction ordering)
- Owns meters

### Meter
- Tracks usage for a service (`owner`, `service_id`)
- Records `total_units` and `total_spent`
- Has lifecycle: Active/Inactive
- Requires `locked_deposit` to operate

### Transactions

- **Mint**: Create new funds (authority-only)
- **OpenMeter**: Create or reopen a meter
- **Consume**: Record usage and deduct cost
- **CloseMeter**: Close meter and return deposit

See `docs/domain_spec.md` for complete specification.

## Testing

```bash
# All tests
cargo test

# Library tests only
cargo test --lib
# or
make test-lib

# Integration tests
cargo test --test basic_flow
# or
make test-flow
```

## Documentation

Generate and view documentation:

```bash
cargo doc --open
# or
make doc
```

## Architecture

The system separates **domain logic** from **infrastructure**:

- **Domain Layer**: Pure state transitions, no side effects
- **Infrastructure Layer**: Storage, CLI, optional consensus

State is fully reconstructible by replaying transactions from genesis. See `docs/architecture.md` for details.

## License

See LICENSE file.
