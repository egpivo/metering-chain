//! Demo: ApplyHook injection for settlement recording (Phase 4 prep).
//!
//! Run: cargo run --example hook_demo

use metering_chain::state::{apply, ApplyHook, State, StateMachine};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use std::collections::HashSet;

/// Example hook that logs consumption (Phase 4: replace with settlement recording).
struct LoggingHook;

impl ApplyHook for LoggingHook {
    fn on_consume_recorded(
        &mut self,
        owner: &str,
        service_id: &str,
        units: u64,
        cost: u64,
        cap_id: Option<&str>,
    ) -> metering_chain::error::Result<()> {
        eprintln!(
            "[Hook] Consume recorded: owner={} service={} units={} cost={} cap_id={:?}",
            owner, service_id, units, cost, cap_id
        );
        Ok(())
    }
}

fn main() -> metering_chain::error::Result<()> {
    let mut minters = HashSet::new();
    minters.insert("authority".to_string());

    let mut state = State::new();
    let ctx = ValidationContext::replay();

    // Mint
    let tx1 = SignedTx::new(
        "authority".to_string(),
        0,
        Transaction::Mint {
            to: "alice".to_string(),
            amount: 1000,
        },
    );
    state = apply(&state, &tx1, &ctx, Some(&minters))?;

    // Open meter
    let tx2 = SignedTx::new(
        "alice".to_string(),
        0,
        Transaction::OpenMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
            deposit: 100,
        },
    );
    state = apply(&state, &tx2, &ctx, Some(&minters))?;

    // Consume with StateMachine<LoggingHook> — hook will log
    let mut sm = StateMachine::new(LoggingHook);
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

    println!("Balance: {}", state.get_account("alice").unwrap().balance());
    Ok(())
}
