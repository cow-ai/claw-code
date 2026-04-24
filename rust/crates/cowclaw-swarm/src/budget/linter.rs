use std::path::Path;
use crate::budget::Tier;

pub fn extract_tier(content: &str) -> Tier {
    let after = match content.strip_prefix("---\n") {
        Some(s) => s,
        None => return Tier::Default,
    };
    let end = match after.find("\n---\n") {
        Some(i) => i,
        None => return Tier::Default,
    };
    let fm = &after[..end];
    for line in fm.lines() {
        if let Some(v) = line.strip_prefix("tier: ") {
            if let Some(t) = Tier::parse(v.trim()) {
                return t;
            }
        }
    }
    Tier::Default
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub path: std::path::PathBuf,
    pub tier: Tier,
    pub max_lines: usize,
    pub actual_lines: usize,
}

#[derive(Debug)]
pub enum Verdict {
    Pass,
    Fail { violations: Vec<Violation> },
}

pub fn lint_file(path: &Path) -> crate::Result<Verdict> {
    let content = std::fs::read_to_string(path)?;
    let tier = extract_tier(&content);
    let line_count = content.lines().count();
    if line_count > tier.max_lines() {
        Ok(Verdict::Fail {
            violations: vec![Violation {
                path: path.to_path_buf(),
                tier,
                max_lines: tier.max_lines(),
                actual_lines: line_count,
            }],
        })
    } else {
        Ok(Verdict::Pass)
    }
}

pub fn lint_tree(root: &Path) -> crate::Result<Vec<Violation>> {
    let mut violations = Vec::new();
    lint_tree_recursive(root, &mut violations)?;
    Ok(violations)
}

fn lint_tree_recursive(dir: &Path, violations: &mut Vec<Violation>) -> crate::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            lint_tree_recursive(&path, violations)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Ok(Verdict::Fail { violations: v }) = lint_file(&path) {
                violations.extend(v);
            }
        }
    }
    Ok(())
}

pub fn lint_xml_plan(path: &Path) -> crate::Result<Verdict> {
    let content = std::fs::read_to_string(path)?;
    let actual_lines = content.lines().count();
    // Extract budget element: <budget tier="..." lines="N"/>
    let declared_lines: Option<usize> = content
        .find("<budget")
        .and_then(|start| content[start..].find("lines=\"")
            .map(|i| start + i + 7))
        .and_then(|start| content[start..].find('"')
            .map(|end| content[start..start + end].parse::<usize>().ok()))
        .flatten();

    if let Some(budget) = declared_lines {
        if actual_lines > budget {
            return Ok(Verdict::Fail {
                violations: vec![Violation {
                    path: path.to_path_buf(),
                    tier: Tier::Default,
                    max_lines: budget,
                    actual_lines,
                }],
            });
        }
    }
    Ok(Verdict::Pass)
}
