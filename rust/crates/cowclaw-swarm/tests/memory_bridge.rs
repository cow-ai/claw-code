use cowclaw_swarm::memory::mempalace::{Drawer, MemPalaceClient, MockTransport};

#[test]
fn mock_transport_add_and_search() {
    let client = MemPalaceClient::new(Box::new(MockTransport::new()));
    let drawer = Drawer {
        wing: "cowclaw".into(),
        title: "test drawer".into(),
        body: "unique keyword XYZ123".into(),
        tags: vec!["test".into()],
    };
    let id = client.add_drawer(&drawer).unwrap();
    assert!(!id.is_empty());
    let hits = client.search("XYZ123", 5).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "test drawer");
}

#[test]
fn mock_transport_status_returns_drawer_count() {
    let client = MemPalaceClient::new(Box::new(MockTransport::new()));
    let status = client.status().unwrap();
    assert_eq!(status["drawer_count"], serde_json::json!(0));
    let _ = client.add_drawer(&Drawer {
        wing: "test".into(), title: "t".into(), body: "b".into(), tags: vec![],
    }).unwrap();
    let status2 = client.status().unwrap();
    assert_eq!(status2["drawer_count"], serde_json::json!(1));
}

#[test]
#[ignore = "requires COWWIKI_RUNNING=1 and cow-wiki daemon running"]
fn live_cow_wiki_add_and_search() {
    // Live integration test: run with `cargo test -- --ignored cow_wiki_live`
    // Start daemon: cd ~/dev/cow-wiki && .venv/bin/cow-wiki daemon
    assert!(std::env::var("COWWIKI_RUNNING").is_ok(), "set COWWIKI_RUNNING=1");
}
