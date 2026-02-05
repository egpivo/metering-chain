#!/usr/bin/env bash
# Load .env and run Dune fetch + analyze_rewards (viz). Run from repo root.
# Requires: DUNE_API_KEY in .env, pip install -r examples/multi_operator/requirements.txt
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
DEMO_DIR="$(dirname "${BASH_SOURCE[0]}")"
MULTI_OP="$REPO_ROOT/examples/multi_operator"

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  source "$REPO_ROOT/.env"
  set +a
fi
if [[ -z "${DUNE_API_KEY:-}" ]]; then
  echo "DUNE_API_KEY not set. Add to .env or export."
  exit 1
fi
python -c "import spice" 2>/dev/null || {
  echo "dune_spice not installed in this Python. Run: pip install dune_spice"
  exit 1
}

DAYS="${DUNE_DAYS:-7}"
LIMIT="${DUNE_LIMIT:-2000}"
CSV="${HELIUM_REWARDS_CSV:-$DEMO_DIR/helium_rewards.csv}"

echo "Fetching Dune IOT transfers (days=$DAYS, limit=$LIMIT)..."
python "$MULTI_OP/fetch_dune_iot_transfers.py" \
  --days "$DAYS" \
  --limit "$LIMIT" \
  --output "$CSV"

echo "Analyzing and generating viz..."
python "$MULTI_OP/analyze_rewards.py" \
  --input "$CSV" \
  --operator-col to_owner \
  --amount-col amount \
  --output-image "$DEMO_DIR/helium_analysis.png" \
  --output-summary "$DEMO_DIR/helium_summary.json" \
  --top-n 12

echo "Done. Viz: $DEMO_DIR/helium_analysis.png | Summary: $DEMO_DIR/helium_summary.json"
