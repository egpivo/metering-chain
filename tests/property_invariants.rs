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

fn total_value(state: &State) -> u128 {
    let balances: u128 = state.accounts.values().map(|a| a.balance() as u128).sum();
    let locked: u128 = state
        .meters
        .values()
        .map(|m| m.locked_deposit() as u128)
        .sum();
    balances + locked
}

fn total_accounted_value(state: &State) -> u128 {
    let balances: u128 = state.accounts.values().map(|a| a.balance() as u128).sum();
    let locked: u128 = state
        .meters
        .values()
        .map(|m| m.locked_deposit() as u128)
        .sum();
    let spent: u128 = state.meters.values().map(|m| m.total_spent() as u128).sum();
    balances + locked + spent
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

    #[test]
    fn prop_deposit_conservation_across_open_and_close(
        deposit in 1_u64..500
    ) {
        let mut minters = HashSet::new();
        minters.insert("minter".to_string());
        let mut state = State::new();

        let mint_tx = SignedTx::new(
            "minter".to_string(),
            0,
            Transaction::Mint {
                to: "alice".to_string(),
                amount: 10_000,
            },
        );
        state = apply(&state, &mint_tx, &ValidationContext::replay(), Some(&minters))
            .expect("mint");
        let baseline = total_value(&state);

        let open_tx = SignedTx::new(
            "alice".to_string(),
            0,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                deposit,
            },
        );
        state = apply(&state, &open_tx, &ValidationContext::replay(), Some(&minters))
            .expect("open should apply");
        prop_assert_eq!(
            total_value(&state),
            baseline,
            "open should move value balance -> locked deposit without loss"
        );

        let close_tx = SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::CloseMeter {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
            },
        );
        state = apply(&state, &close_tx, &ValidationContext::replay(), Some(&minters))
            .expect("close should apply");
        prop_assert_eq!(
            total_value(&state),
            baseline,
            "close should restore locked deposit to balance without value drift"
        );
    }

    #[test]
    fn prop_accounted_value_conservation_across_accepted_sequence(
        units in prop::collection::vec(1_u64..20, 1..20),
        unit_price in 1_u64..5,
        close_meter in any::<bool>()
    ) {
        let (base_txs, minters) = build_valid_sequence(&units, unit_price);
        let mut state = State::new();

        state = apply(&state, &base_txs[0], &ValidationContext::replay(), Some(&minters))
            .expect("mint");
        let baseline = total_accounted_value(&state);

        for tx in &base_txs[1..] {
            state = apply(&state, tx, &ValidationContext::replay(), Some(&minters))
                .expect("accepted tx should apply");
            prop_assert_eq!(
                total_accounted_value(&state),
                baseline,
                "balances + locked + total_spent should remain conserved"
            );
        }

        if close_meter {
            let close_tx = SignedTx::new(
                "alice".to_string(),
                1_u64 + units.len() as u64,
                Transaction::CloseMeter {
                    owner: "alice".to_string(),
                    service_id: "svc".to_string(),
                },
            );
            state = apply(&state, &close_tx, &ValidationContext::replay(), Some(&minters))
                .expect("close should apply");
            prop_assert_eq!(total_accounted_value(&state), baseline);
        }
    }

    #[test]
    fn prop_meter_totals_monotonicity_under_accepted_consumes(
        units in prop::collection::vec(1_u64..25, 1..20),
        unit_price in 1_u64..5
    ) {
        let (txs, minters) = build_valid_sequence(&units, unit_price);
        let mut state = State::new();
        let mut prev_units = 0u64;
        let mut prev_spent = 0u64;

        for tx in txs {
            state = apply(&state, &tx, &ValidationContext::replay(), Some(&minters))
                .expect("accepted tx should apply");
            if let Some(m) = state.get_meter("alice", "svc") {
                prop_assert!(m.total_units() >= prev_units, "total_units must be monotonic");
                prop_assert!(m.total_spent() >= prev_spent, "total_spent must be monotonic");
                prev_units = m.total_units();
                prev_spent = m.total_spent();
            }
        }
    }

    #[test]
    fn prop_meter_uniqueness_rejects_second_active_open(
        deposit in 1_u64..500
    ) {
        let mut minters = HashSet::new();
        minters.insert("minter".to_string());
        let mut state = State::new();

        state = apply(
            &state,
            &SignedTx::new(
                "minter".to_string(),
                0,
                Transaction::Mint {
                    to: "alice".to_string(),
                    amount: 10_000,
                }
            ),
            &ValidationContext::replay(),
            Some(&minters),
        ).expect("mint");

        state = apply(
            &state,
            &SignedTx::new(
                "alice".to_string(),
                0,
                Transaction::OpenMeter {
                    owner: "alice".to_string(),
                    service_id: "svc".to_string(),
                    deposit,
                }
            ),
            &ValidationContext::replay(),
            Some(&minters),
        ).expect("first open");

        let snapshot = state.clone();
        let duplicate_open = SignedTx::new(
            "alice".to_string(),
            1,
            Transaction::OpenMeter {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                deposit,
            },
        );
        let err = apply(
            &state,
            &duplicate_open,
            &ValidationContext::replay(),
            Some(&minters),
        ).expect_err("second active meter open must fail");
        prop_assert_eq!(err.error_code(), "INVALID_TRANSACTION");
        prop_assert_eq!(state, snapshot, "rejected duplicate open must not mutate state");
    }
}
