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
- `total_units: u64` — cumulative usage
- `total_spent: u64` — cumulative cost paid
- `active: bool` — whether the meter accepts consumption
- `locked_deposit: u64` — committed funds (refunded on closure)

**Lifecycle States**
- **Inactive**: meter exists but does not accept consumption (genesis state, or after closure)
- **Active**: meter accepts consumption transactions (after OpenMeter)
- **Transition**: Inactive → Active (via OpenMeter), Active → Inactive (via CloseMeter)

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
- `signer: String`
- `nonce: u64`
- `owner: String`
- `service_id: String`
- `deposit: u64`

**Rules**
- `signer == owner`
- `signer.nonce == nonce`
- Owner balance ≥ deposit
- No existing active meter for `(owner, service_id)`

---

### Consume
Records usage and deducts cost.

**Parameters**
- `signer: String`
- `nonce: u64`
- `owner: String`
- `service_id: String`
- `units: u64`
- `pricing: Pricing`

**Rules**
- `signer == owner`
- `signer.nonce == nonce`
- Meter exists and is active
- Owner balance ≥ computed cost
- Cost computation must not overflow

---

### CloseMeter
Closes a meter and returns locked deposit.

**Parameters**
- `signer: String`
- `nonce: u64`
- `owner: String`
- `service_id: String`

**Rules**
- `signer == owner`
- `signer.nonce == nonce`
- Meter exists and is active

---

## Pricing

Pricing must be explicit to avoid ambiguous cost calculation.

```rust
enum Pricing {
    UnitPrice(u64),   // units × price
    FixedCost(u64),   // fixed total cost
}
```