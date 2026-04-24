use cowclaw_swarm::gates::{GateChain, MockGate};

#[tokio::test]
async fn gate_chain_blocks_on_fail() {
    let chain = GateChain::new(vec![
        Box::new(MockGate::pass("plan_adversarial")),
        Box::new(MockGate::fail("scope_reduction", "requirement X dropped")),
        Box::new(MockGate::pass("security")),
    ]);
    let r = chain.run("plan-01").await.unwrap();
    assert_eq!(r.blocked_by, Some("scope_reduction".to_string()));
    assert_eq!(r.verdicts.len(), 2); // stopped after fail
    assert!(!r.passed());
}

#[tokio::test]
async fn gate_chain_passes_all_pass() {
    let chain = GateChain::new(vec![
        Box::new(MockGate::pass("gate_a")),
        Box::new(MockGate::warn("gate_b", "minor issue")),
        Box::new(MockGate::pass("gate_c")),
    ]);
    let r = chain.run("plan-02").await.unwrap();
    assert!(r.blocked_by.is_none());
    assert_eq!(r.verdicts.len(), 3); // all gates ran
    assert!(r.passed());
}

#[tokio::test]
async fn gate_chain_empty_passes() {
    let chain = GateChain::new(vec![]);
    let r = chain.run("plan-empty").await.unwrap();
    assert!(r.passed());
    assert_eq!(r.verdicts.len(), 0);
}

#[tokio::test]
async fn plan_adversarial_with_mock_runtime_pass() {
    use cowclaw_swarm::gates::plan_adversarial::PlanAdversarialGate;
    use cowclaw_swarm::gates::Gate;

    // MockRuntime returns summary_md with JSON verdict
    struct JsonMockRuntime { verdict: &'static str }

    use async_trait::async_trait;
    use cowclaw_swarm::worker::runtime::{WorkerRuntime, TurnInput, TurnOutput, TurnStatus};

    #[async_trait]
    impl WorkerRuntime for JsonMockRuntime {
        async fn run_turn(&self, _input: TurnInput) -> cowclaw_swarm::Result<TurnOutput> {
            Ok(TurnOutput {
                summary_md: format!("{{\"verdict\":\"{}\",\"findings\":\"ok\"}}", self.verdict),
                evidence_paths: vec![],
                status: TurnStatus::Done,
            })
        }
    }

    let gate = PlanAdversarialGate::new(JsonMockRuntime { verdict: "pass" }, "review this".into());
    let v = gate.run("plan-01").await.unwrap();
    assert_eq!(v, cowclaw_swarm::gates::GateVerdict::Pass);

    let gate_fail = PlanAdversarialGate::new(JsonMockRuntime { verdict: "fail" }, "review".into());
    let v2 = gate_fail.run("plan-01").await.unwrap();
    assert!(v2.is_fail());
}

#[tokio::test]
async fn security_gate_fails_on_sensitive_plan_id() {
    use cowclaw_swarm::gates::security::SecurityGate;
    use cowclaw_swarm::gates::Gate;

    let gate = SecurityGate;
    let v = gate.run("plan-safe").await.unwrap();
    assert_eq!(v, cowclaw_swarm::gates::GateVerdict::Pass);

    let v2 = gate.run("plan-with-~/.ssh-path").await.unwrap();
    assert!(v2.is_fail());
}

#[tokio::test]
async fn gate_chain_emits_gate_results_to_db() {
    use cowclaw_swarm::gates::{GateChain, MockGate};
    use cowclaw_swarm::events::{writer::EventWriter, reader::EventReader, Kind};
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    let db_path = td.path().join("swarm.db");
    let mut ew = EventWriter::open(&db_path).unwrap();

    let chain = GateChain::new(vec![
        Box::new(MockGate::pass("plan_adversarial")),
        Box::new(MockGate::warn("scope_reduction", "minor overlap")),
    ]);

    let result = chain.run_with_events("plan-gate-test", "sess1", &mut ew).await.unwrap();
    assert!(result.passed());

    let reader = EventReader::open(&db_path).unwrap();
    let events = reader.by_plan_id("plan-gate-test").unwrap();
    let gate_events: Vec<_> = events.iter().filter(|e| e.kind == Kind::GateRun).collect();
    assert_eq!(gate_events.len(), 2);
}
