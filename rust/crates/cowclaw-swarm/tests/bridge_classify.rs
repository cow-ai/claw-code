use std::process::Command;

fn run_classify(objective: &str) -> serde_json::Value {
    let bin = env!("CARGO_BIN_EXE_cowclaw-swarm");
    let out = Command::new(bin)
        .args(["--classify-only", "--objective", objective])
        .output()
        .expect("failed to run cowclaw-swarm");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("invalid JSON output")
}

#[test]
fn p1_typo_fix() {
    let r = run_classify("fix typo in CLAUDE.md");
    assert_eq!(r["auto_profile"], "P1");
    assert_eq!(r["final_profile"], "P1");
    assert_eq!(r["dials"]["swarm"], false);
}

#[test]
fn p4_dark_mode_toggle() {
    let r = run_classify("add dark-mode toggle to fixture app");
    assert_eq!(r["auto_profile"], "P4");
    assert_eq!(r["final_profile"], "P4");
    assert_eq!(r["dials"]["swarm"], true);
    assert_eq!(r["dials"]["main_cap"], 0.70);
}

#[test]
fn p6_auth_middleware_force_escalate() {
    let r = run_classify("rewrite auth middleware per compliance brief");
    assert_eq!(r["final_profile"], "P6");
    assert_eq!(r["dials"]["swarm"], true);
}
