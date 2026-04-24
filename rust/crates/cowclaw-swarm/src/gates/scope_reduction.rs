use async_trait::async_trait;
use crate::gates::{Gate, GateVerdict};

/// Stub: scope_reduction gate. Full implementation wires WorkerRuntime + REQUIREMENTS.md parsing.
pub struct ScopeReductionGate;

#[async_trait]
impl Gate for ScopeReductionGate {
    fn name(&self) -> &str { "scope_reduction" }

    async fn run(&self, _plan_id: &str) -> crate::Result<GateVerdict> {
        // Stub: always pass. Full impl parses REQUIREMENTS.md and diffs with PLAN.xml tasks.
        Ok(GateVerdict::Pass)
    }
}
