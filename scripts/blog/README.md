# Blog Scripts

Small utilities for generating blog-friendly artifacts from the current QA/productization work.

## Scripts

- `generate_blog_artifacts.py`
  - reads:
    - `.local/invariant_test_matrix.md`
    - `.local/version_compatibility_matrix.md`
    - `.local/benchmark_baseline.md`
  - writes into `.local/blog_artifacts/`:
    - `fail_closed_examples.md`
    - `fail_closed_examples.json`
    - `perf_variance_review.csv`
    - `perf_variance_review.json`
    - `perf_latency.svg`
    - `perf_throughput.svg`
    - `test_layers.md`

- `run_perf_capture.py`
  - runs `cargo test --test perf_smoke -- --nocapture` multiple times
  - parses emitted `[perf_smoke] ...` lines
  - writes raw and aggregated results into `.local/blog_artifacts/`

## Usage

From repo root:

```bash
python scripts/blog/generate_blog_artifacts.py
python scripts/blog/run_perf_capture.py --runs 5
```

## Notes

- These scripts are reporting/generation helpers only.
- They do not change the release/perf policy by themselves.
- Generated files are intended for blog-post drafting and review, not as canonical source documents.
