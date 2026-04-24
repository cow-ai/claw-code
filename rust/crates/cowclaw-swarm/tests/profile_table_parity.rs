use std::path::PathBuf;

#[test]
fn swarm_toml_profile_count_matches_spec() {
    // Verify .cowclaw/swarm.toml has 9 profiles if it exists.
    // If swarm.toml doesn't exist yet (before M9.7), the test passes trivially.
    let cowclaw_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../../");
    let toml_path = cowclaw_root.join(".cowclaw/swarm.toml");
    if !toml_path.exists() {
        eprintln!("swarm.toml not found — parity test skipped (create in M9.7)");
        return;
    }
    let raw = std::fs::read_to_string(&toml_path).unwrap();
    let val: toml::Value = toml::from_str(&raw).unwrap();
    // Count profiles in [orchestration.profiles]
    if let Some(profiles) = val.get("orchestration").and_then(|o| o.get("profiles")) {
        let count = profiles.as_table().map(|t| t.len()).unwrap_or(0);
        assert_eq!(count, 9, "expected 9 profiles (P1..P9), found {count}");
    }
    // If no orchestration section yet, test passes trivially
}
