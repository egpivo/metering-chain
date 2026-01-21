# Ubiquitous Language — Metering Chain

This document defines the **shared vocabulary** used throughout the metering chain domain. All team members, documentation, and code should use these terms consistently.

## Core Terms

### Account
A **financial entity** identified by an address that holds balance and owns meters.

**Synonyms:** wallet, user account, payer
**Not:** database user, system account

### Meter
A **service metering point** that tracks usage for a specific service owned by an account.

**Synonyms:** usage tracker, billing point
**Not:** physical meter, sensor

### Service ID
A **unique identifier** for a service type (e.g., "electricity", "storage", "api-calls").

**Not:** service instance, meter ID

### Units
The **quantitative measure** of service consumption (e.g., kWh for electricity, GB for storage).

**Synonyms:** quantity, amount consumed
**Not:** cost, price

### Balance
The **available funds** in an account that can be used for service payments.

**Synonyms:** available funds, credit
**Not:** total spent, locked funds

### Deposit
**Funds committed** to enable service usage (may be locked or deducted).

**Synonyms:** collateral, commitment
**Not:** payment, fee

### Consume
The **act of recording** service usage and deducting the corresponding cost.

**Synonyms:** use service, charge usage
**Not:** pay, transfer funds

### Pricing
The **cost calculation method** for service usage (unit price or fixed cost).

**Synonyms:** cost model, billing model
**Not:** price list, tariff

## Transaction Types

### Mint
**Create new funds** in the system (typically by authority).

**Example:** "Mint 1000 tokens to account A"

### OpenMeter
**Create a new meter** for tracking service usage.

**Example:** "Open a meter for electricity service with 500 deposit"

### Consume
**Record service usage** and deduct costs.

**Example:** "Consume 10 units of storage at $0.10 per unit"

## States and Status

### Active Meter
A meter that **can accept consumption** transactions.

**Opposite:** Inactive meter

### Nonce
A **sequence number** that prevents transaction replay and ensures ordering.

**Synonyms:** sequence number, counter
**Not:** timestamp, version

### Invariant
A **business rule** that must always hold true in the system.

**Synonyms:** constraint, rule, property
**Not:** validation, check

## Domain Operations

### Apply
**Execute a transaction** to transform state deterministically.

**Synonyms:** process, execute
**Not:** validate, persist

### Validate
**Check if a transaction** meets all preconditions and invariants.

**Synonyms:** verify, check
**Not:** apply, execute

### Reconstruct
**Replay transactions** from genesis to rebuild state.

**Synonyms:** replay, rebuild
**Not:** restore, load

---

## Usage Guidelines

1. **Use domain terms in code comments** and variable names
2. **Avoid ambiguous synonyms** — prefer the primary term
3. **Update this document** when new terms are introduced
4. **Reference this document** in pull requests and documentation

## References
- Domain spec: `docs/domain_spec.md`
- Architecture: `docs/architecture.md`