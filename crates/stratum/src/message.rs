use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StratumDialect {
    CryptoNote,
    Ethash,
    Stratum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StratumRequest {
    pub id: u64,
    pub method: String,
    pub params: serde_json::Value,
}

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

    pub fn eth_submit_login(wallet: &str, pass: &str, id: u64) -> Self {
        Self {
            id,
            method: "eth_submitLogin".into(),
            params: serde_json::json!([wallet, pass]),
        }
    }

    pub fn mining_subscribe(agent: &str, id: u64) -> Self {
        Self {
            id,
            method: "mining.subscribe".into(),
            params: serde_json::json!([agent]),
        }
    }

    pub fn mining_authorize(wallet: &str, pass: &str, id: u64) -> Self {
        Self {
            id,
            method: "mining.authorize".into(),
            params: serde_json::json!([wallet, pass]),
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

    pub fn eth_submit_work(nonce: &str, header: &str, mix_digest: &str, id: u64) -> Self {
        Self {
            id,
            method: "eth_submitWork".into(),
            params: serde_json::json!([nonce, header, mix_digest]),
        }
    }

    pub fn mining_submit(
        worker: &str,
        job_id: &str,
        ntime: &str,
        nonce_hex: &str,
        solution_hex: &str,
        id: u64,
    ) -> Self {
        Self {
            id,
            method: "mining.submit".into(),
            params: serde_json::json!([worker, job_id, ntime, nonce_hex, solution_hex]),
        }
    }
}

impl StratumResponse {
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
}
