pub mod artifacts;
pub mod xml_plan;
pub mod sizing;
pub mod profile;
pub mod escalate;

use std::path::Path;
use crate::planner::profile::ProfileId;
use crate::planner::xml_plan::Task;
use crate::planner::artifacts::{PhaseNode, PlanningTree, WaveNode};
use crate::planner::sizing::{size_and_split, SizingConfig};
use crate::budget::Tier;

pub struct DecomposeResult {
    pub phase_id: String,
    pub plan_ids: Vec<String>,
    pub profile: ProfileId,
}

pub fn decompose(objective: &str, profile: ProfileId, root: &Path) -> crate::Result<DecomposeResult> {
    // Stub decomposition: create a single phase with one wave and one plan
    let phase_id = format!("phase-{:04x}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos() & 0xFFFF);

    // Stub: single task derived from objective
    let tasks = vec![Task {
        id: "t1".into(),
        action: format!("Implement: {}", &objective[..objective.len().min(100)]),
        verify: "cargo test passes".into(),
        done: "feature works as described".into(),
    }];

    let cfg = SizingConfig {
        auto_split_over: 500,
        auto_merge_under: 80,
        context_headroom: 0.40,
        default_tier: Tier::Default,
    };
    let wave_id = "wave-01";
    let plans = size_and_split(&phase_id, wave_id, tasks, &cfg);
    let plan_ids: Vec<String> = plans.iter().map(|p| p.id.clone()).collect();

    let phase = PhaseNode {
        id: phase_id.clone(),
        profile,
        waves: vec![WaveNode {
            id: format!("{}/{}", phase_id, wave_id),
            plans,
        }],
    };
    PlanningTree::write(root, &phase)?;

    Ok(DecomposeResult { phase_id, plan_ids, profile })
}
