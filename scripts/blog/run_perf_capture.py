#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import statistics
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / ".local" / "blog_artifacts"
LINE_RE = re.compile(
    r"\[perf_smoke\]\s+dataset=(\w+)\s+txs=(\d+)\s+replay_ms=(\d+)\s+"
    r"replay_tx_per_sec=([0-9.]+)\s+snapshot_restore_ms=(\d+)\s+recompute_ms=(\d+)"
)


def run_once() -> dict[str, dict[str, float]]:
    proc = subprocess.run(
        ["cargo", "test", "--test", "perf_smoke", "--", "--nocapture"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    combined = proc.stdout + "\n" + proc.stderr
    metrics: dict[str, dict[str, float]] = {}
    for line in combined.splitlines():
        m = LINE_RE.search(line)
        if not m:
            continue
        ds = m.group(1)
        metrics[ds] = {
            "txs": int(m.group(2)),
            "replay_ms": int(m.group(3)),
            "replay_tx_per_sec": float(m.group(4)),
            "snapshot_restore_ms": int(m.group(5)),
            "recompute_ms": int(m.group(6)),
        }
    if not metrics:
        raise RuntimeError("No perf_smoke metrics parsed from output")
    return metrics


def aggregate(runs: list[dict[str, dict[str, float]]]) -> dict[str, dict[str, float]]:
    datasets = sorted(runs[0].keys())
    out: dict[str, dict[str, float]] = {}
    for ds in datasets:
        replay_vals = [run[ds]["replay_ms"] for run in runs]
        throughput_vals = [run[ds]["replay_tx_per_sec"] for run in runs]
        snapshot_vals = [run[ds]["snapshot_restore_ms"] for run in runs]
        recompute_vals = [run[ds]["recompute_ms"] for run in runs]
        out[ds] = {
            "replay_ms_median": statistics.median(replay_vals),
            "replay_ms_max": max(replay_vals),
            "replay_tx_per_sec_median": statistics.median(throughput_vals),
            "snapshot_restore_ms_median": statistics.median(snapshot_vals),
            "recompute_ms_median": statistics.median(recompute_vals),
            "recompute_ms_max": max(recompute_vals),
        }
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--runs", type=int, default=5, help="Number of perf_smoke runs")
    args = parser.parse_args()

    OUT.mkdir(parents=True, exist_ok=True)
    raw_runs = [run_once() for _ in range(args.runs)]
    aggregated = aggregate(raw_runs)

    raw_path = OUT / "perf_capture_runs.json"
    summary_path = OUT / "perf_capture_summary.json"
    md_path = OUT / "perf_capture_summary.md"

    raw_path.write_text(json.dumps(raw_runs, indent=2) + "\n", encoding="utf-8")
    summary_path.write_text(json.dumps(aggregated, indent=2) + "\n", encoding="utf-8")

    lines = [
        "# Perf Capture Summary",
        "",
        f"Runs: {args.runs}",
        "",
        "| Dataset | Replay ms (median/max) | Replay tx/s median | Snapshot restore ms median | Recompute ms (median/max) |",
        "|---|---:|---:|---:|---:|",
    ]
    for ds in sorted(aggregated.keys()):
        row = aggregated[ds]
        lines.append(
            f"| {ds} | {row['replay_ms_median']} / {row['replay_ms_max']} | "
            f"{row['replay_tx_per_sec_median']:.2f} | {row['snapshot_restore_ms_median']} | "
            f"{row['recompute_ms_median']} / {row['recompute_ms_max']} |"
        )
    md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    print(f"Wrote {raw_path}")
    print(f"Wrote {summary_path}")
    print(f"Wrote {md_path}")


if __name__ == "__main__":
    main()
