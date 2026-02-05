# Phase 3 delegation demo

This demo follows the **Phase 3 demo framing** (see `.local/phase3_plan.md`): it runs the four required scenes without repeating Phase 1/2 ("how is usage accounted" and "who can submit"). It only demonstrates "who can **act on behalf of another**".

## What each phase answers

| Phase | Question answered |
|-------|-------------------|
| Phase 1 | How is usage accounted? |
| Phase 2 | Who can submit transactions? |
| Phase 3 | Who can **act on behalf of another**? |

## Required scenes (this demo)

1. **Delegate signs Consume** — `signer != owner` (delegate/hotspot signs for the account); the same usage is submitted by the delegate.
2. **No proof rejected** — The same Consume without a delegation proof is rejected by validation.
3. **With proof accepted** — After the owner issues a delegation proof, the same Consume is accepted and applied.
4. **Revoke then reject** — After the owner revokes the capability, sending Consume again with the same proof returns `DelegationRevoked`.

## Run

From the repo root:

```bash
./examples/phase3_demo/run_phase3_demo.sh
```

The script uses a temporary data dir, creates three wallets (authority, owner, delegate), runs Mint and OpenMeter, then runs the four scenes above. Output lines like "Expected: rejected." or "Expected: accepted." indicate the expected outcome.

## Dependencies

- `cargo run --bin metering-chain` must work.
- CLI: `wallet create`, `wallet sign` (including `--for-owner`, `--proof-file`, `--nonce`, `--valid-at`), `wallet create-delegation-proof` (requires `--service-id`; optional `--ability`), `wallet revoke-delegation`, `wallet capability-id`, `apply`.
