use rusqlite::{Connection, params};
use crate::events::{Event, schema};
use std::path::Path;

pub struct EventWriter { conn: Connection }

impl EventWriter {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let mut conn = Connection::open(path)?;
        schema::apply(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn append(&mut self, ev: &Event) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO events(session_id, phase_id, plan_id, worker_id, ts, kind, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                ev.session_id,
                ev.phase_id,
                ev.plan_id,
                ev.worker_id,
                ev.ts.to_rfc3339(),
                serde_json::to_string(&ev.kind).unwrap().trim_matches('"'),
                serde_json::to_string(&ev.payload).unwrap(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Explicitly disabled. Event log is append-only.
    pub fn try_update(&mut self, _id: i64, _col: &str, _val: &str) -> Result<(), String> {
        Err("append-only".to_string())
    }
}
