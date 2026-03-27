#!/usr/bin/env python3
from __future__ import annotations

import csv
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
LOCAL = ROOT / ".local"
OUT = LOCAL / "blog_artifacts"


FAIL_CLOSED_EXAMPLES = [
    {
        "failure": "Tampered signed payload",
        "reject_code": "SIGNATURE_VERIFICATION_FAILED",
        "source": "tests/security_abuse.rs::test_security_abuse_tampered_signed_payload_rejected",
        "why_it_matters": "Prevents signed economic events from being mutated after signing.",
    },
    {
        "failure": "Wrong signer / audience binding",
        "reject_code": "DELEGATION_AUDIENCE_SIGNER_MISMATCH",
        "source": "tests/security_abuse.rs::test_security_abuse_wrong_signer_audience_binding_rejected",
        "why_it_matters": "Prevents delegated authority from being replayed by the wrong actor.",
    },
    {
        "failure": "Snapshot cursor beyond tx log tip",
        "reject_code": "STATE_ERROR",
        "source": "tests/replay_recovery.rs::test_recovery_mismatched_snapshot_cursor_vs_log_returns_state_error",
        "why_it_matters": "Prevents corrupted persistence metadata from silently skipping transactions.",
    },
    {
        "failure": "Stale nonce / replayed application command",
        "reject_code": "INVALID_TRANSACTION (Nonce mismatch)",
        "source": "tests/cli_smoke.rs::test_cli_smoke_failure_stale_nonce_rejected",
        "why_it_matters": "Prevents replayed usage/commands from being accepted as fresh intent.",
    },
]


def ensure_out() -> None:
    OUT.mkdir(parents=True, exist_ok=True)


def parse_markdown_table(lines: list[str], header_startswith: str) -> list[dict[str, str]]:
    for idx, line in enumerate(lines):
        if line.startswith(header_startswith):
            header = [c.strip() for c in line.strip().strip("|").split("|")]
            rows: list[dict[str, str]] = []
            for row in lines[idx + 2 :]:
                if not row.startswith("|"):
                    break
                cols = [c.strip() for c in row.strip().strip("|").split("|")]
                if len(cols) != len(header):
                    continue
                rows.append(dict(zip(header, cols)))
            return rows
    raise RuntimeError(f"Could not find markdown table starting with: {header_startswith}")


def write_fail_closed_examples() -> None:
    md_path = OUT / "fail_closed_examples.md"
    json_path = OUT / "fail_closed_examples.json"

    with json_path.open("w", encoding="utf-8") as f:
        json.dump(FAIL_CLOSED_EXAMPLES, f, indent=2)
        f.write("\n")

    lines = [
        "# Fail-Closed Examples",
        "",
        "Curated from the current test suites.",
        "",
        "| Failure | Deterministic reject | Why it matters | Source |",
        "|---|---|---|---|",
    ]
    for item in FAIL_CLOSED_EXAMPLES:
        lines.append(
            f"| {item['failure']} | `{item['reject_code']}` | {item['why_it_matters']} | `{item['source']}` |"
        )
    md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def parse_perf_variance_review() -> list[dict[str, str]]:
    text = (LOCAL / "benchmark_baseline.md").read_text(encoding="utf-8")
    lines = text.splitlines()
    return parse_markdown_table(lines, "| Dataset | Replay ms (median/max) |")


def split_pair(text: str) -> tuple[float, float]:
    left, right = [part.strip() for part in text.split("/")]
    return float(left), float(right)


def write_perf_exports() -> None:
    rows = parse_perf_variance_review()
    csv_path = OUT / "perf_variance_review.csv"
    json_path = OUT / "perf_variance_review.json"

    normalized: list[dict[str, float | str]] = []
    for row in rows:
        replay_med, replay_max = split_pair(row["Replay ms (median/max)"])
        recompute_med, recompute_max = split_pair(row["Recompute ms (median/max)"])
        normalized.append(
            {
                "dataset": row["Dataset"],
                "replay_ms_median": replay_med,
                "replay_ms_max": replay_max,
                "replay_tx_per_sec_median": float(row["Replay tx/s median"]),
                "snapshot_restore_ms_median": float(row["Snapshot restore ms median"]),
                "recompute_ms_median": recompute_med,
                "recompute_ms_max": recompute_max,
            }
        )

    with csv_path.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "dataset",
                "replay_ms_median",
                "replay_ms_max",
                "replay_tx_per_sec_median",
                "snapshot_restore_ms_median",
                "recompute_ms_median",
                "recompute_ms_max",
            ],
        )
        writer.writeheader()
        writer.writerows(normalized)

    with json_path.open("w", encoding="utf-8") as f:
        json.dump(normalized, f, indent=2)
        f.write("\n")

    write_perf_latency_svg(normalized)
    write_perf_throughput_svg(normalized)


def write_perf_latency_svg(rows: list[dict[str, float | str]]) -> None:
    width = 520
    height = 240
    left = 60
    bottom = 30
    top = 20
    chart_h = height - top - bottom
    chart_w = width - left - 20
    max_val = max(
        max(float(r["replay_ms_median"]), float(r["recompute_ms_median"])) for r in rows
    )
    scale = chart_h / max(max_val, 1.0)
    gap = chart_w / max(len(rows), 1)
    bar_w = 24
    colors = {"replay": "#2F6BFF", "recompute": "#F26B38"}

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<style>text{font-family:monospace;font-size:12px;fill:#222}.label{font-size:11px}.title{font-size:14px;font-weight:bold}</style>',
        '<text class="title" x="20" y="18">Perf latency medians (ms)</text>',
        f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#999"/>',
        f'<line x1="{left}" y1="{height-bottom}" x2="{width-20}" y2="{height-bottom}" stroke="#999"/>',
    ]

    for idx, row in enumerate(rows):
        cx = left + gap * idx + gap / 2
        replay_h = float(row["replay_ms_median"]) * scale
        recompute_h = float(row["recompute_ms_median"]) * scale
        parts.append(
            f'<rect x="{cx - bar_w - 2:.1f}" y="{height-bottom-replay_h:.1f}" width="{bar_w}" height="{replay_h:.1f}" fill="{colors["replay"]}"/>'
        )
        parts.append(
            f'<rect x="{cx + 2:.1f}" y="{height-bottom-recompute_h:.1f}" width="{bar_w}" height="{recompute_h:.1f}" fill="{colors["recompute"]}"/>'
        )
        parts.append(
            f'<text class="label" x="{cx - 10:.1f}" y="{height-10}" text-anchor="middle">{row["dataset"]}</text>'
        )
    parts.extend(
        [
            f'<rect x="{width-170}" y="28" width="12" height="12" fill="{colors["replay"]}"/>',
            f'<text x="{width-150}" y="38">replay_ms</text>',
            f'<rect x="{width-90}" y="28" width="12" height="12" fill="{colors["recompute"]}"/>',
            f'<text x="{width-70}" y="38">recompute_ms</text>',
            "</svg>",
        ]
    )
    (OUT / "perf_latency.svg").write_text("\n".join(parts), encoding="utf-8")


def write_perf_throughput_svg(rows: list[dict[str, float | str]]) -> None:
    width = 520
    height = 220
    left = 60
    bottom = 30
    top = 20
    chart_h = height - top - bottom
    chart_w = width - left - 20
    max_val = max(float(r["replay_tx_per_sec_median"]) for r in rows)
    scale = chart_h / max(max_val, 1.0)
    gap = chart_w / max(len(rows), 1)
    bar_w = 38

    parts = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<style>text{font-family:monospace;font-size:12px;fill:#222}.label{font-size:11px}.title{font-size:14px;font-weight:bold}</style>',
        '<text class="title" x="20" y="18">Replay throughput medians (tx/s)</text>',
        f'<line x1="{left}" y1="{top}" x2="{left}" y2="{height-bottom}" stroke="#999"/>',
        f'<line x1="{left}" y1="{height-bottom}" x2="{width-20}" y2="{height-bottom}" stroke="#999"/>',
    ]
    for idx, row in enumerate(rows):
        cx = left + gap * idx + gap / 2
        bar_h = float(row["replay_tx_per_sec_median"]) * scale
        parts.append(
            f'<rect x="{cx - bar_w/2:.1f}" y="{height-bottom-bar_h:.1f}" width="{bar_w}" height="{bar_h:.1f}" fill="#1E9E63"/>'
        )
        parts.append(
            f'<text class="label" x="{cx:.1f}" y="{height-10}" text-anchor="middle">{row["dataset"]}</text>'
        )
    parts.append("</svg>")
    (OUT / "perf_throughput.svg").write_text("\n".join(parts), encoding="utf-8")


def count_tests_in_rust(path: Path) -> int:
    text = path.read_text(encoding="utf-8")
    return len(re.findall(r"(?m)^\s*#\[test\]", text))


def count_tests_in_ts(path: Path) -> int:
    text = path.read_text(encoding="utf-8")
    return len(re.findall(r"\bit\s*\(", text))


def write_test_layers() -> None:
    rust_files = [
        ("Security / abuse", ROOT / "tests" / "security_abuse.rs"),
        ("Property / invariants", ROOT / "tests" / "property_invariants.rs"),
        ("Recovery", ROOT / "tests" / "replay_recovery.rs"),
        ("Compatibility", ROOT / "tests" / "compatibility.rs"),
        ("CLI smoke", ROOT / "tests" / "cli_smoke.rs"),
        ("Perf smoke", ROOT / "tests" / "perf_smoke.rs"),
    ]
    ts_files = [
        ("Frontend page tests", ROOT / "frontend" / "src" / "pages" / "DemoPhase4Page.test.tsx"),
        ("Frontend page tests", ROOT / "frontend" / "src" / "pages" / "SettlementsPage.test.tsx"),
        ("Frontend page tests", ROOT / "frontend" / "src" / "pages" / "DisputesPage.test.tsx"),
        ("Frontend page tests", ROOT / "frontend" / "src" / "pages" / "SettlementDetailPage.test.tsx"),
        ("Frontend page tests", ROOT / "frontend" / "src" / "pages" / "AuditDataPage.test.tsx"),
        ("Frontend page tests", ROOT / "frontend" / "src" / "pages" / "PolicyPage.test.tsx"),
        ("Frontend perf visibility", ROOT / "frontend" / "src" / "adapters" / "snapshot-frontend-adapter.perf.test.ts"),
    ]

    layer_counts: dict[str, int] = {}
    layer_files: dict[str, list[str]] = {}
    for layer, path in rust_files:
        if path.exists():
            layer_counts[layer] = layer_counts.get(layer, 0) + count_tests_in_rust(path)
            layer_files.setdefault(layer, []).append(path.name)
    for layer, path in ts_files:
        if path.exists():
            layer_counts[layer] = layer_counts.get(layer, 0) + count_tests_in_ts(path)
            layer_files.setdefault(layer, []).append(path.name)

    lines = [
        "# Test Layers",
        "",
        "| Layer | Files | Test count |",
        "|---|---|---:|",
    ]
    for layer in sorted(layer_counts.keys()):
        lines.append(
            f"| {layer} | {', '.join(layer_files[layer])} | {layer_counts[layer]} |"
        )
    (OUT / "test_layers.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> None:
    ensure_out()
    write_fail_closed_examples()
    write_perf_exports()
    write_test_layers()
    print(f"Wrote blog artifacts into {OUT}")


if __name__ == "__main__":
    main()
