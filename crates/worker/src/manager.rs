use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use miner_core::algorithm::Algorithm;
use miner_core::config::CoinEntry;
use miner_core::stats::{MinerStats, StatsSnapshot};
use miner_core::types::MiningJob;
use miner_hashers::factory::create_hasher;
use miner_stratum::client::{PoolEvent, StratumClient};
use miner_stratum::message::StratumRequest;

use crate::engine::{FoundShare, MiningEngine};

pub struct WorkerManager {
    workers: HashMap<String, WorkerHandle>,
    stats: Arc<RwLock<HashMap<String, MinerStats>>>,
}

struct WorkerHandle {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    coin_symbol: String,
    #[allow(dead_code)]
    algorithm: Algorithm,
    shutdown: Arc<AtomicBool>,
    shutdown_tx: mpsc::Sender<()>,
}

impl WorkerManager {
    pub fn new() -> Self {
        Self {
            workers: HashMap::new(),
            stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_worker(&mut self, coin: &CoinEntry) -> miner_core::error::Result<String> {
        let worker_id = Uuid::new_v4().to_string();
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let pool_addr = format!("{}:{}", coin.pool.url, coin.pool.port);
        let wallet = coin.wallet.clone();
        let algorithm = coin.algorithm;
        let symbol = coin.symbol.clone();
        let stats = self.stats.clone();
        let wid = worker_id.clone();

        // Create the hasher for this algorithm
        let hasher = create_hasher(algorithm)?;

        // Create the mining engine
        let engine = MiningEngine::new(hasher);
        let engine_stop = engine.stop_handle();
        let engine_stop_clone = engine_stop.clone();

        {
            let mut s = stats.write().await;
            s.insert(wid.clone(), MinerStats::new(algorithm));
        }

        // Share submission channel
        let (share_tx, mut share_rx) = mpsc::unbounded_channel::<FoundShare>();

        tokio::spawn(async move {
            info!(
                worker_id = %wid,
                coin = %symbol,
                algo = %algorithm,
                "Worker starting"
            );

            let mut client = StratumClient::new(pool_addr, wallet, "x".into());

            match client.connect().await {
                Ok((submit_tx, mut event_rx)) => {
                    let mut _current_job: Option<MiningJob> = None;
                    let mut mining_active = false;

                    loop {
                        tokio::select! {
                            Some(event) = event_rx.recv() => {
                                match event {
                                    PoolEvent::NewJob(job) => {
                                        info!(
                                            worker_id = %wid,
                                            job_id = %job.job_id,
                                            "New job received"
                                        );

                                        // Stop any active mining
                                        engine_stop.store(false, Ordering::Relaxed);

                                        _current_job = Some(job.clone());

                                        // Start mining on a blocking thread
                                        if !mining_active {
                                            let share_tx_clone = share_tx.clone();
                                            let _stats_clone = stats.clone();
                                            let wid_clone = wid.clone();
                                            let engine_ref_stop = engine_stop.clone();

                                            mining_active = true;
                                            let job_for_task = job;

                                            tokio::task::spawn_blocking(move || {
                                                engine_ref_stop.store(true, Ordering::Relaxed);

                                                // Create a new engine for this thread
                                                match create_hasher(algorithm) {
                                                    Ok(hasher) => {
                                                        let engine = MiningEngine::new(hasher);
                                                        let start_nonce = rand_nonce();
                                                        if let Err(e) = engine.mine(
                                                            job_for_task,
                                                            start_nonce,
                                                            share_tx_clone,
                                                        ) {
                                                            error!(
                                                                worker_id = %wid_clone,
                                                                error = %e,
                                                                "Mining error"
                                                            );
                                                        }
                                                    }
                                                    Err(e) => {
                                                        error!(
                                                            worker_id = %wid_clone,
                                                            error = %e,
                                                            "Failed to create hasher"
                                                        );
                                                    }
                                                }
                                            });
                                        }
                                    }
                                    PoolEvent::Accepted => {
                                        let mut s = stats.write().await;
                                        if let Some(worker_stats) = s.get_mut(&wid) {
                                            worker_stats.record_share(true);
                                        }
                                        info!(worker_id = %wid, "Share accepted");
                                    }
                                    PoolEvent::Rejected(reason) => {
                                        let mut s = stats.write().await;
                                        if let Some(worker_stats) = s.get_mut(&wid) {
                                            worker_stats.record_share(false);
                                        }
                                        warn!(worker_id = %wid, reason = %reason, "Share rejected");
                                    }
                                    PoolEvent::Disconnected => {
                                        info!(worker_id = %wid, "Disconnected from pool");
                                        engine_stop.store(false, Ordering::Relaxed);
                                        break;
                                    }
                                }
                            }
                            Some(share) = share_rx.recv() => {
                                // Submit the found share to the pool
                                let nonce_hex = format!("{:016x}", share.nonce);
                                let result_hex = hex::encode(&share.hash);
                                let req = StratumRequest::submit(
                                    &share.job_id,
                                    &nonce_hex,
                                    &result_hex,
                                    1,
                                );
                                if let Err(e) = submit_tx.send(req).await {
                                    error!(worker_id = %wid, error = %e, "Failed to submit share");
                                }

                                let mut s = stats.write().await;
                                if let Some(worker_stats) = s.get_mut(&wid) {
                                    worker_stats.record_hashes(1);
                                }
                            }
                            _ = shutdown_rx.recv() => {
                                info!(worker_id = %wid, "Worker shutting down");
                                engine_stop.store(false, Ordering::Relaxed);
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
                shutdown: engine_stop_clone,
                shutdown_tx,
            },
        );

        Ok(worker_id)
    }

    pub async fn stop_worker(&mut self, worker_id: &str) -> bool {
        if let Some(handle) = self.workers.remove(worker_id) {
            handle.shutdown.store(false, Ordering::Relaxed);
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

    pub fn has_worker_for_symbol(&self, symbol: &str) -> bool {
        self.workers.values().any(|h| h.coin_symbol == symbol)
    }

    pub async fn stop_worker_by_symbol(&mut self, symbol: &str) -> bool {
        let id = self
            .workers
            .iter()
            .find(|(_, h)| h.coin_symbol == symbol)
            .map(|(id, _)| id.clone());

        if let Some(id) = id {
            self.stop_worker(&id).await
        } else {
            false
        }
    }
}

fn rand_nonce() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (seed & 0xFFFFFFFFFFFFFFFF) as u64
}
