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

    /// Connect to the pool and begin the read loop.
    /// Returns a sender for submitting shares and a receiver for pool events.
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

        // Writer task: forwards share submissions to the pool.
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

        // Reader task: parses pool responses and emits events.
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                debug!(raw = %line, "Pool message");

                if let Ok(resp) = serde_json::from_str::<StratumResponse>(&line) {
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
