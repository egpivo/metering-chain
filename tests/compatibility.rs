use metering_chain::error::Error;
use metering_chain::evidence::EvidenceBundle;
use metering_chain::replay::replay_to_tip;
use metering_chain::storage::FileStorage;
use metering_chain::tx::{SignedTx, Transaction};
use serde::Serialize;
use tempfile::TempDir;

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

#[derive(Serialize)]
struct LegacyTxPhase1 {
    signer: String,
    nonce: u64,
    kind: Transaction,
}

#[derive(Serialize)]
struct LegacyTxPhase2 {
    signer: String,
    nonce: u64,
    kind: Transaction,
    signature: Option<Vec<u8>>,
}

fn replay_one_entry_log(tx_bytes: Vec<u8>) -> (metering_chain::state::State, u64) {
    let td = TempDir::new().expect("tempdir");
    let tx_log_path = td.path().join("tx.log");
    let state_path = td.path().join("state.bin");
    let mut log_bytes = Vec::with_capacity(8 + tx_bytes.len());
    log_bytes.extend_from_slice(&(tx_bytes.len() as u64).to_le_bytes());
    log_bytes.extend_from_slice(&tx_bytes);
    std::fs::write(&tx_log_path, log_bytes).expect("write tx.log");

    let storage = FileStorage::with_paths(tx_log_path, state_path);
    replay_to_tip(&storage).expect("replay one-entry log")
}

#[test]
fn test_compat_fixture_evidence_bundle_v1_is_accepted() {
    let p = fixtures_dir().join("evidence_bundle_v1.json");
    let raw = std::fs::read_to_string(p).expect("read evidence fixture");
    let bundle: EvidenceBundle = serde_json::from_str(&raw).expect("deserialize fixture");
    bundle.validate_shape().expect("v1 fixture should validate");
}

#[test]
fn test_compat_fixture_signed_tx_v1_is_accepted_via_tx_log_format() {
    let p = fixtures_dir().join("signedtx_v1_mint.json");
    let raw = std::fs::read_to_string(p).expect("read tx fixture");
    let tx: SignedTx = serde_json::from_str(&raw).expect("deserialize tx fixture");

    let tx_bytes = bincode::serialize(&tx).expect("bincode tx bytes");
    let (next, _next_tx_id) = replay_one_entry_log(tx_bytes);
    let bob = next.get_account("bob").expect("mint recipient account");
    assert_eq!(bob.balance(), 100);
}

#[test]
fn test_compat_phase1_legacy_bincode_txlog_is_accepted() {
    let legacy = LegacyTxPhase1 {
        signer: "minter".to_string(),
        nonce: 0,
        kind: Transaction::Mint {
            to: "carol".to_string(),
            amount: 77,
        },
    };
    let tx_bytes = bincode::serialize(&legacy).expect("serialize phase1 legacy payload");
    let (state, next_tx_id) = replay_one_entry_log(tx_bytes);
    assert_eq!(next_tx_id, 1);
    let carol = state.get_account("carol").expect("mint recipient account");
    assert_eq!(carol.balance(), 77);
}

#[test]
fn test_compat_phase2_legacy_bincode_txlog_is_accepted() {
    let legacy = LegacyTxPhase2 {
        signer: "minter".to_string(),
        nonce: 0,
        kind: Transaction::Mint {
            to: "dave".to_string(),
            amount: 55,
        },
        signature: None,
    };
    let tx_bytes = bincode::serialize(&legacy).expect("serialize phase2 legacy payload");
    let (state, next_tx_id) = replay_one_entry_log(tx_bytes);
    assert_eq!(next_tx_id, 1);
    let dave = state.get_account("dave").expect("mint recipient account");
    assert_eq!(dave.balance(), 55);
}

#[test]
fn test_compat_unsupported_schema_error_code_stable() {
    let p = fixtures_dir().join("evidence_bundle_v1.json");
    let raw = std::fs::read_to_string(p).expect("read evidence fixture");
    let mut bundle: EvidenceBundle = serde_json::from_str(&raw).expect("deserialize fixture");
    bundle.schema_version = 999;
    let err = bundle
        .validate_shape()
        .expect_err("must reject unsupported schema");
    assert!(matches!(err, Error::UnsupportedSchemaVersion));
    assert_eq!(err.error_code(), "UNSUPPORTED_SCHEMA_VERSION");
}

#[test]
fn test_compat_replay_protocol_mismatch_error_code_stable() {
    let p = fixtures_dir().join("evidence_bundle_v1.json");
    let raw = std::fs::read_to_string(p).expect("read evidence fixture");
    let mut bundle: EvidenceBundle = serde_json::from_str(&raw).expect("deserialize fixture");
    bundle.replay_protocol_version = 999;
    let err = bundle
        .validate_shape()
        .expect_err("must reject replay protocol mismatch");
    assert!(matches!(err, Error::ReplayProtocolMismatch));
    assert_eq!(err.error_code(), "REPLAY_PROTOCOL_MISMATCH");
}

// Note: UnsupportedTxVersion / UnsupportedEventVersion / MigrationRequired are
// defined for forward-compatible API contracts, but runtime paths are deferred
// until tx/event version fields and migrations are implemented.
