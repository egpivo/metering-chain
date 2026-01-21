# Domain Model Implementation Patterns

This document explains which DDD implementation patterns we use and why.

---

## Pattern Overview

We use a **hybrid approach** combining:
- **Transaction Script** for state transitions (primary pattern)
- **Rich Domain Model** elements for invariants and lifecycle (secondary pattern)

---

## Pattern Definitions

### Transaction Script
Business logic is organized as **procedures** that take domain data as input and produce new domain data.

**Characteristics:**
- Logic lives in functions/services, not in domain objects
- Domain objects are mostly data structures
- Each transaction type has a dedicated procedure

**Example:**
```rust
fn apply_mint(state: &State, tx: &Mint) -> Result<State> {
    // validation + state mutation logic here
}
```

### Rich Domain Model
Business logic lives **inside** domain objects as methods.

**Characteristics:**
- Domain objects have behavior (methods)
- Objects enforce their own invariants
- Objects encapsulate lifecycle transitions

**Example:**
```rust
impl Account {
    fn can_spend(&self, amount: u64) -> bool {
        self.balance >= amount
    }
    
    fn apply_mint(&mut self, amount: u64) -> Result<()> {
        self.balance = self.balance.checked_add(amount)
            .ok_or(Error::Overflow)?;
        Ok(())
    }
}
```

### Active Record
Domain objects know how to persist themselves.

**Characteristics:**
- Objects have `save()`, `load()` methods
- Objects contain persistence logic

**We don't use this** — persistence is infrastructure concern.

### Anemic Domain Model
Domain objects are pure data with no behavior.

**Characteristics:**
- Objects are just structs
- All logic in services

**We avoid this** — we want some behavior in aggregates for invariants.

---

## Our Pattern Choice: Transaction Script + Rich Elements

### Why Transaction Script (Primary)

1. **State transitions are pure functions**
   - `apply(tx, state) -> state'` is naturally a procedure
   - Matches Ethereum Yellow Paper style (formal spec)
   - Easy to test and reason about

2. **Deterministic replay**
   - Replaying transactions is straightforward: call `apply` sequentially
   - No hidden state or side effects

3. **Clear separation**
   - Domain logic (`state::apply`) is separate from infrastructure
   - Easy to swap storage implementations

### Why Rich Domain Model Elements (Secondary)

1. **Invariant enforcement**
   - `Account` and `Meter` have methods that check invariants
   - Example: `Account::can_spend()` checks balance

2. **Lifecycle encapsulation**
   - `Meter` methods handle state transitions (active/inactive)
   - Example: `Meter::close()` returns deposit

3. **Domain semantics**
   - Methods express domain concepts clearly
   - Example: `Meter::consume()` vs external `apply_consume()`

---

## Implementation Mapping

### Transaction Script Layer (`state/apply.rs`)

```rust
pub fn apply(state: &State, tx: &SignedTx) -> Result<State> {
    match &tx.kind {
        Transaction::Mint { to, amount } => apply_mint(state, to, amount),
        Transaction::OpenMeter { owner, service_id, deposit } => 
            apply_open_meter(state, owner, service_id, deposit),
        Transaction::Consume { owner, service_id, units, pricing } => 
            apply_consume(state, owner, service_id, units, pricing),
        Transaction::CloseMeter { owner, service_id } => 
            apply_close_meter(state, owner, service_id),
    }
}
```

**Responsibilities:**
- Orchestrate state transitions
- Call validation functions
- Compose domain object updates

### Rich Domain Model Elements (`state/account.rs`, `state/meter.rs`)

```rust
impl Account {
    /// Check if account can spend amount (invariant check)
    pub fn can_spend(&self, amount: u64) -> bool {
        self.balance >= amount
    }
    
    /// Increment nonce (lifecycle)
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }
}

impl Meter {
    /// Check if meter can accept consumption
    pub fn can_consume(&self) -> bool {
        self.active
    }
    
    /// Record consumption (domain operation)
    pub fn record_consumption(&mut self, units: u64, cost: u64) {
        self.total_units += units;
        self.total_spent += cost;
    }
    
    /// Close meter and return deposit amount
    pub fn close(&mut self) -> u64 {
        self.active = false;
        let deposit = self.locked_deposit;
        self.locked_deposit = 0;
        deposit
    }
}
```

**Responsibilities:**
- Enforce invariants (can_spend, can_consume)
- Encapsulate lifecycle transitions (close)
- Express domain operations (record_consumption)

---

## Pattern Benefits for Our Domain

### Transaction Script Benefits
- ✅ **Testability**: Pure functions are easy to test
- ✅ **Reproducibility**: Deterministic state transitions
- ✅ **Clarity**: State transition logic is explicit and centralized
- ✅ **Medium articles**: Easy to explain "transaction as pure function"

### Rich Domain Model Benefits
- ✅ **Invariant safety**: Objects check their own constraints
- ✅ **Domain expressiveness**: Methods like `meter.consume()` read naturally
- ✅ **Encapsulation**: Lifecycle logic stays with the aggregate

### Hybrid Benefits
- ✅ **Best of both**: Clear transitions + safe invariants
- ✅ **Flexibility**: Can evolve toward richer model if needed
- ✅ **DDD alignment**: Matches Evans' recommendation for complex domains

---

## When to Use Each Pattern

### Use Transaction Script for:
- State transition orchestration (`apply` functions)
- Cross-aggregate operations (multiple aggregates affected)
- Validation that spans multiple domain objects

### Use Rich Domain Model for:
- Single-aggregate operations (`Account::can_spend`)
- Lifecycle transitions (`Meter::close`)
- Invariant checks (`Meter::can_consume`)

### Avoid:
- **Anemic Domain Model**: Don't make everything a data struct
- **Active Record**: Don't mix persistence with domain logic
- **God Objects**: Don't put all logic in one aggregate

---

## Evolution Path

If our domain grows more complex, we can:

1. **Move more logic into aggregates**
   - Example: `Account::apply_mint()` instead of `apply_mint(state, ...)`
   - Keep `apply()` as thin orchestrator

2. **Add domain events**
   - Emit events from aggregates during transitions
   - Use events for audit/reporting

3. **Introduce domain services**
   - For complex operations spanning multiple aggregates
   - Example: `MeteringService::calculate_cost()`

For now, our hybrid approach is sufficient and keeps the codebase simple.

---

## References
- Domain spec: `docs/domain_spec.md`
- State transitions: `docs/state_transitions.md`
- Architecture: `docs/architecture.md`
