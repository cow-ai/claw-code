use async_trait::async_trait;
use crate::gates::{Gate, GateVerdict};
use crate::worker::runtime::{WorkerRuntime, TurnInput};

pub struct PlanAdversarialGate<R: WorkerRuntime> {
    pub runtime: R,
    pub gate_prompt: String,
}

impl<R: WorkerRuntime> PlanAdversarialGate<R> {
    pub fn new(runtime: R, gate_prompt: String) -> Self { Self { runtime, gate_prompt } }
}

#[async_trait]
impl<R: WorkerRuntime + 'static> Gate for PlanAdversarialGate<R> {
    fn name(&self) -> &'static str { "plan_adversarial" }

    async fn run(&self, plan_id: &str) -> crate::Result<GateVerdict> {
        let prompt = format!("{}\n\nPlan ID: {}", self.gate_prompt, plan_id);
        let out = self.runtime.run_turn(TurnInput {
            plan_id: plan_id.to_string(),
            tasks_xml: prompt,
            skills_manifest: serde_json::json!({}),
            context_budget: 50_000,
        }).await?;
        // Parse JSON from summary_md
        Ok(parse_gate_verdict(&out.summary_md))
    }
}

pub(crate) fn parse_gate_verdict(response: &str) -> GateVerdict {
    // Try to find JSON in response
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            let json_str = &response[start..=end];
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
                let verdict = val.get("verdict").and_then(|v| v.as_str()).unwrap_or("pass");
                let findings = val.get("findings").and_then(|v| v.as_str()).unwrap_or("").to_string();
                return match verdict {
                    "fail" => GateVerdict::Fail { findings },
                    "warn" => GateVerdict::Warn { findings },
                    _ => GateVerdict::Pass,
                };
            }
        }
    }
    // Default: pass if we can't parse
    GateVerdict::Pass
}
