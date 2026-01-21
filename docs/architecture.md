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

## CLI
- No business logic
- Responsible only for parsing input and formatting output
- Supports human-readable and JSON output

## Blocks and Consensus (Optional)
- Blocks are an execution detail, not a domain concept
- Enabled only when sequencing across nodes is required
