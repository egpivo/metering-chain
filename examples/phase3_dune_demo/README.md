# Phase 3 Dune Demo (real-data delegation)

Run Phase 3 delegation (signer ≠ owner + proof/scope/revoke) on **real Helium IOT data** from Dune. Same data pipeline as Phase 2; we only add delegation signing and the four demo scenes.

## Goal

- **Validate** that delegation rules hold on real data (no redoing accounting).
- **Reproducible:** fixed dataset → same outcomes.
- **Replayable:** same tx log → same final state.
- **Explainable:** every reject has a clear error class.

## Prerequisites

- Rust: `cargo run --bin metering-chain` works.
- Python 3 + deps for multi_operator:  
  `pip install -r examples/multi_operator/requirements.txt`  
  For Dune fetch: `pip install dune_spice` and `DUNE_API_KEY` in env.
- Run from **repo root**.

## Data flow

1. **Fetch** (optional): `examples/multi_operator/fetch_dune_iot_transfers.py` → `helium_rewards.csv`
2. **Convert**: `helium_rewards_to_consume.py` → `consume.ndjson` (one Consume kind per line)
3. **Demo script**: creates 1 owner + N delegate wallets, N proofs, runs 4 scenes.

## Quick run (with cached CSV)

If you already have `helium_rewards.csv` (e.g. from Phase 2):

```bash
# Use first 5 consume lines for a fast demo (optional: drop DEMO_MAX_CONSUMES to run full set)
export DEMO_MAX_CONSUMES=5
./examples/phase3_dune_demo/run_phase3_dune_demo.sh
```

The script will:

- Use `helium_rewards.csv` from (in order): `HELIUM_REWARDS_CSV`, else `examples/phase3_dune_demo/helium_rewards.csv`, else repo root `helium_rewards.csv`; if missing, fetch from Dune when `DUNE_API_KEY` is set, else fall back to `multi_operator/sample_rewards.csv`.
- Produce `consume.ndjson` (or take first `DEMO_MAX_CONSUMES` lines).
- Create wallets, Mint, OpenMeter, one proof per delegate.
- **Scene 1:** Apply batch signed by delegates **without** proof → expect all rejected (e.g. `Delegated Consume requires payload_version=2`).
- **Scene 2:** Apply same batch **with** proof + correct nonce → expect all accepted; save applied tx log.
- **Scene 3:** Revoke one capability, re-apply one tx using that proof → expect `Delegation revoked`.
- **Scene 4:** Replay: copy data dir (keeps wallets), clear state/tx.log, init, mint+open, apply saved `applied.ndjson`; compare owner report to Scene 2 → same state.

## Full pipeline (fetch from Dune)

```bash
export DUNE_API_KEY="your_key"
# Fetch 7 days, max 5000 rows (or use --start-date/--end-date for fixed window)
python examples/multi_operator/fetch_dune_iot_transfers.py --days 7 --limit 5000 --output helium_rewards.csv

# Convert; owner is the payer (one wallet for whole demo)
OWNER_ADDR="0x..."   # or create below and set
python examples/multi_operator/helium_rewards_to_consume.py \
  --input helium_rewards.csv \
  --hotspot-col to_owner \
  --amount-col amount \
  --owner "$OWNER_ADDR" \
  --scale 1 \
  --aggregate \
  --service-id helium-rewards \
  --require-service-id \
  --mode kind \
  --output consume.ndjson

# Run demo (script creates owner if not set; or pass owner from existing wallets)
export DEMO_MAX_CONSUMES=10
./examples/phase3_dune_demo/run_phase3_dune_demo.sh
```

## Reject classification

The script (or manual inspection) should map apply errors to:

- `Delegated Consume requires payload_version=2` → **DelegatedConsumeRequiresV2**
- `Delegation revoked` → **DelegationRevoked**
- `Delegation scope mismatch` → **DelegationScopeMismatch**
- `Delegation expired or not yet valid` → **DelegationExpiredOrNotValid**
- `Capability limit exceeded` → **CapabilityLimitExceeded**
- Other → **InvalidTransaction** / etc.

Scene 1 and Scene 3 outputs can be summarized as counts per reason.

## Result summary (template)

After a run, record:

- **Throughput:** e.g. N accepts in T seconds.
- **Accept count:** Scene 2.
- **Reject counts by scene:** Scene 1 (no proof), Scene 3 (after revoke), with reason counts.
- **1–2 owners:** e.g. owner balance and nonce before/after Scene 2; meter total_units.
- **Replay:** Scene 4 final state matches Scene 2 (same nonce, balance, meter).

- **Result summary:** Fill `result_summary_template.md` after a run (throughput, accept/reject counts, owner before/after, replay match).
- **Plan:** `.local/phase3_dune_demo_plan.md` (pipeline, scenes, reject classification).

Generated dirs: `data/`, `replay_data/` (can be gitignored or removed between runs).
