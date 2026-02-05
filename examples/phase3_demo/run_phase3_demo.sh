#!/usr/bin/env bash
# Phase 3 delegation demo: four scenes (delegate signs, no proof rejected, with proof accepted, revoke then reject).
# Run from repo root: ./examples/phase3_demo/run_phase3_demo.sh
set -euo pipefail

BIN="cargo run --bin metering-chain --"
DEMO_TMP="${DEMO_TMP:-$(mktemp -d 2>/dev/null || echo /tmp/metering_phase3_demo)}"
DATA_DIR="$DEMO_TMP/data"
DARGS="-d $DATA_DIR"
cleanup() { rm -rf "$DEMO_TMP"; }
trap cleanup EXIT
mkdir -p "$DEMO_TMP"

echo "=== Phase 3 demo: Who can act on behalf of another? ==="
echo ""

$BIN $DARGS init

AUTHORITY=$($BIN $DARGS wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
OWNER=$($BIN $DARGS wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
DELEGATE=$($BIN $DARGS wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
export METERING_CHAIN_MINTERS="$AUTHORITY"

echo "Authority: $AUTHORITY"
echo "Owner (account): $OWNER"
echo "Delegate (e.g. hotspot): $DELEGATE"
echo ""

# --- Setup: Mint and OpenMeter (owner) ---
echo "--- Setup: Mint and OpenMeter (owner) ---"
echo "{\"Mint\":{\"to\":\"$OWNER\",\"amount\":1000}}" > "$DEMO_TMP/k1.json"
$BIN $DARGS wallet sign --address "$AUTHORITY" --file "$DEMO_TMP/k1.json" 2>/dev/null | $BIN $DARGS apply

echo "{\"OpenMeter\":{\"owner\":\"$OWNER\",\"service_id\":\"storage\",\"deposit\":100}}" > "$DEMO_TMP/k2.json"
$BIN $DARGS wallet sign --address "$OWNER" --file "$DEMO_TMP/k2.json" 2>/dev/null | $BIN $DARGS apply

echo "{\"Consume\":{\"owner\":\"$OWNER\",\"service_id\":\"storage\",\"units\":10,\"pricing\":{\"UnitPrice\":2}}}" > "$DEMO_TMP/consume.json"
echo ""

# --- Scene 1 & 2: No proof, delegate signs Consume (expected reject) ---
echo "--- Scene 1 & 2: Delegate signs Consume without delegation proof (expected reject) ---"
echo "(signer = delegate != owner, no proof; validation must reject)"
if $BIN $DARGS wallet sign --address "$DELEGATE" --file "$DEMO_TMP/consume.json" 2>/dev/null | $BIN $DARGS apply 2>&1; then
  echo "UNEXPECTED: apply should have failed (no proof)"
  exit 1
else
  echo "Expected: rejected."
fi
echo ""

# --- Scene 3: With proof, accepted ---
echo "--- Scene 3: Owner issues delegation proof, delegate signs same Consume (expected accept) ---"
VALID_AT=$(date +%s 2>/dev/null || echo 1000)
IAT=0
EXP=$((VALID_AT + 3600))
$BIN $DARGS wallet create-delegation-proof --address "$OWNER" --audience "$DELEGATE" --service-id storage --iat "$IAT" --exp "$EXP" --output "$DEMO_TMP/proof.bin" 2>/dev/null
echo "Proof written to $DEMO_TMP/proof.bin"

# Owner nonce is 1 (OpenMeter used 0)
$BIN $DARGS wallet sign --address "$DELEGATE" --file "$DEMO_TMP/consume.json" \
  --for-owner "$OWNER" --nonce 1 --valid-at "$VALID_AT" --proof-file "$DEMO_TMP/proof.bin" 2>/dev/null \
  | $BIN $DARGS apply
echo "Expected: accepted."
$BIN $DARGS account "$OWNER" 2>/dev/null
$BIN $DARGS meters "$OWNER" 2>/dev/null
echo ""

# --- Scene 4: Revoke then reject ---
echo "--- Scene 4: Owner revokes capability, delegate sends Consume again with same proof (expected DelegationRevoked) ---"
CAP_ID=$($BIN $DARGS wallet capability-id --proof-file "$DEMO_TMP/proof.bin" 2>/dev/null)
$BIN $DARGS wallet revoke-delegation --address "$OWNER" --capability-id "$CAP_ID" --output "$DEMO_TMP/revoke.json" 2>/dev/null
$BIN $DARGS apply --file "$DEMO_TMP/revoke.json" 2>/dev/null
echo "RevokeDelegation applied."

# Owner nonce is now 3 (1=OpenMeter, 2=Consume, 3=RevokeDelegation). Delegate consume with nonce 3
echo "{\"Consume\":{\"owner\":\"$OWNER\",\"service_id\":\"storage\",\"units\":5,\"pricing\":{\"UnitPrice\":2}}}" > "$DEMO_TMP/consume2.json"
if $BIN $DARGS wallet sign --address "$DELEGATE" --file "$DEMO_TMP/consume2.json" \
  --for-owner "$OWNER" --nonce 3 --valid-at "$VALID_AT" --proof-file "$DEMO_TMP/proof.bin" 2>/dev/null \
  | $BIN $DARGS apply 2>&1; then
  echo "UNEXPECTED: apply should have failed (DelegationRevoked)"
  exit 1
else
  echo "Expected: DelegationRevoked."
fi
echo ""

echo "=== Phase 3 demo scenes complete ==="
