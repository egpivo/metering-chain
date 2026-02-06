#!/usr/bin/env bash
# Phase 3 Dune demo: real-data delegation (signer != owner + proof/scope/revoke).
# Run from repo root. Uses multi_operator for fetch/convert.
# Optional: DEMO_MAX_CONSUMES (cap lines), DUNE_API_KEY (to fetch if CSV missing), DATA_DIR.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
BIN="cargo run --bin metering-chain --"
MULTI_OP="$REPO_ROOT/examples/multi_operator"
DEMO_DIR="${DEMO_DIR:-$REPO_ROOT/examples/phase3_dune_demo}"
DATA_DIR="${DATA_DIR:-$DEMO_DIR/data}"
DARGS="-d $DATA_DIR"
mkdir -p "$DATA_DIR" "$DEMO_DIR"
MAX_CONSUMES="${DEMO_MAX_CONSUMES:-5}"

# --- Resolve CSV: HELIUM_REWARDS_CSV, else demo dir, else repo root (current dir), else fetch or sample ---
if [[ -n "${HELIUM_REWARDS_CSV:-}" ]] && [[ -f "$HELIUM_REWARDS_CSV" ]]; then
  CSV="$HELIUM_REWARDS_CSV"
elif [[ -f "$DEMO_DIR/helium_rewards.csv" ]]; then
  CSV="$DEMO_DIR/helium_rewards.csv"
elif [[ -f "$REPO_ROOT/helium_rewards.csv" ]]; then
  CSV="$REPO_ROOT/helium_rewards.csv"
else
  CSV="$DEMO_DIR/helium_rewards.csv"
fi
if [[ ! -f "$CSV" ]] && [[ -n "${DUNE_API_KEY:-}" ]]; then
  echo "Fetching from Dune (limit 500, 7 days)..."
  python "$MULTI_OP/fetch_dune_iot_transfers.py" --days 7 --limit 500 --output "$CSV"
fi
if [[ ! -f "$CSV" ]]; then
  SAMPLE="$MULTI_OP/sample_rewards.csv"
  if [[ -f "$SAMPLE" ]]; then
    echo "Using sample CSV (no Dune fetch)."
    CSV="$SAMPLE"
    HOTSPOT_COL="hotspot"
    AMOUNT_COL="amount"
  else
    echo "No helium_rewards.csv in DEMO_DIR, repo root, or HELIUM_REWARDS_CSV. Set DUNE_API_KEY or ensure $SAMPLE exists."
    exit 1
  fi
else
  HOTSPOT_COL="to_owner"
  AMOUNT_COL="amount"
fi

$BIN $DARGS init 2>/dev/null || true
AUTHORITY=$($BIN $DARGS wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
OWNER=$($BIN $DARGS wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
export METERING_CHAIN_MINTERS="$AUTHORITY"

# Convert to consume.ndjson (one Consume kind per line)
CONSUME_NDJSON="$DEMO_DIR/consume.ndjson"
python "$MULTI_OP/helium_rewards_to_consume.py" \
  --input "$CSV" \
  --hotspot-col "${HOTSPOT_COL:-to_owner}" \
  --amount-col "${AMOUNT_COL:-amount}" \
  --owner "$OWNER" \
  --scale 1 \
  --aggregate \
  --service-id helium-rewards \
  --require-service-id \
  --mode kind \
  --output "$CONSUME_NDJSON"

# Cap lines for demo
CONSUME_CAPPED="$DEMO_DIR/consume_capped.ndjson"
head -n "$MAX_CONSUMES" "$CONSUME_NDJSON" > "$CONSUME_CAPPED"
N=$(wc -l < "$CONSUME_CAPPED")
echo "Owner: $OWNER | Consume lines (capped): $N"

# Create N delegate wallets
DELEGATES=()
for ((i=0;i<N;i++)); do
  DELEGATES+=("$($BIN $DARGS wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')")
done

# Mint and OpenMeter (owner)
printf '{"Mint":{"to":"%s","amount":10000000}}\n' "$OWNER" > "$DEMO_DIR/k_mint.json"
$BIN $DARGS wallet sign --address "$AUTHORITY" --file "$DEMO_DIR/k_mint.json" 2>/dev/null | $BIN $DARGS apply
printf '{"OpenMeter":{"owner":"%s","service_id":"helium-rewards","deposit":100000}}\n' "$OWNER" > "$DEMO_DIR/k_open.json"
$BIN $DARGS wallet sign --address "$OWNER" --file "$DEMO_DIR/k_open.json" 2>/dev/null | $BIN $DARGS apply

VALID_AT=$(date +%s 2>/dev/null || echo 1000)
IAT=0
EXP=$((VALID_AT + 3600))
# One proof per delegate
for ((i=0;i<N;i++)); do
  $BIN $DARGS wallet create-delegation-proof \
    --address "$OWNER" --audience "${DELEGATES[$i]}" \
    --service-id helium-rewards --ability consume \
    --iat "$IAT" --exp "$EXP" \
    --output "$DEMO_DIR/proof_$i.bin" 2>/dev/null
done

# --- Scene 1: No proof -> reject ---
echo "=== Scene 1: No-proof batch (expected all rejected) ==="
REJECT_COUNT=0
idx=0
while IFS= read -r line; do
  printf '%s' "$line" > "$DEMO_DIR/cursor.json"
  if $BIN $DARGS wallet sign --address "${DELEGATES[$idx]}" --file "$DEMO_DIR/cursor.json" 2>/dev/null | $BIN $DARGS apply 2>&1; then
    echo "UNEXPECTED accept at index $idx"
    exit 1
  else
    ((REJECT_COUNT++)) || true
  fi
  ((idx++)) || true
done < "$CONSUME_CAPPED"
echo "Rejects (no proof): $REJECT_COUNT"

# --- Scene 2: With proof -> accept ---
echo "=== Scene 2: With-proof batch (expected all accepted) ==="
APPLIED_LOG="$DEMO_DIR/applied.ndjson"
:> "$APPLIED_LOG"
OWNER_NONCE=1
idx=0
while IFS= read -r line; do
  printf '%s' "$line" > "$DEMO_DIR/cursor.json"
  SIGNED=$($BIN $DARGS wallet sign --address "${DELEGATES[$idx]}" --file "$DEMO_DIR/cursor.json" \
    --for-owner "$OWNER" --nonce "$OWNER_NONCE" --valid-at "$VALID_AT" --proof-file "$DEMO_DIR/proof_$idx.bin" 2>/dev/null)
  echo "$SIGNED" | python3 -c "import json,sys; print(json.dumps(json.load(sys.stdin)))" >> "$APPLIED_LOG"
  if ! echo "$SIGNED" | $BIN $DARGS apply 2>&1; then
    echo "UNEXPECTED reject at index $idx (with proof)"
    exit 1
  fi
  ((OWNER_NONCE++)) || true
  ((idx++)) || true
done < "$CONSUME_CAPPED"
echo "Accepted: $N"
$BIN $DARGS report "$OWNER" 2>/dev/null > "$DEMO_DIR/state_after_scene2.txt" || true

# --- Scene 3: Revoke then reject ---
echo "=== Scene 3: Revoke capability 0, re-apply first tx (expected Delegation revoked) ==="
CAP_ID=$($BIN $DARGS wallet capability-id --proof-file "$DEMO_DIR/proof_0.bin" 2>/dev/null)
$BIN $DARGS wallet revoke-delegation --address "$OWNER" --capability-id "$CAP_ID" --output "$DEMO_DIR/revoke.json" 2>/dev/null
$BIN $DARGS apply --file "$DEMO_DIR/revoke.json" 2>/dev/null
FIRST_TX=$(head -n1 "$APPLIED_LOG")
if echo "$FIRST_TX" | $BIN $DARGS apply 2>&1; then
  echo "UNEXPECTED: apply after revoke should fail"
  exit 1
else
  echo "Expected: Delegation revoked."
fi

# --- Scene 4: Replay check (same log -> same state) ---
echo "=== Scene 4: Replay (same applied log -> same state) ==="
REPLAY_DIR="$DEMO_DIR/replay_data"
rm -rf "$REPLAY_DIR"
mkdir -p "$REPLAY_DIR"
REPLAY_ARGS="-d $REPLAY_DIR"
cp -r "$DATA_DIR"/* "$REPLAY_DIR"/
rm -f "$REPLAY_DIR/state.bin" "$REPLAY_DIR/tx.log" 2>/dev/null || true
$BIN $REPLAY_ARGS init 2>/dev/null || true
printf '{"Mint":{"to":"%s","amount":10000000}}\n' "$OWNER" > "$DEMO_DIR/k_mint2.json"
$BIN $REPLAY_ARGS wallet sign --address "$AUTHORITY" --file "$DEMO_DIR/k_mint2.json" 2>/dev/null | $BIN $REPLAY_ARGS apply
printf '{"OpenMeter":{"owner":"%s","service_id":"helium-rewards","deposit":100000}}\n' "$OWNER" > "$DEMO_DIR/k_open2.json"
$BIN $REPLAY_ARGS wallet sign --address "$OWNER" --file "$DEMO_DIR/k_open2.json" 2>/dev/null | $BIN $REPLAY_ARGS apply
while IFS= read -r line; do
  echo "$line" | $BIN $REPLAY_ARGS apply
done < "$APPLIED_LOG"
$BIN $REPLAY_ARGS report "$OWNER" 2>/dev/null > "$DEMO_DIR/state_after_replay.txt" || true
if diff -q "$DEMO_DIR/state_after_scene2.txt" "$DEMO_DIR/state_after_replay.txt" >/dev/null 2>&1; then
  echo "Replay matches: same state."
else
  echo "Replay state diff:"
  diff "$DEMO_DIR/state_after_scene2.txt" "$DEMO_DIR/state_after_replay.txt" || true
fi

echo ""
echo "=== Phase 3 Dune demo done ==="
echo "Summary: no-proof rejects=$REJECT_COUNT, with-proof accepts=$N, revoke then reject OK, replay check above."
