use cowclaw_swarm::planner::profile::{ProfileId, ProfileTable};

#[test]
fn profile_id_inline_check() {
    assert!(ProfileId::P1.is_inline());
    assert!(ProfileId::P2.is_inline());
    assert!(!ProfileId::P4.is_inline());
    assert!(!ProfileId::P9.is_inline());
}

#[test]
fn profile_table_parses_toml() {
    let toml = r#"
[profiles.P4]
swarm = true
gates = ["plan_adversarial", "scope_reduction"]
main_cap = 0.70
worker_cap = 0.85
retro = false

[profiles.P1]
swarm = false
gates = []
main_cap = 1.0
worker_cap = 1.0
retro = false
"#;
    let table = ProfileTable::from_toml(toml).unwrap();
    let p4 = table.get(ProfileId::P4).unwrap();
    assert!((p4.main_cap - 0.70).abs() < 0.01);
    assert_eq!(p4.gates, vec!["plan_adversarial", "scope_reduction"]);
    let p1 = table.get(ProfileId::P1).unwrap();
    assert!(!p1.swarm);
}

#[test]
fn force_escalate_upgrades_p1_to_p6() {
    use cowclaw_swarm::planner::escalate::apply_force_escalate;
    let patterns = vec![
        "**/migrations/**".to_string(),
        "**/auth/**".into(),
        ".patches/upstream/**".into(),
    ];
    assert_eq!(apply_force_escalate(ProfileId::P1, &["src/lib.rs"], &patterns), ProfileId::P1);
    assert_eq!(apply_force_escalate(ProfileId::P1, &["migrations/001_init.sql"], &patterns), ProfileId::P6);
    assert_eq!(apply_force_escalate(ProfileId::P4, &["src/auth/login.rs"], &patterns), ProfileId::P6);
    // already >= P6 stays
    assert_eq!(apply_force_escalate(ProfileId::P9, &["migrations/x.sql"], &patterns), ProfileId::P9);
}

#[test]
fn xml_plan_roundtrip() {
    use cowclaw_swarm::planner::xml_plan::{XmlPlan, Task};
    use cowclaw_swarm::budget::Tier;
    let p = XmlPlan {
        id: "phase-03/wave-02/plan-01".into(),
        title: "Wire swarm MCP".into(),
        wave: "phase-03/wave-02".into(),
        depends: vec!["phase-03/wave-01/plan-02".into()],
        files: vec!["src/bin/cowclaw-swarm.rs".into()],
        skills_required: vec!["rust-mcp-server".into()],
        tasks: vec![Task {
            id: "t1".into(),
            action: "impl stdio loop".into(),
            verify: "cargo test passes".into(),
            done: "server accepts initialize".into(),
        }],
        budget_tier: Tier::Default,
        budget_lines: 220,
        commit_message_hint: "feat(swarm): MCP stdio".into(),
    };
    let s = p.to_xml().unwrap();
    assert!(s.contains("Wire swarm MCP"));
    let back = XmlPlan::from_xml(&s).unwrap();
    assert_eq!(back.id, p.id);
    assert_eq!(back.tasks.len(), 1);
    assert_eq!(back.tasks[0].id, "t1");
}

#[test]
fn sizing_splits_large_task_set() {
    use cowclaw_swarm::planner::sizing::{size_and_split, SizingConfig};
    use cowclaw_swarm::planner::xml_plan::Task;
    use cowclaw_swarm::budget::Tier;

    let tasks: Vec<Task> = (0..20).map(|i| Task {
        id: format!("t{i}"),
        action: "x".repeat(60),
        verify: "ok".into(),
        done: "done".into(),
    }).collect();

    let cfg = SizingConfig {
        auto_split_over: 500,
        auto_merge_under: 80,
        context_headroom: 0.40,
        default_tier: Tier::Default,
    };
    let plans = size_and_split("phase-00", "wave-00", tasks, &cfg);
    assert!(plans.len() >= 2, "20 large tasks should split into >=2 plans");
    for p in &plans {
        let estimated = p.budget_lines as usize;
        assert!(estimated <= cfg.auto_split_over,
            "plan {} estimated {} > {}", p.id, estimated, cfg.auto_split_over);
    }
}

#[test]
fn sizing_keeps_small_task_set_single_plan() {
    use cowclaw_swarm::planner::sizing::{size_and_split, SizingConfig};
    use cowclaw_swarm::planner::xml_plan::Task;
    use cowclaw_swarm::budget::Tier;

    let tasks: Vec<Task> = (0..3).map(|i| Task {
        id: format!("t{i}"),
        action: "short".into(), verify: "ok".into(), done: "done".into(),
    }).collect();
    let cfg = SizingConfig { auto_split_over: 500, auto_merge_under: 80,
        context_headroom: 0.40, default_tier: Tier::Default };
    let plans = size_and_split("phase-00", "wave-00", tasks, &cfg);
    assert_eq!(plans.len(), 1, "3 small tasks should stay in 1 plan");
}

#[test]
fn planning_tree_write_and_reload() {
    use cowclaw_swarm::planner::artifacts::{PlanningTree, PhaseNode, WaveNode};
    use cowclaw_swarm::planner::xml_plan::{XmlPlan, Task};
    use cowclaw_swarm::planner::profile::ProfileId;
    use cowclaw_swarm::budget::Tier;
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();

    let plan = XmlPlan {
        id: "ph1/w1/plan-01".into(), title: "Test plan".into(),
        wave: "ph1/w1".into(), depends: vec![], files: vec![],
        skills_required: vec![], tasks: vec![Task {
            id: "t1".into(), action: "do thing".into(),
            verify: "check it".into(), done: "done it".into(),
        }],
        budget_tier: Tier::Default, budget_lines: 100,
        commit_message_hint: "feat: test".into(),
    };

    let wave = WaveNode { id: "ph1/w1".into(), plans: vec![plan] };
    let phase = PhaseNode {
        id: "ph1".into(),
        profile: ProfileId::P4,
        waves: vec![wave],
    };

    PlanningTree::write(td.path(), &phase).unwrap();

    // Verify files exist
    assert!(td.path().join("ph1/PHASE.md").exists());
    assert!(td.path().join("ph1/w1/WAVE.md").exists());
    assert!(td.path().join("ph1/w1/plan-01/PLAN.xml").exists());

    // Reload
    let phases = PlanningTree::load(td.path()).unwrap();
    assert_eq!(phases.len(), 1);
    assert_eq!(phases[0].id, "ph1");
    assert_eq!(phases[0].profile, ProfileId::P4);
    assert_eq!(phases[0].waves.len(), 1);
    assert_eq!(phases[0].waves[0].plans.len(), 1);
}

#[test]
fn decompose_stub_writes_phase_and_plans() {
    use cowclaw_swarm::planner::decompose;
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    let result = decompose("add user toggle feature", ProfileId::P4, td.path()).unwrap();
    assert!(!result.phase_id.is_empty());
    assert!(!result.plan_ids.is_empty());
    assert_eq!(result.profile, ProfileId::P4);

    // PHASE.md should exist
    assert!(td.path().join(&result.phase_id).join("PHASE.md").exists());
}

#[test]
fn decompose_escalates_profile_for_migration_files() {
    use cowclaw_swarm::planner::escalate::apply_force_escalate;

    // Simulates: objective touches migrations/, input profile P1 → must escalate to P6
    let patterns = vec!["**/migrations/**".to_string(), ".patches/upstream/**".into()];
    let files = ["migrations/001_add_users.sql"];
    let result = apply_force_escalate(ProfileId::P1, &files, &patterns);
    assert_eq!(result, ProfileId::P6);

    // P9 stays P9 even with escalate paths
    let result9 = apply_force_escalate(ProfileId::P9, &files, &patterns);
    assert_eq!(result9, ProfileId::P9);
}
