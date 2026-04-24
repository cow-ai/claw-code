use async_trait::async_trait;
use crate::gates::{Gate, GateVerdict};

const DENY_PATTERNS: &[&str] = &[
    "~/.ssh", ".ssh/id_rsa", ".ssh/id_ed25519",
    "/etc/passwd", "/etc/shadow",
    ".env", "secrets/", "credentials",
];

pub struct SecurityGate;

#[async_trait]
impl Gate for SecurityGate {
    fn name(&self) -> &str { "security" }

    async fn run(&self, plan_id: &str) -> crate::Result<GateVerdict> {
        // Check if plan_id or any associated files match deny patterns
        // Stub: check plan_id itself for obvious sensitive patterns
        for pattern in DENY_PATTERNS {
            if plan_id.contains(pattern) {
                return Ok(GateVerdict::Fail {
                    findings: format!("plan_id contains sensitive path: {}", pattern),
                });
            }
        }
        Ok(GateVerdict::Pass)
    }
}
