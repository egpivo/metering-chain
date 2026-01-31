#!/usr/bin/env python3
"""Analyze Helium rewards CSV and produce charts + summary.

Outputs:
- analysis.png (2x2 grid)
- summary.json
"""

import argparse
import json
from datetime import datetime
from typing import Optional

import polars as pl
import matplotlib.pyplot as plt

try:
    import seaborn as sns

    _HAVE_SEABORN = True
except Exception:
    _HAVE_SEABORN = False


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Analyze rewards CSV (multi-operator demo)")
    p.add_argument("--input", required=True, help="Input CSV path")
    p.add_argument("--output-image", default="analysis.png", help="Output PNG path")
    p.add_argument("--output-summary", default="summary.json", help="Output summary JSON path")
    p.add_argument("--operator-col", default="to_owner", help="Operator column")
    p.add_argument("--amount-col", default="amount", help="Amount column (units)")
    p.add_argument("--time-col", default="block_time", help="Time column (optional)")
    p.add_argument("--top-n", type=int, default=10, help="Top operators to show")
    p.add_argument(
        "--operator-label-len",
        type=int,
        default=6,
        help="Short label length for operator IDs (for plots)",
    )
    return p.parse_args()


def to_date_series(df: pl.DataFrame, time_col: str) -> Optional[pl.Series]:
    if time_col not in df.columns:
        return None
    try:
        s = df[time_col]
        if s.dtype == pl.Utf8:
            s = (
                s.str.replace(" UTC", "")
                .str.strptime(pl.Datetime, strict=False)
            )
        return s.dt.date()
    except Exception:
        return None


def main() -> int:
    args = parse_args()

    if _HAVE_SEABORN:
        sns.set_theme(style="whitegrid", font_scale=0.9)
        palette = sns.color_palette("viridis", 8)
    else:
        palette = None

    df = pl.read_csv(args.input)
    if args.operator_col not in df.columns:
        raise SystemExit(f"Missing operator column: {args.operator_col}")
    if args.amount_col not in df.columns:
        raise SystemExit(f"Missing amount column: {args.amount_col}")

    df = df.with_columns(
        pl.col(args.amount_col).cast(pl.Float64).alias("amount")
    )

    total_rows = df.height
    total_units = df["amount"].sum()
    distinct_ops = df[args.operator_col].n_unique()

    date_series = to_date_series(df, args.time_col)
    if date_series is not None:
        df = df.with_columns(date_series.alias("date"))
        per_day = (
            df.group_by("date")
            .agg([
                pl.len().alias("txs"),
                pl.col("amount").sum().alias("units"),
            ])
            .sort("date")
        )
        date_min = per_day["date"].min()
        date_max = per_day["date"].max()
    else:
        per_day = None
        date_min = None
        date_max = None

    top_ops = (
        df.group_by(args.operator_col)
        .agg(pl.col("amount").sum().alias("units"))
        .sort("units", descending=True)
        .head(args.top_n)
    )

    # Concentration metrics
    totals = (
        df.group_by(args.operator_col)
        .agg(pl.col("amount").sum().alias("units"))
        .sort("units", descending=True)
    )
    total_units_all = float(totals["units"].sum()) if totals.height > 0 else 0.0
    top10_units = float(totals.head(10)["units"].sum()) if totals.height > 0 else 0.0
    top10_share = (top10_units / total_units_all) if total_units_all > 0 else 0.0

    # HHI (sum of squared shares)
    shares = (totals["units"] / total_units_all) if total_units_all > 0 else None
    hhi = float((shares * shares).sum()) if shares is not None else 0.0

    # Gini (on totals per operator)
    def gini(values: list[float]) -> float:
        if not values:
            return 0.0
        vals = sorted(values)
        n = len(vals)
        cum = 0.0
        for i, v in enumerate(vals, start=1):
            cum += i * v
        return (2 * cum) / (n * sum(vals)) - (n + 1) / n

    gini_val = gini(totals["units"].to_list()) if totals.height > 0 else 0.0

    # Short labels for plotting
    def short_label(s: str) -> str:
        if args.operator_label_len <= 0:
            return s
        return s[: args.operator_label_len] + "…"

    # Build plots
    fig, axes = plt.subplots(2, 2, figsize=(11, 6.5))
    ax1, ax2, ax3, ax4 = axes.flatten()

    if per_day is not None and per_day.height > 0:
        x_dates = per_day["date"].to_list()
        y_txs = per_day["txs"].to_list()
        y_units = per_day["units"].to_list()
        if _HAVE_SEABORN:
            sns.lineplot(x=x_dates, y=y_txs, ax=ax1, marker="o", color=palette[2])
            sns.lineplot(x=x_dates, y=y_units, ax=ax2, marker="o", color=palette[4])
        else:
            ax1.plot(x_dates, y_txs, marker="o")
            ax2.plot(x_dates, y_units, marker="o")
        ax1.set_title("Transfers per day")
        ax1.tick_params(axis="x", rotation=30)

        ax2.set_title("Total units per day")
        ax2.tick_params(axis="x", rotation=30)
    else:
        ax1.set_title("Transfers per day (n/a)")
        ax2.set_title("Total units per day (n/a)")

    labels = [short_label(x) for x in top_ops[args.operator_col].to_list()]
    values = top_ops["units"].to_list()
    if _HAVE_SEABORN:
        sns.barplot(
            x=values,
            y=labels,
            order=labels,
            ax=ax3,
            color=palette[5],
        )
    else:
        ax3.barh(labels, values)
        ax3.invert_yaxis()
    ax3.set_title(f"Top {args.top_n} operators by units")

    # log10 distribution
    import math
    values = [v for v in df["amount"].to_list() if v > 0]
    if values:
        log_vals = [math.log10(v) for v in values]
        if _HAVE_SEABORN:
            sns.histplot(log_vals, bins=20, ax=ax4, color=palette[6], kde=True)
        else:
            ax4.hist(log_vals, bins=20)
        ax4.set_title("Transfer size distribution (log10 units)")
    else:
        ax4.set_title("Transfer size distribution (n/a)")

    fig.tight_layout()
    fig.savefig(args.output_image, dpi=160)

    summary = {
        "rows": total_rows,
        "distinct_operators": int(distinct_ops),
        "total_units": float(total_units) if total_units is not None else 0.0,
        "top10_share": top10_share,
        "hhi": hhi,
        "gini": gini_val,
        "time_window": {
            "start": str(date_min) if date_min is not None else None,
            "end": str(date_max) if date_max is not None else None,
        },
        "top_operators": [
            {
                args.operator_col: row[0],
                "short": short_label(str(row[0])),
                "units": float(row[1]),
            }
            for row in top_ops.iter_rows()
        ],
    }

    with open(args.output_summary, "w") as f:
        json.dump(summary, f, indent=2)

    print(f"Wrote {args.output_image} and {args.output_summary}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
