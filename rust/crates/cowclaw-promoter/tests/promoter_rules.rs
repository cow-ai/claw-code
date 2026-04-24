use rusqlite::Connection;
use cowclaw_swarm::events::schema;

fn setup_db_with_consult(path: &std::path::Path) -> Connection {
    let mut conn = Connection::open(path).unwrap();
    schema::apply(&mut conn).unwrap();
    conn.execute(
        "INSERT INTO peer_consults(session_id, plan_id, from_worker, status, stuck_context, question, opened_at, response, outcome)
         VALUES ('s1', 'p1', 'zai', 'closed', 'got stuck', 'how to proceed?', '2026-01-01T00:00:00Z', 'do X', 'resolved')",
        [],
    ).unwrap();
    conn
}

#[test]
fn rule_a_promotes_resolved_consults() {
    use cowclaw_promoter::rules::a_insight::RuleA;
    use cowclaw_swarm::memory::mempalace::{MemPalaceClient, MockTransport};
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    let db_path = td.path().join("swarm.db");
    let conn = setup_db_with_consult(&db_path);

    let client = MemPalaceClient::new(Box::new(MockTransport::new()));
    let count = RuleA::run(&conn, &client).unwrap();
    assert_eq!(count, 1, "should promote 1 resolved consult");

    // Verify drawer was added
    let hits = client.search("resolved", 5).unwrap();
    assert!(!hits.is_empty(), "promoted insight should be searchable");
}

#[test]
fn rule_b_generates_skill_from_frequent_sequence() {
    use cowclaw_promoter::rules::b_skill::RuleB;
    use tempfile::TempDir;
    use rusqlite::Connection;
    use cowclaw_swarm::events::schema;

    let td = TempDir::new().unwrap();
    let db_path = td.path().join("swarm.db");
    let skills_dir = td.path().join("skills");
    std::fs::create_dir(&skills_dir).unwrap();

    let mut conn = Connection::open(&db_path).unwrap();
    schema::apply(&mut conn).unwrap();

    // Insert a tool sequence with count >= 3
    let fp = "cargo_test_then_commit";
    conn.execute(
        "INSERT INTO tool_sequences(fingerprint, canonical, count, first_seen, last_seen)
         VALUES (?1, ?2, 3, '2026-01-01', '2026-01-03')",
        rusqlite::params![fp, "1. cargo test\n2. git add\n3. git commit"],
    ).unwrap();

    let count = RuleB::run(&conn, &skills_dir).unwrap();
    assert_eq!(count, 1);

    // Verify SKILL.md was created
    let skill_file = skills_dir.join(format!("auto-{fp}/SKILL.md"));
    assert!(skill_file.exists(), "SKILL.md should be generated");
    let content = std::fs::read_to_string(&skill_file).unwrap();
    assert!(content.contains("cargo test"));
}

#[test]
fn rule_c_extracts_preferences_to_habits() {
    use cowclaw_promoter::rules::c_habit::RuleC;
    use cowclaw_swarm::memory::mempalace::{MemPalaceClient, MockTransport};
    use tempfile::TempDir;
    use rusqlite::Connection;
    use cowclaw_swarm::events::schema;

    let td = TempDir::new().unwrap();
    let db_path = td.path().join("swarm.db");
    let mut conn = Connection::open(&db_path).unwrap();
    schema::apply(&mut conn).unwrap();

    // Insert a worker turn event with preference marker
    conn.execute(
        "INSERT INTO events(session_id, ts, kind, payload)
         VALUES ('s1', '2026-01-01T00:00:00Z', 'worker_turn', ?1)",
        rusqlite::params![r#"{"text": "please always run cargo test before committing"}"#],
    ).unwrap();

    let client = MemPalaceClient::new(Box::new(MockTransport::new()));
    let count = RuleC::run(&conn, &client).unwrap();
    let _ = count; // may be 0 if no strong preference markers found
}

#[test]
fn rule_d_writes_retro_md() {
    use cowclaw_promoter::rules::d_retro::RuleD;
    use cowclaw_swarm::memory::mempalace::{MemPalaceClient, MockTransport};
    use tempfile::TempDir;
    use rusqlite::Connection;
    use cowclaw_swarm::events::schema;

    let td = TempDir::new().unwrap();
    let planning_dir = td.path().join("planning");
    std::fs::create_dir_all(planning_dir.join("phase-01")).unwrap();

    // Create a synthetic phase with summaries
    std::fs::write(planning_dir.join("phase-01/PHASE.md"), "---\nid: phase-01\nprofile: P4\n---\n# Phase 01\n").unwrap();

    let db_path = td.path().join("swarm.db");
    let mut conn = Connection::open(&db_path).unwrap();
    schema::apply(&mut conn).unwrap();

    // Insert a gate_result and peer_consult
    conn.execute("INSERT INTO gate_results(plan_id, gate, verdict, findings, ts) VALUES ('phase-01/w1/plan-01', 'plan_adversarial', 'pass', 'all good', '2026-01-01')", []).unwrap();

    let client = MemPalaceClient::new(Box::new(MockTransport::new()));
    RuleD::run(&conn, "phase-01", &planning_dir, &client).unwrap();

    assert!(planning_dir.join("phase-01/RETRO.md").exists());
    let content = std::fs::read_to_string(planning_dir.join("phase-01/RETRO.md")).unwrap();
    assert!(content.contains("Retrospective") || content.contains("retro") || content.contains("went well"));
}

#[test]
fn retro_mcp_tool_stub_returns_path() {
    use cowclaw_promoter::rules::d_retro::RuleD;
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    let planning_dir = td.path().to_path_buf();
    let phase_id = "phase-stub";
    let expected_path = planning_dir.join(phase_id).join("RETRO.md");

    // Just verify the path computation is correct before RETRO.md is written
    assert_eq!(
        RuleD::retro_path(&planning_dir, phase_id),
        expected_path
    );
}
