# Invariants

Invariants define the conditions that must hold for all valid state transitions.
Any transaction that violates an invariant is invalid and must be rejected.

---

## Global Invariants

### INV-1: No Negative Balances
`Account.balance` must never be negative.

- Enforced during all balance-affecting operations
- Violation results in transaction rejection

---

### INV-2: Monotonic Nonce
For each account, `nonce` must increase strictly by one per accepted transaction.

- Used for replay protection and ordering
- Checked for every transaction issued by an account

---

### INV-3: Conservation of Value
Total value is preserved across state transitions, except for explicit mint or burn operations.

- Includes: account balances + locked deposits + recorded spend
- Violations indicate a logic error

---

## Meter Invariants

### INV-4: Ownership Authorization
Only the meter owner may issue consume and close operations for that meter.

- Checked by matching transaction signer and meter owner

---

### INV-5: Meter Uniqueness
At most one active meter may exist for a given `(owner, service_id)` pair.

- Enforced during meter creation
- Closed meters may be reopened

---

### INV-6: Active Meter Requirement
Consumption is allowed only when `meter.active == true`.

- Inactive meters cannot record usage or incur cost

---

### INV-7: Deposit Conservation
Meter locked deposits must be accounted for in total value.

- Deposits move from balance to locked_deposit on meter creation
- Deposits return from locked_deposit to balance on meter closure

---

## Transaction Invariants

### INV-8: Mint Authorization
Only designated authority accounts may issue mint transactions.

---

### INV-9: Nonce Monotonicity
All account-issued transactions require `signer.nonce == account.nonce`.

- Enforced for OpenMeter, Consume, and CloseMeter
- Mint bypasses nonce (authority operation)

---

### INV-10: Sufficient Balance for Deposits
Opening a meter requires the owner's balance to cover the deposit amount.

---

### INV-11: Sufficient Balance for Consumption
Accounts must have enough balance to cover the computed consumption cost.

---

### INV-12: Valid Pricing
Pricing parameters must be strictly positive.

- `UnitPrice(price)`: `price > 0`
- `FixedCost(cost)`: `cost > 0`

---

### INV-13: Positive Units
Consumption transactions require `units > 0`.

---

### INV-14: Overflow Protection
Cost computation must not overflow.

- Applies to `units * unit_price` calculations
- Overflow causes transaction rejection

---

## Data Integrity Invariants

### INV-12: Monotonic Meter Totals
`meter.total_units` and `meter.total_spent` must never decrease.

- Historical usage data is append-only

---

### INV-13: Deterministic Transitions
Given the same initial state and transaction sequence, the resulting state must be identical.

- No randomness
- No hidden side effects

---

## Testing Strategy

Each invariant must be covered by tests that:
- Construct a valid initial state
- Attempt an invalid transition
- Assert rejection or unchanged state

---

## References
- Domain specification: `docs/domain_spec.md`
- State transitions: `docs/state_transitions.md`
