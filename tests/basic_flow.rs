use metering_chain::error::Error;
use metering_chain::state::{apply, State};
use metering_chain::storage::{FileStorage, Storage};
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use metering_chain::wallet::{verify_signature, Wallet};
use std::collections::HashSet;
use tempfile::TempDir;

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
    match storage.load_state().unwrap() {
        Some((snapshot_state, snapshot_tx_id)) => {
            let txs_after_snapshot = storage.load_txs_from(snapshot_tx_id).unwrap();
            let mut current_state = snapshot_state;
            let mut current_tx_id = snapshot_tx_id;
            for tx in txs_after_snapshot {
                current_state = apply(&current_state, &tx, None).unwrap();
                current_tx_id += 1;
            }
            (current_state, current_tx_id)
        }
        None => {
            let all_txs = storage.load_txs_from(0).unwrap();
            let mut current_state = State::new();
            let mut current_tx_id = 0u64;
            for tx in all_txs {
                current_state = apply(&current_state, &tx, None).unwrap();
                current_tx_id += 1;
            }
            (current_state, current_tx_id)
        }
    }
}

/// Test the complete happy path: Mint → OpenMeter → Consume → CloseMeter
#[test]
fn test_happy_path_end_to_end() {
    let (mut storage, _temp_dir) = create_test_storage();
    let minters = get_authorized_minters();
    let mut state = State::new();
    let mut tx_id = 0u64;

    // 1. Mint: Authority mints 1000 to alice
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, Some(&minters)).unwrap();
    storage.append_tx(&tx1).unwrap();
    tx_id += 1;
    storage.persist_state(&state, tx_id).unwrap();

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
    state = apply(&state, &tx2, Some(&minters)).unwrap();
    storage.append_tx(&tx2).unwrap();
    tx_id += 1;
    storage.persist_state(&state, tx_id).unwrap();

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
    state = apply(&state, &tx3, Some(&minters)).unwrap();
    storage.append_tx(&tx3).unwrap();
    tx_id += 1;
    storage.persist_state(&state, tx_id).unwrap();

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
    state = apply(&state, &tx4, Some(&minters)).unwrap();
    storage.append_tx(&tx4).unwrap();
    tx_id += 1;
    storage.persist_state(&state, tx_id).unwrap();

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
    state = apply(&state, &tx5, Some(&minters)).unwrap();
    storage.append_tx(&tx5).unwrap();
    tx_id += 1;
    storage.persist_state(&state, tx_id).unwrap();

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
    let mut tx_id = 0u64;

    // Apply transactions without persisting state (simulating crash)
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, Some(&minters)).unwrap();
    storage.append_tx(&tx1).unwrap();
    tx_id += 1;

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, Some(&minters)).unwrap();
    storage.append_tx(&tx2).unwrap();
    tx_id += 1;

    // Persist state after tx2
    let snapshot_tx_id = tx_id;
    storage.persist_state(&state, snapshot_tx_id).unwrap();

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
    state = apply(&state, &tx3, Some(&minters)).unwrap();
    storage.append_tx(&tx3).unwrap();
    tx_id += 1;

    // Reconstruct state from snapshot + replay
    let (reconstructed_state, reconstructed_tx_id) = load_or_replay_state(&storage);

    // Verify reconstructed state matches current state
    // reconstructed_tx_id should be tx_id (snapshot at 2 + replay tx3 = 3)
    assert_eq!(
        reconstructed_tx_id, tx_id,
        "Reconstructed tx_id should match current tx_id"
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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, Some(&minters)).unwrap();

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
    state = apply(&state, &tx3, Some(&minters)).unwrap();

    // Close meter
    let tx4 = SignedTx::new(
        "alice".to_string(),
        2,
        Transaction::CloseMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
        },
    );
    state = apply(&state, &tx4, Some(&minters)).unwrap();

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
    state = apply(&state, &tx5, Some(&minters)).unwrap();

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
    state = apply(&state, &tx1, Some(&minters)).unwrap();
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
    state = apply(&state, &tx2, Some(&minters)).unwrap();
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

    let result = apply(&state, &tx3, Some(&minters));
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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

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

    let result = apply(&state, &tx2, Some(&minters));
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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 50,
        },
    );
    state = apply(&state, &tx2, Some(&minters)).unwrap();

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

    let result = apply(&state, &tx3, Some(&minters));
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

    let result = apply(&state, &tx, Some(&minters));
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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, Some(&minters)).unwrap();

    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::CloseMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
        },
    );
    state = apply(&state, &tx3, Some(&minters)).unwrap();

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

    let result = apply(&state, &tx4, Some(&minters));
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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "bob".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx2, Some(&minters)).unwrap();

    let tx3 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx3, Some(&minters)).unwrap();

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

    let result = apply(&state, &tx4, Some(&minters));
    assert!(result.is_err());
    match result.unwrap_err() {
        Error::InvalidTransaction(msg) => {
            assert!(msg.contains("Signer") && msg.contains("owner"));
        }
        _ => panic!("Expected InvalidTransaction error"),
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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, Some(&minters)).unwrap();

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

    let result = apply(&state, &tx3, Some(&minters));
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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "api_calls".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, Some(&minters)).unwrap();

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
    state = apply(&state, &tx3, Some(&minters)).unwrap();

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
    state = apply(&state, &tx1, Some(&minters)).unwrap();

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
    state = apply(&state, &tx2, Some(&minters)).unwrap();

    let tx3 = SignedTx::new(
        "alice".to_string(),
        1,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "api_calls".to_string(),
            deposit: 200,
        },
    );
    state = apply(&state, &tx3, Some(&minters)).unwrap();

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
    state = apply(&state, &tx4, Some(&minters)).unwrap();

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
    state = apply(&state, &tx5, Some(&minters)).unwrap();

    // Verify both meters are independent
    let storage_meter = state.get_meter("alice", "storage").unwrap();
    assert_eq!(storage_meter.total_units(), 10);
    assert_eq!(storage_meter.total_spent(), 50);

    let api_meter = state.get_meter("alice", "api_calls").unwrap();
    assert_eq!(api_meter.total_units(), 20);
    assert_eq!(api_meter.total_spent(), 30);

    assert_eq!(state.get_account("alice").unwrap().balance(), 1620); // 2000 - 100 - 200 - 50 - 30
}

/// Phase 2: signed tx apply success (wallet sign → verify → apply)
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

    let state = apply(&State::new(), &signed_tx, Some(&minters)).unwrap();
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
