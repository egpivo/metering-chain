# Invariant Test Matrix (Next-Cycle First Pass)

Source of invariants: [invariants.md](./invariants.md)

Legend:
- Deterministic = explicit example/unit/integration checks
- Property = proptest/generative checks
- Status:
  - Covered (first pass)
  - Partial (first pass)
  - Pending (next pass)

| Invariant | Deterministic coverage | Property coverage | Status |
|---|---|---|---|
| INV-1 No negative balances | validation/apply rejection paths in existing suite (`basic_flow`, `cli_smoke`) | indirectly exercised by generated valid sequences | Partial (first pass) |
| INV-2 Monotonic nonce | `cli_smoke` stale nonce reject | `prop_nonce_monotonicity_for_accepted_sequence`, `prop_rejected_transition_does_not_consume_nonce_or_block_followup` | Covered (first pass) |
| INV-3 Conservation of value | existing flow tests around open/close/meter economics | `prop_accounted_value_conservation_across_accepted_sequence` + `prop_deposit_conservation_across_open_and_close` | Covered (first pass, accounted-value form) |
| INV-4 Ownership authorization | deterministic invalid signer and delegated cases in existing tests/security suite | none yet for broad generated delegated paths | Partial (first pass) |
| INV-5 Meter uniqueness | existing open-meter duplicate rejection paths | `prop_meter_uniqueness_rejects_second_active_open` | Covered (first pass) |
| INV-6 Active meter requirement | deterministic consume-on-closed-meter rejection in existing tests | none yet | Partial (first pass) |
| INV-7 Deposit conservation | open/close accounting in deterministic flow tests | `prop_deposit_conservation_across_open_and_close` | Covered (first pass) |
| INV-8 Mint authorization | `cli_smoke` and other invalid mint tests | none yet | Partial (first pass) |
| INV-9 Nonce monotonicity by nonce account | delegated and non-delegated deterministic tests | nonce properties currently owner-sign path focused | Partial (first pass) |
| INV-10 Sufficient balance for deposits | deterministic invalid open-meter tests | none yet | Partial (first pass) |
| INV-11 Sufficient balance for consumption | deterministic insufficient-balance rejects | none yet | Partial (first pass) |
| INV-12 Valid pricing | deterministic validation checks | generated sequences use valid pricing domain only | Partial (first pass) |
| INV-13 Positive units | deterministic reject + property reject-followup case | `prop_rejected_transition_does_not_consume_nonce_or_block_followup` (units=0 reject) | Covered (first pass) |
| INV-14 Overflow protection | deterministic overflow checks (including `security_abuse`) | none yet | Partial (first pass) |
| INV-15 Monotonic meter totals | deterministic meter progression checks | `prop_meter_totals_monotonicity_under_accepted_consumes` | Covered (first pass) |
| INV-16 Deterministic transitions | replay/recovery deterministic suites | `prop_replay_determinism_across_snapshot_boundary` | Covered (first pass) |

## Intentional gaps left for next pass

- richer delegated-sequence property generation (issuer/audience/scope/caveat permutations)
- explicit property coverage for insufficient-balance and invalid-pricing rejection domains
- separate property for active-meter requirement across close/reopen permutations
- stronger one-to-one linkage from each invariant to a named deterministic test ID in docs
