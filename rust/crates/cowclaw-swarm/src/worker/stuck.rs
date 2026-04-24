use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
pub enum StuckSignal { None, SameErrorRepeated, ChunkTimeout }

pub struct StuckDetector {
    threshold: usize,
    window: VecDeque<String>,
}

impl StuckDetector {
    #[must_use]
    pub fn new(threshold: usize) -> Self {
        Self { threshold, window: VecDeque::new() }
    }

    pub fn observe(&mut self, err: &str) -> StuckSignal {
        let fp = Self::fingerprint(err);
        // If different from last, clear window
        if self.window.back().map(String::as_str) != Some(fp.as_str()) {
            self.window.clear();
        }
        self.window.push_back(fp.clone());
        if self.window.len() >= self.threshold {
            StuckSignal::SameErrorRepeated
        } else {
            StuckSignal::None
        }
    }

    pub fn observe_chunk_timeout(&mut self) -> StuckSignal {
        StuckSignal::ChunkTimeout
    }

    fn fingerprint(err: &str) -> String {
        err.chars()
            .filter(|c| !c.is_ascii_digit())
            .collect::<String>()
            .split_whitespace()
            .take(8)
            .collect::<Vec<_>>()
            .join(" ")
    }
}
