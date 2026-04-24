use serde_json::{Map, Value};
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncWrite};

use super::{read_message, write_message};

pub struct McpServer {
    pub root: PathBuf,
    pub db_path: PathBuf,
}

impl McpServer {
    #[must_use]
    pub fn new(root: PathBuf, db_path: PathBuf) -> Self {
        Self { root, db_path }
    }

    /// Run the request/response loop until EOF.
    pub async fn serve<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        reader: &mut R,
        writer: &mut W,
    ) -> crate::Result<()> {
        loop {
            let Ok(msg) = read_message(reader).await else {
                break;
            };

            let id = msg.get("id").cloned().unwrap_or(Value::Null);
            let method = msg
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let params = msg
                .get("params")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::default()));

            let response = self.dispatch(id, &method, &params).await;
            // "initialized" notification has no response
            if response.is_null() {
                continue;
            }
            write_message(writer, &response).await?;
        }
        Ok(())
    }

    async fn dispatch(&self, id: Value, method: &str, params: &Value) -> Value {
        match method {
            "initialize" => handle_initialize(&id),
            "initialized" => Value::Null,
            "tools/list" => handle_tools_list(&id),
            "tools/call" => self.handle_tools_call(id, params).await,
            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("method not found: {method}")}
            }),
        }
    }

    async fn handle_tools_call(&self, id: Value, params: &Value) -> Value {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::default()));

        let result = match name.as_str() {
            "cowclaw_plan_decompose" => self.tool_plan_decompose(&args),
            "cowclaw_swarm_execute" => self.tool_swarm_execute(&args).await,
            "cowclaw_swarm_execute_wave" => self.tool_swarm_execute_wave(&args).await,
            "cowclaw_swarm_status" => self.tool_swarm_status(&args),
            "cowclaw_swarm_retro" => self.tool_swarm_retro(&args),
            _ => Err(crate::Error::Other(format!("unknown tool: {name}"))),
        };

        match result {
            Ok(content) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [{"type": "text", "text": content.to_string()}]
                }
            }),
            Err(e) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32000, "message": e.to_string()}
            }),
        }
    }

    fn tool_plan_decompose(&self, args: &Value) -> crate::Result<Value> {
        let objective = args
            .get("objective")
            .and_then(Value::as_str)
            .ok_or_else(|| crate::Error::Other("missing objective".into()))?;
        let profile_str = args.get("profile").and_then(Value::as_str).unwrap_or("P4");
        let profile = parse_profile(profile_str);

        let result = crate::planner::decompose(objective, profile, &self.root)?;
        Ok(serde_json::json!({
            "phase_id": result.phase_id,
            "plan_ids": result.plan_ids,
            "profile": result.profile.to_string(),
        }))
    }

    async fn tool_swarm_execute(&self, args: &Value) -> crate::Result<Value> {
        use crate::events::writer::EventWriter;
        use crate::worker::{
            runtime::{MockRuntime, TurnStatus},
            Worker,
        };

        let plan_id = args
            .get("plan_id")
            .and_then(Value::as_str)
            .ok_or_else(|| crate::Error::Other("missing plan_id".into()))?
            .to_owned();

        let session_id = new_session_id("session");
        let worker = Worker {
            id: format!("worker-{}", truncate_id(&plan_id, 8)),
            plan_id: plan_id.clone(),
            phase_id: "phase-unknown".to_string(),
            session_id,
            runtime: MockRuntime {
                next_status: TurnStatus::Done,
            },
            stuck_threshold: 3,
        };

        let mut ew = EventWriter::open(&self.db_path)?;
        let output = worker.execute(&self.root, String::new(), &mut ew).await?;

        Ok(serde_json::json!({
            "status": format!("{:?}", output.status),
            "summary_md": output.summary_md,
            "evidence_paths": output.evidence_paths,
        }))
    }

    async fn tool_swarm_execute_wave(&self, args: &Value) -> crate::Result<Value> {
        use crate::planner::artifacts::PlanningTree;

        let phase_id = args
            .get("phase_id")
            .and_then(Value::as_str)
            .ok_or_else(|| crate::Error::Other("missing phase_id".into()))?
            .to_owned();
        let wave_id = args
            .get("wave_id")
            .and_then(Value::as_str)
            .ok_or_else(|| crate::Error::Other("missing wave_id".into()))?
            .to_owned();
        let max_parallel: usize = args
            .get("max_parallel_workers")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(4);

        let phases = PlanningTree::load(&self.root)?;
        let full_wave_id = format!("{phase_id}/{wave_id}");
        let plan_ids: Vec<String> = phases
            .iter()
            .filter(|p| p.id == phase_id)
            .flat_map(|p| p.waves.iter())
            .filter(|w| w.id == wave_id || w.id == full_wave_id)
            .flat_map(|w| w.plans.iter())
            .map(|p| p.id.clone())
            .collect();

        let session_id = new_session_id("session-wave");
        let mut results = Vec::new();

        for chunk in plan_ids.chunks(max_parallel) {
            let mut handles = Vec::new();
            for pid in chunk {
                let pid = pid.clone();
                let phase_id_clone = phase_id.clone();
                let root = self.root.clone();
                let db_path = self.db_path.clone();
                let sid = session_id.clone();
                handles.push(tokio::spawn(async move {
                    use crate::events::writer::EventWriter;
                    use crate::worker::{
                        runtime::{MockRuntime, TurnStatus},
                        Worker,
                    };
                    let worker = Worker {
                        id: format!("worker-{}", truncate_id(&pid, 8)),
                        plan_id: pid.clone(),
                        phase_id: phase_id_clone,
                        session_id: sid,
                        runtime: MockRuntime {
                            next_status: TurnStatus::Done,
                        },
                        stuck_threshold: 3,
                    };
                    let mut ew = EventWriter::open(&db_path)?;
                    worker.execute(&root, String::new(), &mut ew).await
                }));
            }
            for (pid, handle) in chunk.iter().zip(handles) {
                match handle.await {
                    Ok(Ok(out)) => results.push(serde_json::json!({
                        "plan_id": pid,
                        "status": format!("{:?}", out.status),
                    })),
                    Ok(Err(e)) => results.push(serde_json::json!({
                        "plan_id": pid,
                        "error": e.to_string(),
                    })),
                    Err(e) => results.push(serde_json::json!({
                        "plan_id": pid,
                        "error": e.to_string(),
                    })),
                }
            }
        }

        Ok(serde_json::json!({ "plan_results": results }))
    }

    fn tool_swarm_status(&self, args: &Value) -> crate::Result<Value> {
        use crate::events::reader::EventReader;

        let n = args
            .get("n")
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(20);
        let session_id = args.get("session_id").and_then(Value::as_str);

        let reader = EventReader::open(&self.db_path)?;
        let events = if let Some(sid) = session_id {
            reader.by_session_kind(sid, &crate::events::Kind::WorkerEnd)?
        } else {
            reader.tail(n)?
        };

        let events_json: Vec<Value> = events
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "session_id": e.session_id,
                    "plan_id": e.plan_id,
                    "kind": format!("{:?}", e.kind),
                    "ts": e.ts.to_rfc3339(),
                    "payload": e.payload,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "events": events_json,
            "event_count": events_json.len(),
        }))
    }

    fn tool_swarm_retro(&self, args: &Value) -> crate::Result<Value> {
        use crate::events::reader::EventReader;

        let phase_id = args
            .get("phase_id")
            .and_then(Value::as_str)
            .ok_or_else(|| crate::Error::Other("missing phase_id".into()))?;

        let reader = EventReader::open(&self.db_path)?;
        let tail = reader.tail(200)?;
        let completed_workers = tail
            .iter()
            .filter(|e| {
                e.phase_id.as_deref() == Some(phase_id)
                    && e.kind == crate::events::Kind::WorkerEnd
            })
            .count();

        let retro_md = format!(
            "---\nphase_id: {phase_id}\ngenerated_by: cowclaw_swarm_retro\n---\n\
             # Phase Retrospective: {phase_id}\n\n\
             ## Completed workers\n\n{completed_workers} worker(s) completed\n\n\
             ## Do differently next time\n\n- (to be filled by reflection)\n"
        );
        let phase_dir = self.root.join(phase_id);
        std::fs::create_dir_all(&phase_dir)?;
        std::fs::write(phase_dir.join("RETRO.md"), &retro_md)?;

        Ok(serde_json::json!({
            "phase_id": phase_id,
            "retro": "D-rule retro complete",
            "completed_workers": completed_workers,
            "retro_path": phase_dir.join("RETRO.md").to_string_lossy(),
        }))
    }
}

fn handle_initialize(id: &Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "protocolVersion": "2024-11-05",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "cowclaw-swarm", "version": "0.1.0"}
        }
    })
}

#[must_use]
pub fn handle_tools_list(id: &Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "tools": tool_schemas()
        }
    })
}

fn tool_schemas() -> Value {
    serde_json::json!([
        {
            "name": "cowclaw_plan_decompose",
            "description": "Decompose an objective into phases and plans on disk",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "objective": {"type": "string"},
                    "profile": {"type": "string", "default": "P4"},
                    "phase_scope": {"type": "string"}
                },
                "required": ["objective"]
            }
        },
        {
            "name": "cowclaw_swarm_execute",
            "description": "Execute one plan in a fresh worker context",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "plan_id": {"type": "string"}
                },
                "required": ["plan_id"]
            }
        },
        {
            "name": "cowclaw_swarm_execute_wave",
            "description": "Execute a whole wave in parallel",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "phase_id": {"type": "string"},
                    "wave_id": {"type": "string"},
                    "max_parallel_workers": {"type": "integer", "default": 4}
                },
                "required": ["phase_id", "wave_id"]
            }
        },
        {
            "name": "cowclaw_swarm_status",
            "description": "Query sessions, workers, events and gate results",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "session_id": {"type": "string"},
                    "n": {"type": "integer", "default": 20}
                }
            }
        },
        {
            "name": "cowclaw_swarm_retro",
            "description": "Force D-rule retro for a completed phase",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "phase_id": {"type": "string"}
                },
                "required": ["phase_id"]
            }
        }
    ])
}

fn parse_profile(s: &str) -> crate::planner::profile::ProfileId {
    use crate::planner::profile::ProfileId;
    match s {
        "P1" => ProfileId::P1,
        "P2" => ProfileId::P2,
        "P3" => ProfileId::P3,
        "P5" => ProfileId::P5,
        "P6" => ProfileId::P6,
        "P7" => ProfileId::P7,
        "P8" => ProfileId::P8,
        "P9" => ProfileId::P9,
        _ => ProfileId::P4,
    }
}

fn truncate_id(s: &str, max: usize) -> &str {
    let end = s.char_indices().nth(max).map_or(s.len(), |(i, _)| i);
    &s[..end]
}

fn new_session_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{prefix}-{nanos:08x}")
}
