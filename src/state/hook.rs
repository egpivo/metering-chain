//! Hook: injectable strategy for metering/settlement interception (WS-R1).
//!
//! StateMachine<M: Hook> is an orchestrator that calls hook at each lifecycle stage.
//! Phase 4 SettlementHook will implement Hook to record for settlement windows.

use crate::error::Result;

/// Trait-based hook for execution interception during apply.
///
/// Each callback is invoked at the corresponding stage. Default impls are no-op.
/// Phase 4 SettlementHook implements these to record usage for settlement windows.
pub trait Hook {
    /// Called after a Consume is recorded in state.
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

    /// Called after a meter is opened (or reactivated).
    fn on_meter_opened(&mut self, _owner: &str, _service_id: &str, _deposit: u64) -> Result<()> {
        Ok(())
    }

    /// Called after a meter is closed (deposit returned to owner).
    fn on_meter_closed(&mut self, _owner: &str, _service_id: &str, _deposit_returned: u64) -> Result<()> {
        Ok(())
    }
}

/// No-op hook: default for backward compatibility.
#[derive(Debug, Clone, Default)]
pub struct NoOpHook;

impl Hook for NoOpHook {}
