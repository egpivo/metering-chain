# Domain Model Implementation Patterns

We use a hybrid DDD approach:
- **Transaction Script** for state transitions (primary)
- **Rich Domain Model** elements for local invariants and lifecycle (secondary)

---

## Chosen Pattern

### Transaction Script (Primary)
- `apply(tx, state) -> state'` is a pure function
- Easy replay and testing
- Keeps cross-aggregate logic explicit

### Rich Domain Elements (Secondary)
- Aggregate methods enforce local rules
- Encapsulates lifecycle changes (e.g., `Meter::close`)

---

## Implementation Mapping

- `state/apply.rs`: orchestrates validation and transitions
- `state/account.rs`, `state/meter.rs`: invariant checks and small mutations

## Minimal Examples

```rust
// Transaction Script
fn apply_consume(state: &State, tx: &SignedTx) -> Result<State> {
    // validate + mutate state
}

// Rich Domain Element
impl Meter {
    pub fn close(&mut self) -> u64 {
        self.active = false;
        let deposit = self.locked_deposit;
        self.locked_deposit = 0;
        deposit
    }
}
```

---

## Avoided Patterns

- **Active Record**: no persistence inside domain objects
- **Anemic Model**: avoid pushing all logic into services

---

## Evolution

If complexity grows, move more logic into aggregates or add domain services.

---

## References
- Domain spec: `docs/domain_spec.md`
- State transitions: `docs/state_transitions.md`
- Architecture: `docs/architecture.md`
