# Examples

Minimal example transactions ready to pipe into the CLI.

Notes:
- Nonces must be sequential per signer.
- The files below assume a fresh data directory (no prior txs for the example account).
- Run in order to match nonces.

## Quick run (minimal)

```bash
cargo run --bin metering-chain -- init

# Phase 2 default: signed tx required. For legacy unsigned examples, pass --allow-unsigned.
for f in examples/tx/01_mint_alice.json \
         examples/tx/02_open_storage.json \
         examples/tx/03_consume_storage_unit_price.json \
         examples/tx/05_close_storage.json
do
    cat "$f" | cargo run --bin metering-chain -- apply --allow-unsigned
done
```

## Inspect state

```bash
cargo run --bin metering-chain -- account 0x0000000000000000000000000000000000000A11
cargo run --bin metering-chain -- meters 0x0000000000000000000000000000000000000A11
cargo run --bin metering-chain -- report 0x0000000000000000000000000000000000000A11
```

## DePIN reward demo (SIM Dune API)

See `examples/depin/README.md` for a live-data demo that pulls on-chain reward
distributions and converts them into `Consume` transactions.

## Multi-operator demo (Helium rewards)

See `examples/multi_operator/README.md` for a multi-operator flow that maps
hotspot rewards to per-operator meters.

## Files (minimal)

- `examples/tx/01_mint_alice.json` - Mint 1000 to 0x0000000000000000000000000000000000000A11 (authority only)
- `examples/tx/02_open_storage.json` - Open storage meter with deposit
- `examples/tx/03_consume_storage_unit_price.json` - UnitPrice consume
- `examples/tx/05_close_storage.json` - Close storage meter (returns deposit)
