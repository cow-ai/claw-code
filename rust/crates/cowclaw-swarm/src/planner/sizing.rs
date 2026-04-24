use crate::planner::xml_plan::{Task, XmlPlan};
use crate::budget::Tier;

pub struct SizingConfig {
    pub auto_split_over: usize,  // line threshold to split
    pub auto_merge_under: usize, // line threshold to merge
    pub context_headroom: f32,   // reserved headroom fraction
    pub default_tier: Tier,
}

/// Estimate lines for a task based on content length.
/// Each task has 4 XML wrapper lines plus prose content estimation.
/// We use chars/8 as an approximate line count so that a 60-char action
/// registers as ~8 lines (representative of typical prose wrapping at ~80 cols).
fn estimate_task_lines(task: &Task) -> usize {
    let content_len = task.action.len() + task.verify.len() + task.done.len();
    8 + (content_len / 8).max(1) // 8 wrapper lines + prose estimate
}

pub fn size_and_split(
    phase: &str,
    wave: &str,
    tasks: Vec<Task>,
    cfg: &SizingConfig,
) -> Vec<XmlPlan> {
    let effective_cap = (cfg.auto_split_over as f32 * (1.0 - cfg.context_headroom)) as usize;
    let mut plans: Vec<XmlPlan> = Vec::new();
    let mut current_tasks: Vec<Task> = Vec::new();
    let mut current_lines: usize = 20; // header overhead

    for task in tasks {
        let task_lines = estimate_task_lines(&task);
        if current_lines + task_lines > effective_cap && !current_tasks.is_empty() {
            plans.push(make_plan(phase, wave, plans.len(), std::mem::take(&mut current_tasks), current_lines, cfg));
            current_lines = 20;
        }
        current_lines += task_lines;
        current_tasks.push(task);
    }
    if !current_tasks.is_empty() {
        plans.push(make_plan(phase, wave, plans.len(), current_tasks, current_lines, cfg));
    }
    if plans.is_empty() {
        plans.push(make_plan(phase, wave, 0, vec![], 20, cfg));
    }
    plans
}

fn make_plan(phase: &str, wave: &str, idx: usize, tasks: Vec<Task>, lines: usize, cfg: &SizingConfig) -> XmlPlan {
    XmlPlan {
        id: format!("{}/{}/plan-{:02}", phase, wave, idx + 1),
        title: format!("Auto-split plan {} of {}", idx + 1, phase),
        wave: format!("{}/{}", phase, wave),
        depends: vec![],
        files: vec![],
        skills_required: vec![],
        tasks,
        budget_tier: cfg.default_tier,
        budget_lines: lines.min(cfg.auto_split_over) as u32,
        commit_message_hint: String::new(),
    }
}
