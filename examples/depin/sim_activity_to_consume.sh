#!/usr/bin/env bash
set -euo pipefail

IN="${1:-/dev/stdin}"

: "${TOKEN_ADDRESS:?TOKEN_ADDRESS is required}"
: "${SIGNER:?SIGNER is required}"

OWNER="${OWNER:-$SIGNER}"
SERVICE_ID="${SERVICE_ID:-dimo-rewards}"
START_NONCE="${START_NONCE:-1}"
UNIT_PRICE="${UNIT_PRICE:-1}"

jq -c \
  --arg signer "$SIGNER" \
  --arg owner "$OWNER" \
  --arg service_id "$SERVICE_ID" \
  --arg token "$TOKEN_ADDRESS" \
  --argjson start_nonce "$START_NONCE" \
  --argjson unit_price "$UNIT_PRICE" \
  '
  .activity
  | map(select(
      (.type == "send") and
      (.asset_type == "erc20") and
      ((.token_address | ascii_downcase) == ($token | ascii_downcase))
    ))
  | sort_by(.block_time)
  | to_entries
  | .[]
  | {
      signer: $signer,
      nonce: ($start_nonce + .key),
      kind: {
        Consume: {
          owner: $owner,
          service_id: $service_id,
          units: (.value | tonumber),
          pricing: { UnitPrice: $unit_price }
        }
      }
    }
  ' "$IN"
