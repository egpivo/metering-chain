//! Demo: ApplyHook injection for settlement recording (Phase 4 prep).
//!
//! Run: cargo run --example hook_demo

use metering_chain::state::{apply, Hook, State, StateMachine};
use metering_chain::tx::validation::ValidationContext;
use metering_chain::tx::{Pricing, SignedTx, Transaction};
use std::collections::HashSet;

/// Example hook that logs consumption (Phase 4: replace with settlement recording).
struct LoggingHook;

impl Hook for LoggingHook {
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

    fn on_meter_opened(
        &mut self,
        owner: &str,
        service_id: &str,
        deposit: u64,
    ) -> metering_chain::error::Result<()> {
        eprintln!(
            "[Hook] Meter opened: owner={} service={} deposit={}",
            owner, service_id, deposit
        );
        Ok(())
    }

    fn on_meter_closed(
        &mut self,
        owner: &str,
        service_id: &str,
        deposit_returned: u64,
    ) -> metering_chain::error::Result<()> {
        eprintln!(
            "[Hook] Meter closed: owner={} service={} deposit_returned={}",
            owner, service_id, deposit_returned
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

    // Open meter with StateMachine<LoggingHook> — hook will log
    let mut sm = StateMachine::new(LoggingHook);
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

    // Consume
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

    // Close meter (hook logs on_meter_closed)
    let tx4 = SignedTx::new(
        "alice".to_string(),
        2,
        Transaction::CloseMeter {
            owner: "alice".to_string(),
            service_id: "storage".to_string(),
        },
    );
    state = sm.apply(&state, &tx4, &ctx, Some(&minters))?;

    println!("Balance: {}", state.get_account("alice").unwrap().balance());
    Ok(())
}
