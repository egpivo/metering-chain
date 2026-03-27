use metering_chain::evidence::EvidenceBundle;
use metering_chain::error::Error;
use metering_chain::replay::replay_to_tip;
use metering_chain::storage::FileStorage;
use metering_chain::tx::SignedTx;
use tempfile::TempDir;

fn fixtures_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
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

    // Persisted tx log format is length-prefixed bincode bytes.
    // Build a one-entry log file and replay through storage/replay path.
    let td = TempDir::new().expect("tempdir");
    let tx_log_path = td.path().join("tx.log");
    let state_path = td.path().join("state.bin");
    let tx_bytes = bincode::serialize(&tx).expect("bincode tx bytes");
    let mut log_bytes = Vec::with_capacity(8 + tx_bytes.len());
    log_bytes.extend_from_slice(&(tx_bytes.len() as u64).to_le_bytes());
    log_bytes.extend_from_slice(&tx_bytes);
    std::fs::write(&tx_log_path, log_bytes).expect("write tx.log");

    let storage = FileStorage::with_paths(tx_log_path, state_path);
    let (next, _next_tx_id) = replay_to_tip(&storage).expect("replay to tip");
    let bob = next.get_account("bob").expect("mint recipient account");
    assert_eq!(bob.balance(), 100);
}

#[test]
fn test_compat_unsupported_schema_error_code_stable() {
    let p = fixtures_dir().join("evidence_bundle_v1.json");
    let raw = std::fs::read_to_string(p).expect("read evidence fixture");
    let mut bundle: EvidenceBundle = serde_json::from_str(&raw).expect("deserialize fixture");
    bundle.schema_version = 999;
    let err = bundle.validate_shape().expect_err("must reject unsupported schema");
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
