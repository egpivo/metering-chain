//! G1 Demo: Propose → Finalize → Claim → Pay settlement flow.
//!
//! Run: cargo run --example settlement_demo

use metering_chain::evidence;
use metering_chain::state::{ClaimId, NoOpHook, SettlementId, SettlementStatus};
use metering_chain::state::{ClaimStatus, State, StateMachine};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use std::collections::HashSet;

fn main() -> metering_chain::error::Result<()> {
    let mut minters = HashSet::new();
    minters.insert("authority".to_string());

    let mut state = State::new();
    let ctx = ValidationContext::replay();
    let mut sm = StateMachine::new(NoOpHook);

    println!("=== G1 Settlement Demo ===\n");

    // 1. Create usage: mint, open meter, consume
    println!("1. Mint 1000 to alice");
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = sm.apply(&state, &tx1, &ctx, Some(&minters))?;

    println!("2. Open meter (alice, storage, deposit 100)");
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = sm.apply(&state, &tx2, &ctx, Some(&minters))?;

    println!("3. Consume 10 units @ 5 (cost 50)");
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
    state = sm.apply(&state, &tx3, &ctx, Some(&minters))?;

    let gross_spent = 50u64;
    let meter = state.get_meter("alice", "storage").unwrap();
    assert_eq!(meter.total_spent(), gross_spent);
    println!("   Meter total_spent: {}\n", gross_spent);

    // 2. Propose settlement
    let operator_share = 45u64; // 90%
    let protocol_fee = 5u64; // 10%
    let reserve_locked = 0u64;
    let ev_hash = evidence::evidence_hash(b"alice:storage:w1:0:3");

    println!(
        "4. Propose settlement (w1: gross={}, operator={}, protocol={})",
        gross_spent, operator_share, protocol_fee
    );
    let authority_nonce = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let tx_propose = SignedTx::new(
        "authority".to_string(),
        authority_nonce,
        Transaction::ProposeSettlement {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
            from_tx_id: 0,
            to_tx_id: 3,
            gross_spent,
            operator_share,
            protocol_fee,
            reserve_locked,
            evidence_hash: ev_hash,
        },
    );
    state = sm.apply(&state, &tx_propose, &ctx, Some(&minters))?;

    let sid = SettlementId::new("alice".to_string(), "storage".to_string(), "w1".to_string());
    let s = state.get_settlement(&sid).unwrap();
    assert_eq!(s.status, SettlementStatus::Proposed);
    println!("   Settlement status: {:?}\n", s.status);

    // 3. Finalize settlement
    println!("5. Finalize settlement");
    let authority_nonce = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let tx_finalize = SignedTx::new(
        "authority".to_string(),
        authority_nonce,
        Transaction::FinalizeSettlement {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
        },
    );
    state = sm.apply(&state, &tx_finalize, &ctx, Some(&minters))?;
    let s = state.get_settlement(&sid).unwrap();
    assert!(s.is_finalized());
    println!("   Settlement finalized, payable: {}\n", s.payable());

    // 4. Submit claim
    println!("6. Submit claim (alice claims {})", operator_share);
    let alice_nonce = state.get_account("alice").map(|a| a.nonce()).unwrap_or(0);
    let tx_claim = SignedTx::new(
        "alice".to_string(),
        alice_nonce,
        Transaction::SubmitClaim {
            operator: "alice".to_string(),
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
            claim_amount: operator_share,
        },
    );
    state = sm.apply(&state, &tx_claim, &ctx, None)?;
    let cid = ClaimId::new("alice".to_string(), &sid);
    let c = state.get_claim(&cid).unwrap();
    assert_eq!(c.status, ClaimStatus::Pending);
    println!("   Claim status: {:?}\n", c.status);

    // 5. Pay claim
    let alice_before = state.get_account("alice").unwrap().balance();
    println!("7. Pay claim (authority pays alice)");
    let authority_nonce = state
        .get_account("authority")
        .map(|a| a.nonce())
        .unwrap_or(0);
    let tx_pay = SignedTx::new(
        "authority".to_string(),
        authority_nonce,
        Transaction::PayClaim {
            operator: "alice".to_string(),
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            window_id: "w1".to_string(),
        },
    );
    state = sm.apply(&state, &tx_pay, &ctx, Some(&minters))?;
    let alice_after = state.get_account("alice").unwrap().balance();
    let c = state.get_claim(&cid).unwrap();
    assert_eq!(c.status, ClaimStatus::Paid);
    println!(
        "   Claim paid. Alice balance: {} -> {}\n",
        alice_before, alice_after
    );

    println!("=== Done ===");
    Ok(())
}
