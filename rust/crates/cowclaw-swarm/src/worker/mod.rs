pub mod runtime;
pub mod scratch;
pub mod stuck;

use crate::events::{writer::EventWriter, Event, Kind};
use chrono::Utc;
use serde_json::json;
use std::path::Path;
use stuck::StuckDetector;

pub struct Worker<R: runtime::WorkerRuntime> {
    pub id: String,
    pub plan_id: String,
    pub phase_id: String,
    pub session_id: String,
    pub runtime: R,
    pub stuck_threshold: usize,
}

impl<R: runtime::WorkerRuntime> Worker<R> {
    pub async fn execute(
        &self,
        repo_root: &Path,
        tasks_xml: String,
        ew: &mut EventWriter,
    ) -> crate::Result<runtime::TurnOutput> {
        let scratch = scratch::ScratchWorktree::create(repo_root, &self.plan_id)?;
        ew.append(&self.ev(
            Kind::WorkerStart,
            json!({"scratch": scratch.path().to_string_lossy()}),
        ))?;

        let mut detector = StuckDetector::new(self.stuck_threshold);
        let out = self.runtime.run_turn(runtime::TurnInput {
            plan_id: self.plan_id.clone(),
            tasks_xml,
            skills_manifest: json!({}),
            context_budget: 200_000,
        }).await?;

        // Check if the status itself signals stuck
        let final_status = match &out.status {
            runtime::TurnStatus::StuckSameError => {
                // Runtime already signals stuck — emit WorkerStuck unconditionally
                detector.observe("stuck_same_error");
                ew.append(&self.ev(Kind::WorkerStuck, json!({"reason": "same_error"})))?;
                out.status.clone()
            }
            runtime::TurnStatus::ChunkTimeout => {
                detector.observe_chunk_timeout();
                ew.append(&self.ev(Kind::WorkerStuck, json!({"reason": "chunk_timeout"})))?;
                out.status.clone()
            }
            other => other.clone(),
        };

        ew.append(&self.ev(
            Kind::WorkerEnd,
            json!({"status": format!("{:?}", final_status)}),
        ))?;
        Ok(out)
    }

    fn ev(&self, kind: Kind, payload: serde_json::Value) -> Event {
        Event {
            id: None,
            session_id: self.session_id.clone(),
            phase_id: Some(self.phase_id.clone()),
            plan_id: Some(self.plan_id.clone()),
            worker_id: Some(self.id.clone()),
            ts: Utc::now(),
            kind,
            payload,
        }
    }
}
