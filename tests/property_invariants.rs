use metering_chain::replay;
use metering_chain::state::{apply, State};
use metering_chain::storage::{FileStorage, Storage};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use proptest::prelude::*;
use std::collections::HashSet;
use tempfile::TempDir;

fn build_valid_sequence(units: &[u64], unit_price: u64) -> (Vec<SignedTx>, HashSet<String>) {
    let mut txs = Vec::new();
    let mut minters = HashSet::new();
    minters.insert("minter".to_string());

    txs.push(SignedTx::new(
        "minter".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1_000_000,
        },
    ));
    txs.push(SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "svc".to_string(),
            deposit: 100,
        },
    ));
    for (i, u) in units.iter().enumerate() {
        txs.push(SignedTx::new(
            "alice".to_string(),
            (i as u64) + 1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                units: *u,
                pricing: Pricing::UnitPrice(unit_price),
            },
        ));
    }
    (txs, minters)
}

fn apply_all(mut state: State, txs: &[SignedTx], minters: &HashSet<String>) -> State {
    for tx in txs {
        state = apply(&state, tx, &ValidationContext::replay(), Some(minters))
            .expect("sequence should be valid");
    }
    state
}

fn create_test_storage() -> (FileStorage, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let tx_log_path = temp_dir.path().join("tx.log");
    let state_path = temp_dir.path().join("state.bin");
    let storage = FileStorage::with_paths(tx_log_path, state_path);
    (storage, temp_dir)
}

fn append_all(storage: &mut FileStorage, txs: &[SignedTx]) {
    for tx in txs {
        storage.append_tx(tx).unwrap();
    }
}

proptest! {
    #[test]
    fn prop_nonce_monotonicity_for_accepted_sequence(
        units in prop::collection::vec(1_u64..20, 1..20),
        unit_price in 1_u64..5
    ) {
        let (txs, minters) = build_valid_sequence(&units, unit_price);
        let end = apply_all(State::new(), &txs, &minters);
        let alice = end.accounts.get("alice").expect("alice account exists");
        let expected_nonce = 1_u64 + units.len() as u64; // open meter + consumes
        prop_assert_eq!(alice.nonce(), expected_nonce);
    }

    #[test]
    fn prop_replay_determinism_across_snapshot_boundary(
        units in prop::collection::vec(1_u64..20, 1..30),
        unit_price in 1_u64..5,
        snapshot_selector in 0usize..64
    ) {
        let (txs, minters) = build_valid_sequence(&units, unit_price);
        let snapshot_at = snapshot_selector % (txs.len() + 1);

        let (mut storage_without_snapshot, _dir1) = create_test_storage();
        append_all(&mut storage_without_snapshot, &txs);

        let (mut storage_with_snapshot, _dir2) = create_test_storage();
        append_all(&mut storage_with_snapshot, &txs);
        let snapshot_state = apply_all(State::new(), &txs[..snapshot_at], &minters);
        storage_with_snapshot
            .persist_state(&snapshot_state, snapshot_at as u64)
            .unwrap();

        let (replayed_without_snapshot, next_tx_id_without_snapshot) =
            replay::replay_to_tip(&storage_without_snapshot).unwrap();
        let (replayed_with_snapshot, next_tx_id_with_snapshot) =
            replay::replay_to_tip(&storage_with_snapshot).unwrap();

        prop_assert_eq!(replayed_without_snapshot, replayed_with_snapshot);
        prop_assert_eq!(next_tx_id_without_snapshot, next_tx_id_with_snapshot);
        prop_assert_eq!(next_tx_id_with_snapshot, txs.len() as u64);
    }

    #[test]
    fn prop_rejected_transition_does_not_consume_nonce_or_block_followup(
        units in prop::collection::vec(1_u64..20, 1..10),
        unit_price in 1_u64..5
    ) {
        let (txs, minters) = build_valid_sequence(&units, unit_price);
        let state_before = apply_all(State::new(), &txs, &minters);
        let snapshot = state_before.clone();
        let next_nonce = 1_u64 + units.len() as u64;

        // Invalid consume: units = 0 should be rejected.
        let bad_tx = SignedTx::new(
            "alice".to_string(),
            next_nonce,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                units: 0,
                pricing: Pricing::UnitPrice(unit_price),
            },
        );

        let result = apply(
            &state_before,
            &bad_tx,
            &ValidationContext::replay(),
            Some(&minters),
        );
        prop_assert!(result.is_err());

        let valid_followup = SignedTx::new(
            "alice".to_string(),
            next_nonce,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                units: 1,
                pricing: Pricing::UnitPrice(unit_price),
            },
        );

        let after_valid_followup = apply(
            &state_before,
            &valid_followup,
            &ValidationContext::replay(),
            Some(&minters),
        )
        .expect("same nonce should remain usable after rejection");
        let expected_after_followup = apply(
            &snapshot,
            &valid_followup,
            &ValidationContext::replay(),
            Some(&minters),
        )
        .expect("valid followup should apply from unchanged snapshot");

        let alice = after_valid_followup
            .accounts
            .get("alice")
            .expect("alice account exists");
        prop_assert_eq!(alice.nonce(), next_nonce + 1);
        prop_assert_eq!(after_valid_followup, expected_after_followup);
    }
}
