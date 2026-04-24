use rusqlite::Connection;
use std::fmt::Write as _;
use std::path::Path;
use cowclaw_swarm::memory::mempalace::{Drawer, MemPalaceClient};

pub struct RuleD;

impl RuleD {
    pub fn run(
        conn: &Connection,
        phase_id: &str,
        planning_dir: &Path,
        palace: &MemPalaceClient,
    ) -> cowclaw_swarm::Result<()> {
        // Gather gate results for this phase
        let mut stmt = conn.prepare(
            "SELECT gate, verdict, findings FROM gate_results WHERE plan_id LIKE ?1",
        )?;
        let pattern = format!("{phase_id}%");
        let gate_rows: Vec<(String, String, String)> = stmt
            .query_map([&pattern], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;

        // Gather peer consults for this phase
        let mut stmt2 = conn.prepare(
            "SELECT question, response, outcome FROM peer_consults WHERE plan_id LIKE ?1 AND status='closed'",
        )?;
        let consult_rows: Vec<(String, Option<String>, Option<String>)> = stmt2
            .query_map([&pattern], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;

        // Build RETRO.md
        let mut retro = format!(
            "---\nphase_id: {phase_id}\ngenerated_by: rule_d\n---\n# Phase Retrospective: {phase_id}\n\n"
        );

        retro.push_str("## What went well\n\n");
        let passed: Vec<_> = gate_rows.iter().filter(|(_, v, _)| v == "pass").collect();
        if passed.is_empty() {
            retro.push_str("- (no gate results recorded)\n");
        } else {
            for (gate, _, _) in &passed {
                let _ = writeln!(retro, "- {gate} gate passed");
            }
        }

        retro.push_str("\n## What didn't go well\n\n");
        let failed: Vec<_> = gate_rows
            .iter()
            .filter(|(_, v, _)| v == "fail" || v == "warn")
            .collect();
        if failed.is_empty() {
            retro.push_str("- (no failures)\n");
        } else {
            for (gate, v, findings) in &failed {
                let _ = writeln!(retro, "- {gate} {v}: {findings}");
            }
        }

        retro.push_str("\n## Peer Consultations\n\n");
        if consult_rows.is_empty() {
            retro.push_str("- (none)\n");
        } else {
            for (q, r, o) in &consult_rows {
                let _ = writeln!(retro, "- Q: {q}\n  A: {}\n  Outcome: {}",
                    r.as_deref().unwrap_or(""),
                    o.as_deref().unwrap_or(""));
            }
        }

        retro.push_str("\n## Do differently next time\n\n- (to be filled by reflection)\n");

        let phase_dir = planning_dir.join(phase_id);
        std::fs::create_dir_all(&phase_dir)?;
        std::fs::write(phase_dir.join("RETRO.md"), &retro)?;

        // Add to MemPalace
        palace.add_drawer(&Drawer {
            wing: "cowclaw".to_string(),
            title: format!("Retro: {phase_id}"),
            body: retro.chars().take(500).collect(),
            tags: vec!["retro".to_string(), phase_id.to_string()],
        })?;

        Ok(())
    }

    #[must_use]
    pub fn retro_path(planning_dir: &Path, phase_id: &str) -> std::path::PathBuf {
        planning_dir.join(phase_id).join("RETRO.md")
    }
}
