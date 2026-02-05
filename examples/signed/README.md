# Phase 2 signed examples

Strict flow: **signed tx only** (no `--allow-unsigned`). Uses `wallet create`, `wallet sign`, then `apply`.

## Quick run

From repo root:

```bash
./examples/signed/run_signed_demo.sh
```

The script runs `init`, creates two wallets (authority + user), sets `METERING_CHAIN_MINTERS`, then Mint, OpenMeter, Consume, CloseMeter with signed tx and pipes each into `apply`.

## Kind to signed JSON

`wallet sign` needs a **kind-only** JSON file (no signer/nonce; nonce comes from state). Example:

- **Mint:** `{"Mint":{"to":"<user_address>","amount":1000}}`
- **OpenMeter:** `{"OpenMeter":{"owner":"<user>","service_id":"storage","deposit":100}}`
- **Consume:** `{"Consume":{"owner":"<user>","service_id":"storage","units":10,"pricing":{"UnitPrice":2}}}`
- **CloseMeter:** `{"CloseMeter":{"owner":"<user>","service_id":"storage"}}`

Then:

```bash
cargo run --bin metering-chain -- wallet sign --address <signer_address> --file kind.json
```

Output is full signed tx JSON; pipe to `apply`:

```bash
cargo run --bin metering-chain -- wallet sign --address "$AUTHORITY" --file kind_mint.json | cargo run --bin metering-chain -- apply
```

## Files

- `run_signed_demo.sh` – full signed demo (init + 2 wallets + Mint/Open/Consume/Close).
- `kind_01_mint.json`, `kind_02_open.json`, `kind_03_consume.json`, `kind_04_close.json` – kind-only templates (placeholder `0xUSER`); the script generates JSON with real addresses.
