use metering_chain::error::Error;
use metering_chain::state::{apply, State};
use metering_chain::tx::validation::{validate, DelegationProofMinimal, ValidationContext};
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use metering_chain::wallet::{verify_signature, Wallet};
use std::collections::HashSet;

fn setup_owner_with_active_meter(owner: &str, service_id: &str) -> (State, HashSet<String>) {
    let mut minters = HashSet::new();
    minters.insert("minter".to_string());

    let mut state = State::new();
    state = apply(
        &state,
        &SignedTx::new(
            "minter".to_string(),
            0,
            Transaction::Mint {
                to: owner.to_string(),
                amount: 1_000_000,
            },
        ),
        &ValidationContext::replay(),
        Some(&minters),
    )
    .expect("mint should apply");

    state = apply(
        &state,
        &SignedTx::new(
            owner.to_string(),
            0,
            Transaction::OpenMeter {
                owner: owner.to_string(),
                service_id: service_id.to_string(),
                deposit: 100,
            },
        ),
        &ValidationContext::replay(),
        Some(&minters),
    )
    .expect("open meter should apply");

    (state, minters)
}

#[test]
fn test_security_abuse_tampered_signed_payload_rejected() {
    let signer = Wallet::new_random();
    let original = signer
        .sign_transaction(
            0,
            Transaction::Mint {
                to: "bob".to_string(),
                amount: 10,
            },
        )
        .expect("sign tx");

    let mut tampered = original.clone();
    tampered.kind = Transaction::Mint {
        to: "bob".to_string(),
        amount: 999, // mutate payload after signature
    };

    let err = verify_signature(&tampered).expect_err("tampered payload must fail signature verify");
    assert!(matches!(err, Error::SignatureVerification(_)));
    assert_eq!(err.error_code(), "SIGNATURE_VERIFICATION_FAILED");
}

#[test]
fn test_security_abuse_forged_signature_rejected() {
    let signer = Wallet::new_random();
    let mut tx = signer
        .sign_transaction(
            0,
            Transaction::Mint {
                to: "bob".to_string(),
                amount: 10,
            },
        )
        .expect("sign tx");

    // Keep payload identical and forge signature bytes directly.
    tx.signature = Some(vec![0u8; 64]);
    let err = verify_signature(&tx).expect_err("forged signature must fail verification");
    assert!(matches!(err, Error::SignatureVerification(_)));
    assert_eq!(err.error_code(), "SIGNATURE_VERIFICATION_FAILED");
}

#[test]
fn test_security_abuse_malformed_did_key_in_delegation_proof_rejected() {
    let owner_wallet = Wallet::new_random();
    let delegate_wallet = Wallet::new_random();
    let owner = owner_wallet.address().to_string();

    let (state, _minters) = setup_owner_with_active_meter(&owner, "svc");
    let claims = DelegationProofMinimal {
        iat: 100,
        exp: 200,
        issuer: "did:key:not-base58".to_string(), // malformed did:key principal
        audience: delegate_wallet.address().to_string(),
        service_id: "svc".to_string(),
        ability: Some("consume".to_string()),
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let tx = delegate_wallet
        .sign_transaction_v2(
            1,
            owner.clone(),
            120,
            proof,
            Transaction::Consume {
                owner: owner.clone(),
                service_id: "svc".to_string(),
                units: 1,
                pricing: Pricing::UnitPrice(1),
            },
        )
        .expect("build delegated tx");

    let err = validate(&state, &tx, &ValidationContext::live(120, 10_000), None)
        .expect_err("malformed did:key issuer must be rejected");
    assert!(matches!(err, Error::PrincipalBindingFailed(_)));
    assert_eq!(err.error_code(), "PRINCIPAL_BINDING_FAILED");
}

#[test]
fn test_security_abuse_wrong_signer_audience_binding_rejected() {
    let owner_wallet = Wallet::new_random();
    let delegate_wallet = Wallet::new_random();
    let wrong_audience_wallet = Wallet::new_random();
    let owner = owner_wallet.address().to_string();

    let (state, _minters) = setup_owner_with_active_meter(&owner, "svc");
    let claims = DelegationProofMinimal {
        iat: 100,
        exp: 500,
        issuer: owner.clone(),
        audience: wrong_audience_wallet.address().to_string(), // wrong audience for signer
        service_id: "svc".to_string(),
        ability: Some("consume".to_string()),
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let tx = delegate_wallet
        .sign_transaction_v2(
            1,
            owner.clone(),
            120,
            proof,
            Transaction::Consume {
                owner: owner.clone(),
                service_id: "svc".to_string(),
                units: 1,
                pricing: Pricing::UnitPrice(1),
            },
        )
        .expect("build delegated tx");

    let err = validate(&state, &tx, &ValidationContext::live(120, 10_000), None)
        .expect_err("signer/audience mismatch must be rejected");
    assert!(matches!(err, Error::DelegationAudienceSignerMismatch));
    assert_eq!(err.error_code(), "DELEGATION_AUDIENCE_SIGNER_MISMATCH");
}

#[test]
fn test_security_abuse_future_valid_at_rejected_in_live_mode() {
    let owner_wallet = Wallet::new_random();
    let delegate_wallet = Wallet::new_random();
    let owner = owner_wallet.address().to_string();

    let (state, _minters) = setup_owner_with_active_meter(&owner, "svc");
    let claims = DelegationProofMinimal {
        iat: 100,
        exp: 500,
        issuer: owner.clone(),
        audience: delegate_wallet.address().to_string(),
        service_id: "svc".to_string(),
        ability: Some("consume".to_string()),
        max_units: None,
        max_cost: None,
    };
    let proof = owner_wallet.sign_delegation_proof(&claims);
    let tx = delegate_wallet
        .sign_transaction_v2(
            1,
            owner.clone(),
            300, // future relative to now=200
            proof,
            Transaction::Consume {
                owner: owner.clone(),
                service_id: "svc".to_string(),
                units: 1,
                pricing: Pricing::UnitPrice(1),
            },
        )
        .expect("build delegated tx");

    let err = validate(&state, &tx, &ValidationContext::live(200, 10_000), None)
        .expect_err("future valid_at must be rejected");
    assert!(matches!(err, Error::ReferenceTimeFuture));
    assert_eq!(err.error_code(), "REFERENCE_TIME_FUTURE");
}

#[test]
fn test_security_abuse_overflow_scale_consume_rejected() {
    let owner = "alice";
    let (state, _minters) = setup_owner_with_active_meter(owner, "svc");
    let overflow_tx = SignedTx::new(
        owner.to_string(),
        1,
        Transaction::Consume {
            owner: owner.to_string(),
            service_id: "svc".to_string(),
            units: u64::MAX,
            pricing: Pricing::UnitPrice(2),
        },
    );

    let err = validate(&state, &overflow_tx, &ValidationContext::replay(), None)
        .expect_err("overflow-scale input must be rejected");
    assert!(matches!(err, Error::InvalidTransaction(_)));
    assert_eq!(err.error_code(), "INVALID_TRANSACTION");
    assert!(
        err.to_string().contains("overflow"),
        "expected overflow diagnostic, got: {err}"
    );
}
