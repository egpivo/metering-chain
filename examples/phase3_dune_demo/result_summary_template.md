# Phase 3 Dune Demo — Result Summary (template)

Run ID: `YYYY-MM-DD` or commit / dataset label  
Dataset: e.g. `helium_rewards.csv` (7d, limit 500) or `sample_rewards.csv`

## Throughput

- Consume lines (capped): N
- Scene 2 (with proof) accepts: N
- Time (Scene 2 apply): T s  
- (Optional) tx/s: N/T

## Accept / Reject counts

| Scene | Expected | Accept | Reject | Reject reasons (counts) |
|-------|----------|--------|--------|---------------------------|
| 1 No proof | all reject | 0 | N | e.g. DelegatedConsumeRequiresV2: N |
| 2 With proof | all accept | N | 0 | — |
| 3 After revoke | reject | 0 | 1 | DelegationRevoked: 1 |
| 4 Replay | same state | — | — | — |

## Owner (1–2) before/after

- **Owner** (address): e.g. `0x...`
  - Before Scene 2: balance X, nonce 1 (after OpenMeter)
  - After Scene 2: balance Y, nonce 1+N, meter total_units Z
- (Optional) Second owner if run with different subset

## Replay check

- Same applied log re-applied in fresh state: state match? **yes / no**
- (If no: attach or describe diff)

## Reproducibility

- Fixed inputs: CSV (or fetch params), DEMO_MAX_CONSUMES, same wallet order
- Re-run same inputs → same accept/reject and final state: **yes / no**
