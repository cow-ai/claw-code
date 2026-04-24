use cowclaw_swarm::worker::{Worker, runtime::{MockRuntime, TurnStatus}};
use cowclaw_swarm::events::{writer::EventWriter, reader::EventReader, Kind};
use tempfile::TempDir;
use std::process::Command;

fn init_git_repo(path: &std::path::Path) {
    Command::new("git").args(["init", path.to_str().unwrap()]).output().unwrap();
    Command::new("git").args(["-C", path.to_str().unwrap(), "config", "user.email", "test@test.com"]).output().unwrap();
    Command::new("git").args(["-C", path.to_str().unwrap(), "config", "user.name", "Test"]).output().unwrap();
    std::fs::write(path.join("README.md"), "init").unwrap();
    Command::new("git").args(["-C", path.to_str().unwrap(), "add", "."]).output().unwrap();
    Command::new("git").args(["-C", path.to_str().unwrap(), "commit", "-m", "init"]).output().unwrap();
}

#[tokio::test]
async fn worker_roundtrip_emits_start_end() {
    let repo = TempDir::new().unwrap();
    init_git_repo(repo.path());

    let db = TempDir::new().unwrap();
    let db_path = db.path().join("swarm.db");
    let mut ew = EventWriter::open(&db_path).unwrap();

    let worker = Worker {
        id: "w1".into(),
        plan_id: "plan-smoke".into(),
        phase_id: "ph1".into(),
        session_id: "sess1".into(),
        runtime: MockRuntime { next_status: TurnStatus::Done },
        stuck_threshold: 3,
    };

    let out = worker.execute(repo.path(), "<tasks/>".into(), &mut ew).await.unwrap();
    assert_eq!(out.status, TurnStatus::Done);

    let reader = EventReader::open(&db_path).unwrap();
    let events = reader.by_plan_id("plan-smoke").unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind, Kind::WorkerStart);
    assert_eq!(events[1].kind, Kind::WorkerEnd);
    // plan_id set on both events
    assert_eq!(events[0].plan_id.as_deref(), Some("plan-smoke"));
}
