use rusqlite::{Connection, params};
use crate::events::{Event, Kind, schema};
use std::path::Path;
use chrono::DateTime;

pub struct EventReader { conn: Connection }

impl EventReader {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let mut conn = Connection::open(path)?;
        schema::apply(&mut conn)?;
        Ok(Self { conn })
    }

    fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
        let id: i64 = row.get(0)?;
        let session_id: String = row.get(1)?;
        let phase_id: Option<String> = row.get(2)?;
        let plan_id: Option<String> = row.get(3)?;
        let worker_id: Option<String> = row.get(4)?;
        let ts_str: String = row.get(5)?;
        let kind_str: String = row.get(6)?;
        let payload_str: String = row.get(7)?;

        let ts = DateTime::parse_from_rfc3339(&ts_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let kind: Kind = serde_json::from_str(&format!("\"{kind_str}\""))
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                6,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        let payload: serde_json::Value = serde_json::from_str(&payload_str)
            .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                7,
                rusqlite::types::Type::Text,
                Box::new(e),
            ))?;

        Ok(Event {
            id: Some(id),
            session_id,
            phase_id,
            plan_id,
            worker_id,
            ts,
            kind,
            payload,
        })
    }

    pub fn by_session_kind(&self, session_id: &str, kind: &Kind) -> rusqlite::Result<Vec<Event>> {
        let kind_str = serde_json::to_string(kind).unwrap();
        let kind_str = kind_str.trim_matches('"');
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, phase_id, plan_id, worker_id, ts, kind, payload
             FROM events WHERE session_id = ?1 AND kind = ?2 ORDER BY id"
        )?;
        let rows = stmt.query_map(params![session_id, kind_str], Self::row_to_event)?;
        rows.collect()
    }

    pub fn by_plan_id(&self, plan_id: &str) -> rusqlite::Result<Vec<Event>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, phase_id, plan_id, worker_id, ts, kind, payload
             FROM events WHERE plan_id = ?1 ORDER BY id"
        )?;
        let rows = stmt.query_map(params![plan_id], Self::row_to_event)?;
        rows.collect()
    }

    pub fn tail(&self, n: usize) -> rusqlite::Result<Vec<Event>> {
        let n_i64 = i64::try_from(n).unwrap_or(i64::MAX);
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, phase_id, plan_id, worker_id, ts, kind, payload
             FROM events ORDER BY id DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![n_i64], Self::row_to_event)?;
        let mut events: Vec<Event> = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        events.reverse();
        Ok(events)
    }

    pub fn count(&self) -> rusqlite::Result<usize> {
        self.conn.query_row("SELECT COUNT(*) FROM events", [], |r| r.get::<_, i64>(0))
            .map(|n| usize::try_from(n).unwrap_or(0))
    }
}
