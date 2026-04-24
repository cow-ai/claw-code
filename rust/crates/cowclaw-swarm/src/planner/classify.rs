use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Dials {
    pub swarm: bool,
    pub gates: Vec<String>,
    pub main_cap: f64,
}

#[derive(Debug, Serialize)]
pub struct ClassifyResult {
    pub auto_profile: String,
    pub final_profile: String,
    pub force_escalated: bool,
    pub dials: Dials,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Horizon {
    Short,
    Mid,
    Long,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Risk {
    Low,
    Med,
    High,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Novelty {
    Routine,
    Exploration,
}

/// Returns true if `text` contains `keyword` as a whole word (space/punctuation boundaries).
fn contains_word(text: &str, keyword: &str) -> bool {
    // Multi-word keywords: just use substring match
    if keyword.contains(' ') {
        return text.contains(keyword);
    }
    // Single-word: require word boundaries (start/end of string or non-alphanumeric neighbor)
    let mut start = 0;
    while let Some(pos) = text[start..].find(keyword) {
        let abs_pos = start + pos;
        let before_ok = abs_pos == 0
            || !text.as_bytes()[abs_pos - 1].is_ascii_alphanumeric();
        let end_pos = abs_pos + keyword.len();
        let after_ok = end_pos >= text.len()
            || !text.as_bytes()[end_pos].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
        if start >= text.len() {
            break;
        }
    }
    false
}

fn classify_horizon(objective: &str) -> Horizon {
    let lower = objective.to_lowercase();

    let short_kw = ["typo", "fix", "one line", "rename", "update one", "doc"];
    let long_kw = [
        "rewrite all",
        "redesign",
        "phase",
        "migrate all",
        "refactor entire",
        "complete overhaul",
    ];

    // Check long first (more specific)
    for kw in &long_kw {
        if contains_word(&lower, kw) {
            return Horizon::Long;
        }
    }
    // Check short (use word-boundary matching to avoid "fixture" matching "fix", etc.)
    for kw in &short_kw {
        if contains_word(&lower, kw) {
            return Horizon::Short;
        }
    }
    // Very short objective (≤5 words)
    if objective.split_whitespace().count() <= 5 {
        return Horizon::Short;
    }

    // Check for multiple milestones (presence of "milestone" word more than once, or M\d+ patterns)
    let milestone_count = lower.matches("milestone").count()
        + {
            let re_count = lower
                .split_whitespace()
                .filter(|w| {
                    w.starts_with('m')
                        && w.len() > 1
                        && w[1..].parse::<u32>().is_ok()
                })
                .count();
            re_count
        };
    if milestone_count >= 2 {
        return Horizon::Long;
    }

    Horizon::Mid
}

fn classify_risk(objective: &str, files: &[&str]) -> Risk {
    let lower = objective.to_lowercase();

    // Low risk keywords
    let low_kw = ["typo", "doc", "comment", "test", "fixture", "readme"];
    // High risk keywords
    let high_kw = [
        "auth",
        "security",
        "migration",
        "schema",
        "crypto",
        "secret",
        "compliance",
        "upstream",
    ];

    // Check high risk in objective (word-boundary for short keywords)
    for kw in &high_kw {
        if contains_word(&lower, kw) {
            return Risk::High;
        }
    }

    // Check high risk in file paths (path segment matching)
    let high_path_patterns = [
        "migrations",
        "auth",
        "crypto",
        "secrets",
        ".patches/upstream",
    ];
    for file in files {
        let file_lower = file.to_lowercase();
        for pat in &high_path_patterns {
            // Match path segments (e.g. **/migrations/**)
            if file_lower.contains(&format!("/{pat}/"))
                || file_lower.starts_with(&format!("{pat}/"))
                || file_lower.ends_with(&format!("/{pat}"))
                || file_lower == *pat
                || (pat.contains('/') && file_lower.contains(pat))
            {
                return Risk::High;
            }
        }
    }

    // Check low risk in objective (word-boundary for short keywords)
    for kw in &low_kw {
        if contains_word(&lower, kw) {
            return Risk::Low;
        }
    }

    Risk::Med
}

fn classify_novelty(objective: &str) -> Novelty {
    let lower = objective.to_lowercase();
    let exploration_kw = ["spike", "explore", "research", "unknown", "new tech", "first time"];
    for kw in &exploration_kw {
        if lower.contains(kw) {
            return Novelty::Exploration;
        }
    }
    Novelty::Routine
}

fn profile_number(horizon: Horizon, risk: Risk, novelty: Novelty) -> u8 {
    match (horizon, risk, novelty) {
        (Horizon::Short, Risk::Low, _) => 1,
        (Horizon::Short, Risk::Med, _) => 2,
        (Horizon::Short, Risk::High, _) => 3,
        (Horizon::Mid, Risk::Low | Risk::Med, Novelty::Routine) => 4,
        (Horizon::Mid, Risk::Low | Risk::Med, Novelty::Exploration) => 5,
        (Horizon::Mid, Risk::High, _) => 6,
        (Horizon::Long, Risk::Low | Risk::Med, Novelty::Routine) => 7,
        (Horizon::Long, Risk::Low | Risk::Med, Novelty::Exploration) => 8,
        (Horizon::Long, Risk::High, _) => 9,
    }
}

fn profile_dials(p: u8) -> Dials {
    match p {
        1 => Dials { swarm: false, gates: vec![], main_cap: 1.0 },
        2 => Dials {
            swarm: false,
            gates: vec!["plan_adversarial".into()],
            main_cap: 1.0,
        },
        3 => Dials {
            swarm: true,
            gates: vec![
                "plan_adversarial".into(),
                "scope_reduction".into(),
                "security".into(),
                "oracle_consult".into(),
            ],
            main_cap: 0.70,
        },
        4 | 5 => Dials {
            swarm: true,
            gates: vec!["plan_adversarial".into(), "scope_reduction".into()],
            main_cap: 0.70,
        },
        6 => Dials {
            swarm: true,
            gates: vec![
                "plan_adversarial".into(),
                "scope_reduction".into(),
                "security".into(),
            ],
            main_cap: 0.60,
        },
        7 | 8 => Dials {
            swarm: true,
            gates: vec![
                "plan_adversarial".into(),
                "scope_reduction".into(),
                "security".into(),
                "oracle_consult".into(),
            ],
            main_cap: 0.40,
        },
        9 => Dials {
            swarm: true,
            gates: vec![
                "plan_adversarial".into(),
                "scope_reduction".into(),
                "security".into(),
                "oracle_consult".into(),
            ],
            main_cap: 0.30,
        },
        _ => unreachable!("profile number must be 1..=9"),
    }
}

fn should_force_escalate(objective: &str, files: &[&str]) -> bool {
    let lower = objective.to_lowercase();
    let escalate_kw = ["auth", "migration", "crypto", "secrets"];
    for kw in &escalate_kw {
        if lower.contains(kw) {
            return true;
        }
    }
    for file in files {
        let file_lower = file.to_lowercase();
        for kw in &escalate_kw {
            if file_lower.contains(kw) {
                return true;
            }
        }
    }
    false
}

#[must_use]
pub fn classify(objective: &str, files: &[&str]) -> ClassifyResult {
    let horizon = classify_horizon(objective);
    let risk = classify_risk(objective, files);
    let novelty = classify_novelty(objective);

    let auto_num = profile_number(horizon, risk, novelty);
    let auto_profile = format!("P{auto_num}");

    let force_escalated = should_force_escalate(objective, files) && auto_num < 6;
    let final_num = if force_escalated { 6 } else { auto_num };
    let final_profile = format!("P{final_num}");

    let dials = profile_dials(final_num);

    ClassifyResult {
        auto_profile,
        final_profile,
        force_escalated,
        dials,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_low_risk_is_p1() {
        let r = classify("fix typo in CLAUDE.md", &[]);
        assert_eq!(r.auto_profile, "P1");
        assert_eq!(r.final_profile, "P1");
        assert!(!r.force_escalated);
        assert!(!r.dials.swarm);
    }

    #[test]
    fn mid_routine_is_p4() {
        let r = classify("add dark-mode toggle to fixture app", &[]);
        assert_eq!(r.auto_profile, "P4");
        assert_eq!(r.final_profile, "P4");
        assert!(r.dials.swarm);
    }

    #[test]
    fn auth_escalates_to_p6() {
        let r = classify("rewrite auth middleware per compliance brief", &[]);
        assert_eq!(r.final_profile, "P6");
        assert!(r.dials.swarm);
    }
}
