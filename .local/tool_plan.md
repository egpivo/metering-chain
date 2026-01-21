# Public Tool Plan — `metering-chain` (metering-first)

## Vision
Build a **publicly usable metering-focused chain simulator + analyzer**.

- **Primary goal**: given a wallet address (and optionally a service id), produce **auditable metering + billing insights**.
- **Key differentiator**: DDD-first design with explicit **state transition** and **invariants**, so reports are explainable and reproducible.

---

## Target Users
- Developers building metering/billing systems who want a **reference implementation**.
- Teams needing **auditable usage → cost** accounting for services.
- People writing/reading about DDD applied to “transactions as domain commands”.

---

## Domain Model (metering-first)

### State
- `accounts[address] -> Account { balance, nonce }`
- `meters[(owner, service_id)] -> Meter { total_units, total_spent, active, ... }`

### Transactions (commands)
Recommended shape:
- `SignedTx { from, nonce, kind /* +sig later */ }`
- `kind` is an enum like:
  - `Mint { to, amount }`
  - `OpenMeter { owner, service_id, deposit }`
  - `Consume { owner, service_id, units, pricing }`

#### Pricing semantics (make it explicit)
Avoid ambiguous fields like “price could be unit_price or total cost”. Model it as domain semantics:

```rust
enum Pricing {
    UnitPrice(Price),
    FixedCost(Amount),
}
```

Then:

```rust
Consume {
    owner,
    service_id,
    units,
    pricing: Pricing,
}
```

Benefits:
- Prevents a tx from carrying both `unit_price` and `cost`
- Pricing rules become explicit domain semantics (great for DDD + article clarity)

---

## DDD State Transition (core engine)

### Pipeline
1. **Validate** (preconditions/invariants) under current state
2. **Apply** deterministically to produce new state
3. Emit **receipts/events** for reporting (optional in MVP, recommended soon)

### Invariants to encode early
- Nonce monotonicity per account (replay protection)
- No balance underflow
- Meter ownership + authorization
- Meter uniqueness (per owner+service)
- Consume requires meter active
- Cost calculation rules (unit_price vs total_cost must be explicit)

---

## MVP: CLI Tool (public-facing)

### Command set (minimal)
- `metering-chain init`
  - Creates a local data dir (e.g. `.metering-chain/`) and a genesis state
- `metering-chain apply --tx-file <path>`
  - Reads a list of txs (jsonlines or bincode), validates + applies them, persists resulting state
- `metering-chain apply --tx-file <path> --dry-run`
  - Validates only; does **not** persist state
  - Outputs:
    - which txs will fail
    - failure reasons (which invariant / validation rule)
- `metering-chain account <address>`
  - Shows `balance`, `nonce`
- `metering-chain meters <address>`
  - Lists meters for owner (by `service_id`) with totals and active state
- `metering-chain report <address> [--service <id>]`
  - Summaries:
    - total_units / total_spent
    - unit cost (spent/units) when units > 0
    - top services by spend (if no --service)
    - effective unit price (when applicable)
    - first_seen / last_seen (by timestamp or tx index)

### Output formats
- Human-readable (default)
- `--json` for machine consumption

---

## Storage Strategy (MVP)

### Keep it simple
- Persist `State` as a single snapshot file after each apply:
  - `state.bin` via `bincode` (fast, compact)
  - (Optional) `state.json` via `serde_json` for debugging

### Append-only audit log (recommended soon)
- `tx.log` (jsonlines) or `blocks.bin`
- Enables replays + deterministic audits

> “State must be fully reconstructible by replaying tx.log from genesis.”

---

## Medium Article References (how we tell the story)

### Narrative arc
- Ubiquitous language: *Account*, *Meter*, *Consume*, *Deposit*, *Cost*
- Transactions are domain commands; “chain” is a deterministic sequencer
- State transition function: validate preconditions → apply → preserve invariants

### Copy-ready phrasing
> “A metering transaction is a pure function over state: validate under S, apply to produce S′, while preserving invariants.”

---

## Roadmap (incremental)

### Phase 1 — MVP CLI + state machine
- Implement metering-first structs (Account, Meter, SignedTx, TxKind)
- Implement `validate` + `apply`
- Implement `report` and JSON output

### Phase 2 — Signatures + authorization
- Add signature field to `SignedTx`
- Add key format + address derivation
- Enforce mint authority + owner authorization cryptographically

### Phase 3 — Blocks + consensus (optional for tool)
- Group txs into blocks, add PoA (simpler) or PoW (demo)
- Proofs are less important than *domain semantics* for metering

### Phase 4 — Network/API (optional)
- Provide library APIs for embedding in other apps
- Expose REST/gRPC for remote queries

### Phase 5 — Storage/Performance (optional, scale-driven)
- Write batching for higher throughput (keep append-only semantics)
- Range iterators for reporting or history scans
- Zero-copy reads when log/snapshot size is large
- Storage engine optimizations (LSM/flash) only if required by deployment
