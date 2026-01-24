#!/usr/bin/env bash
set -euo pipefail

: "${SIM_API_KEY:?SIM_API_KEY is required}"
: "${ADDR:?ADDR is required}"

CHAIN_ID="${CHAIN_ID:-1}"
LIMIT="${LIMIT:-100}"
OUT="${OUT:-activity.json}"

curl -s \
  "https://api.sim.dune.com/v1/evm/activity/${ADDR}?chain_ids=${CHAIN_ID}&limit=${LIMIT}" \
  -H "X-Sim-Api-Key: ${SIM_API_KEY}" \
  -H "Accept: application/json" \
  > "${OUT}"

echo "Wrote ${OUT}" >&2
