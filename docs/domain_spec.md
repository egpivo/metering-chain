# Domain Specification — Metering Chain

## Overview
Metering Chain models **service usage metering and billing** as a domain-driven system. The core domain is **metering**: tracking service consumption, calculating costs, and ensuring fair billing.

## Core Domain Concepts

### Aggregate: Account
Represents a **financial entity** that can own meters and pay for service usage.

**Attributes:**
- `balance: u64` — Available funds (in smallest currency unit)
- `nonce: u64` — Sequence number for transaction ordering and replay protection

**Invariants:**
- Balance never goes negative
- Nonce increases monotonically per account

### Aggregate: Meter
Represents a **service metering point** owned by an account.

**Attributes:**
- `service_id: String` — Unique service identifier
- `total_units: u64` — Cumulative units consumed
- `total_spent: u64` — Cumulative amount spent
- `active: bool` — Whether the meter is active
- `owner: String` — Account address that owns this meter

**Invariants:**
- Only owner can operate on meter
- One active meter per (owner, service_id) pair
- Units and spent are non-decreasing

### Domain Commands (Transactions)

#### Mint
**Purpose:** Create new funds (typically by authority)

**Parameters:**
- `to: String` — Recipient account address
- `amount: u64` — Amount to mint

**Preconditions:**
- Only authority can execute mint
- `to` account exists (or will be created)

#### OpenMeter
**Purpose:** Create a new metering point for a service

**Parameters:**
- `owner: String` — Account that will own the meter
- `service_id: String` — Service identifier
- `deposit: u64` — Initial deposit amount

**Preconditions:**
- `owner` has sufficient balance for deposit
- No active meter exists for (owner, service_id)

#### Consume
**Purpose:** Record service usage and deduct costs

**Parameters:**
- `owner: String` — Meter owner
- `service_id: String` — Service being consumed
- `units: u64` — Units consumed
- `pricing: Pricing` — Pricing model (UnitPrice or FixedCost)

**Preconditions:**
- Meter exists and is active
- Owner has sufficient funds
- Units > 0

### Pricing Models

```rust
enum Pricing {
    UnitPrice(u64),    // Cost per unit
    FixedCost(u64),    // Fixed total cost
}
```

## Domain Events (Optional, for future extensions)
- `MeterOpened { owner, service_id, deposit }`
- `ConsumptionRecorded { owner, service_id, units, cost }`
- `FundsTransferred { from, to, amount }`

## Business Rules Summary
1. **Conservation**: Total balance across all accounts + locked funds is preserved (except for mint/burn)
2. **Authorization**: Only meter owners can consume against their meters
3. **Fairness**: Costs are calculated deterministically based on declared pricing
4. **Auditability**: All state changes are traceable through transaction history

---

## References
- Architecture: `docs/architecture.md`
- Invariants: `docs/invariants.md`
- State transitions: `docs/state_transitions.md`