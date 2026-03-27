use metering_chain::evidence::EvidenceBundle;
use metering_chain::error::Error;
use metering_chain::state::{apply, State};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::SignedTx;

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
fn test_compat_fixture_signed_tx_v1_is_accepted_where_intended() {
    let p = fixtures_dir().join("signedtx_v1_mint.json");
    let raw = std::fs::read_to_string(p).expect("read tx fixture");
    let tx: SignedTx = serde_json::from_str(&raw).expect("deserialize tx fixture");

    // Replay context + no minter check is the intended compatibility path for old logs.
    let next = apply(&State::new(), &tx, &ValidationContext::replay(), None).expect("apply tx");
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

#[test]
fn test_compat_versioning_error_codes_exist() {
    assert_eq!(
        Error::UnsupportedTxVersion.error_code(),
        "UNSUPPORTED_TX_VERSION"
    );
    assert_eq!(
        Error::UnsupportedEventVersion.error_code(),
        "UNSUPPORTED_EVENT_VERSION"
    );
    assert_eq!(Error::MigrationRequired.error_code(), "MIGRATION_REQUIRED");
}
