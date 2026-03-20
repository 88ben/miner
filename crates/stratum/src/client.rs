use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use miner_core::error::{MinerError, Result};
use miner_core::types::MiningJob;

use crate::message::{StratumDialect, StratumRequest, StratumResponse};

pub struct StratumClient {
    pool_addr: String,
    worker: String,
    password: String,
    dialect: StratumDialect,
    request_id: u64,
}

pub enum PoolEvent {
    NewJob(MiningJob),
    Accepted,
    Rejected(String),
    Disconnected,
}

impl StratumClient {
    pub fn new(
        pool_addr: String,
        worker: String,
        password: String,
        dialect: StratumDialect,
    ) -> Self {
        Self {
            pool_addr,
            worker,
            password,
            dialect,
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
        info!(pool = %self.pool_addr, dialect = ?self.dialect, "Connecting to pool");

        let stream = TcpStream::connect(&self.pool_addr)
            .await
            .map_err(|e| MinerError::PoolConnection(e.to_string()))?;

        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        let (subscribe_id, authorize_id) = self.send_handshake(&mut writer).await?;

        let (submit_tx, mut submit_rx) = mpsc::channel::<StratumRequest>(64);
        let (event_tx, event_rx) = mpsc::channel::<PoolEvent>(64);
        let dialect = self.dialect;

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

        let mut extranonce1: Option<Vec<u8>> = None;
        let mut extranonce2_size: Option<usize> = None;

        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                debug!(raw = %line, "Pool message");

                let parsed: serde_json::Value = match serde_json::from_str(&line) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(error = %e, "Failed to parse pool message");
                        continue;
                    }
                };

                // Check if this is a notification (has "method" field, no "id" or id is null)
                if let Some(job) = try_parse_job(&parsed, dialect, &extranonce1, &extranonce2_size) {
                    info!(job_id = %job.job_id, "Job from pool");
                    let _ = event_tx.send(PoolEvent::NewJob(job)).await;
                    continue;
                }

                // Handle mining.set_target / mining.set_difficulty
                if let Some(method) = parsed.get("method").and_then(|m| m.as_str()) {
                    match method {
                        "mining.set_target" | "mining.set_difficulty" => {
                            info!(method, "Difficulty/target update received");
                            continue;
                        }
                        _ => {}
                    }
                }

                // Response with an id — match to our requests
                let resp_id = parsed.get("id").and_then(|id| id.as_u64());

                if let Some(result) = parsed.get("result") {
                    // Subscribe response: result is an array like [null, "extranonce1", size]
                    if resp_id == Some(subscribe_id) && result.is_array() {
                        if let Some(arr) = result.as_array() {
                            if arr.len() >= 3 {
                                if let Some(en1) = arr[1].as_str() {
                                    extranonce1 = hex_decode(en1);
                                    info!(extranonce1 = %en1, "Extranonce1 received");
                                }
                                if let Some(en2_size) = arr[2].as_u64() {
                                    extranonce2_size = Some(en2_size as usize);
                                    info!(extranonce2_size = en2_size, "Extranonce2 size received");
                                }
                            }
                        }
                        continue;
                    }

                    // Authorize response
                    if resp_id == Some(authorize_id) {
                        if result.as_bool() == Some(true) {
                            info!("Pool authorization successful");
                        } else {
                            error!("Pool authorization FAILED — check wallet address");
                        }
                        continue;
                    }

                    // CryptoNote login response with embedded job
                    if let Some(job) = try_parse_job_from_result(result, dialect, &extranonce1, &extranonce2_size) {
                        info!(job_id = %job.job_id, "Job from login response");
                        let _ = event_tx.send(PoolEvent::NewJob(job)).await;
                        continue;
                    }

                    // Share submission response
                    if result.as_bool() == Some(true) {
                        let _ = event_tx.send(PoolEvent::Accepted).await;
                        continue;
                    }
                }

                if let Ok(resp) = serde_json::from_value::<StratumResponse>(parsed.clone()) {
                    if let Some(err) = &resp.error {
                        warn!(code = err.code, error = %err.message, "Pool error response");
                        let _ = event_tx
                            .send(PoolEvent::Rejected(err.message.clone()))
                            .await;
                        continue;
                    }
                }

                debug!(?parsed, "Unhandled pool message");
            }

            let _ = event_tx.send(PoolEvent::Disconnected).await;
        });

        Ok((submit_tx, event_rx))
    }

    async fn send_handshake(
        &mut self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
    ) -> Result<(u64, u64)> {
        let (subscribe_id, authorize_id) = match self.dialect {
            StratumDialect::CryptoNote => {
                let id = self.next_id();
                let req = StratumRequest::login(&self.worker, &self.password, id);
                self.send_request(writer, &req).await?;
                info!("CryptoNote login sent");
                (id, id)
            }
            StratumDialect::Ethash => {
                let id = self.next_id();
                let req = StratumRequest::eth_submit_login(&self.worker, &self.password, id);
                self.send_request(writer, &req).await?;
                info!("Ethash eth_submitLogin sent");
                (id, id)
            }
            StratumDialect::Stratum => {
                let sub_id = self.next_id();
                let agent = format!("miner/{}", env!("CARGO_PKG_VERSION"));
                let req = StratumRequest::mining_subscribe(&agent, sub_id);
                self.send_request(writer, &req).await?;
                info!("Stratum mining.subscribe sent");

                let auth_id = self.next_id();
                let req = StratumRequest::mining_authorize(&self.worker, &self.password, auth_id);
                self.send_request(writer, &req).await?;
                info!("Stratum mining.authorize sent");
                (sub_id, auth_id)
            }
        };
        Ok((subscribe_id, authorize_id))
    }

    async fn send_request(
        &self,
        writer: &mut tokio::net::tcp::OwnedWriteHalf,
        req: &StratumRequest,
    ) -> Result<()> {
        let json = serde_json::to_string(req)?;
        debug!(json = %json, "Sending to pool");
        writer
            .write_all(format!("{json}\n").as_bytes())
            .await
            .map_err(|e| MinerError::PoolConnection(e.to_string()))?;
        Ok(())
    }
}

fn try_parse_job(
    msg: &serde_json::Value,
    dialect: StratumDialect,
    extranonce1: &Option<Vec<u8>>,
    extranonce2_size: &Option<usize>,
) -> Option<MiningJob> {
    let method = msg.get("method")?.as_str()?;

    match method {
        "job" => {
            let params = msg.get("params")?;
            parse_cryptonote_job(params)
        }
        "mining.notify" => {
            let params = msg.get("params")?;
            match dialect {
                StratumDialect::Ethash => parse_ethash_notify(params),
                StratumDialect::Stratum => parse_stratum_notify(params, extranonce1, extranonce2_size),
                _ => parse_ethash_notify(params),
            }
        }
        _ => None,
    }
}

fn try_parse_job_from_result(
    result: &serde_json::Value,
    dialect: StratumDialect,
    extranonce1: &Option<Vec<u8>>,
    extranonce2_size: &Option<usize>,
) -> Option<MiningJob> {
    if let Some(job) = result.get("job") {
        return parse_cryptonote_job(job);
    }

    if dialect == StratumDialect::Ethash {
        if let Some(arr) = result.as_array() {
            if arr.len() >= 3 {
                return parse_ethproxy_result(arr);
            }
        }
    }

    let _ = (extranonce1, extranonce2_size);
    None
}

fn parse_cryptonote_job(params: &serde_json::Value) -> Option<MiningJob> {
    let job_id = params.get("job_id")?.as_str()?.to_string();
    let blob_hex = params.get("blob")?.as_str()?;
    let target_hex = params.get("target")?.as_str()?;

    let blob = hex_decode(blob_hex)?;
    let target = hex_decode(target_hex)?;

    let difficulty = target_to_difficulty(&target);
    let height = params.get("height").and_then(|h| h.as_u64());

    Some(MiningJob {
        job_id,
        blob,
        target,
        difficulty,
        height,
        seed_hash: None,
        ntime: None,
        extranonce1: None,
        extranonce2_size: None,
    })
}

fn parse_ethproxy_result(arr: &[serde_json::Value]) -> Option<MiningJob> {
    let header_hash = arr[0].as_str()?;
    let seed_hash_hex = arr[1].as_str()?;
    let boundary = arr[2].as_str()?;

    let blob = hex_decode(header_hash)?;
    let seed_hash = hex_decode(seed_hash_hex);
    let target = hex_decode(boundary).unwrap_or_else(|| vec![0xFF; 32]);
    let difficulty = target_to_difficulty(&target);

    info!(
        header_len = blob.len(),
        target_len = target.len(),
        difficulty,
        "EthProxy job parsed"
    );

    Some(MiningJob {
        job_id: header_hash.to_string(),
        blob,
        target,
        difficulty,
        height: None,
        seed_hash,
        ntime: None,
        extranonce1: None,
        extranonce2_size: None,
    })
}

fn parse_ethash_notify(params: &serde_json::Value) -> Option<MiningJob> {
    let arr = params.as_array()?;
    if arr.len() < 3 {
        return None;
    }

    let job_id = arr[0].as_str()?.to_string();
    let seed_hash_hex = arr[1].as_str()?;
    let header_hash = arr[2].as_str()?;

    let blob = hex_decode(header_hash)?;
    let seed_hash = hex_decode(seed_hash_hex);
    let target = vec![0xFF; 32];

    Some(MiningJob {
        job_id,
        blob,
        target,
        difficulty: 1.0,
        height: None,
        seed_hash,
        ntime: None,
        extranonce1: None,
        extranonce2_size: None,
    })
}

/// Standard Stratum mining.notify for Equihash:
/// params = [job_id, version, prevhash, merkleroot, reserved, time, bits, clean_jobs]
fn parse_stratum_notify(
    params: &serde_json::Value,
    extranonce1: &Option<Vec<u8>>,
    extranonce2_size: &Option<usize>,
) -> Option<MiningJob> {
    let arr = params.as_array()?;
    if arr.len() < 7 {
        debug!(len = arr.len(), "Stratum mining.notify has too few params");
        return None;
    }

    let job_id = arr[0].as_str()?.to_string();
    let version = arr[1].as_str()?;
    let prevhash = arr[2].as_str()?;
    let merkleroot = arr[3].as_str()?;
    let _reserved = arr[4].as_str().unwrap_or("");
    let time = arr[5].as_str()?;
    let bits = arr[6].as_str()?;

    let mut header = Vec::new();
    header.extend_from_slice(&hex_decode(version)?);
    header.extend_from_slice(&hex_decode(prevhash)?);
    header.extend_from_slice(&hex_decode(merkleroot)?);
    header.extend_from_slice(&hex_decode(time)?);
    header.extend_from_slice(&hex_decode(bits)?);

    let target = hex_decode(bits)?;
    let difficulty = target_to_difficulty(&target);

    Some(MiningJob {
        job_id,
        blob: header,
        target,
        difficulty,
        height: None,
        seed_hash: None,
        ntime: Some(time.to_string()),
        extranonce1: extranonce1.clone(),
        extranonce2_size: *extranonce2_size,
    })
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() % 2 != 0 {
        return None;
    }
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
    let mut val: u64 = 0;
    for (i, &b) in target.iter().rev().enumerate().take(8) {
        val |= (b as u64) << (i * 8);
    }
    if val == 0 {
        return f64::MAX;
    }
    u64::MAX as f64 / val as f64
}
