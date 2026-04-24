use cowclaw_swarm::memory::skill_loader::SkillManifest;

#[test]
fn skill_loader_parses_frontmatter() {
    let content = "---\nname: brainstorming\ndescription: Brainstorm\ntrigger_hints: new feature\n---\n# body\nline2";
    let m = SkillManifest::parse("brainstorming", content).unwrap();
    assert_eq!(m.name, "brainstorming");
    assert_eq!(m.description, "Brainstorm");
    assert_eq!(m.trigger_hints, "new feature");
    assert_eq!(m.line_count, 2); // lines after closing ---
}

#[test]
fn skill_loader_handles_missing_trigger_hints() {
    let content = "---\nname: foo\ndescription: Foo skill\n---\n# content";
    let m = SkillManifest::parse("foo", content).unwrap();
    assert_eq!(m.trigger_hints, "");
}

#[test]
fn skill_registry_registers_from_dir() {
    use cowclaw_swarm::memory::skill_loader::SkillRegistry;
    use tempfile::TempDir;

    let td = TempDir::new().unwrap();
    // Create fixture skills
    for (name, desc) in [("skill_a", "Skill A"), ("skill_b", "Skill B")] {
        let skill_dir = td.path().join(name);
        std::fs::create_dir(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {desc}\ntrigger_hints: test\n---\n# body"),
        ).unwrap();
    }

    let db = td.path().join("swarm.db");
    let mut reg = SkillRegistry::open(&db).unwrap();
    let count = reg.register_from_dir(td.path()).unwrap();
    assert_eq!(count, 2);

    let manifests = reg.list_all().unwrap();
    assert_eq!(manifests.len(), 2);
}
