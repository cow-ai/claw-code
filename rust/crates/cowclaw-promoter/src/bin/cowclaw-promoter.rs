use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args
        .iter()
        .position(|a| a == "--mode")
        .and_then(|i| args.get(i + 1))
        .map_or("session_end", String::as_str);

    let db_path = PathBuf::from(".cowclaw/swarm.db");
    let skills_dir = PathBuf::from(".cowclaw/skills");
    let planning_dir = PathBuf::from(".cowclaw/planning");

    match mode {
        "session_end" => {
            eprintln!("[cowclaw-promoter] session_end: running rules A, B, C");
            if let Err(e) = run_session_end(&db_path, &skills_dir) {
                eprintln!("[cowclaw-promoter] error (non-fatal): {e}");
            }
        }
        "phase_end" => {
            let phase_id = args
                .iter()
                .position(|a| a == "--phase-id")
                .and_then(|i| args.get(i + 1))
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            eprintln!("[cowclaw-promoter] phase_end for {phase_id}: running rule D");
            if let Err(e) = run_phase_end(&db_path, &planning_dir, &phase_id) {
                eprintln!("[cowclaw-promoter] error (non-fatal): {e}");
            }
        }
        other => {
            eprintln!("[cowclaw-promoter] unknown mode: {other}");
            std::process::exit(1);
        }
    }
}

fn run_session_end(db_path: &Path, skills_dir: &Path) -> cowclaw_swarm::Result<()> {
    use cowclaw_swarm::memory::mempalace::{MemPalaceClient, MockTransport};
    use cowclaw_promoter::rules::{a_insight::RuleA, b_skill::RuleB, c_habit::RuleC};

    if !db_path.exists() {
        eprintln!("[cowclaw-promoter] swarm.db not found, skipping");
        return Ok(());
    }

    let conn = rusqlite::Connection::open(db_path)?;
    std::fs::create_dir_all(skills_dir)?;

    // Use mock transport in binary (real MCP wiring in M13)
    let palace = MemPalaceClient::new(Box::new(MockTransport::new()));

    let a = RuleA::run(&conn, &palace)?;
    let b = RuleB::run(&conn, skills_dir)?;
    let c = RuleC::run(&conn, &palace)?;
    eprintln!("[cowclaw-promoter] Rule A: {a} drawers, Rule B: {b} skills, Rule C: {c} habits");
    Ok(())
}

fn run_phase_end(
    db_path: &Path,
    planning_dir: &Path,
    phase_id: &str,
) -> cowclaw_swarm::Result<()> {
    use cowclaw_swarm::memory::mempalace::{MemPalaceClient, MockTransport};
    use cowclaw_promoter::rules::d_retro::RuleD;

    let conn = rusqlite::Connection::open(db_path)?;
    let palace = MemPalaceClient::new(Box::new(MockTransport::new()));
    RuleD::run(&conn, phase_id, planning_dir, &palace)?;
    eprintln!("[cowclaw-promoter] Rule D: retro written for {phase_id}");
    Ok(())
}
