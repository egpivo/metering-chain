# State Transitions

This document defines valid state transitions for the metering domain.
All transitions are deterministic and side-effect free.

---

## Notation

- `S`   : current state (accounts, meters)
- `T`   : transaction
- `S'`  : resulting state
- `✓`   : validation predicate

A transaction is applied as:

\[
(S, T) \rightarrow S' \quad \text{iff} \quad ✓(S, T)
\]

If validation fails, the transition is rejected and state remains unchanged.

---

## Transaction: Mint

### Preconditions
- `T.from ∈ authorized_minters`
- `T.amount > 0`
- `T.to` exists or is created

### State Update
- `accounts[T.to].balance += T.amount`

All other state remains unchanged.

---

## Transaction: OpenMeter

### Preconditions
- `accounts[T.owner].balance ≥ T.deposit`
- No active meter exists for `(T.owner, T.service_id)`
- `T.deposit > 0`
- `accounts[T.signer].nonce == T.nonce`

### State Update
- Create meter `(T.owner, T.service_id)` with:
  - `active = true`
  - `total_units = 0`
  - `total_spent = 0`
  - `locked_deposit = T.deposit`
- `accounts[T.owner].balance -= T.deposit`
- `accounts[T.signer].nonce += 1`

---

## Transaction: Consume

### Cost Function

\[
cost =
\begin{cases}
units \times unit\_price & \text{if pricing = UnitPrice} \\
fixed\_cost              & \text{if pricing = FixedCost}
\end{cases}
\]

### Preconditions
- Meter `(T.owner, T.service_id)` exists
- Meter is active
- `T.signer == T.owner`
- `units > 0`
- `cost > 0` (no overflow in computation)
- `accounts[T.owner].balance ≥ cost`
- `accounts[T.signer].nonce == T.nonce`

### State Update
- `meters[(T.owner, T.service_id)].total_units += units`
- `meters[(T.owner, T.service_id)].total_spent += cost`
- `accounts[T.owner].balance -= cost`
- `accounts[T.signer].nonce += 1`

---

## Transaction: CloseMeter

### Preconditions
- Meter `(T.owner, T.service_id)` exists
- Meter is active
- `T.signer == T.owner`
- `accounts[T.signer].nonce == T.nonce`

### State Update
- `meters[(T.owner, T.service_id)].active = false`
- `accounts[T.owner].balance += meters[(T.owner, T.service_id)].locked_deposit`
- `accounts[T.signer].nonce += 1`

---

## State Reconstruction

State is fully derivable from the transaction log.

Let:

\[
S_0 = genesis
\]

\[
S_i = apply(S_{i-1}, T_i), \quad i = 1 \dots n
\]

Then `S_n` is the unique final state for transaction sequence
`T_1 … T_n`.

---

## Implementation Constraints

- Transitions must be implemented as pure functions
- No hidden state, randomness, or external side effects
- Arithmetic must be overflow-safe
- Errors must identify the violated invariant

---

## References
- Domain specification: `docs/domain_spec.md`
- Invariants: `docs/invariants.md`
- Architecture: `docs/architecture.md`
