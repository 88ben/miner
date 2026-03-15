use serde::{Deserialize, Serialize};

/// JSON-RPC request sent to the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumRequest {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

/// JSON-RPC response from the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumResponse {
    pub id: Option<u64>,
    pub result: Option<serde_json::Value>,
    pub error: Option<StratumError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumError {
    pub code: i32,
    pub message: String,
}

/// Notifications pushed by the pool (new jobs, difficulty changes, etc).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumNotification {
    pub method: String,
    pub params: serde_json::Value,
}

impl StratumRequest {
    pub fn login(worker: &str, pass: &str, id: u64) -> Self {
        Self {
            id,
            method: "login".into(),
            params: serde_json::json!({
                "login": worker,
                "pass": pass,
                "agent": format!("miner/{}", env!("CARGO_PKG_VERSION")),
            }),
        }
    }

    pub fn submit(job_id: &str, nonce: &str, result: &str, id: u64) -> Self {
        Self {
            id,
            method: "submit".into(),
            params: serde_json::json!({
                "id": job_id,
                "nonce": nonce,
                "result": result,
            }),
        }
    }
}

impl StratumResponse {
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
}
