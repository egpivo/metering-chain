use metering_chain::config::Config;
use metering_chain::error::Error;
use metering_chain::replay::replay_to_tip;
use metering_chain::state::State;
use metering_chain::storage::{FileStorage, Storage};
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use tempfile::TempDir;

fn test_storage() -> (FileStorage, TempDir) {
    let td = TempDir::new().expect("tempdir");
    let cfg = Config::with_data_dir(td.path().to_path_buf());
    (FileStorage::new(&cfg), td)
}

fn base_txs() -> Vec<SignedTx> {
    vec![
        SignedTx::new(
            "minter".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 1_000,
            },
        ),
        SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                deposit: 100,
            },
        ),
        SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                units: 5,
                pricing: Pricing::UnitPrice(3),
            },
        ),
    ]
}

#[test]
fn test_recovery_crash_after_append_before_snapshot_persist() {
    let (mut storage, _td) = test_storage();
    let txs = base_txs();
    storage.append_tx(&txs[0]).expect("append tx0");
    storage.append_tx(&txs[1]).expect("append tx1");

    // No snapshot persisted -> replay from tx.log only.
    let (state, next_tx_id) = replay_to_tip(&storage).expect("replay to tip");
    assert_eq!(next_tx_id, 2);
    let alice = state.get_account("alice").expect("alice exists");
    assert_eq!(alice.nonce(), 1, "open meter should consume alice nonce");
}

#[test]
fn test_recovery_crash_after_snapshot_persist_before_next_append() {
    let (mut storage, _td) = test_storage();
    let txs = base_txs();
    storage.append_tx(&txs[0]).expect("append tx0");
    storage.append_tx(&txs[1]).expect("append tx1");

    let (snap_state, snap_next) = replay_to_tip(&storage).expect("replay before snapshot");
    storage
        .persist_state(&snap_state, snap_next)
        .expect("persist snapshot");

    // New tx appended after snapshot; replay should apply only tail from next_tx_id.
    storage.append_tx(&txs[2]).expect("append tx2");
    let (state, next_tx_id) = replay_to_tip(&storage).expect("replay with snapshot + tail");
    assert_eq!(next_tx_id, 3);
    let meter = state.get_meter("alice", "svc").expect("meter exists");
    assert_eq!(meter.total_units(), 5);
}

#[test]
fn test_recovery_truncated_tx_log_returns_state_error() {
    let (mut storage, td) = test_storage();
    let txs = base_txs();
    storage.append_tx(&txs[0]).expect("append tx0");
    storage.append_tx(&txs[1]).expect("append tx1");

    // Truncate inside the last tx payload to simulate partial write/corruption.
    let tx_log_path = td.path().join("tx.log");
    let meta = std::fs::metadata(&tx_log_path).expect("tx.log metadata");
    assert!(meta.len() > 4);
    let f = std::fs::OpenOptions::new()
        .write(true)
        .open(&tx_log_path)
        .expect("open tx.log for truncate");
    f.set_len(meta.len() - 3).expect("truncate tx.log");

    let err = replay_to_tip(&storage).unwrap_err();
    assert!(matches!(err, Error::StateError(_)));
    assert_eq!(err.error_code(), "STATE_ERROR");
}

#[test]
fn test_recovery_corrupted_state_bin_returns_state_error() {
    let (mut storage, td) = test_storage();
    let txs = base_txs();
    storage.append_tx(&txs[0]).expect("append tx0");
    storage.append_tx(&txs[1]).expect("append tx1");
    let (state, next_tx_id) = replay_to_tip(&storage).expect("replay baseline");
    storage
        .persist_state(&state, next_tx_id)
        .expect("persist snapshot");

    // Corrupt snapshot file bytes.
    let state_path = td.path().join("state.bin");
    std::fs::write(&state_path, [0xde, 0xad, 0xbe, 0xef]).expect("overwrite state.bin");

    let err = replay_to_tip(&storage).unwrap_err();
    assert!(matches!(err, Error::StateError(_)));
    assert_eq!(err.error_code(), "STATE_ERROR");
}

#[test]
fn test_recovery_mismatched_snapshot_cursor_vs_log_is_deterministic() {
    let (mut storage, _td) = test_storage();
    let txs = base_txs();
    storage.append_tx(&txs[0]).expect("append tx0");
    storage.append_tx(&txs[1]).expect("append tx1");

    let (snap_state, _snap_next) = replay_to_tip(&storage).expect("replay baseline");
    // Persist snapshot with an intentionally wrong cursor beyond tx log tip.
    storage
        .persist_state(&snap_state, 999)
        .expect("persist mismatched cursor");

    let (state1, next1) = replay_to_tip(&storage).expect("replay 1");
    let (state2, next2) = replay_to_tip(&storage).expect("replay 2");
    assert_eq!(state1, state2);
    assert_eq!(next1, next2);
    assert_eq!(state1, snap_state);
    assert_eq!(next1, 999);
}

#[test]
fn test_recovery_missing_snapshot_is_genesis_replay() {
    let (storage, _td) = test_storage();
    let (state, next_tx_id) = replay_to_tip(&storage).expect("empty replay");
    assert_eq!(state, State::new());
    assert_eq!(next_tx_id, 0);
}
