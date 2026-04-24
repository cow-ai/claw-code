use cowclaw_swarm::worker::stuck::{StuckDetector, StuckSignal};

#[test]
fn same_error_threshold_triggers() {
    let mut d = StuckDetector::new(3);
    let err = "tool call failed: status=500 body=rate_limited";
    assert_eq!(d.observe(err), StuckSignal::None);
    assert_eq!(d.observe(err), StuckSignal::None);
    assert_eq!(d.observe(err), StuckSignal::SameErrorRepeated);
    // Fourth occurrence still SameErrorRepeated
    assert_eq!(d.observe(err), StuckSignal::SameErrorRepeated);
    // Different error resets
    assert_eq!(d.observe("other error"), StuckSignal::None);
}

#[test]
fn chunk_timeout_signal() {
    let mut d = StuckDetector::new(3);
    assert_eq!(d.observe_chunk_timeout(), StuckSignal::ChunkTimeout);
}
