# Architecture Overview

## Domain vs Infrastructure Separation

This project follows Domain-Driven Design (DDD) principles, clearly separating **domain logic** from **infrastructure concerns**.

### Execution Model
- **Single-threaded, deterministic**: All state transitions are pure functions over state. No concurrency or randomness.
- **Event sourcing pattern**: Transactions are commands that produce deterministic state changes.
- **Reproducibility**: Given the same transaction sequence, the system always reaches the same state.

### Domain Layer
- **Core business logic**: `state/` module (Account, Meter, apply logic)
- **Commands**: `tx/` module (Transaction types as domain commands)
- **Invariants**: See `docs/invariants.md`
- **State transitions**: See `docs/state_transitions.md`

### Infrastructure Layer
- **Persistence**: `storage/` module (event log + snapshots)
- **CLI**: `cli.rs` (command-line interface mapping to domain commands)
- **Consensus**: `chain/` module (optional PoW/PoA for sequencing)

### CLI Layer
- **Command → Domain mapping**: Each CLI command translates to domain operations
- **Input validation**: Infrastructure-level checks (file exists, formats valid)
- **Output formatting**: Human-readable vs JSON

### Optional: Blocks / Consensus
- **Purpose**: Provides sequencing when needed (e.g., for distributed deployments)
- **Default**: Single-node, trust-based ordering via CLI apply
- **When to enable**: Multi-node deployments requiring Byzantine fault tolerance

---

## References
- Domain spec: `docs/domain_spec.md`
- Ubiquitous language: `docs/ubiquitous_language.md`
- Invariants: `docs/invariants.md`