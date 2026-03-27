# Benchmark Baseline Policy (Draft)

Date (run capture): 2026-02-10  
Date (doc added to current workstream): 2026-03-27  
Scope: local perf baselines for replay path and settlement-window recompute.

## Purpose

- Define a repeatable way to capture baseline performance before release.
- Track drift across branches/releases without turning perf into a hard CI gate yet.
- Prepare thresholds that can be promoted to release checks after variance review.

## Harness

- Test file: `tests/perf_smoke.rs`
- Command: `cargo test --test perf_smoke -- --nocapture`
- Metrics emitted per dataset:
  - replay latency (`replay_ms`)
  - replay throughput (`replay_tx_per_sec`)
  - snapshot restore latency (`snapshot_restore_ms`)
  - recompute latency (`recompute_ms`)

## Dataset tiers

- Small: 100 consume txs (+ mint/open envelope)
- Medium: 500 consume txs (+ mint/open envelope)
- Large: 1000 consume txs (+ mint/open envelope)

## Capture protocol

1. Run locally with minimal background load.
2. Execute the harness at least 3 times for quick baseline checks.
3. For variance review / threshold discussion, execute 5 runs.
4. Record median value for each metric/dataset.
5. Keep raw logs for reproducibility (copy terminal output into release notes or PR comment).

## Baseline table (observed medians)

| Dataset | Replay ms | Replay tx/s | Snapshot restore ms | Recompute ms |
|---|---:|---:|---:|---:|
| Small | 0 | 149963.39 | 0 | 1 |
| Medium | 2 | 176325.96 | 1 | 6 |
| Large | 5 | 180497.19 | 2 | 11 |

Reference run set provenance: values were backfilled from local runs captured on 2026-02-10
and then recorded in this workstream on 2026-03-27.

## Policy now (Phase 5 first pass)

- Reporting-only: no hard fail in CI.
- A change is "perf-risk" when:
  - replay latency regresses > 20% in two or more datasets, or
  - recompute latency regresses > 25% for medium/large datasets.
- Perf-risk changes require:
  - explicit note in PR ("expected regression" or mitigation plan), and
  - follow-up issue if regression is accepted temporarily.

## Promotion path to hard gates

- After collecting at least 5 runs across different days/machines:
  - set conservative lower-bound throughput and upper-bound latencies.
  - gate only large-dataset replay/recompute first.
  - keep small/medium as warning-only until variance stabilizes.

## Variance review (Batch J first pass)

Run set:
- Date: 2026-03-27
- Command: `cargo test --test perf_smoke -- --nocapture`
- Runs: 5 consecutive local runs on the current branch

Observed medians / max:

| Dataset | Replay ms (median/max) | Replay tx/s median | Snapshot restore ms median | Recompute ms (median/max) |
|---|---:|---:|---:|---:|
| Small | 0 / 0 | 148706.26 | 0 | 1 / 1 |
| Medium | 2 / 2 | 186262.23 | 1 | 5 / 6 |
| Large | 5 / 5 | 190462.61 | 2 | 12 / 12 |

Decision (first pass):
- Keep CI in `warning-only` mode for perf.
- Promote to hard gate only for `large` dataset after:
  - at least one more day of samples, and
  - at least one additional machine class (or CI-hosted trend data).

Local provisional gate candidates (large dataset, not enforced yet; not CI-ready thresholds):
- replay latency upper bound: `<= 7 ms`
- recompute latency upper bound: `<= 14 ms`
- replay throughput lower bound: `>= 150000 tx/s`

## Release checklist hook

- Before release tag:
  - run `cargo test --test perf_smoke -- --nocapture`
  - compare to this baseline table
  - document regression/variance decision in release notes
