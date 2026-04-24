use crate::planner::profile::ProfileId;
use globset::{Glob, GlobSetBuilder};

#[must_use]
pub fn apply_force_escalate(
    profile: ProfileId,
    files: &[&str],
    patterns: &[String],
) -> ProfileId {
    if patterns.is_empty() || files.is_empty() {
        return profile;
    }
    let mut builder = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = Glob::new(p) {
            builder.add(g);
        }
    }
    let Ok(globset) = builder.build() else { return profile; };
    let matches_any = files.iter().any(|f| globset.is_match(f));
    if matches_any {
        profile.escalate_to_p6()
    } else {
        profile
    }
}
