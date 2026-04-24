use cowclaw_swarm::events::{schema, writer::EventWriter, reader::EventReader, Kind, Event};
use tempfile::TempDir;
use rusqlite::Connection;

#[test]
fn schema_creates_all_tables() {
    let td = TempDir::new().unwrap();
    let path = td.path().join("swarm.db");
    let mut conn = Connection::open(&path).unwrap();
    schema::apply(&mut conn).unwrap();
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tables, vec![
        "events", "gate_results", "peer_consults",
        "planning_index", "skill_manifests", "tool_sequences",
    ]);
}

#[test]
fn event_kind_roundtrip_json() {
    use cowclaw_swarm::events::Kind;
    let cases = vec![
        Kind::SessionStart, Kind::SessionEnd,
        Kind::PhaseStart, Kind::PhaseEnd,
        Kind::PlanStart, Kind::PlanEnd, Kind::PlanDecompose,
        Kind::GateRun,
        Kind::WorkerStart, Kind::WorkerTurn, Kind::WorkerStuck, Kind::WorkerEnd,
        Kind::PeerConsultOpen, Kind::PeerConsultAnswer, Kind::PeerConsultClose,
        Kind::OracleEscalate, Kind::OracleAnswer,
        Kind::SkillLoad,
        Kind::ChunkTimeout,
    ];
    for k in &cases {
        let s = serde_json::to_string(k).unwrap();
        let back: Kind = serde_json::from_str(&s).unwrap();
        assert_eq!(&back, k);
    }
}

#[test]
fn writer_appends_and_assigns_id() {
    let td = TempDir::new().unwrap();
    let path = td.path().join("s.db");
    let mut w = EventWriter::open(&path).unwrap();
    let ev = Event {
        id: None, session_id: "s1".into(), phase_id: None, plan_id: None,
        worker_id: None, ts: chrono::Utc::now(), kind: Kind::SessionStart,
        payload: serde_json::json!({"hello": "world"}),
    };
    let id = w.append(&ev).unwrap();
    assert!(id > 0);
    let second = w.append(&ev).unwrap();
    assert_eq!(second, id + 1);
}

#[test]
fn writer_refuses_update() {
    let td = TempDir::new().unwrap();
    let path = td.path().join("s.db");
    let mut w = EventWriter::open(&path).unwrap();
    let result = w.try_update(1, "kind", "changed");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("append-only"));
}

#[test]
fn reader_filters_by_session_and_kind() {
    let td = TempDir::new().unwrap();
    let path = td.path().join("s.db");
    let mut w = EventWriter::open(&path).unwrap();
    for k in [Kind::SessionStart, Kind::PhaseStart, Kind::PlanStart] {
        w.append(&Event { id: None, session_id: "s1".into(), phase_id: None, plan_id: None,
            worker_id: None, ts: chrono::Utc::now(), kind: k, payload: serde_json::json!({})
        }).unwrap();
    }
    let r = EventReader::open(&path).unwrap();
    let phases = r.by_session_kind("s1", &Kind::PhaseStart).unwrap();
    assert_eq!(phases.len(), 1);
}

#[test]
fn wal_concurrent_writes_no_corruption() {
    let td = TempDir::new().unwrap();
    let path = td.path().join("s.db");
    let mut w = EventWriter::open(&path).unwrap();
    for i in 0..200u64 {
        w.append(&Event {
            id: None, session_id: "s1".into(), phase_id: None, plan_id: None,
            worker_id: None, ts: chrono::Utc::now(), kind: Kind::WorkerTurn,
            payload: serde_json::json!({"n": i}),
        }).unwrap();
    }
    let r = EventReader::open(&path).unwrap();
    assert_eq!(r.count().unwrap(), 200);
}
