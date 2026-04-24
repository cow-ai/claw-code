use async_trait::async_trait;
use crate::gates::{Gate, GateVerdict};
use crate::oracle::Oracle;

pub struct OracleConsultGate {
    pub oracle: Oracle,
    pub gate_prompt: String,
}

impl OracleConsultGate {
    pub fn new(oracle: Oracle, gate_prompt: String) -> Self { Self { oracle, gate_prompt } }
}

#[async_trait]
impl Gate for OracleConsultGate {
    fn name(&self) -> &str { "oracle_consult" }

    async fn run(&self, plan_id: &str) -> crate::Result<GateVerdict> {
        let prompt = format!("{}\n\nPlan: {}", self.gate_prompt, plan_id);
        let response = self.oracle.consult(&prompt, plan_id).await
            .unwrap_or_else(|e| format!("{{\"verdict\":\"warn\",\"findings\":\"oracle unavailable: {e}\"}}"));
        // Parse response
        if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&response[start..=end]) {
                    let verdict = val.get("verdict").and_then(|v| v.as_str()).unwrap_or("pass");
                    let findings = val.get("findings").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    return Ok(match verdict {
                        "fail" => GateVerdict::Fail { findings },
                        "warn" => GateVerdict::Warn { findings },
                        _ => GateVerdict::Pass,
                    });
                }
            }
        }
        Ok(GateVerdict::Warn { findings: response.chars().take(200).collect() })
    }
}
