# Wallet Design (Phase 2)

Wallets are **infrastructure concern**, not domain logic. They handle key management and transaction signing.

---

## When to Implement

**Phase 1 (MVP)**: Not needed
- Use simple String addresses
- No cryptographic signatures
- Focus on domain logic (state transitions, invariants)

**Phase 2**: Implement wallets
- Add cryptographic signatures to `SignedTx`
- Generate addresses from public keys
- Enable secure transaction signing

---

## Design (Infrastructure Layer)

### Location
- `src/wallet.rs` or `src/infrastructure/wallet.rs`
- Keep separate from domain (`state/`, `tx/`)

### Structure

```rust
pub struct Wallet {
    pub address: String,      // Derived from public key
    pub private_key: Vec<u8>, // Keep secure, never expose
    pub public_key: Vec<u8>,
}

pub struct Wallets {
    wallets: HashMap<String, Wallet>,
    file_path: PathBuf,
}

impl Wallets {
    pub fn new(file_path: PathBuf) -> Self {
        // Load or create wallet file
    }
    
    pub fn create_wallet(&mut self) -> Result<String> {
        // Generate keypair
        // Derive address from public key
        // Save to file
        // Return address
    }
    
    pub fn get_addresses(&self) -> Vec<String> {
        // Return all addresses
    }
    
    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        // Get wallet by address
    }
    
    pub fn sign_transaction(&self, address: &str, tx: &Transaction) -> Result<SignedTx> {
        // Get wallet
        // Get current nonce from state
        // Sign transaction
        // Return SignedTx with signature
    }
    
    fn load_from_file(&mut self) -> Result<()> {
        // Load wallets from file (encrypted)
    }
    
    fn save_to_file(&self) -> Result<()> {
        // Save wallets to file (encrypted)
    }
}
```

---

## Key Decisions

### Address Format
- **Option 1**: Hash of public key (like Ethereum)
- **Option 2**: Base58 encoded public key (like Bitcoin)
- **Option 3**: Simple hex-encoded public key (for MVP simplicity)

### Signature Scheme
- **Option 1**: ECDSA (like Bitcoin/Ethereum)
- **Option 2**: Ed25519 (simpler, faster)
- **Recommendation**: Ed25519 for simplicity

### Storage
- Encrypted wallet file (password-protected)
- Never store private keys in plaintext
- Use `wallet.dat` or `.wallets` file

---

## Integration with Domain

### Domain Layer (No Changes)
- `Account` still uses `String` address
- `SignedTx` adds `signature: Vec<u8>` field
- Domain logic doesn't know about wallets

### Infrastructure Layer
- CLI uses `Wallets` to sign transactions
- Validation verifies signatures (infrastructure concern)
- Domain only checks `signer == owner` (logical check)

---

## MVP Approach (Phase 1)

For now, skip wallets:
- Use simple String addresses: `"alice"`, `"bob"`, `"authority"`
- No signatures (just logical validation)
- Focus on domain correctness

When ready for Phase 2:
- Add `wallet` module
- Add signature field to `SignedTx`
- Update validation to check signatures
- Update CLI to use wallets

---

## References
- Architecture: `docs/architecture.md`
- Tool plan: `.local/tool_plan.md`
