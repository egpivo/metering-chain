# DePIN Reward Distributor Demo (SIM Dune Activity API)

This example shows how to pull real on-chain reward distributions and convert
activity into `Consume` transactions for a deterministic billing demo.

The flow uses SIM Dune's Activity API and a token address (e.g. DIMO) to find
systematic reward transfers.

## Prereqs

- `curl`
- `jq`
- SIM Dune API key (set `SIM_API_KEY`)
- Optional: copy `.env.example` to `.env` and export variables

## Step 1: Fetch activity

Choose a candidate distributor address and token (DIMO example below):

```bash
export SIM_API_KEY="your_key"
export ADDR="0xE261D618a959aFfFd53168Cd07D12E37B26761db"
export CHAIN_ID=1

./fetch_sim_activity.sh
```

This writes `activity.json` in the current directory.

## Step 2: Convert to Consume transactions

```bash
export TOKEN_ADDRESS="0xE261D618a959aFfFd53168Cd07D12E37B26761db"
export SIGNER="0x0000000000000000000000000000000000000A11"
export OWNER="$SIGNER"
export SERVICE_ID="dimo-rewards"
export START_NONCE=1
export UNIT_PRICE=1

./sim_activity_to_consume.sh activity.json > consume.ndjson
```

Tip: you can `source .env` (after filling it) instead of exporting each var.

### Rust converter (decimals-aware)

This uses token decimals from the API (or `--decimals`) and lets you choose
the target precision.

```bash
cargo run --bin sim-activity-to-consume -- \
  --input activity.json \
  --output consume.ndjson \
  --token-address "$TOKEN_ADDRESS" \
  --signer "$SIGNER" \
  --owner "$OWNER" \
  --service-id "$SERVICE_ID" \
  --start-nonce "$START_NONCE" \
  --unit-price "$UNIT_PRICE" \
  --target-decimals 6 \
  --rounding floor
```

Notes:
- `target-decimals` <= token decimals; use a smaller value to keep integer units.
- Use `--rounding reject` to fail on fractional remainders.

Notes:
- This demo aggregates all usage under a single account (`OWNER`) so nonces are
  sequential. In a real system you would sign per user (or delegate).
- `units` are taken from the raw `value` field. If you need human-readable
  units, pre-scale the amounts before conversion.

## Step 3: Apply transactions

```bash
cargo run --bin metering-chain -- init

echo '{"signer":"0x0000000000000000000000000000000000000AAA","nonce":0,"kind":{"Mint":{"to":"0x0000000000000000000000000000000000000A11","amount":1000000000}}}' | \
  cargo run --bin metering-chain -- apply

echo '{"signer":"0x0000000000000000000000000000000000000A11","nonce":0,"kind":{"OpenMeter":{"owner":"0x0000000000000000000000000000000000000A11","service_id":"dimo-rewards","deposit":1000}}}' | \
  cargo run --bin metering-chain -- apply

while IFS= read -r line; do
  printf '%s' "$line" | cargo run --bin metering-chain -- apply
  printf '\n'
done < consume.ndjson
```

## Files

- `fetch_sim_activity.sh` - calls SIM Dune Activity API
- `sim_activity_to_consume.sh` - converts activity JSON to `Consume` txs
