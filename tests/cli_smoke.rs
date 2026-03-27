use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_metering-chain")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("tx")
        .join(name)
}

fn run_ok(data_dir: &Path, args: &[&str]) -> String {
    run_with_minters_ok(data_dir, "0x0000000000000000000000000000000000000AAA", args)
}

fn run_with_minters_ok(data_dir: &Path, minters: &str, args: &[&str]) -> String {
    let data_dir_str = data_dir.to_str().expect("data_dir utf8");
    let out = Command::new(bin_path())
        .env("METERING_CHAIN_MINTERS", minters)
        .arg("--data-dir")
        .arg(data_dir_str)
        .args(args)
        .output()
        .expect("spawn cli");
    assert!(
        out.status.success(),
        "command failed: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf8 stdout")
}

fn run_err(data_dir: &Path, args: &[&str]) -> String {
    let data_dir_str = data_dir.to_str().expect("data_dir utf8");
    let out = Command::new(bin_path())
        .env(
            "METERING_CHAIN_MINTERS",
            "0x0000000000000000000000000000000000000AAA",
        )
        .arg("--data-dir")
        .arg(data_dir_str)
        .args(args)
        .output()
        .expect("spawn cli");
    assert!(
        !out.status.success(),
        "command unexpectedly succeeded: {:?}\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stderr).expect("utf8 stderr")
}

fn object_keys(v: &Value) -> BTreeSet<String> {
    v.as_object()
        .expect("json object")
        .keys()
        .cloned()
        .collect()
}

fn init_and_apply_minimal_flow(data_dir: &Path) {
    run_ok(data_dir, &["init"]);
    run_ok(
        data_dir,
        &[
            "apply",
            "--allow-unsigned",
            "--file",
            fixture("01_mint_alice.json").to_str().expect("fixture path"),
        ],
    );
    run_ok(
        data_dir,
        &[
            "apply",
            "--allow-unsigned",
            "--file",
            fixture("02_open_storage.json")
                .to_str()
                .expect("fixture path"),
        ],
    );
    run_ok(
        data_dir,
        &[
            "apply",
            "--allow-unsigned",
            "--file",
            fixture("03_consume_storage_unit_price.json")
                .to_str()
                .expect("fixture path"),
        ],
    );
}

#[test]
fn test_cli_smoke_init_and_apply_unsigned_fixture() {
    let td = TempDir::new().expect("tempdir");
    init_and_apply_minimal_flow(td.path());

    let tx_log = td.path().join("tx.log");
    let state_bin = td.path().join("state.bin");
    assert!(tx_log.exists(), "tx.log should be created");
    assert!(state_bin.exists(), "state.bin should be created");
}

#[test]
fn test_cli_smoke_account_meters_report_json_shape() {
    let td = TempDir::new().expect("tempdir");
    init_and_apply_minimal_flow(td.path());
    let address = "0x0000000000000000000000000000000000000A11";

    let account_raw = run_ok(td.path(), &["--format", "json", "account", address]);
    let account: Value = serde_json::from_str(&account_raw).expect("account json");
    assert_eq!(account["address"], address);
    assert!(account.get("balance").is_some());
    assert!(account.get("nonce").is_some());

    let meters_raw = run_ok(td.path(), &["--format", "json", "meters", address]);
    let meters: Value = serde_json::from_str(&meters_raw).expect("meters json");
    assert_eq!(meters["address"], address);
    assert!(meters["meters"].is_array(), "meters.meters should be array");
    let first_meter = &meters["meters"][0];
    assert_eq!(first_meter["owner"], address);
    assert!(first_meter.get("service_id").is_some());
    assert!(first_meter.get("total_units").is_some());
    assert!(first_meter.get("total_spent").is_some());

    let report_raw = run_ok(td.path(), &["--format", "json", "report", address]);
    let report: Value = serde_json::from_str(&report_raw).expect("report json");
    assert!(report["reports"].is_array(), "report.reports should be array");
    let first_report = &report["reports"][0];
    assert_eq!(first_report["account"], address);
    assert!(first_report.get("service_id").is_some());
    assert!(first_report.get("total_units").is_some());
    assert!(first_report.get("total_spent").is_some());
}

#[test]
fn test_cli_json_contract_account_meters_report_keys_stable() {
    let td = TempDir::new().expect("tempdir");
    init_and_apply_minimal_flow(td.path());
    let address = "0x0000000000000000000000000000000000000A11";

    let account_raw = run_ok(td.path(), &["--format", "json", "account", address]);
    let account: Value = serde_json::from_str(&account_raw).expect("account json");
    assert_eq!(
        object_keys(&account),
        BTreeSet::from(["address".to_string(), "balance".to_string(), "nonce".to_string()])
    );
    assert!(account["balance"].is_u64());
    assert!(account["nonce"].is_u64());

    let meters_raw = run_ok(td.path(), &["--format", "json", "meters", address]);
    let meters: Value = serde_json::from_str(&meters_raw).expect("meters json");
    assert_eq!(
        object_keys(&meters),
        BTreeSet::from(["address".to_string(), "meters".to_string()])
    );
    let first_meter = &meters["meters"][0];
    assert_eq!(
        object_keys(first_meter),
        BTreeSet::from([
            "active".to_string(),
            "locked_deposit".to_string(),
            "owner".to_string(),
            "service_id".to_string(),
            "total_spent".to_string(),
            "total_units".to_string(),
        ])
    );

    let report_raw = run_ok(td.path(), &["--format", "json", "report", address]);
    let report: Value = serde_json::from_str(&report_raw).expect("report json");
    assert_eq!(object_keys(&report), BTreeSet::from(["reports".to_string()]));
    let first_report = &report["reports"][0];
    assert_eq!(
        object_keys(first_report),
        BTreeSet::from([
            "account".to_string(),
            "active".to_string(),
            "effective_unit_price".to_string(),
            "service_id".to_string(),
            "total_spent".to_string(),
            "total_units".to_string(),
        ])
    );
}

#[test]
fn test_cli_smoke_signed_apply_path_minimal() {
    let td = TempDir::new().expect("tempdir");
    run_ok(td.path(), &["init"]);

    // 1) Create wallet and capture generated address.
    let create_out = run_ok(td.path(), &["wallet", "create"]);
    let created_line = create_out.trim();
    let signer = created_line
        .strip_prefix("Created wallet: ")
        .expect("wallet create output format")
        .trim()
        .to_string();

    // 2) Create mint-kind JSON for signing.
    let kind_file = td.path().join("mint_kind.json");
    fs::write(
        &kind_file,
        r#"{"Mint":{"to":"0x0000000000000000000000000000000000000B0B","amount":7}}"#,
    )
    .expect("write kind file");

    // 3) Wallet sign -> signed tx JSON.
    let signed_tx = run_ok(
        td.path(),
        &[
            "wallet",
            "sign",
            "--address",
            &signer,
            "--file",
            kind_file.to_str().expect("kind path"),
        ],
    );

    // 4) Apply signed tx without --allow-unsigned. Authorize signer as minter for this test.
    run_with_minters_ok(td.path(), &signer, &["apply", "--tx", signed_tx.trim()]);

    // 5) Verify recipient account got minted balance in JSON output.
    let acct = run_with_minters_ok(
        td.path(),
        &signer,
        &[
            "--format",
            "json",
            "account",
            "0x0000000000000000000000000000000000000B0B",
        ],
    );
    let v: Value = serde_json::from_str(&acct).expect("account json");
    assert_eq!(v["balance"], 7);
}

#[test]
fn test_cli_smoke_failure_unsigned_without_allow_rejected() {
    let td = TempDir::new().expect("tempdir");
    run_ok(td.path(), &["init"]);
    let stderr = run_err(
        td.path(),
        &[
            "apply",
            "--file",
            fixture("01_mint_alice.json").to_str().expect("fixture path"),
        ],
    );
    assert!(
        stderr.contains("Unsigned tx rejected"),
        "expected unsigned reject message, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Error: Signature verification failed:"),
        "expected stable diagnostic prefix, got: {}",
        stderr
    );
}

#[test]
fn test_cli_smoke_failure_stale_nonce_rejected() {
    let td = TempDir::new().expect("tempdir");
    init_and_apply_minimal_flow(td.path());
    let stderr = run_err(
        td.path(),
        &[
            "apply",
            "--allow-unsigned",
            "--file",
            fixture("03_consume_storage_unit_price.json")
                .to_str()
                .expect("fixture path"),
        ],
    );
    assert!(
        stderr.contains("Nonce mismatch"),
        "expected nonce mismatch, got: {}",
        stderr
    );
    assert!(
        stderr.contains("Error: Invalid transaction:"),
        "expected stable diagnostic prefix, got: {}",
        stderr
    );
}
