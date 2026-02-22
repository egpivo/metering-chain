use metering_chain::error::Error;
use metering_chain::replay;
use metering_chain::state::{apply, Account, Meter, State};
use metering_chain::storage::{FileStorage, Storage};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::{DisputeVerdict, Pricing, SignedTx, Transaction};
use metering_chain::wallet::{verify_signature, Wallet};
use std::collections::HashSet;
use tempfile::TempDir;

fn replay_ctx() -> ValidationContext {
    ValidationContext::replay()
}

fn get_authorized_minters() -> HashSet<String> {
    let mut minters = HashSet::new();
    minters.insert("authority".to_string());
    minters
}

fn create_test_storage() -> (FileStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let tx_log_path = temp_dir.path().join("tx.log");
    let state_path = temp_dir.path().join("state.bin");
    let storage = FileStorage::with_paths(tx_log_path, state_path);
    (storage, temp_dir)
}

fn load_or_replay_state(storage: &FileStorage) -> (State, u64) {
    replay::replay_to_tip(storage).unwrap()
}

/// Test the complete happy path: Mint, OpenMeter, Consume, CloseMeter
#[test]
fn test_happy_path_end_to_end() {
    let (mut storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();
    let mut next_tx_id = 0u64;

    // 1. Mint: Authority mints 1000 to alice
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx1).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    // Verify: alice has 1000 balance
    assert_eq!(state.get_account("alice").unwrap().balance(), 1000);
    assert_eq!(state.get_account("alice").unwrap().nonce(), 0);

    // 2. OpenMeter: alice opens a storage meter with 100 deposit
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx2).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    // Verify: alice balance decreased, meter created
    assert_eq!(state.get_account("alice").unwrap().balance(), 900);
    assert_eq!(state.get_account("alice").unwrap().nonce(), 1);
    let meter = state.get_meter("alice", "storage").unwrap();
    assert!(meter.is_active());
    assert_eq!(meter.locked_deposit(), 100);
    assert_eq!(meter.total_units(), 0);
    assert_eq!(meter.total_spent(), 0);

    // 3. Consume: alice consumes 10 units at 5 per unit (cost: 50)
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx3).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    // Verify: balance decreased, meter totals updated
    assert_eq!(state.get_account("alice").unwrap().balance(), 850);
    assert_eq!(state.get_account("alice").unwrap().nonce(), 2);
    let meter = state.get_meter("alice", "storage").unwrap();
    assert_eq!(meter.total_units(), 10);
    assert_eq!(meter.total_spent(), 50);

    // 4. Consume again: 5 more units at 5 per unit (cost: 25)
    let tx4 = SignedTx::new(
        "alice".to_string(),
        2,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 5,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx4, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx4).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    // Verify: cumulative totals
    assert_eq!(state.get_account("alice").unwrap().balance(), 825);
    let meter = state.get_meter("alice", "storage").unwrap();
    assert_eq!(meter.total_units(), 15);
    assert_eq!(meter.total_spent(), 75);

    // 5. CloseMeter: alice closes the meter
    let tx5 = SignedTx::new(
        "alice".to_string(),
        3,
        Transaction::CloseMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
        },
    );
    state = apply(&state, &tx5, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx5).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    // Verify: deposit returned, meter inactive
    assert_eq!(state.get_account("alice").unwrap().balance(), 925); // 825 + 100 deposit
    assert_eq!(state.get_account("alice").unwrap().nonce(), 4);
    let meter = state.get_meter("alice", "storage").unwrap();
    assert!(!meter.is_active());
    assert_eq!(meter.locked_deposit(), 0);
    // Historical totals preserved
    assert_eq!(meter.total_units(), 15);
    assert_eq!(meter.total_spent(), 75);
}

/// Test state reconstruction from transaction log
#[test]
fn test_state_reconstruction() {
    let (mut storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();
    let mut next_tx_id = 0u64;

    // Apply transactions without persisting state (simulating crash)
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx1).unwrap();
    next_tx_id += 1;

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx2).unwrap();
    next_tx_id += 1;

    // Persist state after tx2
    let snapshot_next_tx_id = next_tx_id;
    storage.persist_state(&state, snapshot_next_tx_id).unwrap();

    // Apply more transactions (not persisted in snapshot)
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx3).unwrap();
    next_tx_id += 1;

    // Reconstruct state from snapshot + replay
    let (reconstructed_state, reconstructed_next_tx_id) = load_or_replay_state(&storage);

    // Verify reconstructed state matches current state
    // reconstructed_next_tx_id should be next_tx_id (snapshot at 2 + replay tx3 = 3)
    assert_eq!(
        reconstructed_next_tx_id, next_tx_id,
        "Reconstructed next_tx_id should match current next_tx_id"
    );
    assert_eq!(
        reconstructed_state.get_account("alice").unwrap().balance(),
        state.get_account("alice").unwrap().balance()
    );
    assert_eq!(
        reconstructed_state.get_account("alice").unwrap().nonce(),
        state.get_account("alice").unwrap().nonce()
    );
    let reconstructed_meter = reconstructed_state.get_meter("alice", "storage").unwrap();
    let current_meter = state.get_meter("alice", "storage").unwrap();
    assert_eq!(
        reconstructed_meter.total_units(),
        current_meter.total_units()
    );
    assert_eq!(
        reconstructed_meter.total_spent(),
        current_meter.total_spent()
    );
}

/// Replay cursor consistency: next_tx_id semantics, snapshot + replay yields correct final cursor.
#[test]
fn test_replay_cursor_consistency() {
    let (mut storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();
    let mut next_tx_id = 0u64;

    // Append 4 txs to log
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx1).unwrap();
    next_tx_id += 1;

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx2).unwrap();
    next_tx_id += 1;

    // Persist snapshot at cursor 2 (next tx to apply = 2)
    let snapshot_at = next_tx_id;
    storage.persist_state(&state, snapshot_at).unwrap();

    // Append 2 more txs (not in snapshot)
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx3).unwrap();
    next_tx_id += 1;

    let tx4 = SignedTx::new(
        "alice".to_string(),
        2,
        Transaction::CloseMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
        },
    );
    state = apply(&state, &tx4, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx4).unwrap();
    next_tx_id += 1;

    // Replay: snapshot (cursor 2) + txs from 2..4
    let (replayed_state, replayed_next_tx_id) = load_or_replay_state(&storage);

    // Cursor consistency: replayed_next_tx_id == total txs in log (4)
    assert_eq!(
        replayed_next_tx_id, next_tx_id,
        "replay_to_tip must return next_tx_id = count of applied txs"
    );
    assert_eq!(replayed_next_tx_id, 4);

    // load_txs_from(snapshot_at) returns remaining txs (tx3, tx4)
    let remaining = replay::load_tx_slice(&storage, snapshot_at).unwrap();
    assert_eq!(remaining.len(), 2, "load_txs_from(2) returns txs 2..4");

    // Replayed state matches live state
    assert_eq!(replayed_state, state);
}

/// Test meter reopening scenario
#[test]
fn test_meter_reopening() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup: mint and open meter
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();

    // Consume some units
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 20,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();

    // Close meter
    let tx4 = SignedTx::new(
        "alice".to_string(),
        2,
        Transaction::CloseMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
        },
    );
    state = apply(&state, &tx4, &replay_ctx(), Some(&minters)).unwrap();

    // Verify meter is inactive but totals preserved
    let meter = state.get_meter("alice", "storage").unwrap();
    assert!(!meter.is_active());
    assert_eq!(meter.total_units(), 20);
    assert_eq!(meter.total_spent(), 100);

    // Reopen meter with new deposit
    let tx5 = SignedTx::new(
        "alice".to_string(),
        3,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 150,
        },
    );
    state = apply(&state, &tx5, &replay_ctx(), Some(&minters)).unwrap();

    // Verify: meter reactivated, totals preserved, new deposit set
    let meter = state.get_meter("alice", "storage").unwrap();
    assert!(meter.is_active());
    assert_eq!(meter.locked_deposit(), 150);
    assert_eq!(meter.total_units(), 20); // Preserved
    assert_eq!(meter.total_spent(), 100); // Preserved
    assert_eq!(state.get_account("alice").unwrap().balance(), 750); // 1000 - 100 - 100 - 50 (new deposit)
}

/// Test rejection: invalid nonce
#[test]
fn test_rejection_invalid_nonce() {
    let (mut storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup: mint and open meter
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx1).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();
    storage.append_tx(&tx2).unwrap();

    // Try to consume with wrong nonce (should be 1, but using 0)
    let tx3 = SignedTx::new(
        "alice".to_string(),
        0, // Wrong nonce! Should be 1
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );

    let result = apply(&state, &tx3, &replay_ctx(), Some(&minters));
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("Nonce mismatch"));
        }
        _ => panic!("Expected InvalidTransaction error"),
    }
}

/// Test rejection: insufficient balance for deposit
#[test]
fn test_rejection_insufficient_balance_deposit() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Mint only 50 to alice
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 50,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    // Try to open meter with 100 deposit (insufficient balance)
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100, // More than balance!
        },
    );

    let result = apply(&state, &tx2, &replay_ctx(), Some(&minters));
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("Insufficient balance"));
        }
        _ => panic!("Expected InvalidTransaction error"),
    }
}

/// Test rejection: insufficient balance for consumption
#[test]
fn test_rejection_insufficient_balance_consumption() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup: mint 100, open meter with 50 deposit
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 100,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 50,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();

    // Try to consume with cost 100 (but balance is only 50)
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 20,
            pricing: Pricing::UnitPrice(5), // Cost: 100
        },
    );

    let result = apply(&state, &tx3, &replay_ctx(), Some(&minters));
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("Insufficient balance"));
        }
        _ => panic!("Expected InvalidTransaction error"),
    }
}

/// Test rejection: unauthorized mint
#[test]
fn test_rejection_unauthorized_mint() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let state = State::new();

    // Try to mint from non-authorized account
    let tx = SignedTx::new(
        "alice".to_string(), // Not authorized!
        0,
        Transaction::Mint {
            to: "bob".to_string(),
            amount: 100,
        },
    );

    let result = apply(&state, &tx, &replay_ctx(), Some(&minters));
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("authorized minter"));
        }
        _ => panic!("Expected InvalidTransaction error"),
    }
}

/// Test rejection: consume on inactive meter
#[test]
fn test_rejection_consume_inactive_meter() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup: mint, open meter, then close it
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();

    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::CloseMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();

    // Try to consume on inactive meter
    let tx4 = SignedTx::new(
        "alice".to_string(),
        2,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );

    let result = apply(&state, &tx4, &replay_ctx(), Some(&minters));
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("not active") || msg.contains("inactive"));
        }
        _ => panic!("Expected InvalidTransaction error"),
    }
}

/// Test rejection: wrong signer (not the owner)
#[test]
fn test_rejection_wrong_signer() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup: mint to alice and bob, alice opens meter
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "bob".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();

    let tx3 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();

    // Bob tries to consume on alice's meter
    let tx4 = SignedTx::new(
        "bob".to_string(), // Wrong signer!
        0,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );

    let result = apply(&state, &tx4, &replay_ctx(), Some(&minters));
    assert!(result.is_err());
    // Rejected: either owner-signed path (Signer != owner) or delegated path (missing proof)
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("Signer") && msg.contains("owner"));
        }
        Error::DelegationProofMissing | Error::DelegatedConsumeRequiresV2 => {}
        e => panic!(
            "Expected InvalidTransaction or delegation error, got {:?}",
            e
        ),
    }
}

/// Test rejection: zero units
#[test]
fn test_rejection_zero_units() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup: mint and open meter
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();

    // Try to consume 0 units
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 0, // Invalid!
            pricing: Pricing::UnitPrice(5),
        },
    );

    let result = apply(&state, &tx3, &replay_ctx(), Some(&minters));
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("Units") || msg.contains("zero"));
        }
        _ => panic!("Expected InvalidTransaction error"),
    }
}

/// Test FixedCost pricing model
#[test]
fn test_fixed_cost_pricing() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "api_calls".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();

    // Consume with fixed cost (regardless of units)
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "api_calls".to_string(),
            units: 100,                      // Units don't matter for fixed cost
            pricing: Pricing::FixedCost(50), // Fixed cost: 50
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();

    // Verify: cost is 50 regardless of units
    assert_eq!(state.get_account("alice").unwrap().balance(), 850); // 1000 - 100 - 50
    let meter = state.get_meter("alice", "api_calls").unwrap();
    assert_eq!(meter.total_units(), 100);
    assert_eq!(meter.total_spent(), 50);
}

/// Test multiple meters for same account
#[test]
fn test_multiple_meters() {
    let (_storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();

    // Setup
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 2000,
        },
    );
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();

    // Open two different meters
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();

    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "api_calls".to_string(),
            deposit: 200,
        },
    );
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();

    // Consume on both meters
    let tx4 = SignedTx::new(
        "alice".to_string(),
        2,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx4, &replay_ctx(), Some(&minters)).unwrap();

    let tx5 = SignedTx::new(
        "alice".to_string(),
        3,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "api_calls".to_string(),
            units: 20,
            pricing: Pricing::FixedCost(30),
        },
    );
    state = apply(&state, &tx5, &replay_ctx(), Some(&minters)).unwrap();

    // Verify both meters are independent
    let storage_meter = state.get_meter("alice", "storage").unwrap();
    assert_eq!(storage_meter.total_units(), 10);
    assert_eq!(storage_meter.total_spent(), 50);

    let api_meter = state.get_meter("alice", "api_calls").unwrap();
    assert_eq!(api_meter.total_units(), 20);
    assert_eq!(api_meter.total_spent(), 30);

    assert_eq!(state.get_account("alice").unwrap().balance(), 1620); // 2000 - 100 - 200 - 50 - 30
}

/// Phase 2: signed tx apply success (wallet sign, verify, apply)
#[test]
fn test_phase2_signed_apply_success() {
    let wallet = Wallet::new_random();
    let address = wallet.address().to_string();
    let mut minters = HashSet::new();
    minters.insert(address.clone());

    let kind = Transaction::Mint {
        to: address.clone(),
        amount: 1000,
    };
    let signed_tx = wallet.sign_transaction(0, kind).unwrap();
    verify_signature(&signed_tx).unwrap();

    let state = apply(&State::new(), &signed_tx, &replay_ctx(), Some(&minters)).unwrap();
    assert_eq!(state.get_account(&address).unwrap().balance(), 1000);
}

/// Phase 2: unsigned tx rejected by verify_signature (no --allow-unsigned path)
#[test]
fn test_phase2_unsigned_rejected() {
    let tx = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::Mint {
            to: "bob".to_string(),
            amount: 100,
        },
    );
    assert!(tx.signature.is_none());
    assert!(verify_signature(&tx).is_err());
}

// --- Phase 3 delegation tests ---

/// v1 legacy tx (no payload_version) still replays: deserialize and apply with Replay context.
#[test]
fn test_phase3_v1_legacy_replay() {
    let minters = get_authorized_minters();
    let mut state = State::new();
    let tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 500,
        },
    );
    assert!(tx.payload_version.is_none());
    state = apply(&state, &tx, &replay_ctx(), Some(&minters)).unwrap();
    assert_eq!(state.get_account("alice").unwrap().balance(), 500);
}

/// Delegated consume with v1 (or absent) payload_version is rejected in verify_signature.
#[test]
fn test_phase3_delegated_consume_v1_rejected() {
    let tx = SignedTx {
        payload_version: None,
        signer: "delegate".to_string(),
        nonce: 0,
        nonce_account: Some("alice".to_string()),
        valid_at: Some(1000),
        delegation_proof: Some(vec![0u8; 8]),
        kind: Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "s".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(1),
        },
        signature: Some(vec![0u8; 64]),
    };
    assert!(tx.is_delegated_consume());
    let err = verify_signature(&tx).unwrap_err();
    match err {
        Error::DelegatedConsumeRequiresV2 => {}
        _ => panic!("expected DelegatedConsumeRequiresV2, got {:?}", err),
    }
}

/// Delegated consume v1 is rejected in validation even without verify (e.g. --allow-unsigned path).
#[test]
fn test_phase3_delegated_consume_v1_rejected_in_validation() {
    let mut state = State::new();
    state
        .accounts
        .insert("alice".to_string(), Account::with_balance(1000));
    state.insert_meter(Meter::new("alice".into(), "s".into(), 100));

    let tx = SignedTx {
        payload_version: None,
        signer: "delegate".to_string(),
        nonce: 0,
        nonce_account: Some("alice".to_string()),
        valid_at: Some(1000),
        delegation_proof: Some(vec![0u8; 8]),
        kind: Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "s".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(1),
        },
        signature: None,
    };
    assert!(tx.is_delegated_consume());
    let ctx = ValidationContext::replay();
    let res = metering_chain::tx::validation::validate(&state, &tx, &ctx, None);
    assert!(res.is_err());
    match res.unwrap_err() {
        Error::DelegatedConsumeRequiresV2 => {}
        e => panic!(
            "expected DelegatedConsumeRequiresV2 (v2 gate in validation), got {:?}",
            e
        ),
    }
}

/// Delegated consume with v2 + valid proof: verify and validate pass, apply increments owner nonce.
#[test]
fn test_phase3_delegated_consume_v2_accepted() {
    use metering_chain::tx::transaction::PAYLOAD_VERSION_V2;
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut minters = HashSet::new();
    minters.insert("authority".to_string());
    let mut state = State::new();

    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 1000,
        },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        )
        .unwrap();
    state = apply(&state, &open_tx, &replay_ctx(), Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 1);

    let valid_at = 1000u64;
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let consume_kind = Transaction::Consume {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        units: 10,
        pricing: Pricing::UnitPrice(5),
    };
    let delegated_tx = delegate_wallet
        .sign_transaction_v2(1, owner_addr.clone(), valid_at, proof, consume_kind)
        .unwrap();
    assert_eq!(delegated_tx.effective_payload_version(), PAYLOAD_VERSION_V2);
    verify_signature(&delegated_tx).unwrap();

    let live_ctx = ValidationContext::live(valid_at, 300);
    let cost_opt =
        metering_chain::tx::validation::validate(&state, &delegated_tx, &live_ctx, Some(&minters))
            .unwrap();
    assert_eq!(cost_opt, Some(50));

    state = apply(&state, &delegated_tx, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().balance(), 850);
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);
    assert_eq!(
        state
            .get_meter(&owner_addr, "storage")
            .unwrap()
            .total_units(),
        10
    );
}

/// Delegated consume with proof.issuer as did:key (not 0x): verification uses principal_to_public_key, so did:key works.
#[test]
fn test_phase3_delegated_consume_issuer_did_key() {
    use metering_chain::tx::transaction::PAYLOAD_VERSION_V2;
    use metering_chain::tx::{
        build_signed_proof, delegation_claims_to_sign, DelegationProofMinimal,
    };

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    // Build did:key for owner (same key as owner_addr)
    let hex_body = owner_addr.strip_prefix("0x").unwrap();
    let key_bytes: [u8; 32] = hex::decode(hex_body).unwrap().try_into().unwrap();
    let mut payload = vec![0xed, 0x01];
    payload.extend_from_slice(&key_bytes);
    let owner_did_key = "did:key:z".to_string() + &bs58::encode(payload).into_string();

    let mut minters = HashSet::new();
    minters.insert("authority".to_string());
    let mut state = State::new();
    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 1000,
        },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        )
        .unwrap();
    state = apply(&state, &open_tx, &replay_ctx(), Some(&minters)).unwrap();

    let valid_at = 1000u64;
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_did_key.clone(),
        audience: delegate_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let message = delegation_claims_to_sign(&claims);
    let sig = owner_wallet.sign_bytes(&message);
    let proof = build_signed_proof(&claims, sig);
    let consume_kind = Transaction::Consume {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        units: 10,
        pricing: Pricing::UnitPrice(5),
    };
    let delegated_tx = delegate_wallet
        .sign_transaction_v2(1, owner_addr.clone(), valid_at, proof, consume_kind)
        .unwrap();
    assert_eq!(delegated_tx.effective_payload_version(), PAYLOAD_VERSION_V2);
    verify_signature(&delegated_tx).unwrap();

    let live_ctx = ValidationContext::live(valid_at, 300);
    let cost_opt =
        metering_chain::tx::validation::validate(&state, &delegated_tx, &live_ctx, Some(&minters))
            .unwrap();
    assert_eq!(cost_opt, Some(50));
    state = apply(&state, &delegated_tx, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);
    assert_eq!(
        state
            .get_meter(&owner_addr, "storage")
            .unwrap()
            .total_units(),
        10
    );
}

/// Live context: missing now or max_age rejects delegated consume validation.
#[test]
fn test_phase3_live_context_requires_now_max_age() {
    use metering_chain::tx::validation::{ValidationContext, ValidationMode};
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut state = State::new();
    state
        .accounts
        .insert(owner_addr.clone(), Account::with_balance(1000));
    state.insert_meter(Meter::new(owner_addr.clone(), "s".into(), 100));

    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 9999,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "s".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let tx = SignedTx {
        payload_version: Some(2),
        signer: delegate_addr.clone(),
        nonce: 0,
        nonce_account: Some(owner_addr.clone()),
        valid_at: Some(100),
        delegation_proof: Some(proof),
        kind: Transaction::Consume {
            owner: owner_addr.clone(),
            service_id: "s".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(10),
        },
        signature: None,
    };
    let ctx_no_now = ValidationContext {
        mode: ValidationMode::Live,
        now: None,
        max_age: Some(300),
        next_tx_id: None,
    };
    let ctx_no_max_age = ValidationContext {
        mode: ValidationMode::Live,
        now: Some(100),
        max_age: None,
        next_tx_id: None,
    };
    assert!(metering_chain::tx::validation::validate(&state, &tx, &ctx_no_now, None).is_err());
    assert!(metering_chain::tx::validation::validate(&state, &tx, &ctx_no_max_age, None).is_err());
}

/// Replay context: no wall clock; only iat <= valid_at < exp is checked.
#[test]
fn test_phase3_replay_no_wall_clock() {
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut state = State::new();
    state
        .accounts
        .insert(owner_addr.clone(), Account::with_balance(1000));
    state.insert_meter(Meter::new(owner_addr.clone(), "s".into(), 100));

    let claims = DelegationProofMinimal {
        iat: 100,
        exp: 200,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "s".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let tx = SignedTx {
        payload_version: Some(2),
        signer: delegate_addr.clone(),
        nonce: 0,
        nonce_account: Some(owner_addr.clone()),
        valid_at: Some(150),
        delegation_proof: Some(proof),
        kind: Transaction::Consume {
            owner: owner_addr.clone(),
            service_id: "s".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(10),
        },
        signature: None,
    };
    let replay_ctx = ValidationContext::replay();
    let res = metering_chain::tx::validation::validate(&state, &tx, &replay_ctx, None);
    assert!(
        res.is_ok(),
        "Replay should accept valid_at within iat/exp without now: {:?}",
        res
    );
}

/// Forged proof (signed by delegate instead of owner) is rejected.
#[test]
fn test_phase3_forged_proof_rejected() {
    use metering_chain::tx::{
        build_signed_proof, delegation_claims_to_sign, DelegationProofMinimal,
    };

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut state = State::new();
    state
        .accounts
        .insert(owner_addr.clone(), Account::with_balance(1000));
    state.insert_meter(Meter::new(owner_addr.clone(), "s".into(), 100));

    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 9999,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "s".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let message = delegation_claims_to_sign(&claims);
    let wrong_signature = delegate_wallet.sign_bytes(&message);
    let forged_proof = build_signed_proof(&claims, wrong_signature);

    let tx = SignedTx {
        payload_version: Some(2),
        signer: delegate_addr.clone(),
        nonce: 0,
        nonce_account: Some(owner_addr.clone()),
        valid_at: Some(100),
        delegation_proof: Some(forged_proof),
        kind: Transaction::Consume {
            owner: owner_addr.clone(),
            service_id: "s".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(10),
        },
        signature: None,
    };
    let ctx = ValidationContext::live(100, 300);
    let res = metering_chain::tx::validation::validate(&state, &tx, &ctx, None);
    assert!(
        res.is_err(),
        "Forged proof (signed by delegate) must be rejected: {:?}",
        res
    );
}

/// Delegation scope: proof.service_id must match Consume tx service_id; wrong service_id -> DelegationScopeMismatch.
#[test]
fn test_phase3_delegation_scope_service_id_mismatch() {
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut state = State::new();
    state
        .accounts
        .insert(owner_addr.clone(), Account::with_balance(1000));
    state.insert_meter(Meter::new(owner_addr.clone(), "storage".into(), 100));

    // Proof scoped to "other", tx consumes "storage"
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 9999,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "other".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let tx = SignedTx {
        payload_version: Some(2),
        signer: delegate_addr.clone(),
        nonce: 0,
        nonce_account: Some(owner_addr.clone()),
        valid_at: Some(100),
        delegation_proof: Some(proof),
        kind: Transaction::Consume {
            owner: owner_addr.clone(),
            service_id: "storage".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(10),
        },
        signature: None,
    };
    let ctx = ValidationContext::live(100, 300);
    let res = metering_chain::tx::validation::validate(&state, &tx, &ctx, None);
    assert!(
        res.is_err(),
        "service_id mismatch must be rejected: {:?}",
        res
    );
    match res.unwrap_err() {
        Error::DelegationScopeMismatch => {}
        e => panic!("expected DelegationScopeMismatch, got {:?}", e),
    }
}

/// Delegation scope: if proof.ability is set, it must equal "consume" for Consume tx; wrong ability -> DelegationScopeMismatch.
#[test]
fn test_phase3_delegation_scope_ability_mismatch() {
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut state = State::new();
    state
        .accounts
        .insert(owner_addr.clone(), Account::with_balance(1000));
    state.insert_meter(Meter::new(owner_addr.clone(), "s".into(), 100));

    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 9999,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "s".to_string(),
        ability: Some("other".to_string()),
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let tx = SignedTx {
        payload_version: Some(2),
        signer: delegate_addr.clone(),
        nonce: 0,
        nonce_account: Some(owner_addr.clone()),
        valid_at: Some(100),
        delegation_proof: Some(proof),
        kind: Transaction::Consume {
            owner: owner_addr.clone(),
            service_id: "s".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(10),
        },
        signature: None,
    };
    let ctx = ValidationContext::live(100, 300);
    let res = metering_chain::tx::validation::validate(&state, &tx, &ctx, None);
    assert!(res.is_err(), "ability mismatch must be rejected: {:?}", res);
    match res.unwrap_err() {
        Error::DelegationScopeMismatch => {}
        e => panic!("expected DelegationScopeMismatch, got {:?}", e),
    }
}

/// Delegation scope: proof.ability = Some("consume") is accepted (explicit ability passes).
#[test]
fn test_phase3_delegation_ability_consume_accepted() {
    use metering_chain::tx::transaction::PAYLOAD_VERSION_V2;
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut minters = HashSet::new();
    minters.insert("authority".to_string());
    let mut state = State::new();
    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 1000,
        },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        )
        .unwrap();
    state = apply(&state, &open_tx, &replay_ctx(), Some(&minters)).unwrap();

    let valid_at = 1000u64;
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "storage".to_string(),
        ability: Some("consume".to_string()),
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let consume_kind = Transaction::Consume {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        units: 5,
        pricing: Pricing::UnitPrice(4),
    };
    let delegated_tx = delegate_wallet
        .sign_transaction_v2(1, owner_addr.clone(), valid_at, proof, consume_kind)
        .unwrap();
    assert_eq!(delegated_tx.effective_payload_version(), PAYLOAD_VERSION_V2);
    verify_signature(&delegated_tx).unwrap();

    let live_ctx = ValidationContext::live(valid_at, 300);
    let cost_opt =
        metering_chain::tx::validation::validate(&state, &delegated_tx, &live_ctx, Some(&minters))
            .unwrap();
    assert_eq!(cost_opt, Some(20));
    state = apply(&state, &delegated_tx, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(
        state
            .get_meter(&owner_addr, "storage")
            .unwrap()
            .total_units(),
        5
    );
}

/// Caveat max_cost: first consume under limit ok, second exceed returns CapabilityLimitExceeded.
#[test]
fn test_phase3_caveat_max_cost_enforced() {
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut minters = HashSet::new();
    minters.insert("authority".to_string());
    let mut state = State::new();

    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 1000,
        },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        )
        .unwrap();
    state = apply(&state, &open_tx, &replay_ctx(), Some(&minters)).unwrap();

    let valid_at = 1000u64;
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: Some(50),
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);

    let consume1 = delegate_wallet
        .sign_transaction_v2(
            1,
            owner_addr.clone(),
            valid_at,
            proof.clone(),
            Transaction::Consume {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                units: 5,
                pricing: Pricing::UnitPrice(6),
            },
        )
        .unwrap();
    let live_ctx = ValidationContext::live(valid_at, 300);
    state = apply(&state, &consume1, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().balance(), 870);

    let consume2 = delegate_wallet
        .sign_transaction_v2(
            2,
            owner_addr.clone(),
            valid_at,
            proof,
            Transaction::Consume {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                units: 5,
                pricing: Pricing::UnitPrice(6),
            },
        )
        .unwrap();
    let res =
        metering_chain::tx::validation::validate(&state, &consume2, &live_ctx, Some(&minters));
    assert!(res.is_err());
    match res.unwrap_err() {
        Error::CapabilityLimitExceeded => {}
        e => panic!("expected CapabilityLimitExceeded, got {:?}", e),
    }
}

/// Owner-signed consume with nonce_account set to another account is rejected (no nonce pollution).
#[test]
fn test_phase3_owner_signed_nonce_account_forbidden() {
    let minters = get_authorized_minters();
    let mut state = State::new();
    state
        .accounts
        .insert("alice".to_string(), Account::with_balance(1000));
    state
        .accounts
        .insert("bob".to_string(), Account::with_balance(1000));
    state.insert_meter(Meter::new("alice".into(), "s".into(), 100));

    let tx = SignedTx {
        payload_version: None,
        signer: "alice".to_string(),
        nonce: 0,
        nonce_account: Some("bob".to_string()),
        valid_at: None,
        delegation_proof: None,
        kind: Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "s".to_string(),
            units: 1,
            pricing: Pricing::UnitPrice(10),
        },
        signature: None,
    };
    let ctx = ValidationContext::replay();
    let res = metering_chain::tx::validation::validate(&state, &tx, &ctx, Some(&minters));
    assert!(res.is_err());
    match res.unwrap_err() {
        Error::NonceAccountMissingOrInvalid => {}
        e => panic!("expected NonceAccountMissingOrInvalid, got {:?}", e),
    }
}

// --- M4 test matrix: multi-delegate nonce competition, retry, replay determinism ---

/// Multi-delegate nonce competition: two delegates share owner nonce; first use wins, second with same nonce fails; sequential with correct nonce succeeds.
#[test]
fn test_phase3_multi_delegate_nonce_competition() {
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate1_wallet = Wallet::new_random();
    let delegate1_addr = delegate1_wallet.address().to_string();
    let delegate2_wallet = Wallet::new_random();
    let delegate2_addr = delegate2_wallet.address().to_string();

    let minters = get_authorized_minters();
    let mut state = State::new();

    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 2000,
        },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        )
        .unwrap();
    state = apply(&state, &open_tx, &replay_ctx(), Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 1);

    let valid_at = 1000u64;
    let claims_d1 = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate1_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let claims_d2 = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate2_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof_d1 = owner_wallet.sign_delegation_proof(&claims_d1);
    let proof_d2 = owner_wallet.sign_delegation_proof(&claims_d2);
    let live_ctx = ValidationContext::live(valid_at, 300);

    let consume_kind = |u: u64| Transaction::Consume {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        units: u,
        pricing: Pricing::UnitPrice(5),
    };

    // Delegate1 consumes owner nonce 1; success; owner nonce becomes 2
    let tx1 = delegate1_wallet
        .sign_transaction_v2(
            1,
            owner_addr.clone(),
            valid_at,
            proof_d1.clone(),
            consume_kind(10),
        )
        .unwrap();
    state = apply(&state, &tx1, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);

    // Delegate2 tries owner nonce 1 (already used); rejected
    let tx2_same_nonce = delegate2_wallet
        .sign_transaction_v2(
            1,
            owner_addr.clone(),
            valid_at,
            proof_d2.clone(),
            consume_kind(5),
        )
        .unwrap();
    let res = apply(&state, &tx2_same_nonce, &live_ctx, Some(&minters));
    assert!(res.is_err(), "second delegate using same nonce must fail");
    match res.unwrap_err() {
        Error::InvalidTransaction(_) => {}
        e => panic!("expected InvalidTransaction (nonce mismatch), got {:?}", e),
    }
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);

    // Delegate2 uses owner nonce 2; success
    let tx2_correct = delegate2_wallet
        .sign_transaction_v2(2, owner_addr.clone(), valid_at, proof_d2, consume_kind(5))
        .unwrap();
    state = apply(&state, &tx2_correct, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 3);
    assert_eq!(
        state
            .get_meter(&owner_addr, "storage")
            .unwrap()
            .total_units(),
        15
    );
}

/// Retry: delegate signs with nonce N, apply fails (insufficient balance); state unchanged; retry with same nonce N after minting more succeeds.
#[test]
fn test_phase3_retry_same_nonce_after_failure() {
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let minters = get_authorized_minters();
    let mut state = State::new();

    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 100,
        },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 50,
            },
        )
        .unwrap();
    state = apply(&state, &open_tx, &replay_ctx(), Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().balance(), 50);
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 1);

    let valid_at = 1000u64;
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let live_ctx = ValidationContext::live(valid_at, 300);

    // Consume would cost 100; owner balance 50; insufficient
    let consume_tx = delegate_wallet
        .sign_transaction_v2(
            1,
            owner_addr.clone(),
            valid_at,
            proof.clone(),
            Transaction::Consume {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                units: 20,
                pricing: Pricing::UnitPrice(5),
            },
        )
        .unwrap();
    let res = apply(&state, &consume_tx, &live_ctx, Some(&minters));
    assert!(res.is_err());
    match &res.unwrap_err() {
        Error::InvalidTransaction(msg) if msg.contains("Insufficient balance") => {}
        e => panic!(
            "expected InvalidTransaction (insufficient balance), got {:?}",
            e
        ),
    }
    assert_eq!(
        state.get_account(&owner_addr).unwrap().nonce(),
        1,
        "nonce unchanged after failed apply"
    );
    assert_eq!(state.get_account(&owner_addr).unwrap().balance(), 50);

    // Mint more to owner, retry with same nonce 1; success
    let extra_mint = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 100,
        },
    );
    state = apply(&state, &extra_mint, &replay_ctx(), Some(&minters)).unwrap();
    state = apply(&state, &consume_tx, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);
    assert_eq!(state.get_account(&owner_addr).unwrap().balance(), 50);
    assert_eq!(
        state
            .get_meter(&owner_addr, "storage")
            .unwrap()
            .total_units(),
        20
    );
}

/// Replay determinism: same tx log (with delegated consumes) replayed twice yields identical state.
#[test]
fn test_phase3_replay_determinism() {
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let minters = get_authorized_minters();
    let valid_at = 1000u64;
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);

    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 1000,
        },
    );
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        )
        .unwrap();
    let consume_tx = delegate_wallet
        .sign_transaction_v2(
            1,
            owner_addr.clone(),
            valid_at,
            proof,
            Transaction::Consume {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        )
        .unwrap();

    let txs = vec![mint_tx.clone(), open_tx.clone(), consume_tx.clone()];
    let rctx = replay_ctx();

    let mut state_a = State::new();
    for tx in &txs {
        state_a = apply(
            &state_a,
            tx,
            &rctx,
            if matches!(tx.kind, Transaction::Mint { .. }) {
                Some(&minters)
            } else {
                None
            },
        )
        .unwrap();
    }

    let mut state_b = State::new();
    for tx in &txs {
        state_b = apply(
            &state_b,
            tx,
            &rctx,
            if matches!(tx.kind, Transaction::Mint { .. }) {
                Some(&minters)
            } else {
                None
            },
        )
        .unwrap();
    }

    assert_eq!(
        state_a, state_b,
        "same tx log replayed twice must yield identical state"
    );
    assert_eq!(state_a.get_account(&owner_addr).unwrap().nonce(), 2);
    assert_eq!(
        state_a
            .get_meter(&owner_addr, "storage")
            .unwrap()
            .total_units(),
        10
    );
}

/// Revocation: owner revokes capability; delegated Consume with that capability is rejected (DelegationRevoked).
#[test]
fn test_phase3_revocation_rejected_consume() {
    use metering_chain::tx::validation::capability_id;
    use metering_chain::tx::DelegationProofMinimal;

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let minters = get_authorized_minters();
    let mut state = State::new();

    let mint_tx = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: owner_addr.clone(),
            amount: 2000,
        },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet
        .sign_transaction(
            0,
            Transaction::OpenMeter {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        )
        .unwrap();
    state = apply(&state, &open_tx, &replay_ctx(), Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 1);

    let valid_at = 1000u64;
    let claims = DelegationProofMinimal {
        iat: 0,
        exp: 2000,
        issuer: owner_addr.clone(),
        audience: delegate_addr.clone(),
        service_id: "storage".to_string(),
        ability: None,
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let cap_id = capability_id(&proof);
    let live_ctx = ValidationContext::live(valid_at, 300);

    // Delegate consumes once -> success
    let consume1 = delegate_wallet
        .sign_transaction_v2(
            1,
            owner_addr.clone(),
            valid_at,
            proof.clone(),
            Transaction::Consume {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        )
        .unwrap();
    state = apply(&state, &consume1, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);

    // Owner revokes the capability
    let revoke_tx = owner_wallet
        .sign_transaction(
            2,
            Transaction::RevokeDelegation {
                owner: owner_addr.clone(),
                capability_id: cap_id.clone(),
            },
        )
        .unwrap();
    state = apply(&state, &revoke_tx, &replay_ctx(), Some(&minters)).unwrap();
    assert!(state.is_capability_revoked(&cap_id));
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 3);

    // Delegate tries to consume again with same proof -> DelegationRevoked
    let consume2 = delegate_wallet
        .sign_transaction_v2(
            3,
            owner_addr.clone(),
            valid_at,
            proof,
            Transaction::Consume {
                owner: owner_addr.clone(),
                service_id: "storage".to_string(),
                units: 5,
                pricing: Pricing::UnitPrice(5),
            },
        )
        .unwrap();
    let res = apply(&state, &consume2, &live_ctx, Some(&minters));
    assert!(res.is_err());
    match res.unwrap_err() {
        Error::DelegationRevoked => {}
        e => panic!("expected DelegationRevoked, got {:?}", e),
    }
}

// --- Pre-Phase 4 refactoring regression baseline ---

/// Pre-Phase 4: determinism and invariant regression baseline.
/// Replays same tx log twice and asserts identical state. Documents extension points for Phase 4.
#[test]
fn test_pre_phase4_replay_determinism_baseline() {
    let minters = get_authorized_minters();

    let txs = vec![
        SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
    ];

    let rctx = replay_ctx();
    let mut state_a = State::new();
    for tx in &txs {
        state_a = apply(
            &state_a,
            tx,
            &rctx,
            if matches!(tx.kind, Transaction::Mint { .. }) {
                Some(&minters)
            } else {
                None
            },
        )
        .unwrap();
    }

    let mut state_b = State::new();
    for tx in &txs {
        state_b = apply(
            &state_b,
            tx,
            &rctx,
            if matches!(tx.kind, Transaction::Mint { .. }) {
                Some(&minters)
            } else {
                None
            },
        )
        .unwrap();
    }

    assert_eq!(
        state_a, state_b,
        "Pre-Phase 4: same tx log replayed twice must yield identical state"
    );
}

/// Pre-Phase 4: error_code() returns deterministic codes for UI mapping.
#[test]
fn test_pre_phase4_error_code_deterministic() {
    assert_eq!(Error::DelegationRevoked.error_code(), "DELEGATION_REVOKED");
    assert_eq!(
        Error::DelegatedConsumeRequiresV2.error_code(),
        "DELEGATED_CONSUME_REQUIRES_V2"
    );
}

// --- G1 Phase 4A: Settlement MVP ---

/// G1: propose → finalize → submit claim → pay claim flow.
#[test]
fn test_g1_settlement_flow_propose_finalize_claim_pay() {
    use metering_chain::evidence;
    use metering_chain::state::{ClaimStatus, SettlementStatus};

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    // 1. Create usage: mint, open meter, consume
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &rctx, Some(&minters)).unwrap();
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &rctx, Some(&minters)).unwrap();
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &rctx, Some(&minters)).unwrap();
    let gross_spent = 50u64; // 10 * 5
    let meter = state.get_meter("alice", "storage").unwrap();
    assert_eq!(meter.total_spent(), gross_spent);

    // 2. Propose settlement (operator 90%, protocol 10%, reserve 0)
    let operator_share = 45u64;
    let protocol_fee = 5u64;
    let reserve_locked = 0u64;
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");
    let authority_nonce = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let tx_propose = SignedTx::new(
        "authority".to_string(),
        authority_nonce,
        Transaction::ProposeSettlement {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
            from_tx_id: 0,
            to_tx_id: 3,
            gross_spent,
            operator_share,
            protocol_fee,
            reserve_locked,
            evidence_hash: ev_hash.clone(),
        },
    );
    state = apply(&state, &tx_propose, &rctx, Some(&minters)).unwrap();
    let sid = metering_chain::state::SettlementId::new(
        "alice".to_string(),
        "storage".to_string(),
        "w1".to_string(),
    );
    let s = state.get_settlement(&sid).unwrap();
    assert_eq!(s.status, SettlementStatus::Proposed);
    assert_eq!(s.gross_spent, gross_spent);
    assert_eq!(s.operator_share, operator_share);

    // 3. Finalize settlement
    let authority_nonce = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let tx_finalize = SignedTx::new(
        "authority".to_string(),
        authority_nonce,
        Transaction::FinalizeSettlement {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
        },
    );
    state = apply(&state, &tx_finalize, &rctx, Some(&minters)).unwrap();
    let s = state.get_settlement(&sid).unwrap();
    assert!(s.is_finalized());
    assert_eq!(s.payable(), operator_share);

    // 4. Submit claim (operator = alice, claim full amount)
    let cid = metering_chain::state::ClaimId::new("alice".to_string(), &sid);
    assert!(state.get_claim(&cid).is_none());
    let tx_claim = SignedTx::new(
        "alice".to_string(),
        2, // alice nonce after consume
        Transaction::SubmitClaim {
            operator: "alice".to_string(),
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
            claim_amount: operator_share,
        },
    );
    let alice_bal_before = state.get_account("alice").unwrap().balance();
    state = apply(&state, &tx_claim, &rctx, None).unwrap();
    let c = state.get_claim(&cid).unwrap();
    assert_eq!(c.status, ClaimStatus::Pending);

    // 5. Pay claim (protocol/admin signs)
    let authority_nonce = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let tx_pay = SignedTx::new(
        "authority".to_string(),
        authority_nonce,
        Transaction::PayClaim {
            operator: "alice".to_string(),
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
        },
    );
    state = apply(&state, &tx_pay, &rctx, Some(&minters)).unwrap();
    let alice_bal_after = state.get_account("alice").unwrap().balance();
    assert_eq!(alice_bal_after, alice_bal_before + operator_share);
    let c = state.get_claim(&cid).unwrap();
    assert_eq!(c.status, ClaimStatus::Paid);
    let s = state.get_settlement(&sid).unwrap();
    assert_eq!(s.total_paid, operator_share);
}

/// Regression: PayClaim rejects when claim_amount exceeds remaining payable
/// after a prior payment (prevents overpay).
#[test]
fn test_g1_pay_claim_rejects_overpay_after_partial_payment() {
    use metering_chain::evidence;
    use metering_chain::state::SettlementId;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    // 1. Usage + propose + finalize
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &rctx, Some(&minters)).unwrap();
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &rctx, Some(&minters)).unwrap();
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &rctx, Some(&minters)).unwrap();
    let gross_spent = 50u64;
    let operator_share = 45u64; // total payable pool (must satisfy gross_spent == operator_share + protocol_fee + reserve_locked)
    let protocol_fee = 5u64;
    let reserve_locked = 0u64;
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());

    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent,
                operator_share,
                protocol_fee,
                reserve_locked,
                evidence_hash: ev_hash.clone(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();

    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();

    // 2. Alice claims 30, Bob claims 40 (both valid at submit time: payable=45)
    let alice_n = state.get_account("alice").map(|a| a.nonce()).unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            alice_n,
            Transaction::SubmitClaim {
                operator: "alice".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                claim_amount: 30,
            },
        ),
        &rctx,
        None,
    )
    .unwrap();

    let bob_n = state.get_account("bob").map(|a| a.nonce()).unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "bob".to_string(),
            bob_n,
            Transaction::SubmitClaim {
                operator: "bob".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                claim_amount: 40,
            },
        ),
        &rctx,
        None,
    )
    .unwrap();

    // 3. Pay alice first: total_paid=30, payable=15
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PayClaim {
                operator: "alice".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();

    assert_eq!(state.get_settlement(&sid).unwrap().total_paid, 30);
    assert_eq!(state.get_settlement(&sid).unwrap().payable(), 15);

    // 4. Pay bob: claim_amount 40 > payable 15 → ClaimAmountExceedsPayable
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PayClaim {
                operator: "bob".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    );
    assert!(
        matches!(res, Err(Error::ClaimAmountExceedsPayable)),
        "expected ClaimAmountExceedsPayable, got {:?}",
        res
    );
}

// --- G2 Phase 4B: Dispute freeze + resolve ---

/// G2: Open dispute freezes payout; Resolve Dismissed allows PayClaim again.
#[test]
fn test_g2_dispute_freezes_payout_then_resolve_dismissed_allows_pay() {
    use metering_chain::evidence;
    use metering_chain::state::{DisputeId, DisputeStatus, SettlementId, SettlementStatus};

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    // 1. Usage + propose + finalize
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &rctx, Some(&minters)).unwrap();
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &rctx, Some(&minters)).unwrap();
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &rctx, Some(&minters)).unwrap();
    let gross_spent = 50u64;
    let operator_share = 45u64;
    let protocol_fee = 5u64;
    let reserve_locked = 0u64;
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());

    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent,
                operator_share,
                protocol_fee,
                reserve_locked,
                evidence_hash: ev_hash.clone(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();

    // 2. Submit claim
    let alice_n = state.get_account("alice").map(|a| a.nonce()).unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            alice_n,
            Transaction::SubmitClaim {
                operator: "alice".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                claim_amount: operator_share,
            },
        ),
        &rctx,
        None,
    )
    .unwrap();

    // 3. Open dispute → settlement disputed
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::OpenDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                reason_code: "evidence_mismatch".to_string(),
                evidence_hash: ev_hash,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    assert_eq!(
        state.get_settlement(&sid).unwrap().status,
        SettlementStatus::Disputed
    );
    let did = DisputeId::new(&sid);
    assert!(state.get_dispute(&did).unwrap().is_open());

    // 4. PayClaim rejected (payout frozen)
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PayClaim {
                operator: "alice".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    );
    assert!(
        res.is_err(),
        "PayClaim must fail while dispute is open; got {:?}",
        res
    );

    // 5. Resolve dispute Dismissed → settlement reverted to Finalized (G4: replay evidence)
    let s = state.get_settlement(&sid).unwrap();
    let replay_summary = metering_chain::evidence::ReplaySummary::new(
        s.from_tx_id,
        s.to_tx_id,
        s.to_tx_id.saturating_sub(s.from_tx_id),
        s.gross_spent,
        s.operator_share,
        s.protocol_fee,
        s.reserve_locked,
    );
    let replay_hash = replay_summary.replay_hash();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ResolveDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                verdict: DisputeVerdict::Dismissed,
                evidence_hash: s.evidence_hash.clone(),
                replay_hash: replay_hash.clone(),
                replay_summary: replay_summary.clone(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    assert_eq!(
        state.get_dispute(&did).unwrap().status,
        DisputeStatus::Dismissed
    );
    assert!(state.get_settlement(&sid).unwrap().is_finalized());

    // 6. PayClaim now succeeds
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PayClaim {
                operator: "alice".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    assert_eq!(
        state.get_settlement(&sid).unwrap().total_paid,
        operator_share
    );
}

/// G2: Resolve dispute Upheld → settlement stays Disputed, PayClaim remains rejected.
#[test]
fn test_g2_resolve_dispute_upheld_keeps_payout_frozen() {
    use metering_chain::evidence;
    use metering_chain::state::{DisputeId, DisputeStatus, SettlementId, SettlementStatus};

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    // 1. Usage + propose + finalize + submit claim
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &rctx, Some(&minters)).unwrap();
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &rctx, Some(&minters)).unwrap();
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &rctx, Some(&minters)).unwrap();
    let gross_spent = 50u64;
    let operator_share = 45u64;
    let protocol_fee = 5u64;
    let reserve_locked = 0u64;
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());

    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent,
                operator_share,
                protocol_fee,
                reserve_locked,
                evidence_hash: ev_hash.clone(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let alice_n = state.get_account("alice").map(|a| a.nonce()).unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            alice_n,
            Transaction::SubmitClaim {
                operator: "alice".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                claim_amount: operator_share,
            },
        ),
        &rctx,
        None,
    )
    .unwrap();

    // 2. Open dispute
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::OpenDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                reason_code: "evidence_mismatch".to_string(),
                evidence_hash: ev_hash,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let did = DisputeId::new(&sid);

    // 3. Resolve with Upheld → dispute closed, settlement stays Disputed (G4: replay evidence)
    let s = state.get_settlement(&sid).unwrap();
    let replay_summary = metering_chain::evidence::ReplaySummary::new(
        s.from_tx_id,
        s.to_tx_id,
        s.to_tx_id.saturating_sub(s.from_tx_id),
        s.gross_spent,
        s.operator_share,
        s.protocol_fee,
        s.reserve_locked,
    );
    let replay_hash = replay_summary.replay_hash();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ResolveDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                verdict: DisputeVerdict::Upheld,
                evidence_hash: s.evidence_hash.clone(),
                replay_hash: replay_hash.clone(),
                replay_summary: replay_summary.clone(),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    assert_eq!(
        state.get_dispute(&did).unwrap().status,
        DisputeStatus::Upheld
    );
    assert_eq!(
        state.get_settlement(&sid).unwrap().status,
        SettlementStatus::Disputed,
        "Upheld: settlement must remain Disputed (payouts stay frozen)"
    );

    // 4. PayClaim still rejected (payout frozen after Upheld)
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PayClaim {
                operator: "alice".to_string(),
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &rctx,
        Some(&minters),
    );
    assert!(
        res.is_err(),
        "PayClaim must remain rejected after Resolve Upheld; got {:?}",
        res
    );
}

// --- G3 Phase 4C: Policy ---

/// G3: Publish global policy v1; new settlement uses v1 split and has bound policy snapshot.
#[test]
fn test_g3_publish_global_policy_v1_applies_to_new_settlements() {
    use metering_chain::evidence;
    use metering_chain::state::{PolicyConfig, PolicyScope, SettlementId};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    // 1. Mint, open, consume (3 txs → next_tx_id = 3)
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &rctx, Some(&minters)).unwrap();
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &rctx, Some(&minters)).unwrap();
    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &rctx, Some(&minters)).unwrap();
    let gross_spent = 50u64;

    // 2. Publish global policy: 90% operator, 10% protocol, dispute_window 3600, effective at tx_id 3
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let config = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };
    assert!(config.validate());
    let mut ctx_publish = ValidationContext::replay();
    ctx_publish.next_tx_id = Some(3);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 1,
                effective_from_tx_id: 3,
                config: config.clone(),
            },
        ),
        &ctx_publish,
        Some(&minters),
    )
    .unwrap();

    // 3. Propose settlement with split matching policy (90% / 10%); use ctx with next_tx_id = 4 (after publish)
    let operator_share = 45u64;
    let protocol_fee = 5u64;
    let reserve_locked = 0u64;
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let mut ctx_propose = ValidationContext::replay();
    ctx_propose.next_tx_id = Some(4);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent,
                operator_share,
                protocol_fee,
                reserve_locked,
                evidence_hash: ev_hash,
            },
        ),
        &ctx_propose,
        Some(&minters),
    )
    .unwrap();

    let s = state.get_settlement(&sid).unwrap();
    assert_eq!(s.gross_spent, gross_spent);
    assert_eq!(s.operator_share, operator_share);
    assert_eq!(s.protocol_fee, protocol_fee);
    assert_eq!(
        s.policy_scope_key.as_deref(),
        Some("global"),
        "Settlement must have bound policy snapshot"
    );
    assert_eq!(s.policy_version, Some(1));
    assert_eq!(s.dispute_window_secs, Some(3600));
}

/// G3: OwnerService scope overrides Global.
#[test]
fn test_g3_owner_service_override_precedence() {
    use metering_chain::evidence;
    use metering_chain::state::{PolicyConfig, PolicyScope, SettlementId};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let gross_spent = 50u64;

    let global_cfg = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 5000,
            protocol_fee_bps: 5000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };
    let mut ctx = ValidationContext::replay();
    ctx.next_tx_id = Some(3);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 1,
                effective_from_tx_id: 3,
                config: global_cfg,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();

    let override_cfg = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 7200,
        },
    };
    ctx.next_tx_id = Some(4);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::OwnerService {
                    owner: "alice".to_string(),
                    service_id: "storage".to_string(),
                },
                version: 1,
                effective_from_tx_id: 4,
                config: override_cfg,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();

    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");
    ctx.next_tx_id = Some(5);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent,
                operator_share: 45,
                protocol_fee: 5,
                reserve_locked: 0,
                evidence_hash: ev_hash,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();

    let s = state.get_settlement(&sid).unwrap();
    assert_eq!(s.operator_share, 45);
    assert_eq!(s.protocol_fee, 5);
    assert_eq!(
        s.policy_scope_key.as_deref(),
        Some("owner_service:alice:storage")
    );
    assert_eq!(s.dispute_window_secs, Some(7200));
}

/// G3: v2 effective at 5; propose at next_tx_id 4 uses v1.
#[test]
fn test_g3_future_effective_tx_id_uses_old_policy_before_cutover() {
    use metering_chain::evidence;
    use metering_chain::state::{PolicyConfig, PolicyScope, SettlementId};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let gross_spent = 50u64;

    let mut ctx = ValidationContext::replay();
    ctx.next_tx_id = Some(3);
    let c1 = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 1,
                effective_from_tx_id: 3,
                config: c1,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();

    ctx.next_tx_id = Some(4);
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent,
                operator_share: 45,
                protocol_fee: 5,
                reserve_locked: 0,
                evidence_hash: ev_hash,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();

    let s = state.get_settlement(&sid).unwrap();
    assert_eq!(s.policy_version, Some(1));
    assert_eq!(s.operator_share, 45);

    ctx.next_tx_id = Some(5);
    let c2 = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 5000,
            protocol_fee_bps: 5000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let _ = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 2,
                effective_from_tx_id: 5,
                config: c2,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
}

/// G3: Dispute window from bound policy; beyond window rejected.
#[test]
fn test_g3_dispute_window_from_bound_policy_snapshot() {
    use metering_chain::evidence;
    use metering_chain::state::{PolicyConfig, PolicyScope, SettlementId};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();

    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();

    let cfg = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 50,
        },
    };
    let mut ctx = ValidationContext::replay();
    ctx.next_tx_id = Some(3);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 1,
                effective_from_tx_id: 3,
                config: cfg,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();

    let ev_hash = evidence::evidence_hash(b"alice:storage:w2:0:3");
    let _sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w2".to_string());
    ctx.next_tx_id = Some(4);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w2".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent: 50,
                operator_share: 45,
                protocol_fee: 5,
                reserve_locked: 0,
                evidence_hash: ev_hash,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();

    let mut live_ctx = ValidationContext::live(100, 300);
    live_ctx.next_tx_id = Some(5);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w2".to_string(),
            },
        ),
        &live_ctx,
        Some(&minters),
    )
    .unwrap();

    let mut ctx_out = ValidationContext::live(151, 300);
    ctx_out.next_tx_id = Some(6);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::OpenDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w2".to_string(),
                reason_code: "test".to_string(),
                evidence_hash: String::new(),
            },
        ),
        &ctx_out,
        Some(&minters),
    );
    assert!(
        res.is_err(),
        "OpenDispute must fail outside dispute window; got {:?}",
        res
    );
}

/// G3: Invalid publish (bps != 10000) → InvalidPolicyParameters.
#[test]
fn test_g3_invalid_publish_rejected_with_deterministic_error() {
    use metering_chain::state::{PolicyConfig, PolicyScope};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let state = State::new();
    let mut ctx = ValidationContext::replay();
    ctx.next_tx_id = Some(0);
    let bad_cfg = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 8000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 1,
                effective_from_tx_id: 0,
                config: bad_cfg,
            },
        ),
        &ctx,
        Some(&minters),
    );
    assert!(
        matches!(res, Err(Error::InvalidPolicyParameters)),
        "expected InvalidPolicyParameters, got {:?}",
        res
    );
}

/// G3: effective_from_tx_id < next_tx_id → RetroactivePolicyForbidden.
#[test]
fn test_g3_retroactive_policy_forbidden() {
    use metering_chain::state::{PolicyConfig, PolicyScope};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let mut state = State::new();
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 100,
            },
        ),
        &replay_ctx(),
        Some(&minters),
    )
    .unwrap();
    let mut ctx = ValidationContext::replay();
    ctx.next_tx_id = Some(1);
    let cfg = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 1,
                effective_from_tx_id: 0,
                config: cfg,
            },
        ),
        &ctx,
        Some(&minters),
    );
    assert!(
        matches!(res, Err(Error::RetroactivePolicyForbidden)),
        "expected RetroactivePolicyForbidden, got {:?}",
        res
    );
}

/// G3: Version must be strictly greater than latest for scope (monotonic).
#[test]
fn test_g3_publish_non_monotonic_version_rejected() {
    use metering_chain::state::{PolicyConfig, PolicyScope};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let mut ctx = ValidationContext::replay();
    ctx.next_tx_id = Some(1);
    let cfg = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 1,
                effective_from_tx_id: 1,
                config: cfg.clone(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    ctx.next_tx_id = Some(2);
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            1,
            Transaction::PublishPolicyVersion {
                scope: PolicyScope::Global,
                version: 0,
                effective_from_tx_id: 2,
                config: cfg,
            },
        ),
        &ctx,
        Some(&minters),
    );
    assert!(
        matches!(res, Err(Error::PolicyVersionConflict)),
        "expected PolicyVersionConflict (non-monotonic version), got {:?}",
        res
    );
}

/// G3: Replaying same tx sequence yields identical policy selection and settlement totals.
#[test]
fn test_g3_replay_reconstructs_identical_policy_selection() {
    use metering_chain::evidence;
    use metering_chain::state::{PolicyConfig, PolicyScope, SettlementId};
    use metering_chain::tx::validation::ValidationContext;

    let minters = get_authorized_minters();
    let rctx = replay_ctx();
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");

    let cfg = PolicyConfig {
        fee_policy: metering_chain::state::FeePolicy {
            operator_share_bps: 9000,
            protocol_fee_bps: 1000,
        },
        reserve_policy: metering_chain::state::ReservePolicy::None,
        dispute_policy: metering_chain::state::DisputePolicy {
            dispute_window_secs: 3600,
        },
    };

    fn run_sequence(
        minters: &std::collections::HashSet<String>,
        rctx: &ValidationContext,
        cfg: &PolicyConfig,
        ev_hash: &str,
        gross_spent: u64,
    ) -> State {
        let mut state = State::new();
        state = apply(
            &state,
            &SignedTx::new(
                "authority".to_string(),
                0,
                Transaction::Mint {
                    to: "alice".to_string(),
                    amount: 1000,
                },
            ),
            rctx,
            Some(minters),
        )
        .unwrap();
        state = apply(
            &state,
            &SignedTx::new(
                "alice".to_string(),
                0,
                Transaction::OpenMeter {
                    owner: "alice".to_string(),
                    service_id: "storage".to_string(),
                    deposit: 100,
                },
            ),
            rctx,
            Some(minters),
        )
        .unwrap();
        state = apply(
            &state,
            &SignedTx::new(
                "alice".to_string(),
                1,
                Transaction::Consume {
                    owner: "alice".to_string(),
                    service_id: "storage".to_string(),
                    units: 10,
                    pricing: Pricing::UnitPrice(5),
                },
            ),
            rctx,
            Some(minters),
        )
        .unwrap();
        let mut ctx = ValidationContext::replay();
        ctx.next_tx_id = Some(3);
        let auth_n = state
            .get_account("authority")
            .map(|a| a.nonce())
            .unwrap_or(0);
        state = apply(
            &state,
            &SignedTx::new(
                "authority".to_string(),
                auth_n,
                Transaction::PublishPolicyVersion {
                    scope: PolicyScope::Global,
                    version: 1,
                    effective_from_tx_id: 3,
                    config: cfg.clone(),
                },
            ),
            &ctx,
            Some(minters),
        )
        .unwrap();
        ctx.next_tx_id = Some(4);
        let auth_n = state
            .get_account("authority")
            .map(|a| a.nonce())
            .unwrap_or(0);
        state = apply(
            &state,
            &SignedTx::new(
                "authority".to_string(),
                auth_n,
                Transaction::ProposeSettlement {
                    owner: "alice".to_string(),
                    service_id: "storage".to_string(),
                    window_id: "w1".to_string(),
                    from_tx_id: 0,
                    to_tx_id: 3,
                    gross_spent,
                    operator_share: 45,
                    protocol_fee: 5,
                    reserve_locked: 0,
                    evidence_hash: ev_hash.to_string(),
                },
            ),
            &ctx,
            Some(minters),
        )
        .unwrap();
        state
    }

    let ev_hash_s = ev_hash.as_str();
    let state1 = run_sequence(&minters, &rctx, &cfg, ev_hash_s, 50);
    let state2 = run_sequence(&minters, &rctx, &cfg, ev_hash_s, 50);

    let s1 = state1.get_settlement(&sid).unwrap();
    let s2 = state2.get_settlement(&sid).unwrap();

    assert_eq!(s1.policy_scope_key, s2.policy_scope_key);
    assert_eq!(s1.policy_version, s2.policy_version);
    assert_eq!(s1.gross_spent, s2.gross_spent);
    assert_eq!(s1.operator_share, s2.operator_share);
    assert_eq!(s1.protocol_fee, s2.protocol_fee);
    assert_eq!(s1.dispute_window_secs, s2.dispute_window_secs);
}

// --- G4 (Evidence Finality) ---

/// G4: ResolveDispute with empty replay_hash → InvalidEvidenceBundle.
#[test]
fn test_g4_resolve_dispute_requires_valid_evidence_bundle() {
    use metering_chain::evidence::ReplaySummary;
    use metering_chain::state::SettlementId;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let _sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let ev_hash = metering_chain::evidence::evidence_hash(b"alice:storage:w1:0:3");
    let mut ctx = metering_chain::tx::validation::ValidationContext::replay();
    ctx.next_tx_id = Some(4);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent: 50,
                operator_share: 45,
                protocol_fee: 5,
                reserve_locked: 0,
                evidence_hash: ev_hash,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    ctx.next_tx_id = Some(5);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::OpenDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                reason_code: "test".to_string(),
                evidence_hash: String::new(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let summary = ReplaySummary::new(0, 3, 3, 50, 45, 5, 0);
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            state
                .get_account("authority")
                .map(|a| a.nonce())
                .unwrap_or(0),
            Transaction::ResolveDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                verdict: metering_chain::tx::DisputeVerdict::Dismissed,
                evidence_hash: String::new(),
                replay_hash: String::new(),
                replay_summary: summary,
            },
        ),
        &ctx,
        Some(&minters),
    );
    assert!(
        matches!(res, Err(Error::InvalidEvidenceBundle)),
        "expected InvalidEvidenceBundle, got {:?}",
        res
    );
}

/// G4: ResolveDispute with replay_summary totals not matching settlement → ReplayMismatch.
#[test]
fn test_g4_resolve_dispute_rejects_replay_mismatch() {
    use metering_chain::evidence::ReplaySummary;
    use metering_chain::state::SettlementId;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let _sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let ev_hash = metering_chain::evidence::evidence_hash(b"alice:storage:w1:0:3");
    let mut ctx = metering_chain::tx::validation::ValidationContext::replay();
    ctx.next_tx_id = Some(4);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent: 50,
                operator_share: 45,
                protocol_fee: 5,
                reserve_locked: 0,
                evidence_hash: ev_hash,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    ctx.next_tx_id = Some(5);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::OpenDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                reason_code: "test".to_string(),
                evidence_hash: String::new(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let s = state.get_settlement(&sid).unwrap();
    let wrong_summary = ReplaySummary::new(0, 3, 3, 99, 45, 5, 0);
    let replay_hash = wrong_summary.replay_hash();
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            state
                .get_account("authority")
                .map(|a| a.nonce())
                .unwrap_or(0),
            Transaction::ResolveDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                verdict: metering_chain::tx::DisputeVerdict::Dismissed,
                evidence_hash: s.evidence_hash.clone(),
                replay_hash: replay_hash.clone(),
                replay_summary: wrong_summary,
            },
        ),
        &ctx,
        Some(&minters),
    );
    assert!(
        matches!(res, Err(Error::ReplayMismatch)),
        "expected ReplayMismatch, got {:?}",
        res
    );
}

/// G4: ResolveDispute with replay_summary window not matching settlement → ReplayMismatch (no self-filled summary bypass).
#[test]
fn test_g4_resolve_dispute_rejects_wrong_window() {
    use metering_chain::evidence::ReplaySummary;
    use metering_chain::state::SettlementId;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let ev_hash = metering_chain::evidence::evidence_hash(b"alice:storage:w1:0:3");
    let mut ctx = metering_chain::tx::validation::ValidationContext::replay();
    ctx.next_tx_id = Some(4);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent: 50,
                operator_share: 45,
                protocol_fee: 5,
                reserve_locked: 0,
                evidence_hash: ev_hash,
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    ctx.next_tx_id = Some(5);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::OpenDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                reason_code: "test".to_string(),
                evidence_hash: String::new(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let s = state.get_settlement(&sid).unwrap();
    // Correct totals but wrong window (1..4 instead of 0..3) → must be rejected
    let wrong_window_summary = ReplaySummary::new(1, 4, 3, 50, 45, 5, 0);
    let replay_hash = wrong_window_summary.replay_hash();
    let res = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            state
                .get_account("authority")
                .map(|a| a.nonce())
                .unwrap_or(0),
            Transaction::ResolveDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                verdict: metering_chain::tx::DisputeVerdict::Dismissed,
                evidence_hash: s.evidence_hash.clone(),
                replay_hash,
                replay_summary: wrong_window_summary,
            },
        ),
        &ctx,
        Some(&minters),
    );
    assert!(
        matches!(res, Err(Error::ReplayMismatch)),
        "expected ReplayMismatch (wrong window), got {:?}",
        res
    );
}

/// G4: ResolveDispute with matching replay stores resolution_audit on dispute.
#[test]
fn test_g4_resolve_dispute_accepts_matching_replay() {
    use metering_chain::evidence::ReplaySummary;
    use metering_chain::state::SettlementId;

    let minters = get_authorized_minters();
    let mut state = State::new();
    let rctx = replay_ctx();
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    state = apply(
        &state,
        &SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
        &rctx,
        Some(&minters),
    )
    .unwrap();
    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let ev_hash = metering_chain::evidence::evidence_hash(b"alice:storage:w1:0:3");
    let mut ctx = metering_chain::tx::validation::ValidationContext::replay();
    ctx.next_tx_id = Some(4);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ProposeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                from_tx_id: 0,
                to_tx_id: 3,
                gross_spent: 50,
                operator_share: 45,
                protocol_fee: 5,
                reserve_locked: 0,
                evidence_hash: ev_hash.clone(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    ctx.next_tx_id = Some(5);
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::FinalizeSettlement {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::OpenDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                reason_code: "test".to_string(),
                evidence_hash: String::new(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let s = state.get_settlement(&sid).unwrap();
    let replay_summary = ReplaySummary::new(
        s.from_tx_id,
        s.to_tx_id,
        s.to_tx_id.saturating_sub(s.from_tx_id),
        s.gross_spent,
        s.operator_share,
        s.protocol_fee,
        s.reserve_locked,
    );
    let replay_hash = replay_summary.replay_hash();
    let auth_n = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    state = apply(
        &state,
        &SignedTx::new(
            "authority".to_string(),
            auth_n,
            Transaction::ResolveDispute {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                window_id: "w1".to_string(),
                verdict: metering_chain::tx::DisputeVerdict::Dismissed,
                evidence_hash: s.evidence_hash.clone(),
                replay_hash: replay_hash.clone(),
                replay_summary: replay_summary.clone(),
            },
        ),
        &ctx,
        Some(&minters),
    )
    .unwrap();
    let did = metering_chain::state::DisputeId::new(&sid);
    let d = state.get_dispute(&did).unwrap();
    assert!(
        d.resolution_audit.is_some(),
        "G4: resolution_audit must be set"
    );
    let audit = d.resolution_audit.as_ref().unwrap();
    assert_eq!(audit.replay_hash, replay_hash);
    assert_eq!(audit.replay_summary.gross_spent, 50);
    let bundle = state.get_evidence_bundle(&sid).unwrap();
    assert_eq!(bundle.replay_hash, replay_hash);
    assert_eq!(bundle.settlement_key, sid.key());
}

/// G4: EvidenceBundle canonical serialization yields deterministic bundle_hash; roundtrip preserves hash.
#[test]
fn test_g4_evidence_bundle_roundtrip_deterministic_hash() {
    use metering_chain::evidence::{EvidenceBundle, ReplaySummary};

    let summary = ReplaySummary::new(0, 3, 3, 50, 45, 5, 0);
    let replay_hash = summary.replay_hash();
    let bundle = EvidenceBundle {
        schema_version: metering_chain::evidence::CURRENT_EVIDENCE_SCHEMA_VERSION,
        replay_protocol_version: metering_chain::evidence::REPLAY_PROTOCOL_VERSION,
        settlement_key: "alice:storage:w1".to_string(),
        from_tx_id: 0,
        to_tx_id: 3,
        evidence_hash: "abc123".to_string(),
        replay_hash: replay_hash.clone(),
        replay_summary: summary.clone(),
    };
    let h1 = bundle.bundle_hash();
    let h2 = bundle.bundle_hash();
    assert_eq!(h1, h2, "bundle_hash must be deterministic");

    let bytes = bundle.canonical_bytes();
    let restored: EvidenceBundle = bincode::deserialize(&bytes).unwrap();
    assert_eq!(
        restored.bundle_hash(),
        h1,
        "roundtrip must preserve bundle_hash"
    );
    assert!(restored.validate_shape().is_ok());
}

/// G4: replay_slice_to_summary yields same replay_hash when called twice for same window (determinism).
#[test]
fn test_g4_replay_hash_determinism_across_replay_to_tip() {
    use metering_chain::tx::Pricing;

    let (mut storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();
    let mut next_tx_id = 0u64;
    let rctx = replay_ctx();

    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &rctx, Some(&minters)).unwrap();
    storage.append_tx(&tx1).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &rctx, Some(&minters)).unwrap();
    storage.append_tx(&tx2).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::Consume {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            units: 10,
            pricing: Pricing::UnitPrice(5),
        },
    );
    state = apply(&state, &tx3, &rctx, Some(&minters)).unwrap();
    storage.append_tx(&tx3).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    let (summary1, ev1) =
        replay::replay_slice_to_summary(&storage, 0, 3, "alice", "storage", 45, 5, 0).unwrap();
    let (summary2, ev2) =
        replay::replay_slice_to_summary(&storage, 0, 3, "alice", "storage", 45, 5, 0).unwrap();
    assert_eq!(
        summary1.replay_hash(),
        summary2.replay_hash(),
        "replay_hash determinism"
    );
    assert_eq!(ev1, ev2);
    assert_eq!(summary1.gross_spent, 50);
}

/// G4: After ResolveDispute, persisted state replayed via replay_to_tip has same resolution_audit.
#[test]
fn test_g4_audit_fields_persist_and_replay_consistent() {
    use metering_chain::evidence::ReplaySummary;
    use metering_chain::state::SettlementId;
    use metering_chain::tx::Pricing;

    let (mut storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();
    let mut next_tx_id = 0u64;
    let mut ctx = metering_chain::tx::validation::ValidationContext::replay();

    let txs = [
        SignedTx::new(
            "authority".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1000,
            },
        ),
        SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                deposit: 100,
            },
        ),
        SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "storage".to_string(),
                units: 10,
                pricing: Pricing::UnitPrice(5),
            },
        ),
    ];
    for tx in &txs {
        ctx.next_tx_id = Some(next_tx_id);
        state = apply(&state, tx, &ctx, Some(&minters)).unwrap();
        storage.append_tx(tx).unwrap();
        next_tx_id += 1;
        storage.persist_state(&state, next_tx_id).unwrap();
    }
    let ev_hash = metering_chain::evidence::evidence_hash(b"alice:storage:w1:0:3");
    ctx.next_tx_id = Some(next_tx_id);
    let tx_propose = SignedTx::new(
        "authority".to_string(),
        state
            .get_account("authority")
            .map(|a| a.nonce())
            .unwrap_or(0),
        Transaction::ProposeSettlement {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
            from_tx_id: 0,
            to_tx_id: 3,
            gross_spent: 50,
            operator_share: 45,
            protocol_fee: 5,
            reserve_locked: 0,
            evidence_hash: ev_hash,
        },
    );
    state = apply(&state, &tx_propose, &ctx, Some(&minters)).unwrap();
    storage.append_tx(&tx_propose).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    ctx.next_tx_id = Some(next_tx_id);
    let tx_finalize = SignedTx::new(
        "authority".to_string(),
        state
            .get_account("authority")
            .map(|a| a.nonce())
            .unwrap_or(0),
        Transaction::FinalizeSettlement {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
        },
    );
    state = apply(&state, &tx_finalize, &ctx, Some(&minters)).unwrap();
    storage.append_tx(&tx_finalize).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    ctx.next_tx_id = Some(next_tx_id);
    let tx_open = SignedTx::new(
        "authority".to_string(),
        state
            .get_account("authority")
            .map(|a| a.nonce())
            .unwrap_or(0),
        Transaction::OpenDispute {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
            reason_code: "test".to_string(),
            evidence_hash: String::new(),
        },
    );
    state = apply(&state, &tx_open, &ctx, Some(&minters)).unwrap();
    storage.append_tx(&tx_open).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let s = state.get_settlement(&sid).unwrap();
    let replay_summary = ReplaySummary::new(
        s.from_tx_id,
        s.to_tx_id,
        s.to_tx_id.saturating_sub(s.from_tx_id),
        s.gross_spent,
        s.operator_share,
        s.protocol_fee,
        s.reserve_locked,
    );
    let replay_hash = replay_summary.replay_hash();
    ctx.next_tx_id = Some(next_tx_id);
    let tx_resolve = SignedTx::new(
        "authority".to_string(),
        state
            .get_account("authority")
            .map(|a| a.nonce())
            .unwrap_or(0),
        Transaction::ResolveDispute {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
            verdict: metering_chain::tx::DisputeVerdict::Dismissed,
            evidence_hash: s.evidence_hash.clone(),
            replay_hash: replay_hash.clone(),
            replay_summary: replay_summary.clone(),
        },
    );
    state = apply(&state, &tx_resolve, &ctx, Some(&minters)).unwrap();
    storage.append_tx(&tx_resolve).unwrap();
    next_tx_id += 1;
    storage.persist_state(&state, next_tx_id).unwrap();

    let (replayed_state, replayed_next) = replay::replay_to_tip(&storage).unwrap();
    assert_eq!(replayed_next, next_tx_id);
    let audit = replayed_state
        .get_dispute_resolution_audit(&sid)
        .expect("resolution_audit must exist after replay");
    assert_eq!(audit.replay_hash, replay_hash);
    assert_eq!(audit.replay_summary.gross_spent, replay_summary.gross_spent);
    assert_eq!(audit.replay_summary.from_tx_id, replay_summary.from_tx_id);
    assert_eq!(audit.replay_summary.to_tx_id, replay_summary.to_tx_id);
}
