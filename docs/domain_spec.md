# Domain Specification — Metering Chain

## Overview
Metering Chain models **service usage and billing** as a deterministic state machine.
The domain focuses on metering correctness rather than product workflows.

---

## Core Domain Concepts

### Account (Aggregate)
Represents a payer with balance and transaction ordering.

**State**
- `balance: u64`
- `nonce: u64`

**Invariants**
- Balance never becomes negative
- Nonce is strictly increasing per account

---

### Meter (Aggregate)
Represents a usage ledger for a specific service owned by an account.

**Identity**
- `(owner, service_id)`

**State**
- `total_units: u64`
- `total_spent: u64`
- `active: bool`
- `locked_deposit: u64`

**Invariants**
- Only the owner may operate the meter
- At most one active meter per `(owner, service_id)`
- `total_units` and `total_spent` are monotonic
- `locked_deposit` represents committed funds

---

## Domain Commands (Transactions)

Commands represent **intent**, not state patches.

### Mint
Creates new funds (authority-only).

**Parameters**
- `to: String`
- `amount: u64`

**Rules**
- Caller must be authorized
- Target account must exist or be created

---

### OpenMeter
Creates a new meter for a service.

**Parameters**
- `owner: String`
- `service_id: String`
- `deposit: u64`

**Rules**
- Owner balance ≥ deposit
- No existing active meter for `(owner, service_id)`

---

### Consume
Records usage and deducts cost.

**Parameters**
- `owner: String`
- `service_id: String`
- `units: u64`
- `pricing: Pricing`

**Rules**
- Meter exists and is active
- Units > 0
- Owner balance ≥ computed cost

---

## Pricing

Pricing must be explicit to avoid ambiguous cost calculation.

```rust
enum Pricing {
    UnitPrice(u64),   // units × price
    FixedCost(u64),   // fixed total cost
}
```