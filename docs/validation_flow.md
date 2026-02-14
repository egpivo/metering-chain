# Validation Flow (WS-R2)

Normalized validation pipeline: **auth checks → domain checks → replay/evidence checks**.

## Entrypoint

- `validate(state, tx, ctx, authorized_minters)` — dispatches to per-tx validators.

## Flow by Tx Type

### Mint
- **Auth**: signer in `authorized_minters` (or skip when `None` in replay)
- **Domain**: amount > 0

### OpenMeter
- **Auth**: signer == owner
- **Domain**: account exists, nonce match, deposit > 0, sufficient balance, no active meter for (owner, service_id)

### Consume (owner-signed)
- **Metering** (shared): meter exists, active, units > 0, pricing valid, cost computed
- **Auth**: signer == owner; nonce_account None or Some(owner)
- **Domain**: account exists, nonce match, sufficient balance

### Consume (delegated)
- **Metering** (shared): meter exists, active, units > 0, pricing valid, cost computed
- **Auth**: payload_version=2, proof present, valid_at present, nonce_account=owner; issuer/audience binding; scope (service_id, ability); caveats (max_units, max_cost); not revoked
- **Replay/Evidence**: Live: now, max_age, valid_at within window; Replay: no wall clock, only iat ≤ valid_at < exp
- **Domain**: nonce match (owner), sufficient balance (owner)

### CloseMeter
- **Auth**: signer == owner
- **Domain**: account exists, nonce match, meter exists, meter active

### RevokeDelegation
- **Auth**: signer == owner
- **Domain**: account exists, nonce match

## Validator Entrypoints

| Tx Type    | Validator         | Returns        |
|------------|-------------------|----------------|
| Mint       | validate_mint     | ()             |
| OpenMeter  | validate_open_meter | ()           |
| Consume    | validate_consume  | u64 (cost)     |
| CloseMeter | validate_close_meter | ()          |
| RevokeDelegation | validate_revoke_delegation | () |
