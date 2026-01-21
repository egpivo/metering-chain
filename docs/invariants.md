# Invariants — Business Rules That Must Always Hold

Invariants are **business rules** that define the **always-true** properties of the metering domain. They constrain valid state transitions and prevent invalid operations.

## Global Invariants (System-Wide)

### INV-1: No Negative Balances
**Statement:** `Account.balance >= 0` for all accounts.

**Rationale:** Prevents debt or negative balances in the system.

**Enforcement:** Checked during consume and open-meter operations.

**Violation consequence:** Transaction rejected.

### INV-2: Monotonic Nonce per Account
**Statement:** For each account, `nonce` increases monotonically and transactions must have `nonce == current_nonce`.

**Rationale:** Prevents replay attacks and ensures transaction ordering.

**Enforcement:** Checked for every transaction from an account.

**Violation consequence:** Transaction rejected.

### INV-3: Conservation of Value
**Statement:** Total balance across all accounts + locked funds + spent amounts is preserved (except for mint/burn operations).

**Rationale:** Ensures no funds are created or destroyed unexpectedly.

**Enforcement:** Aggregate check across state transitions.

## Meter-Specific Invariants

### INV-4: Meter Ownership Authorization
**Statement:** Only the meter owner can execute consume operations on that meter.

**Rationale:** Ensures users can only be charged for their own service usage.

**Enforcement:** `transaction.from == meter.owner` for consume operations.

**Violation consequence:** Transaction rejected.

### INV-5: Meter Uniqueness
**Statement:** At most one active meter exists per (owner, service_id) pair.

**Rationale:** Prevents duplicate metering points for the same service.

**Enforcement:** Checked during open-meter operations.

**Violation consequence:** Transaction rejected (unless reopening inactive meter).

### INV-6: Meter Active State Required
**Statement:** Consume operations require `meter.active == true`.

**Rationale:** Prevents charging against inactive or closed meters.

**Enforcement:** Checked during consume operations.

**Violation consequence:** Transaction rejected.

## Transaction-Specific Invariants

### INV-7: Mint Authorization
**Statement:** Only designated authority accounts can execute mint operations.

**Rationale:** Controls money supply and prevents unauthorized fund creation.

**Enforcement:** `transaction.from` must be in authorized mint addresses.

**Violation consequence:** Transaction rejected.

### INV-8: Sufficient Funds for Deposit
**Statement:** `account.balance >= deposit_amount` for open-meter operations.

**Rationale:** Ensures deposits are backed by available funds.

**Enforcement:** Checked during open-meter validation.

**Violation consequence:** Transaction rejected.

### INV-9: Sufficient Funds for Consumption
**Statement:** Account has enough balance to cover calculated consumption cost.

**Rationale:** Prevents over-spending.

**Enforcement:** Calculated based on pricing model during consume validation.

**Violation consequence:** Transaction rejected.

### INV-10: Valid Pricing Model
**Statement:** Pricing enum values are valid (unit_price > 0 for UnitPrice, cost > 0 for FixedCost).

**Rationale:** Prevents zero-cost or invalid pricing.

**Enforcement:** Checked during consume validation.

**Violation consequence:** Transaction rejected.

### INV-11: Non-Zero Units
**Statement:** `units > 0` for consume operations.

**Rationale:** Prevents meaningless zero-consumption transactions.

**Enforcement:** Checked during consume validation.

**Violation consequence:** Transaction rejected.

## Data Integrity Invariants

### INV-12: Non-Decreasing Totals
**Statement:** `meter.total_units` and `meter.total_spent` never decrease.

**Rationale:** Historical consumption data should be immutable.

**Enforcement:** Checked during state transitions.

**Violation consequence:** State corruption (should not occur in correct implementation).

### INV-13: Deterministic State Transitions
**Statement:** Given the same initial state and transaction sequence, the system always produces the same final state.

**Rationale:** Ensures reproducibility and auditability.

**Enforcement:** By construction (pure functions, no randomness).

---

## Testing Invariants

Each invariant should have corresponding unit tests that:
1. Set up valid initial state
2. Execute operations that would violate the invariant
3. Assert the operation fails or state remains unchanged

## References
- Domain spec: `docs/domain_spec.md`
- State transitions: `docs/state_transitions.md`