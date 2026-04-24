use rusqlite::{Connection, params};
use std::path::Path;
use crate::events::schema;
use crate::hivemind::protocol::ConsultRequest;

pub struct Broker { conn: Connection }

impl Broker {
    pub fn open(path: &Path) -> crate::Result<Self> {
        let mut conn = Connection::open(path)?;
        schema::apply(&mut conn)?;
        Ok(Self { conn })
    }

    /// Exposed only for tests — not part of public API.
    #[doc(hidden)]
    pub fn conn_for_test(&self) -> &Connection { &self.conn }

    pub fn open_consult(&mut self, req: &ConsultRequest) -> crate::Result<i64> {
        self.conn.execute(
            "INSERT INTO peer_consults(session_id, plan_id, from_worker, status, stuck_context, question, opened_at)
             VALUES (?1, ?2, ?3, 'open', ?4, ?5, ?6)",
            params![
                req.session_id, req.plan_id, req.from_worker,
                req.stuck_context, req.question,
                chrono::Utc::now().to_rfc3339()
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn close_consult(&mut self, id: i64, to_worker: &str, resp: &str, outcome: &str) -> crate::Result<()> {
        self.conn.execute(
            "UPDATE peer_consults SET status='closed', to_worker=?1, response=?2, outcome=?3, closed_at=?4 WHERE id=?5",
            params![to_worker, resp, outcome, chrono::Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    pub fn pending_consults(&self) -> crate::Result<Vec<i64>> {
        let mut stmt = self.conn.prepare("SELECT id FROM peer_consults WHERE status='open'")?;
        let ids: Vec<i64> = stmt.query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ids)
    }
}
