use rusqlite::Connection;

pub const CREATE_SQL: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA busy_timeout=5000;

CREATE TABLE IF NOT EXISTS events (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL,
  phase_id   TEXT,
  plan_id    TEXT,
  worker_id  TEXT,
  ts         TEXT NOT NULL,
  kind       TEXT NOT NULL,
  payload    TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_phase   ON events(phase_id);
CREATE INDEX IF NOT EXISTS idx_events_plan    ON events(plan_id);
CREATE INDEX IF NOT EXISTS idx_events_kind    ON events(kind);

CREATE TABLE IF NOT EXISTS peer_consults (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id    TEXT NOT NULL,
  plan_id       TEXT,
  from_worker   TEXT NOT NULL,
  to_worker     TEXT,
  status        TEXT NOT NULL,
  stuck_context TEXT NOT NULL,
  question      TEXT NOT NULL,
  response      TEXT,
  outcome       TEXT,
  opened_at     TEXT NOT NULL,
  closed_at     TEXT
);

CREATE TABLE IF NOT EXISTS tool_sequences (
  fingerprint    TEXT PRIMARY KEY,
  canonical      TEXT NOT NULL,
  count          INTEGER NOT NULL DEFAULT 1,
  first_seen     TEXT NOT NULL,
  last_seen      TEXT NOT NULL,
  promoted_skill TEXT
);

CREATE TABLE IF NOT EXISTS skill_manifests (
  name          TEXT PRIMARY KEY,
  path          TEXT NOT NULL,
  description   TEXT NOT NULL,
  trigger_hints TEXT NOT NULL,
  source        TEXT NOT NULL,
  line_count    INTEGER NOT NULL,
  tier          TEXT NOT NULL,
  created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS planning_index (
  id         TEXT PRIMARY KEY,
  kind       TEXT NOT NULL,
  path       TEXT NOT NULL,
  status     TEXT NOT NULL,
  parent_id  TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS gate_results (
  id       INTEGER PRIMARY KEY AUTOINCREMENT,
  plan_id  TEXT NOT NULL,
  gate     TEXT NOT NULL,
  verdict  TEXT NOT NULL,
  findings TEXT NOT NULL,
  ts       TEXT NOT NULL
);
"#;

pub fn apply(conn: &mut Connection) -> rusqlite::Result<()> {
    conn.execute_batch(CREATE_SQL)?;
    Ok(())
}
