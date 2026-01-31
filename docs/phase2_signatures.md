# Phase 2: Signatures and Authorization

Phase 1 proved deterministic replay. Phase 2 adds cryptographic authorization: only the right actor can mint or spend.

## Threat model (minimal)

- **Fake Mint**: attacker creates credits from thin air.
- **Fake Consume**: attacker spends on behalf of user.
- **Tampered usage**: operator claims usage user never authorized.

Goal: only the owner can authorize `Consume`; only the authority (or wallet addresses) can `Mint`.

## New concepts (infrastructure)

- **Wallet**: Ed25519 keypair; address = `0x` + hex(32-byte public key).
- **SignedTx**: optional `signature: Option<Vec<u8>>` over canonical payload (signer, nonce, kind).
- **Verification**: before apply, `verify_signature(tx)`; legacy `signature: None` allowed for Phase 1 replay.

## DDD boundary

- Domain layer never handles cryptography.
- Infrastructure: `wallet` (keypair, sign, verify), `verify_signature(tx)`.
- Domain: logical rules only (signer == owner, mint authority).

## CLI flow (Phase 2)

1. `metering-chain init`
2. `metering-chain wallet create` → prints address (e.g. `0x...`)
3. `metering-chain wallet list`
4. Create kind JSON (e.g. `{"Mint":{"to":"0x...","amount":1000}}`); pipe or use `--file`.
5. `metering-chain wallet sign --address <addr> --file kind.json` → signed tx JSON
6. Pipe signed tx into `metering-chain apply`
7. `metering-chain account <addr>`, `meters <addr>`, `report`

## Authorized minters

- Legacy: `"authority"` (string, for unsigned/Phase 1).
- Phase 2: all addresses in `wallets.json` can mint (any created wallet is a minter).

## Backward compatibility

- **Unsigned tx** (`signature: None`): allowed; verification skips (replay of Phase 1 logs).
- **Signed tx**: address must be hex(32-byte pubkey); signature verified over canonical bincode payload.

## Files

- `src/wallet.rs`: Wallet, Wallets, `verify_signature`, address derivation.
- `src/tx/transaction.rs`: `SignablePayload`, `SignedTx.signature`, `message_to_sign()`.
- `src/cli.rs`: `wallet create/list/sign`, verify before apply, authorized minters from wallets.
