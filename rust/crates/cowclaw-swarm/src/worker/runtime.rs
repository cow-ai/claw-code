use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnInput {
    pub plan_id: String,
    pub tasks_xml: String,
    pub skills_manifest: serde_json::Value,
    pub context_budget: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TurnStatus { Done, StuckSameError, ChunkTimeout, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOutput {
    pub summary_md: String,
    pub evidence_paths: Vec<String>,
    pub status: TurnStatus,
}

#[async_trait]
pub trait WorkerRuntime: Send + Sync {
    async fn run_turn(&self, input: TurnInput) -> crate::Result<TurnOutput>;
}

pub struct MockRuntime { pub next_status: TurnStatus }

#[async_trait]
impl WorkerRuntime for MockRuntime {
    async fn run_turn(&self, input: TurnInput) -> crate::Result<TurnOutput> {
        Ok(TurnOutput {
            summary_md: format!("# mock summary for {}", input.plan_id),
            evidence_paths: vec![],
            status: self.next_status.clone(),
        })
    }
}

/// Maps an `api::error::ApiError` to the corresponding `TurnStatus`.
/// `ChunkTimeout` → `TurnStatus::ChunkTimeout` for stuck-detector wiring.
fn api_error_to_turn_status(err: &api::ApiError) -> TurnStatus {
    match err {
        api::ApiError::ChunkTimeout => TurnStatus::ChunkTimeout,
        _ => TurnStatus::Failed,
    }
}

/// Wires to ZAI/MiniMax via api crate. Full streaming impl in later milestone.
pub struct ApiProviderRuntime {
    pub provider_name: String,
    pub model: String,
}

#[async_trait]
impl WorkerRuntime for ApiProviderRuntime {
    async fn run_turn(&self, input: TurnInput) -> crate::Result<TurnOutput> {
        // TODO: full streaming impl in M3.3+; for now return a stub.
        // Error translation: api_error_to_turn_status() routes ChunkTimeout
        // into TurnStatus::ChunkTimeout so stuck-detector fires correctly.
        let _ = (input, api_error_to_turn_status);
        Ok(TurnOutput {
            summary_md: format!("# api stub for {}/{}", self.provider_name, self.model),
            evidence_paths: vec![],
            status: TurnStatus::Done,
        })
    }
}
