# Helium multi-operator demo (free data)

This demo simulates **multiple operators** (hotspots) using Helium rewards data.
It keeps billing deterministic and maps each hotspot to a service_id.

**Python:** `helium_rewards_to_consume.py` uses stdlib only. For `analyze_rewards.py` and `fetch_dune_iot_transfers.py`, run:
```bash
pip install -r examples/multi_operator/requirements.txt
```

## 0) Data source (real data)

Two options:
1) **Dune + spice** (recommended): fetch Helium IOT token transfers (real data) via Dune API.
2) **Relay API** (community plan): fetch rewards + hotspots via Relay (requires a Relay API key).

## 1) Prepare a CSV

Your CSV must include these columns (header names configurable):
- `hotspot` (gateway address)
- `amount` (reward amount as decimal)

Example format:

```csv
hotspot,amount
1123abc...,1.2345
1123abc...,0.1000
9988def...,2.5000
```

## 1a) Fetch real data via Dune + spice (IOT transfers)

This uses Dune’s `tokens_solana.transfers` table and the Helium IOT mint.
The output CSV columns are `block_time`, `to_owner`, `amount`, `token_mint_address`.
We treat `to_owner` as the **operator** for this demo.

Install spice once:

```bash
python -m pip install dune_spice
```

Ensure your API key is set (e.g., in `.env`):

```bash
export DUNE_API_KEY="YOUR_KEY"
```

```bash
# Requires DUNE_API_KEY in environment (see .env)
./examples/multi_operator/fetch_dune_iot_transfers.py \\
  --days 7 \\
  --limit 5000 \\
  --output helium_rewards.csv
```

Then convert to Consume using `--hotspot-col to_owner` and `--amount-col amount`:

```bash
./examples/multi_operator/helium_rewards_to_consume.py \\
  --input helium_rewards.csv \\
  --hotspot-col to_owner \\
  --amount-col amount \\
  --owner 0xUSER \\
  --scale 1 \\
  --aggregate \\
  --service-id helium-rewards \\
  --require-service-id \\
  --mode kind \\
  > consume.ndjson
```

## 1b) Analyze rewards (optional)

Generate a 2x2 chart + summary JSON (similar to the SIM Dune demo):

```bash
./examples/multi_operator/analyze_rewards.py \
  --input helium_rewards.csv \
  --operator-col to_owner \
  --amount-col amount \
  --output-image helium_analysis.png \
  --output-summary helium_summary.json
```

If you don't have matplotlib installed:

```bash
python -m pip install matplotlib
```

## 2) Convert CSV to Consume NDJSON (manual CSV)

```bash
./examples/multi_operator/helium_rewards_to_consume.py \
  --input helium_rewards.csv \
  --owner 0xUSER \
  --scale 1000000 \
  --rounding floor \
  --aggregate \
  --service-id helium-rewards \
  --mode kind \
  > consume.ndjson
```

Notes:
- `--scale 1000000` converts 6 decimals into integer units.
- `--aggregate` groups by hotspot so each hotspot becomes one Consume tx.
- `--service-id helium-rewards` makes all consumes use one meter (matches the OpenMeter in section 3). Omit it to get per-hotspot meters (`helium:<hotspot>`).
- Use `--require-service-id` with `--aggregate` to force a reminder if you forget `--service-id` (avoids apply failures).
- Zero-unit rows are skipped by default; use `--allow-zero` to include them.

## 3) Signed apply (Phase 2 default)

```bash
cargo run --bin metering-chain -- init

# Create wallets
AUTH=$(cargo run --bin metering-chain -- wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
USER=$(cargo run --bin metering-chain -- wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
export METERING_CHAIN_MINTERS="$AUTH"

# Mint and open meter
printf '{"Mint":{"to":"%s","amount":1000000}}' "$USER" > /tmp/k_mint.json
cargo run --bin metering-chain -- wallet sign --address "$AUTH" --file /tmp/k_mint.json | \
  cargo run --bin metering-chain -- apply

# OpenMeter: service_id must match the Consume service_id from step 2 (e.g. helium-rewards).
printf '{"OpenMeter":{"owner":"%s","service_id":"helium-rewards","deposit":1000}}' "$USER" > /tmp/k_open.json
cargo run --bin metering-chain -- wallet sign --address "$USER" --file /tmp/k_open.json | \
  cargo run --bin metering-chain -- apply

# Apply consume lines (each line is a kind-only Consume)
while IFS= read -r line; do
  printf '%s' "$line" > /tmp/k_consume.json
  cargo run --bin metering-chain -- wallet sign --address "$USER" --file /tmp/k_consume.json | \
    cargo run --bin metering-chain -- apply
  printf '\n'
done < consume.ndjson

cargo run --bin metering-chain -- report "$USER"
```

## 4) Legacy unsigned apply (optional)

If you want to use unsigned tx for quick import:

```bash
./examples/multi_operator/helium_rewards_to_consume.py \
  --input helium_rewards.csv \
  --owner 0xUSER \
  --scale 1000000 \
  --aggregate \
  --service-id helium-rewards \
  --mode unsigned \
  --start-nonce 0 \
  > consume_unsigned.ndjson

while IFS= read -r line; do
  printf '%s' "$line" | cargo run --bin metering-chain -- apply --allow-unsigned
  printf '\n'
done < consume_unsigned.ndjson
```

## Notes
- Multi-operator is modeled by **service_id per hotspot**.
- Owner/payer is a single user wallet (simple demo). For true operator delegation, see Phase 3 (UCAN/ReCaps).
