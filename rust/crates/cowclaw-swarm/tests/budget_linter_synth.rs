use cowclaw_swarm::budget::Tier;

#[test]
fn tier_caps() {
    assert_eq!(Tier::Xl.max_lines(), 1600);
    assert_eq!(Tier::Large.max_lines(), 1000);
    assert_eq!(Tier::Default.max_lines(), 500);
    assert_eq!(Tier::Tight.max_lines(), 200);
}

#[test]
fn tier_parse() {
    assert_eq!(Tier::parse("xl"), Some(Tier::Xl));
    assert_eq!(Tier::parse("LARGE"), Some(Tier::Large));
    assert_eq!(Tier::parse("default"), Some(Tier::Default));
    assert_eq!(Tier::parse("tight"), Some(Tier::Tight));
    assert_eq!(Tier::parse("unknown"), None);
}

#[test]
fn extract_tier_from_frontmatter() {
    use cowclaw_swarm::budget::linter::extract_tier;
    let content = "---\ntier: tight\n---\nsome content";
    assert_eq!(extract_tier(content), Tier::Tight);
    // Default when no frontmatter
    let no_fm = "# just a heading\ncontent";
    assert_eq!(extract_tier(no_fm), Tier::Default);
    // Default when frontmatter has no tier key
    let no_tier = "---\nname: foo\n---\ncontent";
    assert_eq!(extract_tier(no_tier), Tier::Default);
}

#[test]
fn linter_flags_oversize_file() {
    use cowclaw_swarm::budget::linter::{lint_file, Verdict};
    let td = tempfile::TempDir::new().unwrap();
    let big = td.path().join("big.md");
    std::fs::write(&big, "---\ntier: tight\n---\n".to_string() + &"x\n".repeat(300)).unwrap();
    let r = lint_file(&big).unwrap();
    assert!(matches!(r, Verdict::Fail { .. }), "expected Fail for 300 lines > 200 cap");
}

#[test]
fn linter_passes_undersize_file() {
    use cowclaw_swarm::budget::linter::{lint_file, Verdict};
    let td = tempfile::TempDir::new().unwrap();
    let small = td.path().join("small.md");
    std::fs::write(&small, "---\ntier: tight\n---\n".to_string() + &"x\n".repeat(50)).unwrap();
    let r = lint_file(&small).unwrap();
    assert!(matches!(r, Verdict::Pass));
}

#[test]
fn xml_plan_budget_attribute_check() {
    use cowclaw_swarm::budget::linter::lint_xml_plan;
    let td = tempfile::TempDir::new().unwrap();
    let xml = td.path().join("PLAN.xml");
    // budget attribute says 5 lines, but actual content has 20 lines
    let content = format!(
        r#"<?xml version="1.0"?><plan><budget tier="tight" lines="5"/>{}</plan>"#,
        "<task/>\n".repeat(20)
    );
    std::fs::write(&xml, &content).unwrap();
    let result = lint_xml_plan(&xml).unwrap();
    assert!(matches!(result, cowclaw_swarm::budget::linter::Verdict::Fail { .. }));
}

#[test]
fn lint_real_cowclaw_planning_tree() {
    // This test verifies no existing .cowclaw/planning/ files exceed their tier.
    // Expected: PASS initially (directory may not exist yet or is empty).
    let cowclaw_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors().nth(4) // reach cowclaw root from crates/cowclaw-swarm
        .unwrap()
        .join(".cowclaw/planning");
    if !cowclaw_dir.exists() {
        return; // no planning dir yet — test passes trivially
    }
    let violations = cowclaw_swarm::budget::linter::lint_tree(&cowclaw_dir).unwrap();
    assert!(violations.is_empty(), "budget violations found: {violations:?}");
}
