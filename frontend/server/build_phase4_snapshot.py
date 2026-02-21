#!/usr/bin/env python3
"""Build frontend Phase 4 demo snapshot JSON from Dune transfer CSV.

Input CSV expected columns (default names):
- block_time (ISO timestamp)
- to_owner (owner/operator id)
- amount (numeric string)

Output JSON shape matches frontend DemoSnapshotAdapter expectations.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import hashlib
import json
from collections import defaultdict
from dataclasses import dataclass
from decimal import Decimal, ROUND_FLOOR
from pathlib import Path
from typing import Dict, List, Tuple


@dataclass
class Row:
    block_time: dt.datetime
    owner: str
    amount_units: int


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Build phase4 demo snapshot from CSV")
    p.add_argument("--input", required=True, help="Input CSV from fetch_dune_iot_transfers.py")
    p.add_argument("--output", required=True, help="Output JSON path (e.g. frontend/public/demo_data/phase4_snapshot.json)")
    p.add_argument("--service-id", default="helium-iot", help="service_id to use for demo windows")
    p.add_argument("--owner-col", default="to_owner", help="Owner column in CSV")
    p.add_argument("--time-col", default="block_time", help="Timestamp column in CSV")
    p.add_argument("--amount-col", default="amount", help="Amount column in CSV")
    p.add_argument("--amount-scale", type=int, default=1, help="Multiply amount by this integer before flooring to units")
    p.add_argument("--min-window-units", type=int, default=1, help="Drop windows below this gross_spent")
    p.add_argument("--max-windows", type=int, default=80, help="Cap windows in output for demo stability")
    return p.parse_args()


def parse_time(raw: str) -> dt.datetime:
    s = raw.strip()
    if s.endswith("Z"):
        s = s[:-1] + "+00:00"
    elif s.endswith(" UTC"):
        s = s[:-4] + "+00:00"
    t = dt.datetime.fromisoformat(s)
    if t.tzinfo is None:
        t = t.replace(tzinfo=dt.timezone.utc)
    return t.astimezone(dt.timezone.utc)


def to_units(amount_str: str, scale: int) -> int:
    val = (Decimal(amount_str) * Decimal(scale)).to_integral_value(rounding=ROUND_FLOOR)
    return int(val)


def load_rows(path: Path, owner_col: str, time_col: str, amount_col: str, scale: int) -> List[Row]:
    out: List[Row] = []
    with path.open("r", newline="") as f:
        reader = csv.DictReader(f)
        for r in reader:
            owner = (r.get(owner_col) or "").strip()
            raw_t = (r.get(time_col) or "").strip()
            raw_amount = (r.get(amount_col) or "").strip()
            if not owner or not raw_t or not raw_amount:
                continue
            try:
                t = parse_time(raw_t)
                units = to_units(raw_amount, scale)
            except Exception:
                continue
            if units <= 0:
                continue
            out.append(Row(block_time=t, owner=owner, amount_units=units))
    return out


def day_id(t: dt.datetime) -> str:
    return t.strftime("%Y-%m-%d")


def sha12(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()[:12]


def build_snapshot(rows: List[Row], service_id: str, min_window_units: int, max_windows: int) -> Dict:
    # group by day + owner
    by_window: Dict[Tuple[str, str], List[Row]] = defaultdict(list)
    # day totals for concentration
    by_day_total: Dict[str, int] = defaultdict(int)
    day_owners: Dict[str, set] = defaultdict(set)

    for r in rows:
        d = day_id(r.block_time)
        by_window[(d, r.owner)].append(r)
        by_day_total[d] += r.amount_units
        day_owners[d].add(r.owner)

    windows = []
    for (d, owner), rs in by_window.items():
        gross = sum(x.amount_units for x in rs)
        if gross < min_window_units:
            continue
        day_total = max(1, by_day_total[d])
        top_share = (gross / day_total) * 100.0
        op_count = len(day_owners[d])

        from_ts = dt.datetime.strptime(d, "%Y-%m-%d").replace(tzinfo=dt.timezone.utc)
        to_ts = from_ts + dt.timedelta(days=1)

        windows.append(
            {
                "owner": owner,
                "service_id": service_id,
                "window_id": d,
                "from_ts": from_ts.isoformat().replace("+00:00", "Z"),
                "to_ts": to_ts.isoformat().replace("+00:00", "Z"),
                "gross_spent": gross,
                "operator_share": int(gross * 0.9),
                "protocol_fee": gross - int(gross * 0.9),
                "reserve_locked": 0,
                "top_n_share": round(top_share, 2),
                "operator_count": op_count,
                "_tx_count": len(rs),
            }
        )

    windows.sort(key=lambda w: (w["gross_spent"], w["window_id"], w["owner"]), reverse=True)
    windows = windows[:max_windows]

    # assign synthetic tx ranges and evidence/replay status for demo scenes
    tx_cursor = 0
    for idx, w in enumerate(windows):
        tx_count = max(1, int(w.pop("_tx_count")))
        w["from_tx_id"] = tx_cursor
        w["to_tx_id"] = tx_cursor + tx_count
        tx_cursor += tx_count

        base = f"{w['owner']}|{w['service_id']}|{w['window_id']}|{w['from_tx_id']}|{w['to_tx_id']}|{w['gross_spent']}"
        w["evidence_hash"] = sha12("evidence:" + base)

        # deterministic demo statuses
        if idx == 0:
            # MATCH: finalized + replay fields consistent
            w["status"] = "Finalized"
            w["replay_summary"] = {
                "from_tx_id": w["from_tx_id"],
                "to_tx_id": w["to_tx_id"],
                "tx_count": tx_count,
                "gross_spent": w["gross_spent"],
                "operator_share": w["operator_share"],
                "protocol_fee": w["protocol_fee"],
                "reserve_locked": w["reserve_locked"],
            }
            w["replay_hash"] = sha12("replay:" + json.dumps(w["replay_summary"], sort_keys=True))
        elif idx == 1:
            # MISSING: disputed but no replay
            w["status"] = "Disputed"
            w["replay_summary"] = None
            w["replay_hash"] = None
        elif idx == 2:
            # MISMATCH: disputed with replay that does not match gross
            w["status"] = "Disputed"
            mismatch_summary = {
                "from_tx_id": w["from_tx_id"],
                "to_tx_id": w["to_tx_id"],
                "tx_count": tx_count,
                "gross_spent": max(0, w["gross_spent"] - 1),
                "operator_share": max(0, w["operator_share"] - 1),
                "protocol_fee": w["protocol_fee"],
                "reserve_locked": w["reserve_locked"],
            }
            w["replay_summary"] = mismatch_summary
            w["replay_hash"] = sha12("replay:" + json.dumps(mismatch_summary, sort_keys=True))
        else:
            w["status"] = "Proposed"
            w["replay_summary"] = None
            w["replay_hash"] = None

    usage_rows = [
        {
            "ts": r.block_time.isoformat().replace("+00:00", "Z"),
            "owner": r.owner,
            "service_id": service_id,
            "operator": r.owner,
            "units": r.amount_units,
            "cost": r.amount_units,
            "tx_ref": sha12(f"tx:{r.owner}:{r.block_time.isoformat()}:{r.amount_units}"),
        }
        for r in rows[:1000]
    ]

    return {
        "version": 1,
        "generated_at": dt.datetime.now(dt.timezone.utc).isoformat().replace("+00:00", "Z"),
        "windows": windows,
        "usage_rows": usage_rows,
    }


def main() -> int:
    args = parse_args()
    rows = load_rows(Path(args.input), args.owner_col, args.time_col, args.amount_col, args.amount_scale)
    if not rows:
        raise SystemExit("No valid rows parsed from input CSV")
    snapshot = build_snapshot(rows, args.service_id, args.min_window_units, args.max_windows)

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(snapshot, indent=2), encoding="utf-8")

    # keep .local mirror for inspection/blog assets if writing frontend/public path
    if "frontend/public/demo_data/phase4_snapshot.json" in str(out_path):
        local_mirror = Path(".local/demo_data/phase4_snapshot.json")
        local_mirror.parent.mkdir(parents=True, exist_ok=True)
        local_mirror.write_text(json.dumps(snapshot, indent=2), encoding="utf-8")

    print(f"Wrote snapshot with {len(snapshot['windows'])} windows to {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
