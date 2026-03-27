use metering_chain::config::Config;
use metering_chain::replay::{replay_slice_to_summary, replay_to_tip};
use metering_chain::storage::{FileStorage, Storage};
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use std::time::Instant;
use tempfile::TempDir;

fn build_dataset(consumes: usize) -> Vec<SignedTx> {
    let mut txs = Vec::with_capacity(consumes + 2);
    txs.push(SignedTx::new(
        "minter".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 5_000_000,
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
    for i in 0..consumes {
        txs.push(SignedTx::new(
            "alice".to_string(),
            (i as u64) + 1,
            Transaction::Consume {
                owner: "alice".to_string(),
                service_id: "svc".to_string(),
                units: 1,
                pricing: Pricing::UnitPrice(2),
            },
        ));
    }
    txs
}

#[test]
fn test_perf_smoke_reports_replay_snapshot_and_recompute_baselines() {
    // Local/reporting baseline only: no hard performance thresholds.
    let datasets = [
        ("small", 100usize),
        ("medium", 500usize),
        ("large", 1000usize),
    ];

    for (name, consumes) in datasets {
        let txs = build_dataset(consumes);
        let td = TempDir::new().expect("tempdir");
        let cfg = Config::with_data_dir(td.path().to_path_buf());
        let mut storage = FileStorage::new(&cfg);

        for tx in &txs {
            storage.append_tx(tx).expect("append tx");
        }

        let cold_start = Instant::now();
        let (state_after_replay, next_tx_id) = replay_to_tip(&storage).expect("cold replay");
        let cold_elapsed = cold_start.elapsed();

        storage
            .persist_state(&state_after_replay, next_tx_id)
            .expect("persist snapshot");

        let warm_start = Instant::now();
        let (state_after_snapshot_restore, next_tx_id_warm) =
            replay_to_tip(&storage).expect("warm replay from snapshot");
        let warm_elapsed = warm_start.elapsed();

        let recompute_start = Instant::now();
        let (summary, evidence_hash) = replay_slice_to_summary(
            &storage,
            1, // start from OpenMeter
            next_tx_id_warm,
            "alice",
            "svc",
            0,
            0,
            0,
        )
        .expect("recompute replay summary");
        let recompute_elapsed = recompute_start.elapsed();

        let tx_count = txs.len() as f64;
        let cold_secs = cold_elapsed.as_secs_f64().max(f64::EPSILON);
        let throughput = tx_count / cold_secs;

        assert_eq!(state_after_replay, state_after_snapshot_restore);
        assert_eq!(next_tx_id, next_tx_id_warm);
        assert_eq!(summary.tx_count, next_tx_id_warm.saturating_sub(1));
        assert!(!evidence_hash.is_empty());
        assert!(throughput.is_finite() && throughput > 0.0);

        eprintln!(
            "[perf_smoke] dataset={name} txs={} replay_ms={} replay_tx_per_sec={:.2} snapshot_restore_ms={} recompute_ms={}",
            txs.len(),
            cold_elapsed.as_millis(),
            throughput,
            warm_elapsed.as_millis(),
            recompute_elapsed.as_millis()
        );
    }
}
