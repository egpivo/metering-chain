#!/usr/bin/env python3
"""Generate an extended Phase 4 demo snapshot (2026-01-01 to today) without Dune.

Use when you want more demo data for GitHub/frontend without DUNE_API_KEY.
Output schema matches build_phase4_snapshot.py for frontend DemoSnapshotAdapter.
"""

from __future__ import annotations

import hashlib
import json
import random
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

random.seed(42)


def sha12(s: str) -> str:
    return hashlib.sha256(s.encode("utf-8")).hexdigest()[:12]


def seed_int(s: str) -> int:
    return int(hashlib.sha256(s.encode()).hexdigest()[:16], 16)


# Deterministic but varied owner-like IDs (base58 style, 44 chars)
def owner_id(seed: int) -> str:
    r = random.Random(seed)
    chars = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    return "".join(r.choices(chars, k=44))


def main() -> int:
    out_path = Path(__file__).resolve().parent.parent / "public" / "demo_data" / "phase4_snapshot.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)

    start = date(2026, 1, 1)
    end = date(2026, 2, 22)  # through Feb 22
    service_id = "helium-iot"
    num_owners = 80
    windows_per_day = 85  # (day, owner) combos per day -> ~53 * 85 ≈ 4500 windows

    windows: list[dict] = []
    tx_cursor = 0
    day_count = (end - start).days + 1

    for day_offset in range(day_count):
        d = start + timedelta(days=day_offset)
        window_id = d.strftime("%Y-%m-%d")
        from_ts = datetime(d.year, d.month, d.day, tzinfo=timezone.utc)
        to_ts = from_ts + timedelta(days=1)
        from_ts_s = from_ts.isoformat().replace("+00:00", "Z")
        to_ts_s = to_ts.isoformat().replace("+00:00", "Z")

        # Vary operator_count per day for realism
        operator_count = 70 + (day_offset * 2) % 50
        day_total_seed = seed_int(window_id + ":0")
        day_total = 50_000_000_000 + (day_total_seed % 100_000_000_000)

        for i in range(windows_per_day):
            seed = seed_int(f"{window_id}:{i}")
            owner = owner_id(seed)
            gross = 100_000_000 + (seed % 900_000_000_000)
            if gross > day_total:
                day_total = gross + 1
            operator_share = int(gross * 0.9)
            protocol_fee = int(gross * 0.1)
            reserve_locked = 0
            top_n_share = round((gross / day_total) * 100, 2) if day_total else 0

            tx_count = max(1, (seed % 20) + 1)
            from_tx_id = tx_cursor
            to_tx_id = tx_cursor + tx_count
            tx_cursor += tx_count

            base = f"{owner}|{service_id}|{window_id}|{from_tx_id}|{to_tx_id}|{gross}"
            evidence_hash = sha12("evidence:" + base)

            # Mix of statuses: ~5% Finalized with replay, ~5% Disputed (no replay), ~5% Disputed (mismatch), rest Proposed
            idx = len(windows)
            if idx % 20 == 0:
                status = "Finalized"
                replay_summary = {
                    "from_tx_id": from_tx_id,
                    "to_tx_id": to_tx_id,
                    "tx_count": tx_count,
                    "gross_spent": gross,
                    "operator_share": operator_share,
                    "protocol_fee": protocol_fee,
                    "reserve_locked": reserve_locked,
                }
                replay_hash = sha12("replay:" + json.dumps(replay_summary, sort_keys=True))
            elif idx % 20 == 1:
                status = "Disputed"
                replay_summary = None
                replay_hash = None
            elif idx % 20 == 2:
                status = "Disputed"
                replay_summary = {
                    "from_tx_id": from_tx_id,
                    "to_tx_id": to_tx_id,
                    "tx_count": tx_count,
                    "gross_spent": max(0, gross - 1),
                    "operator_share": max(0, operator_share - 1),
                    "protocol_fee": protocol_fee,
                    "reserve_locked": reserve_locked,
                }
                replay_hash = sha12("replay:" + json.dumps(replay_summary, sort_keys=True))
            else:
                status = "Proposed"
                replay_summary = None
                replay_hash = None

            windows.append({
                "owner": owner,
                "service_id": service_id,
                "window_id": window_id,
                "from_ts": from_ts_s,
                "to_ts": to_ts_s,
                "gross_spent": gross,
                "operator_share": operator_share,
                "protocol_fee": protocol_fee,
                "reserve_locked": reserve_locked,
                "top_n_share": top_n_share,
                "operator_count": operator_count,
                "from_tx_id": from_tx_id,
                "to_tx_id": to_tx_id,
                "evidence_hash": evidence_hash,
                "status": status,
                "replay_summary": replay_summary,
                "replay_hash": replay_hash,
            })

    payload = {
        "version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "windows": windows,
    }
    out_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    size_mb = out_path.stat().st_size / (1024 * 1024)
    print(f"Wrote {len(windows)} windows to {out_path} ({size_mb:.2f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
