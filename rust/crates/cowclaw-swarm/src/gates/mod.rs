pub mod plan_adversarial;
pub mod scope_reduction;
pub mod security;
pub mod oracle_consult;

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GateVerdict {
    Pass,
    Warn { findings: String },
    Fail { findings: String },
}

impl GateVerdict {
    pub fn is_fail(&self) -> bool { matches!(self, GateVerdict::Fail { .. }) }
    pub fn findings(&self) -> Option<&str> {
        match self {
            GateVerdict::Warn { findings } | GateVerdict::Fail { findings } => Some(findings),
            GateVerdict::Pass => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateResult {
    pub gate_name: String,
    pub verdict: GateVerdict,
}

#[derive(Debug)]
pub struct ChainResult {
    pub verdicts: Vec<GateResult>,
    pub blocked_by: Option<String>,
}

impl ChainResult {
    pub fn passed(&self) -> bool { self.blocked_by.is_none() }
}

#[async_trait]
pub trait Gate: Send + Sync {
    fn name(&self) -> &str;
    async fn run(&self, plan_id: &str) -> crate::Result<GateVerdict>;
}

pub struct MockGate {
    name: String,
    verdict: GateVerdict,
}

impl MockGate {
    pub fn pass(name: &str) -> Self {
        Self { name: name.to_string(), verdict: GateVerdict::Pass }
    }
    pub fn fail(name: &str, findings: &str) -> Self {
        Self { name: name.to_string(), verdict: GateVerdict::Fail { findings: findings.to_string() } }
    }
    pub fn warn(name: &str, findings: &str) -> Self {
        Self { name: name.to_string(), verdict: GateVerdict::Warn { findings: findings.to_string() } }
    }
}

#[async_trait]
impl Gate for MockGate {
    fn name(&self) -> &str { &self.name }
    async fn run(&self, _plan_id: &str) -> crate::Result<GateVerdict> {
        Ok(self.verdict.clone())
    }
}

pub struct GateChain {
    gates: Vec<Box<dyn Gate>>,
}

impl GateChain {
    pub fn new(gates: Vec<Box<dyn Gate>>) -> Self { Self { gates } }

    pub async fn run(&self, plan_id: &str) -> crate::Result<ChainResult> {
        let mut verdicts = Vec::new();
        let mut blocked_by = None;
        for gate in &self.gates {
            let verdict = gate.run(plan_id).await?;
            let is_fail = verdict.is_fail();
            let name = gate.name().to_string();
            verdicts.push(GateResult { gate_name: name.clone(), verdict });
            if is_fail {
                blocked_by = Some(name);
                break; // stop on first fail
            }
        }
        Ok(ChainResult { verdicts, blocked_by })
    }

    pub async fn run_with_events(
        &self, plan_id: &str, session_id: &str,
        ew: &mut crate::events::writer::EventWriter,
    ) -> crate::Result<ChainResult> {
        let mut verdicts = Vec::new();
        let mut blocked_by = None;
        for gate in &self.gates {
            let verdict = gate.run(plan_id).await?;
            let is_fail = verdict.is_fail();
            let name = gate.name().to_string();
            // Emit GateRun event
            use crate::events::{Event, Kind};
            use chrono::Utc;
            ew.append(&Event {
                id: None, session_id: session_id.to_string(),
                phase_id: None, plan_id: Some(plan_id.to_string()),
                worker_id: None, ts: Utc::now(),
                kind: Kind::GateRun,
                payload: serde_json::json!({
                    "gate": name,
                    "verdict": format!("{:?}", verdict),
                }),
            }).map_err(crate::Error::Sql)?;
            verdicts.push(GateResult { gate_name: name.clone(), verdict });
            if is_fail {
                blocked_by = Some(name);
                break;
            }
        }
        Ok(ChainResult { verdicts, blocked_by })
    }
}
