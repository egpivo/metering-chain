#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

if [[ -f .env ]]; then
  set -a
  # shellcheck source=/dev/null
  source .env
  set +a
fi

if [[ -z "${DUNE_API_KEY:-}" ]]; then
  echo "DUNE_API_KEY is missing. Put it in .env or export it first."
  exit 1
fi

CSV_OUT="${1:-.local/demo_data/helium_rewards_latest.csv}"
JSON_OUT="${2:-frontend/public/demo_data/phase4_snapshot.json}"
DAYS="${DUNE_DAYS:-14}"
LIMIT="${DUNE_LIMIT:-50000}"
SERVICE_ID="${DEMO_SERVICE_ID:-helium-iot}"
# Optional: START_DATE and END_DATE (YYYY-MM-DD) for range 2026-01-01 to now
START_DATE="${START_DATE:-}"
END_DATE="${END_DATE:-}"
MAX_WINDOWS="${MAX_WINDOWS:-2000}"

mkdir -p "$(dirname "$CSV_OUT")"
mkdir -p "$(dirname "$JSON_OUT")"

if [[ -n "$START_DATE" && -n "$END_DATE" ]]; then
  python examples/multi_operator/fetch_dune_iot_transfers.py \
    --start-date "$START_DATE" \
    --end-date "$END_DATE" \
    --limit "$LIMIT" \
    --output "$CSV_OUT"
else
  python examples/multi_operator/fetch_dune_iot_transfers.py \
    --days "$DAYS" \
    --limit "$LIMIT" \
    --output "$CSV_OUT"
fi

python frontend/server/build_phase4_snapshot.py \
  --input "$CSV_OUT" \
  --output "$JSON_OUT" \
  --service-id "$SERVICE_ID" \
  --max-windows "$MAX_WINDOWS"

cp "$JSON_OUT" .local/demo_data/phase4_snapshot.json

echo "Done."
echo "CSV:  $CSV_OUT"
echo "JSON: $JSON_OUT"
echo "Now run: cd frontend && npm run dev"
