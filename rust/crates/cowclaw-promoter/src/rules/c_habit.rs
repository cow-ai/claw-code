use rusqlite::Connection;
use cowclaw_swarm::memory::mempalace::{Drawer, MemPalaceClient};

const PREFERENCE_MARKERS: &[&str] = &[
    "please always",
    "always use",
    "never use",
    "i prefer",
    "prefer to",
    "make sure to",
    "remember to",
    "don't forget",
];

pub struct RuleC;

impl RuleC {
    pub fn run(conn: &Connection, palace: &MemPalaceClient) -> cowclaw_swarm::Result<usize> {
        let mut stmt = conn.prepare(
            "SELECT payload FROM events WHERE kind='worker_turn' ORDER BY id DESC LIMIT 100",
        )?;
        let payloads: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<_>>()?;

        let mut count = 0;
        for payload in payloads {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&payload) {
                let text = val.get("text").and_then(|t| t.as_str()).unwrap_or("");
                let text_lower = text.to_lowercase();
                if PREFERENCE_MARKERS.iter().any(|m| text_lower.contains(m)) {
                    let drawer = Drawer {
                        wing: "habits".to_string(),
                        title: format!(
                            "Preference: {}",
                            text.chars().take(60).collect::<String>()
                        ),
                        body: text.to_string(),
                        tags: vec!["habit".to_string(), "preference".to_string()],
                    };
                    palace.add_drawer(&drawer)?;
                    count += 1;
                }
            }
        }
        Ok(count)
    }
}
