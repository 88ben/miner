use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use miner_core::error::{MinerError, Result};
use miner_core::types::MiningJob;

use crate::message::{StratumRequest, StratumResponse};

pub struct StratumClient {
    pool_addr: String,
    worker: String,
    password: String,
    request_id: u64,
}

pub enum PoolEvent {
    NewJob(MiningJob),
    Accepted,
    Rejected(String),
    Disconnected,
}

impl StratumClient {
    pub fn new(pool_addr: String, worker: String, password: String) -> Self {
        Self {
            pool_addr,
            worker,
            password,
            request_id: 0,
        }
    }

    fn next_id(&mut self) -> u64 {
        self.request_id += 1;
        self.request_id
    }

    pub async fn connect(
        &mut self,
    ) -> Result<(
        mpsc::Sender<StratumRequest>,
        mpsc::Receiver<PoolEvent>,
    )> {
        info!(pool = %self.pool_addr, "Connecting to pool");

        let stream = TcpStream::connect(&self.pool_addr)
            .await
            .map_err(|e| MinerError::PoolConnection(e.to_string()))?;

        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        let id = self.next_id();
        let login = StratumRequest::login(&self.worker, &self.password, id);
        let login_json = serde_json::to_string(&login)?;
        writer
            .write_all(format!("{login_json}\n").as_bytes())
            .await
            .map_err(|e| MinerError::PoolConnection(e.to_string()))?;

        debug!("Login request sent");

        let (submit_tx, mut submit_rx) = mpsc::channel::<StratumRequest>(64);
        let (event_tx, event_rx) = mpsc::channel::<PoolEvent>(64);

        tokio::spawn(async move {
            while let Some(req) = submit_rx.recv().await {
                match serde_json::to_string(&req) {
                    Ok(json) => {
                        if let Err(e) = writer.write_all(format!("{json}\n").as_bytes()).await {
                            error!("Failed to write to pool: {e}");
                            break;
                        }
                    }
                    Err(e) => error!("Failed to serialize request: {e}"),
                }
            }
        });

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                debug!(raw = %line, "Pool message");

                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                // Check if this is a job notification
                if let Some(job) = try_parse_job(&parsed) {
                    let _ = event_tx.send(PoolEvent::NewJob(job)).await;
                    continue;
                }

                // Check for login response containing initial job
                if let Some(result) = parsed.get("result") {
                    if let Some(job) = try_parse_job_from_result(result) {
                        let _ = event_tx.send(PoolEvent::NewJob(job)).await;
                        continue;
                    }
                }

                // Regular response
                if let Ok(resp) = serde_json::from_value::<StratumResponse>(parsed) {
                    if resp.is_ok() {
                        let _ = event_tx.send(PoolEvent::Accepted).await;
                    } else if let Some(err) = &resp.error {
                        warn!(error = %err.message, "Share rejected");
                        let _ = event_tx
                            .send(PoolEvent::Rejected(err.message.clone()))
                            .await;
                    }
                }
            }

            let _ = event_tx.send(PoolEvent::Disconnected).await;
        });

        Ok((submit_tx, event_rx))
    }
}

/// Try to parse a job from a notification (method: "job")
fn try_parse_job(msg: &serde_json::Value) -> Option<MiningJob> {
    let method = msg.get("method")?.as_str()?;

    match method {
        // CryptoNote / Monero style
        "job" => {
            let params = msg.get("params")?;
            parse_cryptonote_job(params)
        }
        // Ethash / Stratum style
        "mining.notify" => {
            let params = msg.get("params")?;
            parse_ethash_notify(params)
        }
        _ => None,
    }
}

/// Parse job from the login response result (CryptoNote pools include the first job here)
fn try_parse_job_from_result(result: &serde_json::Value) -> Option<MiningJob> {
    let job = result.get("job")?;
    parse_cryptonote_job(job)
}

/// CryptoNote-style job: { blob, job_id, target, height }
fn parse_cryptonote_job(params: &serde_json::Value) -> Option<MiningJob> {
    let job_id = params.get("job_id")?.as_str()?.to_string();
    let blob_hex = params.get("blob")?.as_str()?;
    let target_hex = params.get("target")?.as_str()?;

    let blob = hex_decode(blob_hex)?;
    let target = hex_decode(target_hex)?;

    let difficulty = target_to_difficulty(&target);
    let height = params
        .get("height")
        .and_then(|h| h.as_u64());

    Some(MiningJob {
        job_id,
        blob,
        target,
        difficulty,
        height,
    })
}

/// Ethash/Stratum mining.notify: [job_id, seed_hash, header_hash, clean_jobs]
fn parse_ethash_notify(params: &serde_json::Value) -> Option<MiningJob> {
    let arr = params.as_array()?;
    if arr.len() < 3 {
        return None;
    }

    let job_id = arr[0].as_str()?.to_string();
    let _seed_hash = arr[1].as_str()?;
    let header_hash = arr[2].as_str()?;

    let blob = hex_decode(header_hash)?;
    let target = vec![0xFF; 32]; // Will be set from mining.set_difficulty

    Some(MiningJob {
        job_id,
        blob,
        target,
        difficulty: 1.0,
        height: None,
    })
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).ok()?;
        bytes.push(byte);
    }
    Some(bytes)
}

fn target_to_difficulty(target: &[u8]) -> f64 {
    if target.is_empty() {
        return 1.0;
    }
    // Convert compact target to difficulty
    let mut val: u64 = 0;
    for (i, &b) in target.iter().rev().enumerate().take(8) {
        val |= (b as u64) << (i * 8);
    }
    if val == 0 {
        return f64::MAX;
    }
    u64::MAX as f64 / val as f64
}
