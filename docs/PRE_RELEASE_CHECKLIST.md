# Pre-release checklist

Run before tagging a release. All items must be satisfied for **GO**.

- [ ] **Full test suite green** — `cargo test --workspace`
- [ ] **Deterministic replay regression green** — Phase 4 / G4 tests in `tests/basic_flow.rs`
- [ ] **Coverage gate green** — CI coverage job passes (see `.github/workflows/ci.yml`)
- [ ] **No unresolved P0/P1 bug** — Triage open issues; document known issues in release notes if needed
- [ ] **Closure scope agreed** — `docs/closure_scope.md`: scope freeze, tag, date, sign-off

**Release decision:**

- [ ] **GO** — Proceed to tag and release
- [ ] **NO-GO** — Blocking list: _(list blocking items)_

After release: update `docs/RELEASE_NOTES.md` (move [Unreleased] into versioned section).
