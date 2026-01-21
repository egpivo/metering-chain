# Architecture Overview

This project separates **domain logic** from **infrastructure**.
The goal is to keep metering semantics explicit, testable, and reproducible.

## Execution Model
- Deterministic, single-threaded execution
- State is updated only through explicit transactions
- Replaying the same transactions always produces the same state

## Domain Layer
- Core logic lives in `state/` (Account, Meter, state transitions)
- Transactions are modeled as domain commands in `tx/`
- All business rules are enforced as invariants  
  (see `docs/invariants.md` and `docs/state_transitions.md`)

## Infrastructure Layer
- Persistence via append-only logs and snapshots (`storage/`)
- CLI (`cli.rs`) translates user input into domain commands
- Optional sequencing logic lives in `chain/` (PoA / PoW)

## Storage Abstraction (Minimal Semantics)
Domain logic depends on a small storage interface. Implementations may use sled,
files, or other engines, but must preserve append-only and snapshot semantics.

```rust
pub trait Storage {
    // append-only
    fn append_tx(&mut self, tx: &SignedTx) -> Result<()>;

    // snapshot (state + last applied tx id)
    fn load_state(&self) -> Result<Option<(State, u64)>>;
    fn persist_state(&mut self, state: &State, last_tx_id: u64) -> Result<()>;

    // replay
    fn load_txs_from(&self, from_tx_id: u64) -> Result<Vec<SignedTx>>;
}
```

## Filesystem Semantics (Infrastructure)
Filesystem interactions must be crash-safe and deterministic.

- Transaction log writes are append-only and fsync before ack
- Snapshots are written to a new file, fsync, then atomically renamed
- Replay reads the log sequentially; old log segments may be rotated or pruned

## CLI
- No business logic
- Responsible only for parsing input and formatting output
- Supports human-readable and JSON output

## Blocks and Consensus (Optional)
- Blocks are an execution detail, not a domain concept
- Enabled only when sequencing across nodes is required
