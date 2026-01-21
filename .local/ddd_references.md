# DDD References for `metering-chain` (for future Medium posts)

> Goal: collect “classic” domain-driven references that inform how we model **transactions**, **state transitions**, and **invariants**.  
> This repo is intentionally moving toward **domain semantics first** (not “product narrative first”).

---

## Tier 1 — Classic references (must-read, even if only the design/spec parts)

### 1) Bitcoin Core — UTXO model / transaction semantics “the original”

#### Why it matters
- Bitcoin has minimal “product story”; it is mostly **transaction semantics + verification rules**.
- It’s one of the purest examples of **domain semantics** as a first-class system.
- The code is effectively a living specification of **UTXO invariants** and **block connection rules**.

#### What to look at (not everything)
- **`CheckTxInputs`**: what it means for inputs to be valid given the UTXO set
- **`ConnectBlock`**: how a block is applied, what checks happen, and what invariants must hold
- **UTXO set invariants**: the “always true” rules the system preserves as blocks/tx are processed

#### DDD angle (how to use it)
- Don’t “learn Bitcoin”; learn **what it means for a transaction to be a first-class domain object**:
  - A transaction isn’t just a data blob—it carries **intent + constraints + validity rules**
  - Validity is checked against **current domain state** (UTXO set)
  - The system preserves **invariants** when applying domain events (transactions)

#### Copy-ready line for your article
> “In a UTXO-style domain model, transaction validity is defined as a set of invariants over the current UTXO set, not as ad-hoc business logic.”

---

### 2) Ethereum Yellow Paper — state transition “textbook”

#### Why it matters
- It is not a tutorial; it’s a **formal definition of state transitions**.
- Each rule answers:
  - “Given this **state**, is this **transaction** valid?”
  - “If valid, what **new state** results?”

#### DDD angle (how to use it)
- Treat a transaction as a **pure function over state** (conceptually):
  - You can describe domain logic as \( \text{State} \times \text{Tx} \rightarrow \text{State}' \) plus receipts/events
- Great template for writing a Medium post where you define:
  - Preconditions (validity rules)
  - Transition function (how state changes)
  - Postconditions (invariants still hold)

#### Copy-ready line for your article
> “Following the Ethereum-style state transition model, a transaction is a pure function over state.”

---

## Notes for our repo (how these map to `metering-chain`)

- **Transaction as domain object**: we’re pivoting to a **metering-first** transaction model (domain commands) rather than UTXO-first.
- **State transition**: `state::apply` will be the “Yellow Paper style” transition function: validate preconditions, then apply changes deterministically.
- **Invariants**: we’ll explicitly name invariants like nonce monotonicity, no balance underflow, authorization, meter uniqueness, and cost accounting.

## TODO (future additions)
- Add concrete links to specific Bitcoin Core source locations and Yellow Paper sections once we pin down the exact flow we implement.
- Add notes on how “meter readings” map into our transaction/state model (e.g., special output type, data-commitment, or separate domain event).

