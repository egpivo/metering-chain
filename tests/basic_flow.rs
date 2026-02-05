use metering_chain::error::Error;
use metering_chain::state::{apply, Account, Meter, State};
use metering_chain::storage::{FileStorage, Storage};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::{Pricing, SignedTx, Transaction};
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
    match storage.load_state().unwrap() {
        Some((snapshot_state, snapshot_tx_id)) => {
            let txs_after_snapshot = storage.load_txs_from(snapshot_tx_id).unwrap();
            let mut current_state = snapshot_state;
            let mut current_tx_id = snapshot_tx_id;
            for tx in txs_after_snapshot {
                current_state = apply(&current_state, &tx, &replay_ctx(), None).unwrap();
                current_tx_id += 1;
            }
            (current_state, current_tx_id)
        }
        None => {
            let all_txs = storage.load_txs_from(0).unwrap();
            let mut current_state = State::new();
            let mut current_tx_id = 0u64;
            for tx in all_txs {
                current_state = apply(&current_state, &tx, &replay_ctx(), None).unwrap();
                current_tx_id += 1;
            }
            (current_state, current_tx_id)
        }
    }
}

/// Test the complete happy path: Mint, OpenMeter, Consume, CloseMeter
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
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();
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
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();
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
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();
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
    state = apply(&state, &tx4, &replay_ctx(), Some(&minters)).unwrap();
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
    state = apply(&state, &tx5, &replay_ctx(), Some(&minters)).unwrap();
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
    state = apply(&state, &tx1, &replay_ctx(), Some(&minters)).unwrap();
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
    state = apply(&state, &tx2, &replay_ctx(), Some(&minters)).unwrap();
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
    state = apply(&state, &tx3, &replay_ctx(), Some(&minters)).unwrap();
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
        e => panic!("Expected InvalidTransaction or delegation error, got {:?}", e),
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
    state.accounts.insert("alice".to_string(), Account::with_balance(1000));
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
        e => panic!("expected DelegatedConsumeRequiresV2 (v2 gate in validation), got {:?}", e),
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
    let open_tx = owner_wallet.sign_transaction(0, Transaction::OpenMeter {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        deposit: 100,
    }).unwrap();
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
    let cost_opt = metering_chain::tx::validation::validate(
        &state,
        &delegated_tx,
        &live_ctx,
        Some(&minters),
    ).unwrap();
    assert_eq!(cost_opt, Some(50));

    state = apply(&state, &delegated_tx, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().balance(), 850);
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);
    assert_eq!(state.get_meter(&owner_addr, "storage").unwrap().total_units(), 10);
}

/// Delegated consume with proof.issuer as did:key (not 0x): verification uses principal_to_public_key, so did:key works.
#[test]
fn test_phase3_delegated_consume_issuer_did_key() {
    use metering_chain::tx::transaction::PAYLOAD_VERSION_V2;
    use metering_chain::tx::{build_signed_proof, delegation_claims_to_sign, DelegationProofMinimal};

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
        Transaction::Mint { to: owner_addr.clone(), amount: 1000 },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet.sign_transaction(0, Transaction::OpenMeter {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        deposit: 100,
    }).unwrap();
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
    let cost_opt = metering_chain::tx::validation::validate(
        &state,
        &delegated_tx,
        &live_ctx,
        Some(&minters),
    ).unwrap();
    assert_eq!(cost_opt, Some(50));
    state = apply(&state, &delegated_tx, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);
    assert_eq!(state.get_meter(&owner_addr, "storage").unwrap().total_units(), 10);
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
    state.accounts.insert(owner_addr.clone(), Account::with_balance(1000));
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
    };
    let ctx_no_max_age = ValidationContext {
        mode: ValidationMode::Live,
        now: Some(100),
        max_age: None,
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
    state.accounts.insert(owner_addr.clone(), Account::with_balance(1000));
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
    assert!(res.is_ok(), "Replay should accept valid_at within iat/exp without now: {:?}", res);
}

/// Forged proof (signed by delegate instead of owner) is rejected.
#[test]
fn test_phase3_forged_proof_rejected() {
    use metering_chain::tx::{build_signed_proof, delegation_claims_to_sign, DelegationProofMinimal};

    let owner_wallet = Wallet::new_random();
    let owner_addr = owner_wallet.address().to_string();
    let delegate_wallet = Wallet::new_random();
    let delegate_addr = delegate_wallet.address().to_string();

    let mut state = State::new();
    state.accounts.insert(owner_addr.clone(), Account::with_balance(1000));
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
    assert!(res.is_err(), "Forged proof (signed by delegate) must be rejected: {:?}", res);
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
    state.accounts.insert(owner_addr.clone(), Account::with_balance(1000));
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
    assert!(res.is_err(), "service_id mismatch must be rejected: {:?}", res);
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
    state.accounts.insert(owner_addr.clone(), Account::with_balance(1000));
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
        Transaction::Mint { to: owner_addr.clone(), amount: 1000 },
    );
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet.sign_transaction(0, Transaction::OpenMeter {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        deposit: 100,
    }).unwrap();
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
    let cost_opt = metering_chain::tx::validation::validate(
        &state,
        &delegated_tx,
        &live_ctx,
        Some(&minters),
    ).unwrap();
    assert_eq!(cost_opt, Some(20));
    state = apply(&state, &delegated_tx, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_meter(&owner_addr, "storage").unwrap().total_units(), 5);
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

    let mint_tx = SignedTx::new("authority".to_string(), 0, Transaction::Mint { to: owner_addr.clone(), amount: 1000 });
    state = apply(&state, &mint_tx, &replay_ctx(), Some(&minters)).unwrap();
    let open_tx = owner_wallet.sign_transaction(0, Transaction::OpenMeter {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        deposit: 100,
    }).unwrap();
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

    let consume1 = delegate_wallet.sign_transaction_v2(
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
    ).unwrap();
    let live_ctx = ValidationContext::live(valid_at, 300);
    state = apply(&state, &consume1, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().balance(), 870);

    let consume2 = delegate_wallet.sign_transaction_v2(
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
    ).unwrap();
    let res = metering_chain::tx::validation::validate(&state, &consume2, &live_ctx, Some(&minters));
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
    state.accounts.insert("alice".to_string(), Account::with_balance(1000));
    state.accounts.insert("bob".to_string(), Account::with_balance(1000));
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
    let open_tx = owner_wallet.sign_transaction(0, Transaction::OpenMeter {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        deposit: 100,
    }).unwrap();
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
        .sign_transaction_v2(1, owner_addr.clone(), valid_at, proof_d1.clone(), consume_kind(10))
        .unwrap();
    state = apply(&state, &tx1, &live_ctx, Some(&minters)).unwrap();
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 2);

    // Delegate2 tries owner nonce 1 (already used); rejected
    let tx2_same_nonce = delegate2_wallet
        .sign_transaction_v2(1, owner_addr.clone(), valid_at, proof_d2.clone(), consume_kind(5))
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
    assert_eq!(state.get_meter(&owner_addr, "storage").unwrap().total_units(), 15);
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
    let open_tx = owner_wallet.sign_transaction(0, Transaction::OpenMeter {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        deposit: 50,
    }).unwrap();
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
        e => panic!("expected InvalidTransaction (insufficient balance), got {:?}", e),
    }
    assert_eq!(state.get_account(&owner_addr).unwrap().nonce(), 1, "nonce unchanged after failed apply");
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
    assert_eq!(state.get_meter(&owner_addr, "storage").unwrap().total_units(), 20);
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
        .sign_transaction(0, Transaction::OpenMeter {
            owner: owner_addr.clone(),
            service_id: "storage".to_string(),
            deposit: 100,
        })
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

    assert_eq!(state_a, state_b, "same tx log replayed twice must yield identical state");
    assert_eq!(state_a.get_account(&owner_addr).unwrap().nonce(), 2);
    assert_eq!(state_a.get_meter(&owner_addr, "storage").unwrap().total_units(), 10);
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
    let open_tx = owner_wallet.sign_transaction(0, Transaction::OpenMeter {
        owner: owner_addr.clone(),
        service_id: "storage".to_string(),
        deposit: 100,
    }).unwrap();
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
