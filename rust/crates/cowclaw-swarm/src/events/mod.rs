use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
pub mod schema;
pub mod writer;
pub mod reader;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    SessionStart, SessionEnd,
    PhaseStart, PhaseEnd,
    PlanStart, PlanEnd, PlanDecompose,
    GateRun,
    WorkerStart, WorkerTurn, WorkerStuck, WorkerEnd,
    PeerConsultOpen, PeerConsultAnswer, PeerConsultClose,
    OracleEscalate, OracleAnswer,
    SkillLoad,
    ChunkTimeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<i64>,
    pub session_id: String,
    pub phase_id: Option<String>,
    pub plan_id: Option<String>,
    pub worker_id: Option<String>,
    pub ts: DateTime<Utc>,
    pub kind: Kind,
    pub payload: serde_json::Value,
}
