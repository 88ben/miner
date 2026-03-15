use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{error, info};
use uuid::Uuid;

use miner_core::algorithm::Algorithm;
use miner_core::config::CoinEntry;
use miner_core::stats::{MinerStats, StatsSnapshot};
use miner_stratum::client::{PoolEvent, StratumClient};

/// Manages mining workers across multiple coins.
pub struct WorkerManager {
    workers: HashMap<String, WorkerHandle>,
    stats: Arc<RwLock<HashMap<String, MinerStats>>>,
}

#[allow(dead_code)]
struct WorkerHandle {
    id: String,
    coin_symbol: String,
    algorithm: Algorithm,
    shutdown_tx: mpsc::Sender<()>,
}

impl WorkerManager {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start a mining worker for a given coin configuration.
    pub async fn start_worker(&mut self, coin: &CoinEntry) -> miner_core::error::Result<String> {
        let worker_id = Uuid::new_v4().to_string();
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let pool_addr = format!("{}:{}", coin.pool.url, coin.pool.port);
        let wallet = coin.wallet.clone();
        let algorithm = coin.algorithm;
        let symbol = coin.symbol.clone();
        let stats = self.stats.clone();
        let wid = worker_id.clone();

        {
            let mut s = stats.write().await;
            s.insert(wid.clone(), MinerStats::new(algorithm));
        }

        tokio::spawn(async move {
            info!(
                worker_id = %wid,
                coin = %symbol,
                algo = %algorithm,
                "Worker starting"
            );

            let mut client = StratumClient::new(pool_addr, wallet, "x".into());

            match client.connect().await {
                Ok((_submit_tx, mut event_rx)) => {
                    loop {
                        tokio::select! {
                            Some(event) = event_rx.recv() => {
                                let mut s = stats.write().await;
                                if let Some(worker_stats) = s.get_mut(&wid) {
                                    match event {
                                        PoolEvent::Accepted => worker_stats.record_share(true),
                                        PoolEvent::Rejected(_) => worker_stats.record_share(false),
                                        PoolEvent::Disconnected => {
                                            info!(worker_id = %wid, "Disconnected from pool");
                                            break;
                                        }
                                        PoolEvent::NewJob(_) => {}
                                    }
                                }
                            }
                            _ = shutdown_rx.recv() => {
                                info!(worker_id = %wid, "Worker shutting down");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!(worker_id = %wid, error = %e, "Failed to connect to pool");
                }
            }
        });

        self.workers.insert(
            worker_id.clone(),
            WorkerHandle {
                id: worker_id.clone(),
                coin_symbol: coin.symbol.clone(),
                algorithm: coin.algorithm,
                shutdown_tx,
            },
        );

        Ok(worker_id)
    }

    pub async fn stop_worker(&mut self, worker_id: &str) -> bool {
        if let Some(handle) = self.workers.remove(worker_id) {
            let _ = handle.shutdown_tx.send(()).await;
            let mut s = self.stats.write().await;
            s.remove(worker_id);
            true
        } else {
            false
        }
    }

    pub async fn stop_all(&mut self) {
        let ids: Vec<String> = self.workers.keys().cloned().collect();
        for id in ids {
            self.stop_worker(&id).await;
        }
    }

    pub async fn get_stats(&self) -> Vec<StatsSnapshot> {
        let s = self.stats.read().await;
        s.values().map(StatsSnapshot::from).collect()
    }

    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }
}
