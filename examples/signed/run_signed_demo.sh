#!/usr/bin/env bash
# Phase 2 signed demo: init, 2 wallets, Mint → OpenMeter → Consume → CloseMeter (no --allow-unsigned).
# Run from repo root: ./examples/signed/run_signed_demo.sh
set -euo pipefail

BIN="cargo run --bin metering-chain --"
DEMO_TMP="${DEMO_TMP:-$(mktemp -d 2>/dev/null || echo /tmp/metering_signed_demo)}"
cleanup() { rm -rf "$DEMO_TMP"; }
trap cleanup EXIT
mkdir -p "$DEMO_TMP"

$BIN init

AUTHORITY=$($BIN wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
USER=$($BIN wallet create 2>/dev/null | grep -oE '0x[a-fA-F0-9]+')
export METERING_CHAIN_MINTERS="$AUTHORITY"

echo "Authority: $AUTHORITY"
echo "User: $USER"

# Mint (authority → user)
echo "{\"Mint\":{\"to\":\"$USER\",\"amount\":1000}}" > "$DEMO_TMP/k1.json"
$BIN wallet sign --address "$AUTHORITY" --file "$DEMO_TMP/k1.json" 2>/dev/null | $BIN apply

# OpenMeter (user)
echo "{\"OpenMeter\":{\"owner\":\"$USER\",\"service_id\":\"storage\",\"deposit\":100}}" > "$DEMO_TMP/k2.json"
$BIN wallet sign --address "$USER" --file "$DEMO_TMP/k2.json" 2>/dev/null | $BIN apply

# Consume (user)
echo "{\"Consume\":{\"owner\":\"$USER\",\"service_id\":\"storage\",\"units\":10,\"pricing\":{\"UnitPrice\":2}}}" > "$DEMO_TMP/k3.json"
$BIN wallet sign --address "$USER" --file "$DEMO_TMP/k3.json" 2>/dev/null | $BIN apply

# CloseMeter (user)
echo "{\"CloseMeter\":{\"owner\":\"$USER\",\"service_id\":\"storage\"}}" > "$DEMO_TMP/k4.json"
$BIN wallet sign --address "$USER" --file "$DEMO_TMP/k4.json" 2>/dev/null | $BIN apply

echo "---"
$BIN account "$USER"
$BIN meters "$USER"
$BIN report "$USER"
