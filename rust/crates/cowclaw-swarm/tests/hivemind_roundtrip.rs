use cowclaw_swarm::hivemind::{broker::Broker, protocol::ConsultRequest};
use tempfile::TempDir;

#[test]
fn broker_open_and_close_consult() {
    let td = TempDir::new().unwrap();
    let db = td.path().join("swarm.db");
    let mut b = Broker::open(&db).unwrap();
    let req = ConsultRequest {
        session_id: "s1".into(), plan_id: "p1".into(),
        from_worker: "zai-glm".into(), stuck_context: "got stuck".into(),
        question: "what to do?".into(), max_response_tokens: 500,
    };
    let id = b.open_consult(&req).unwrap();
    assert!(id > 0);
    b.close_consult(id, "minimax-m2", "do X", "resolved").unwrap();
    // verify status is now closed by direct SQL query
    let status: String = b.conn_for_test()
        .query_row("SELECT status FROM peer_consults WHERE id=?1", [id], |r| r.get::<_, String>(0))
        .unwrap();
    assert_eq!(status, "closed");
}

#[test]
fn pick_responder_prefers_cross_provider() {
    use cowclaw_swarm::hivemind::pick_responder;
    let pool = vec!["zai-glm-5.1".to_string(), "minimax-m-2.7".to_string()];
    let picked = pick_responder("zai-glm-5.1", &pool, true).unwrap();
    assert_eq!(picked, "minimax-m-2.7");
}

#[test]
fn pick_responder_falls_back_same_provider_when_only_option() {
    use cowclaw_swarm::hivemind::pick_responder;
    let pool = vec!["zai-glm-5.1".to_string()];
    let picked = pick_responder("zai-glm-5.1", &pool, true).unwrap();
    assert_eq!(picked, "zai-glm-5.1");
}

#[tokio::test]
async fn hivemind_roundtrip_with_mock_runtimes() {
    use cowclaw_swarm::hivemind::{broker::Broker, protocol::ConsultRequest};
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    let db = td.path().join("swarm.db");
    let mut broker = Broker::open(&db).unwrap();

    let req = ConsultRequest {
        session_id: "sess1".into(),
        plan_id: "plan-e2e".into(),
        from_worker: "zai-glm".into(),
        stuck_context: "Error: tool_call failed 3 times".into(),
        question: "How should I proceed?".into(),
        max_response_tokens: 200,
    };

    // A reports stuck, broker opens consult
    let consult_id = broker.open_consult(&req).unwrap();
    assert!(consult_id > 0);

    // B answers (simulated)
    let answer = "Try a different approach: break the task into smaller steps";
    broker.close_consult(consult_id, "minimax-m2", answer, "resolved").unwrap();

    // Verify closed
    let pending = broker.pending_consults().unwrap();
    assert!(!pending.contains(&consult_id));
}

#[test]
fn pick_responder_returns_error_for_empty_pool() {
    use cowclaw_swarm::hivemind::pick_responder;
    let result = pick_responder("zai-glm", &[], true);
    assert!(result.is_err());
}
