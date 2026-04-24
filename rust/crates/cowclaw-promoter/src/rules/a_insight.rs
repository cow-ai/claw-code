use rusqlite::Connection;
use cowclaw_swarm::memory::mempalace::{Drawer, MemPalaceClient};

pub struct RuleA;

impl RuleA {
    pub fn run(conn: &Connection, palace: &MemPalaceClient) -> cowclaw_swarm::Result<usize> {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, plan_id, question, response, outcome
             FROM peer_consults
             WHERE status='closed' AND outcome IS NOT NULL AND response IS NOT NULL",
        )?;
        let rows: Vec<(i64, String, Option<String>, String, String, String)> = stmt
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;

        let mut count = 0;
        for (id, session_id, plan_id, question, response, outcome) in rows {
            let drawer = Drawer {
                wing: "cowclaw".to_string(),
                title: format!(
                    "Peer Consult Insight: {}",
                    question.chars().take(60).collect::<String>()
                ),
                body: format!(
                    "Session: {}\nPlan: {}\nQuestion: {}\nAnswer: {}\nOutcome: {}",
                    session_id,
                    plan_id.unwrap_or_default(),
                    question,
                    response,
                    outcome
                ),
                tags: vec!["peer_consult".to_string(), "insight".to_string()],
            };
            palace.add_drawer(&drawer)?;
            count += 1;
            let _ = id; // would update promoted_at in production
        }
        Ok(count)
    }
}
