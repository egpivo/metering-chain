# Utils Requirements

This document lists utility functions that are needed for the current DDD
metering-chain scope. Utilities should live in infrastructure code; the domain
model should not depend on them directly.

---

## Required Now (MVP)

- **Encoding/decoding** for `SignedTx` and `State` to support append-only logs
  and snapshots (e.g., `serde` + `bincode` or JSON).
- **Optional tx id hashing** if you need stable transaction identifiers for
  lookup or reporting.

---

## Not Required for DDD Core

The following are out of scope for the current domain model and should stay in
future-phase or wallet design docs:

- Keypair generation, signing, signature verification
- Address encoding/decoding (base58, ripemd160, etc.)
- Proof-of-work or block mining utilities

---

## References
- Architecture: `docs/architecture.md`
- Domain spec: `docs/domain_spec.md`
