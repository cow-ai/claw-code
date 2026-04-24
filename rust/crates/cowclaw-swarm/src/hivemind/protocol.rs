use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultRequest {
    pub session_id: String,
    pub plan_id: String,
    pub from_worker: String,
    pub stuck_context: String,
    pub question: String,
    pub max_response_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsultResponse {
    pub consult_id: i64,
    pub to_worker: String,
    pub answer: String,
    pub outcome: Option<String>,
}
