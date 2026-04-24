use cowclaw_swarm::oracle::{Oracle, OracleConfig};
use tempfile::TempDir;
use std::path::PathBuf;

fn oracle_with_mock_claude(cap_dir: &std::path::Path) -> Oracle {
    // Prepend tests/fixtures/bin to PATH so our mock `claude` is found
    let fixtures_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin");
    Oracle::new(OracleConfig {
        model: "claude-opus-4-7".into(),
        daily_cap: 5,
        timeout_secs: 10,
        cap_state_path: cap_dir.join("oracle_usage.json"),
        claude_path: Some(fixtures_bin.join("claude")),
    })
}

#[tokio::test]
async fn oracle_invokes_mock_claude() {
    let td = TempDir::new().unwrap();
    let oracle = oracle_with_mock_claude(td.path());
    let ans = oracle.consult("hello world", "plan-01").await.unwrap();
    assert!(ans.contains("mocked-opus-answer"), "got: {ans}");
}

#[tokio::test]
async fn oracle_cap_reached_on_6th_call() {
    let td = TempDir::new().unwrap();
    let oracle = Oracle::new(OracleConfig {
        model: "claude-opus-4-7".into(),
        daily_cap: 5,
        timeout_secs: 10,
        cap_state_path: td.path().join("oracle_usage.json"),
        claude_path: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bin/claude")),
    });
    for _ in 0..5 {
        oracle.consult("hi", "p1").await.unwrap();
    }
    let err = oracle.consult("hi", "p1").await.unwrap_err();
    assert!(matches!(err, cowclaw_swarm::oracle::OracleError::CapReached));
}

#[tokio::test]
async fn oracle_emits_escalate_and_answer_events() {
    use cowclaw_swarm::events::{writer::EventWriter, reader::EventReader, Kind};
    let td = TempDir::new().unwrap();
    let oracle = oracle_with_mock_claude(td.path());
    let db_path = td.path().join("swarm.db");
    let mut ew = EventWriter::open(&db_path).unwrap();

    oracle.consult_with_events("question", "plan-oracle", "sess1", &mut ew).await.unwrap();

    let reader = EventReader::open(&db_path).unwrap();
    let all = reader.by_plan_id("plan-oracle").unwrap();
    assert!(all.iter().any(|e| e.kind == Kind::OracleEscalate));
    assert!(all.iter().any(|e| e.kind == Kind::OracleAnswer));
}
