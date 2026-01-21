# Blockchain Struct Design Discussion

## Pivot: metering-first domain model (Account / Meter state machine)

### Why we pivot away from UTXO-first
- The repo’s primary product goal is **metering** (open meter, consume units, settle costs). This maps naturally to an **account-based state machine**.
- UTXO is great to learn “transaction semantics”, but it adds extra concepts (UTXO set, input/output linking) that are not the center of the metering domain.
- For DDD + Medium writing, the clearest story is:
  - **State** (Accounts + Meters) is the aggregate root of invariants
  - **Transaction** is a domain command/event
  - **Apply(tx, state) -> state'** is the state transition function (Ethereum-style)

### Core state (our domain)

#### `state/account.rs`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub balance: u64,
    pub nonce: u64,
}
```

#### `state/meter.rs`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meter {
    pub service_id: String,
    pub total_units: u64,
    pub total_spent: u64,
    pub active: bool,
}
```

**Design note (recommended):**
- We likely need ownership identity on meters. Two common options:
  - Store `owner: String` in `Meter`
  - Or use a composite key `(owner, service_id)` in the state store

### Transactions: metering-first commands

#### `tx/transaction.rs` (transaction kinds)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transaction {
    Mint { to: String, amount: u64 },
    OpenMeter { owner: String, service_id: String, deposit: u64 },
    Consume { owner: String, service_id: String, units: u64, pricing: Pricing, nonce: u64 },
}
```

**Design note (recommended consistency):**
- `nonce` should be consistent across all transaction types, not only `Consume`.
- A clearer model is to wrap the enum with shared metadata:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTx {
    pub from: String,
    pub nonce: u64,
    pub kind: Transaction,
    // pub sig: Vec<u8>, // later
}
```

This makes validation/apply logic much cleaner and more “Ethereum-like”.

#### Pricing semantics (make it explicit)
Avoid “price could be unit price or total cost” ambiguity by modeling pricing as domain semantics:

```rust
enum Pricing {
    UnitPrice(Price),
    FixedCost(Amount),
}
```

### Domain invariants (what must always hold)

#### Global invariants
- **No negative balances**: `Account.balance` never underflows.
- **Monotonic nonce per account**: accepted tx must have `tx.nonce == account.nonce`, then increment `account.nonce += 1`.
- **Authorization**: only the right party can operate on a meter (owner controls their meter).

#### Mint invariants
- **Mint authorization**: only chain authority (PoA key / genesis authority) may mint.
- **Supply policy** (optional): fixed cap or unlimited mint; must be explicit.

#### OpenMeter invariants
- **Meter uniqueness**: `(owner, service_id)` must not already exist (or must be inactive to reopen).
- **Deposit available**: `owner.balance >= deposit`.
- **Deposit accounting**: define whether deposit is:
  - deducted permanently (like a fee), or
  - locked (preferred: track locked funds), or
  - transferred to a “service escrow”.

#### Consume invariants
- **Meter must be active**.
- **Price model**: clarify if `price` is unit price or total cost.
  - If unit price: `cost = units * unit_price` (check overflow)
  - If total: `cost = price`
- **Sufficient funds**: the payer must have enough available balance (or enough locked deposit) to cover the cost.
- **Accounting**:
  - `meter.total_units += units`
  - `meter.total_spent += cost`
  - balances/escrow updated accordingly

### State transition (Ethereum-style phrasing for Medium)
> “A metering transaction is a pure function over state: given a state \(S\) and a transaction \(T\), if \(T\) is valid under \(S\), the system produces a new state \(S'\) while preserving invariants.”

---

## Core Structs to Define

### Chain Module (`src/chain/`)

#### Block (`block.rs`)
```rust
pub struct Block {
    timestamp: i64,
    pre_block_hash: String,
    hash: String,
    transactions: Vec<Transaction>,
    nonce: i64,
    height: usize,
}
```

**Fields:**
- `timestamp`: Block creation time (i64 - Unix timestamp)
- `pre_block_hash`: Hash of previous block (String)
- `hash`: Current block hash (String)
- `transactions`: List of transactions in this block (Vec<Transaction>)
- `nonce`: Proof-of-work nonce (i64)
- `height`: Block height in the chain (usize)

**Methods:**
```rust
pub fn new_block(pre_block_hash: String, transactions: &[Transaction], height: usize) -> Block {
    let mut block = Block {
        timestamp: crate::current_timestamp(),
        pre_block_hash,
        hash: String::new(),
        transactions: transactions.to_vec(),
        nonce: 0,
        height,
    };
    // ... (hash calculation to be added)
}
```

**Notes:**
- Uses `crate::current_timestamp()` utility function (needs to be implemented)
- Hash is initially empty (String::new()) - needs to be calculated
- Nonce starts at 0 (for PoW mining)
- Transactions are cloned from slice

**Questions:**
- Hash calculation method (what data is hashed?)
- How is the hash computed? (SHA256, etc.)
- Transaction ordering within block
- Where should `current_timestamp()` utility be placed?

#### Blockchain (`blockchain.rs`)
- How to store the chain?
- Genesis block handling
- Chain validation logic

#### Consensus (`consensus.rs`)
```rust
pub struct ProofOfWork {
    block: Block,
    target: BigInt,
}
```

**Fields:**
- `block`: The block to be mined (Block)
- `target`: Difficulty target (BigInt) - determines mining difficulty

**Notes:**
- Using PoW (Proof of Work) consensus mechanism
- `BigInt` type needed (likely from `num-bigint` crate)
- Target determines how difficult it is to find a valid hash

**Methods:**
```rust
pub fn run(&self) -> (i64, String) {
    let mut nonce = 0;
    let mut hash = Vec::new();
    println!("Mining the block");
    while nonce < MAX_NONCE {
        let data = self.prepare_data(nonce);
        hash = crate::sha256_digest(data.as_slice());
        let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());
        if hash_int.lt(self.target.borrow()) {
            println!("{}", HEXLOWER.encode(hash.as_slice()));
            break;
        } else {
            nonce += 1;
        }
    }
    println!();
    return (nonce, HEXLOWER.encode(hash.as_slice()));
}
```

**Mining Algorithm:**
1. Start with nonce = 0
2. Prepare block data with current nonce
3. Calculate SHA256 hash
4. Convert hash to BigInt
5. Compare with target - if hash < target, mining successful
6. Otherwise increment nonce and repeat
7. Returns (nonce, hash_string) when found

**Dependencies Needed:**
- `sha256_digest()` utility function (needs to be implemented)
- `BigInt` from `num-bigint` crate
- `HEXLOWER` from `hex` crate (for hex encoding)
- `MAX_NONCE` constant (needs to be defined)

**Questions:**
- How is the target calculated/adjusted?
- What does `prepare_data(nonce)` do? (serializes block data with nonce?)
- What is the value of `MAX_NONCE`?
- Where should `sha256_digest()` utility be placed?

### Transaction Module (`src/tx/`)

#### Transaction (`transaction.rs`)
```rust
pub struct Transaction {
    id: Vec<u8>,
    vin: Vec<TXInput>,
    vout: Vec<TXOutput>,
}
```

**Fields:**
- `id`: Transaction ID (Vec<u8> - likely hash of transaction data)
- `vin`: Transaction inputs (Vec<TXInput>) - references to previous transaction outputs
- `vout`: Transaction outputs (Vec<TXOutput>) - new outputs created by this transaction

**Notes:**
- UTXO (Unspent Transaction Output) model
- Similar to Bitcoin's transaction structure

#### TXInput (`transaction.rs`)
```rust
pub struct TXInput {
    txid: Vec<u8>,
    vout: usize,
    signature: Vec<u8>,
    pub_key: Vec<u8>,
}
```

**Fields:**
- `txid`: Transaction ID of the previous transaction (Vec<u8>)
- `vout`: Index of the output in the previous transaction (usize)
- `signature`: Digital signature proving ownership (Vec<u8>)
- `pub_key`: Public key of the sender (Vec<u8>)

**Notes:**
- References a previous transaction's output (UTXO)
- Signature proves the sender owns the referenced output

#### TXOutput (`transaction.rs`)
```rust
pub struct TXOutput {
    value: i32,
    pub_key_hash: Vec<u8>,
}
```

**Fields:**
- `value`: Output value/amount (i32)
- `pub_key_hash`: Hash of the recipient's public key (Vec<u8>)

**Notes:**
- Creates a new UTXO that can be spent in future transactions
- `pub_key_hash` is used to lock the output to a specific recipient

**Questions:**
- How is transaction ID calculated? (hash of what data?)
- How does this relate to meter readings? (special transaction type?)
- What cryptographic functions are used for signatures and hashing?

#### Validation (`validation.rs`)
- Signature verification
- Balance checks
- Double-spend prevention

### State Module (`src/state/`)

#### Account (`account.rs`)
- Account structure
- Balance tracking
- Account metadata

#### Meter (`meter.rs`)
- Meter reading structure
- Meter metadata
- Reading history

#### Apply (`apply.rs`)
- State transition logic
- How transactions modify state

### Storage Module (`src/storage/`)

#### KV Store (`kv.rs`)
- Storage backend choice (file-based, embedded DB)
- Key-value schema
- Persistence strategy

### Error Handling (`error.rs`)
- Error types for each module
- Unified error enum

---

## Questions to Discuss

1. **Block Structure**: What data should each block contain?
2. **Transaction Types**: What operations can transactions perform?
3. **Consensus**: PoA or PoW? Simple or more complex?
4. **State Model**: How do we track meter readings and accounts?
5. **Storage**: File-based JSON, embedded DB (RocksDB, SQLite), or custom format?
