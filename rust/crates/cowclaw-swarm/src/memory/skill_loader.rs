use serde::{Serialize, Deserialize};
use std::path::{Path, PathBuf};
use rusqlite::{Connection, params};
use crate::events::schema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,
    pub trigger_hints: String,
    pub line_count: usize,
    pub path: PathBuf,
}

impl SkillManifest {
    pub fn parse(name: &str, content: &str) -> Result<Self, String> {
        let after_first = content.strip_prefix("---\n").ok_or("no front-matter")?;
        let end = after_first.find("\n---\n").ok_or("unclosed front-matter")?;
        let fm = &after_first[..end];
        let body = &after_first[end + 5..]; // skip "\n---\n"
        let line_count = body.lines().count();

        let mut description = String::new();
        let mut trigger_hints = String::new();

        for line in fm.lines() {
            if let Some(v) = line.strip_prefix("description: ") {
                description = v.to_string();
            } else if let Some(v) = line.strip_prefix("trigger_hints: ") {
                trigger_hints = v.to_string();
            }
        }

        Ok(SkillManifest {
            name: name.to_string(),
            description,
            trigger_hints,
            line_count,
            path: PathBuf::new(),
        })
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| e.to_string())?;
        // For SKILL.md files, use the parent directory name as the skill name
        // (convention: .cowclaw/skills/<name>/SKILL.md)
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
        let name = if stem == "SKILL" {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string()
        } else {
            stem.to_string()
        };
        let mut m = Self::parse(&name, &content)?;
        m.path = path.to_path_buf();
        Ok(m)
    }

    /// Scan a directory for SKILL.md files and parse them.
    #[must_use]
    pub fn from_dir(dir: &Path) -> Vec<Self> {
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.file_name().and_then(|s| s.to_str()) == Some("SKILL.md") {
                    if let Ok(m) = Self::from_file(&p) {
                        results.push(m);
                    }
                } else if p.is_dir() {
                    // Recurse one level for .cowclaw/skills/<name>/SKILL.md
                    results.extend(Self::from_dir(&p));
                }
            }
        }
        results
    }
}

pub struct SkillRegistry {
    conn: Connection,
}

impl SkillRegistry {
    pub fn open(path: &Path) -> crate::Result<Self> {
        let mut conn = Connection::open(path)?;
        schema::apply(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn register_from_dir(&mut self, dir: &Path) -> crate::Result<usize> {
        let manifests = SkillManifest::from_dir(dir);
        let count = manifests.len();
        for m in &manifests {
            self.conn.execute(
                "INSERT OR REPLACE INTO skill_manifests(name, path, description, trigger_hints, source, line_count, tier, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'filesystem', ?5, 'default', ?6)",
                params![
                    m.name, m.path.to_string_lossy().as_ref(),
                    m.description, m.trigger_hints, i64::try_from(m.line_count).unwrap_or(i64::MAX),
                    chrono::Utc::now().to_rfc3339(),
                ],
            )?;
        }
        Ok(count)
    }

    pub fn list_all(&self) -> crate::Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT name FROM skill_manifests ORDER BY name")?;
        let names: Vec<String> = stmt.query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(names)
    }
}
