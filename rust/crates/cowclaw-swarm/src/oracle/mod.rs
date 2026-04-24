use std::path::PathBuf;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct OracleConfig {
    pub model: String,
    pub daily_cap: u32,
    pub timeout_secs: u64,
    pub cap_state_path: PathBuf,
    /// Override for testing — use specific claude binary path
    pub claude_path: Option<PathBuf>,
}

pub struct Oracle { cfg: OracleConfig }

#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    #[error("daily cap reached")] CapReached,
    #[error("timeout")] Timeout,
    #[error("subprocess error: {0}")] Proc(String),
    #[error("io: {0}")] Io(#[from] std::io::Error),
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
struct CapState {
    day: String,
    count: u32,
}

impl Oracle {
    #[must_use]
    pub fn new(cfg: OracleConfig) -> Self { Self { cfg } }

    pub async fn consult(&self, prompt: &str, _plan_id: &str) -> Result<String, OracleError> {
        self.check_and_increment_cap()?;
        let claude = self.cfg.claude_path.clone()
            .unwrap_or_else(|| PathBuf::from("claude"));
        let out = tokio::time::timeout(
            std::time::Duration::from_secs(self.cfg.timeout_secs),
            Command::new(&claude)
                .args(["-p", "--model", &self.cfg.model, prompt])
                .output()
        ).await
            .map_err(|_| OracleError::Timeout)?
            .map_err(|e| OracleError::Proc(e.to_string()))?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    pub async fn consult_with_events(
        &self, prompt: &str, plan_id: &str, session_id: &str,
        ew: &mut crate::events::writer::EventWriter,
    ) -> Result<String, OracleError> {
        use crate::events::{Event, Kind};
        use chrono::Utc;
        ew.append(&Event {
            id: None, session_id: session_id.to_string(),
            phase_id: None, plan_id: Some(plan_id.to_string()),
            worker_id: None, ts: Utc::now(),
            kind: Kind::OracleEscalate, payload: serde_json::json!({"prompt": prompt}),
        }).map_err(|e| OracleError::Proc(e.to_string()))?;
        let answer = self.consult(prompt, plan_id).await?;
        ew.append(&Event {
            id: None, session_id: session_id.to_string(),
            phase_id: None, plan_id: Some(plan_id.to_string()),
            worker_id: None, ts: Utc::now(),
            kind: Kind::OracleAnswer, payload: serde_json::json!({"answer": &answer}),
        }).map_err(|e| OracleError::Proc(e.to_string()))?;
        Ok(answer)
    }

    fn check_and_increment_cap(&self) -> Result<(), OracleError> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let mut state: CapState = if self.cfg.cap_state_path.exists() {
            let s = std::fs::read_to_string(&self.cfg.cap_state_path)?;
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            CapState::default()
        };
        if state.day != today {
            state = CapState { day: today, count: 0 };
        }
        if state.count >= self.cfg.daily_cap {
            return Err(OracleError::CapReached);
        }
        state.count += 1;
        // Atomic write: temp file + rename
        let tmp = self.cfg.cap_state_path.with_extension("tmp");
        if let Some(parent) = tmp.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&tmp, serde_json::to_string(&state).unwrap())?;
        std::fs::rename(&tmp, &self.cfg.cap_state_path)?;
        Ok(())
    }
}
