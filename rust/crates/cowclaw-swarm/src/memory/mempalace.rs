use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drawer {
    pub wing: String,
    pub title: String,
    pub body: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub drawer_id: String,
    pub title: String,
    pub score: f32,
    pub snippet: String,
}

/// Transport trait — mock in tests, real stdio-JSON-RPC in production
pub trait PalaceTransport: Send + Sync {
    fn add_drawer(&self, drawer: &Drawer) -> crate::Result<String>;
    fn search(&self, query: &str, n: usize) -> crate::Result<Vec<SearchHit>>;
    fn status(&self) -> crate::Result<HashMap<String, serde_json::Value>>;
}

pub struct MemPalaceClient {
    transport: Box<dyn PalaceTransport>,
}

impl MemPalaceClient {
    pub fn new(transport: Box<dyn PalaceTransport>) -> Self { Self { transport } }
    pub fn add_drawer(&self, drawer: &Drawer) -> crate::Result<String> {
        self.transport.add_drawer(drawer)
    }
    pub fn search(&self, query: &str, n: usize) -> crate::Result<Vec<SearchHit>> {
        self.transport.search(query, n)
    }
    pub fn status(&self) -> crate::Result<HashMap<String, serde_json::Value>> {
        self.transport.status()
    }
}

/// In-memory mock transport for tests
pub struct MockTransport {
    pub drawers: std::sync::Mutex<HashMap<String, Drawer>>,
}

impl MockTransport {
    pub fn new() -> Self { Self { drawers: std::sync::Mutex::new(HashMap::new()) } }
}

impl Default for MockTransport {
    fn default() -> Self { Self::new() }
}

impl PalaceTransport for MockTransport {
    fn add_drawer(&self, drawer: &Drawer) -> crate::Result<String> {
        let id = format!("drawer_{}", uuid_simple());
        self.drawers.lock().unwrap().insert(id.clone(), drawer.clone());
        Ok(id)
    }
    fn search(&self, query: &str, n: usize) -> crate::Result<Vec<SearchHit>> {
        let guard = self.drawers.lock().unwrap();
        let hits: Vec<SearchHit> = guard.values()
            .filter(|d| d.body.contains(query) || d.title.contains(query))
            .take(n)
            .map(|d| SearchHit {
                drawer_id: "mock".into(), title: d.title.clone(),
                score: 1.0, snippet: d.body.chars().take(100).collect(),
            })
            .collect();
        Ok(hits)
    }
    fn status(&self) -> crate::Result<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new();
        m.insert("drawer_count".into(), serde_json::json!(self.drawers.lock().unwrap().len()));
        Ok(m)
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{:x}", t)
}
