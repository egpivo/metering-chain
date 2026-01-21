# State Transitions — Formal Specification

Following the Ethereum Yellow Paper approach, we specify state transitions as **pure functions** over state. Each transaction type defines preconditions (validation) and postconditions (state changes).

## Notation

- \( S \) = Current state (accounts, meters)
- \( T \) = Transaction
- \( S' \) = New state after applying T
- \( \rightarrow \) = State transition function
- \( \checkmark \) = Validation predicate (true if transaction is valid)

## General Form

A metering transaction is a pure function over state:

\[
(S, T) \rightarrow (S', \text{receipt}) \text{ where } \checkmark(S, T) = \text{true}
\]

If \( \checkmark(S, T) = \text{false} \), the transition is rejected and state remains unchanged.

## Transaction: Mint

### Preconditions (\( \checkmark(S, T) \))
- \( T.from \in \text{authorized\_minters} \)
- \( T.to \) account exists or will be created
- \( T.amount > 0 \)

### Postconditions
- \( S'.accounts[T.to].balance = S.accounts[T.to].balance + T.amount \)
- All other state unchanged
- Receipt: `{ type: "mint", to: T.to, amount: T.amount }`

## Transaction: OpenMeter

### Preconditions (\( \checkmark(S, T) \))
- \( S.accounts[T.owner].balance \geq T.deposit \)
- No active meter exists for \( (T.owner, T.service\_id) \)
- \( T.deposit > 0 \)

### Postconditions
- Create new meter: \( S'.meters[(T.owner, T.service\_id)] = \{ \text{active}: \text{true}, \text{total\_units}: 0, \text{total\_spent}: 0, \text{owner}: T.owner \} \)
- \( S'.accounts[T.owner].balance = S.accounts[T.owner].balance - T.deposit \)
- \( S'.accounts[T.owner].nonce = S.accounts[T.owner].nonce + 1 \)
- Receipt: `{ type: "open_meter", owner: T.owner, service_id: T.service_id, deposit: T.deposit }`

## Transaction: Consume

### Cost Calculation Function
\[
\text{cost} =
\begin{cases}
T.units \times T.pricing.unit\_price & \text{if } T.pricing = \text{UnitPrice}(unit\_price) \\
T.pricing.fixed\_cost & \text{if } T.pricing = \text{FixedCost}(fixed\_cost)
\end{cases}
\]

### Preconditions (\( \checkmark(S, T) \))
- Meter exists: \( (T.owner, T.service\_id) \in S.meters \)
- \( S.meters[(T.owner, T.service\_id)].active = \text{true} \)
- \( T.owner = S.meters[(T.owner, T.service\_id)].owner \)
- \( T.units > 0 \)
- \( \text{cost} > 0 \) (based on pricing model)
- \( S.accounts[T.owner].balance \geq \text{cost} \)
- \( S.accounts[T.owner].nonce = T.nonce \)

### Postconditions
- \( S'.meters[(T.owner, T.service\_id)].total\_units = S.meters[(T.owner, T.service\_id)].total\_units + T.units \)
- \( S'.meters[(T.owner, T.service\_id)].total\_spent = S.meters[(T.owner, T.service\_id)].total\_spent + \text{cost} \)
- \( S'.accounts[T.owner].balance = S.accounts[T.owner].balance - \text{cost} \)
- \( S'.accounts[T.owner].nonce = S.accounts[T.owner].nonce + 1 \)
- Receipt: `{ type: "consume", owner: T.owner, service_id: T.service_id, units: T.units, cost: cost, pricing: T.pricing }`

## State Reconstruction

**Theorem:** State is fully reconstructible by replaying transaction log from genesis.

Given transaction sequence \( T_1, T_2, ..., T_n \):

\[
S_0 = \text{genesis\_state}
\]
\[
S_{i} = \text{apply}(S_{i-1}, T_i) \text{ for } i = 1 \to n
\]
\[
S_n = \text{final\_state}
\]

This property enables **auditability** and **deterministic testing**.

## Implementation Notes

1. **Pure Functions:** All transitions should be implemented as pure functions with no side effects.
2. **Error Handling:** Invalid transitions return error with specific invariant violation.
3. **Receipts:** Optional for MVP, but recommended for event-driven architectures.
4. **Overflow Protection:** All arithmetic operations must check for overflow/underflow.

## References
- Domain spec: `docs/domain_spec.md`
- Invariants: `docs/invariants.md`
- Architecture: `docs/architecture.md`