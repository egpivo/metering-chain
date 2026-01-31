#!/usr/bin/env python3
"""Fetch Helium IOT token transfers from Dune via spice and write CSV.

Requires:
- DUNE_API_KEY in environment (e.g., from .env)
- dune_spice installed
"""

import argparse
import os
import sys

try:
    import spice
except Exception as exc:
    raise SystemExit("spice not installed; run: pip install dune_spice") from exc

IOT_MINT = "iotEVVZLEywoTn1QdwNPddxPWszn3zFhEot3MfL9fns"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fetch Helium IOT transfers from Dune (tokens_solana.transfers)",
    )
    parser.add_argument("--output", required=True, help="Output CSV path")
    parser.add_argument("--days", type=int, default=7, help="Lookback window in days")
    parser.add_argument("--start-date", help="Start date (YYYY-MM-DD), overrides --days")
    parser.add_argument("--end-date", help="End date (YYYY-MM-DD), overrides --days")
    parser.add_argument("--limit", type=int, default=5000, help="Max rows to fetch")
    parser.add_argument(
        "--mint",
        default=IOT_MINT,
        help="Token mint address (default: Helium IOT mint)",
    )
    parser.add_argument(
        "--table",
        default="tokens_solana.transfers",
        help="Dune table to query",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not os.environ.get("DUNE_API_KEY"):
        raise SystemExit("Missing DUNE_API_KEY in environment")

    if args.start_date and args.end_date:
        time_clause = (
            f"block_time >= timestamp '{args.start_date}' "
            f"AND block_time < timestamp '{args.end_date}'"
        )
    else:
        time_clause = f"block_time >= now() - interval '{args.days}' day"

    q = f"""
SELECT
  block_time,
  to_owner,
  amount,
  token_mint_address
FROM {args.table}
WHERE token_mint_address = '{args.mint}'
  AND action = 'transfer'
  AND {time_clause}
ORDER BY block_time DESC
LIMIT {args.limit}
"""

    df = spice.query(q, refresh=True)
    df.write_csv(args.output)
    print(f"Wrote {df.height} rows to {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
