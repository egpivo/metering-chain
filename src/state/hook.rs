//! ApplyHook: injectable strategy for metering/settlement interception (WS-R1).
//!
//! StateMachine<M: ApplyHook> coordinates the hook and core state transitions.
//! Phase 4 Settlement can implement ApplyHook to record consumption for settlement windows.

use crate::error::Result;

/// Hook for metering/settlement interception during apply.
///
/// Default impl is a no-op. Phase 4 SettlementHook will record consumption
/// for settlement window building.
pub trait ApplyHook {
    /// Called after a Consume is recorded in state.
    /// Use for settlement recording, usage aggregation, etc.
    fn on_consume_recorded(
        &mut self,
        _owner: &str,
        _service_id: &str,
        _units: u64,
        _cost: u64,
        _cap_id: Option<&str>,
    ) -> Result<()> {
        Ok(())
    }
}

/// No-op hook: default for backward compatibility.
#[derive(Debug, Clone, Default)]
pub struct NoOpHook;

impl ApplyHook for NoOpHook {}
