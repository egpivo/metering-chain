# Ubiquitous Language

This document defines the domain vocabulary used throughout the metering chain.
Terms listed here must be used consistently in code and documentation.

---

## Core Terms

### Account
An address-identified entity that holds balance and owns meters.

- Holds: balance, nonce
- Owns: meters
- Not a system or database user

---

### Meter
A logical usage ledger for a specific service owned by an account.

- Identity: `(owner, service_id)`
- Tracks: total usage, spending, and locked deposit
- Lifecycle: can be inactive or active
- Not a physical device or sensor

---

### Service ID
A stable identifier for a service type (e.g. `storage`, `api_calls`).

- Identifies service category
- Not a meter identifier or instance ID

---

### Units
A quantitative measure of service consumption.

- Represents usage only
- Never represents cost or value

---

### Balance
Spendable funds held by an account.

- Used to pay for consumption
- Excludes historical spend and locked deposits

---

### Locked Deposit
Funds committed to enable a meter and held until meter closure.

- Backed by account balance at meter creation
- Returned to balance when meter is closed
- Not spendable while locked

---

### Signer
The account that authorized and issued a transaction.

- Must match relevant owners for authorization
- Used for nonce checking and replay protection

---

### Mint From
The authority account recorded on a mint transaction.

- Used only by `Mint`
- Must be a member of `authorized_minters`

---

### Authorized Minter
An account permitted to mint new funds.

- Identified by membership in `authorized_minters`
- Used to validate `Mint.from`

---

### Consume
A domain operation that records usage and deducts cost.

- Mutates both meter and account state
- Requires signer authorization
- Not a fund transfer

---

### Pricing
Defines how usage cost is computed.

- Explicit and deterministic
- Either unit-based or fixed-cost

---

## Transaction Types

### Mint
Creates new funds (authority-only).

---

### OpenMeter
Creates a new meter for a service.

---

### Consume
Records usage and applies cost.

---

## State and Control

### Active Meter
A meter that accepts consumption.

---

### Nonce
A per-account sequence number used for ordering and replay protection.

---

### Invariant
A condition that must hold for all valid state transitions.

---

## Operations

### Validate
Checks whether a transaction satisfies all invariants.

---

### Apply
Executes a valid transaction to produce a new state.

---

### Reconstruct
Replays transactions from genesis to derive state.

---

## References
- Domain specification: `docs/domain_spec.md`
- Architecture: `docs/architecture.md`
