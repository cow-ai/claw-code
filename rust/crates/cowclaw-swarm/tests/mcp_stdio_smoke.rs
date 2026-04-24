use cowclaw_swarm::mcp_stdio::{read_message, write_message};
use cowclaw_swarm::mcp_stdio::server::McpServer;
use tempfile::TempDir;
use tokio::io::duplex;

// ── helpers ──────────────────────────────────────────────────────────────────

async fn roundtrip_server(
    root: &std::path::Path,
    db_path: &std::path::Path,
    req: serde_json::Value,
) -> serde_json::Value {
    let (mut client_w, mut server_r) = duplex(65536);
    let (mut server_w, mut client_r) = duplex(65536);

    let server = McpServer::new(root.to_path_buf(), db_path.to_path_buf());

    write_message(&mut client_w, &req).await.unwrap();
    drop(client_w); // signal EOF after one message

    server.serve(&mut server_r, &mut server_w).await.unwrap();
    drop(server_w);

    read_message(&mut client_r).await.unwrap()
}

// ── M12.1: framing roundtrip ─────────────────────────────────────────────────

#[tokio::test]
async fn framing_roundtrip() {
    let (mut writer, mut reader) = duplex(4096);
    let msg = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "test", "params": {}});
    write_message(&mut writer, &msg).await.unwrap();
    let received = read_message(&mut reader).await.unwrap();
    assert_eq!(msg, received);
}

#[tokio::test]
async fn framing_large_payload() {
    let (mut writer, mut reader) = duplex(1 << 20);
    let big = "x".repeat(50_000);
    let msg = serde_json::json!({"data": big});
    write_message(&mut writer, &msg).await.unwrap();
    let received = read_message(&mut reader).await.unwrap();
    assert_eq!(msg["data"], received["data"]);
}

// ── M12.2: initialize + tools/list ───────────────────────────────────────────

#[tokio::test]
async fn initialize_returns_capabilities() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("swarm.db");

    let resp = roundtrip_server(
        tmp.path(),
        &db,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "0.1"}
            }
        }),
    )
    .await;

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
    assert!(resp["result"]["capabilities"].is_object());
    assert_eq!(resp["result"]["serverInfo"]["name"], "cowclaw-swarm");
}

#[tokio::test]
async fn tools_list_returns_five_tools() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("swarm.db");

    let resp = roundtrip_server(
        tmp.path(),
        &db,
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
    )
    .await;

    let tools = resp["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 5, "expected 5 tools, got {}", tools.len());

    let names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"cowclaw_plan_decompose"),  "missing cowclaw_plan_decompose");
    assert!(names.contains(&"cowclaw_swarm_execute"),   "missing cowclaw_swarm_execute");
    assert!(names.contains(&"cowclaw_swarm_execute_wave"), "missing cowclaw_swarm_execute_wave");
    assert!(names.contains(&"cowclaw_swarm_status"),    "missing cowclaw_swarm_status");
    assert!(names.contains(&"cowclaw_swarm_retro"),     "missing cowclaw_swarm_retro");
}

// ── M12.3: cowclaw_plan_decompose ────────────────────────────────────────────

#[tokio::test]
async fn plan_decompose_e2e() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("swarm.db");

    let resp = roundtrip_server(
        tmp.path(),
        &db,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "cowclaw_plan_decompose",
                "arguments": {"objective": "add dark-mode toggle", "profile": "P4"}
            }
        }),
    )
    .await;

    assert!(resp.get("error").is_none(), "unexpected error: {resp}");
    let text = resp["result"]["content"][0]["text"]
        .as_str()
        .expect("content text");
    let parsed: serde_json::Value = serde_json::from_str(text).expect("valid JSON in content");
    assert!(parsed["phase_id"].is_string(), "phase_id missing");
    assert!(parsed["plan_ids"].is_array(), "plan_ids missing");
    assert_eq!(parsed["profile"], "P4");
}

// ── M12.4: cowclaw_swarm_execute + _execute_wave + _status ───────────────────

#[tokio::test]
async fn swarm_execute_e2e() {
    use std::process::Command;
    let tmp = TempDir::new().unwrap();
    // init git repo so ScratchWorktree can work
    Command::new("git")
        .args(["init", tmp.path().to_str().unwrap()])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", tmp.path().to_str().unwrap(), "config", "user.email", "t@t.com"])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", tmp.path().to_str().unwrap(), "config", "user.name", "T"])
        .output()
        .unwrap();
    std::fs::write(tmp.path().join("README.md"), "init").unwrap();
    Command::new("git")
        .args(["-C", tmp.path().to_str().unwrap(), "add", "."])
        .output()
        .unwrap();
    Command::new("git")
        .args(["-C", tmp.path().to_str().unwrap(), "commit", "-m", "init"])
        .output()
        .unwrap();

    let db = tmp.path().join("swarm.db");
    let resp = roundtrip_server(
        tmp.path(),
        &db,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "cowclaw_swarm_execute",
                "arguments": {"plan_id": "plan-test-01"}
            }
        }),
    )
    .await;

    assert!(resp.get("error").is_none(), "unexpected error: {resp}");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["status"], "Done");
}

#[tokio::test]
async fn swarm_status_e2e() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("swarm.db");

    let resp = roundtrip_server(
        tmp.path(),
        &db,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "cowclaw_swarm_status",
                "arguments": {"n": 10}
            }
        }),
    )
    .await;

    assert!(resp.get("error").is_none(), "unexpected error: {resp}");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(parsed["events"].is_array(), "events array missing");
    assert!(parsed["event_count"].is_number(), "event_count missing");
}

// ── M12.5: cowclaw_swarm_retro ───────────────────────────────────────────────

#[tokio::test]
async fn swarm_retro_e2e() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("swarm.db");

    let resp = roundtrip_server(
        tmp.path(),
        &db,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "cowclaw_swarm_retro",
                "arguments": {"phase_id": "phase-smoke"}
            }
        }),
    )
    .await;

    assert!(resp.get("error").is_none(), "unexpected error: {resp}");
    let text = resp["result"]["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["phase_id"], "phase-smoke");
    assert_eq!(parsed["retro"], "D-rule retro complete");
    // RETRO.md should have been written
    assert!(
        tmp.path().join("phase-smoke").join("RETRO.md").exists(),
        "RETRO.md not written"
    );
}

// ── unknown method error ──────────────────────────────────────────────────────

#[tokio::test]
async fn unknown_method_returns_error() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("swarm.db");

    let resp = roundtrip_server(
        tmp.path(),
        &db,
        serde_json::json!({"jsonrpc": "2.0", "id": 99, "method": "no_such_method", "params": {}}),
    )
    .await;

    assert!(resp["error"].is_object(), "expected error response");
    assert_eq!(resp["error"]["code"], -32601);
}
