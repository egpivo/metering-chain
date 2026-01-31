#!/usr/bin/env python3
"""Convert Helium rewards CSV to Metering Chain Consume NDJSON.

Expected CSV columns (configurable):
- hotspot (gateway address)
- amount (reward amount, decimal)

By default, each hotspot becomes a distinct service_id: "helium:<hotspot>".
"""

import argparse
import csv
import json
import sys
from decimal import Decimal, ROUND_FLOOR, getcontext
from typing import Optional


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Convert Helium rewards CSV to Metering Chain Consume NDJSON",
    )
    parser.add_argument("--input", help="CSV input path (default: stdin)")
    parser.add_argument("--output", help="NDJSON output path (default: stdout)")
    parser.add_argument("--owner", required=True, help="Owner address (payer)")
    parser.add_argument("--hotspot-col", default="hotspot", help="CSV column for hotspot address")
    parser.add_argument("--amount-col", default="amount", help="CSV column for reward amount")
    parser.add_argument(
        "--scale",
        type=int,
        default=1,
        help="Multiply amount by this integer scale to get units (e.g., 1e6)",
    )
    parser.add_argument(
        "--rounding",
        choices=["floor", "reject"],
        default="floor",
        help="How to handle fractional units after scaling",
    )
    parser.add_argument(
        "--service-id",
        default=None,
        help="If set, all consumes use this service_id",
    )
    parser.add_argument(
        "--service-prefix",
        default="helium",
        help="Prefix used when service-id is not set",
    )
    parser.add_argument(
        "--unit-price",
        type=int,
        default=1,
        help="UnitPrice for Consume pricing",
    )
    parser.add_argument(
        "--mode",
        choices=["kind", "unsigned"],
        default="kind",
        help="Output mode: kind-only or full unsigned SignedTx",
    )
    parser.add_argument(
        "--start-nonce",
        type=int,
        default=0,
        help="Starting nonce for unsigned SignedTx output",
    )
    parser.add_argument(
        "--aggregate",
        action="store_true",
        help="Aggregate by hotspot (sum amounts). Recommended.",
    )
    parser.add_argument(
        "--require-service-id",
        action="store_true",
        help="When using --aggregate, require --service-id so Consume matches a single OpenMeter (avoids apply failures).",
    )
    parser.add_argument(
        "--allow-zero",
        action="store_true",
        help="Allow zero-unit Consume entries (default: skip).",
    )
    return parser.parse_args()


def read_rows(path: Optional[str]) -> csv.DictReader:
    if path:
        f = open(path, "r", newline="")
    else:
        f = sys.stdin
    return csv.DictReader(f)


def scale_amount(amount_str: str, scale: int, rounding: str) -> int:
    getcontext().prec = 50
    amount = Decimal(amount_str)
    units = amount * Decimal(scale)
    if units == units.to_integral_value():
        return int(units)
    if rounding == "floor":
        return int(units.to_integral_value(rounding=ROUND_FLOOR))
    raise ValueError(f"Non-integer units after scaling: {units}")


def service_id_for_hotspot(prefix: str, hotspot: str) -> str:
    return f"{prefix}:{hotspot}"


def main() -> int:
    args = parse_args()

    if args.require_service_id and args.aggregate and args.service_id is None:
        sys.stderr.write(
            "Error: --require-service-id set with --aggregate but --service-id not set. "
            "Use --service-id <id> (e.g. helium-rewards) so Consume service_id matches your OpenMeter.\n"
        )
        return 1

    rows = read_rows(args.input)
    fieldnames = rows.fieldnames or []
    if args.hotspot_col not in fieldnames:
        raise SystemExit(
            f"Missing column '{args.hotspot_col}'. Columns: {', '.join(fieldnames)}"
        )
    if args.amount_col not in fieldnames:
        raise SystemExit(
            f"Missing column '{args.amount_col}'. Columns: {', '.join(fieldnames)}"
        )

    units_by_hotspot: dict[str, int] = {}
    items: list[tuple[str, int]] = []

    skipped_zero = 0
    for row in rows:
        hotspot = (row.get(args.hotspot_col) or "").strip()
        amount_str = (row.get(args.amount_col) or "").strip()
        if not hotspot or not amount_str:
            continue
        units = scale_amount(amount_str, args.scale, args.rounding)
        if units == 0 and not args.allow_zero:
            skipped_zero += 1
            continue
        if args.aggregate:
            units_by_hotspot[hotspot] = units_by_hotspot.get(hotspot, 0) + units
        else:
            items.append((hotspot, units))

    if args.aggregate:
        items = sorted(units_by_hotspot.items())

    out = open(args.output, "w") if args.output else sys.stdout
    nonce = args.start_nonce

    for hotspot, units in items:
        service_id = args.service_id or service_id_for_hotspot(args.service_prefix, hotspot)
        consume = {
            "owner": args.owner,
            "service_id": service_id,
            "units": units,
            "pricing": {"UnitPrice": args.unit_price},
        }
        kind = {"Consume": consume}
        if args.mode == "kind":
            out.write(json.dumps(kind))
        else:
            tx = {
                "signer": args.owner,
                "nonce": nonce,
                "kind": kind,
            }
            out.write(json.dumps(tx))
            nonce += 1
        out.write("\n")

    if args.output:
        out.close()
    if skipped_zero:
        print(
            f"Skipped {skipped_zero} rows with 0 units (use --allow-zero to include)",
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
