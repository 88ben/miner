use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::mpsc;
use tracing::{debug, info};

use miner_core::algorithm::Hasher;
use miner_core::error::Result;
use miner_core::types::{MiningJob, Nonce};

const HASH_REPORT_INTERVAL_MS: u128 = 1000;

pub struct MiningEngine {
    hasher: Box<dyn Hasher>,
    running: Arc<AtomicBool>,
    total_hashes: Arc<AtomicU64>,
}

pub enum MiningEvent {
    Share {
        job_id: String,
        nonce: u64,
        hash: Vec<u8>,
        ntime: Option<String>,
        extranonce2_size: Option<usize>,
    },
    HashReport(u64),
}

impl MiningEngine {
    pub fn new(hasher: Box<dyn Hasher>) -> Self {
        Self {
            hasher,
            running: Arc::new(AtomicBool::new(false)),
            total_hashes: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn total_hashes(&self) -> u64 {
        self.total_hashes.load(Ordering::Relaxed)
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn mine(
        &self,
        job: MiningJob,
        start_nonce: u64,
        event_tx: mpsc::UnboundedSender<MiningEvent>,
    ) -> Result<()> {
        self.running.store(true, Ordering::Relaxed);

        let batch_size = self.hasher.preferred_batch_size();
        let use_batch = batch_size > 1;

        info!(
            algo = %self.hasher.algorithm(),
            job_id = %job.job_id,
            gpu = self.hasher.is_gpu(),
            batch_size,
            "Mining started"
        );

        if use_batch {
            self.mine_batch(job, start_nonce, batch_size, event_tx)
        } else {
            self.mine_single(job, start_nonce, event_tx)
        }
    }

    fn mine_batch(
        &self,
        job: MiningJob,
        start_nonce: u64,
        batch_size: u64,
        event_tx: mpsc::UnboundedSender<MiningEvent>,
    ) -> Result<()> {
        let mut nonce = start_nonce;
        let mut last_report = Instant::now();
        let mut hashes_since_report: u64 = 0;

        while self.running.load(Ordering::Relaxed) {
            let found = self.hasher.hash_batch(&job, nonce, batch_size)?;

            for hit in &found {
                debug!(nonce = hit.nonce, "Share found (batch)");
                let _ = event_tx.send(MiningEvent::Share {
                    job_id: job.job_id.clone(),
                    nonce: hit.nonce,
                    hash: hit.hash.clone(),
                    ntime: job.ntime.clone(),
                    extranonce2_size: job.extranonce2_size,
                });
            }

            self.total_hashes.fetch_add(batch_size, Ordering::Relaxed);
            hashes_since_report += batch_size;
            nonce = nonce.wrapping_add(batch_size);

            if last_report.elapsed().as_millis() >= HASH_REPORT_INTERVAL_MS {
                let _ = event_tx.send(MiningEvent::HashReport(hashes_since_report));
                hashes_since_report = 0;
                last_report = Instant::now();
            }
        }

        if hashes_since_report > 0 {
            let _ = event_tx.send(MiningEvent::HashReport(hashes_since_report));
        }

        info!("Mining stopped");
        Ok(())
    }

    fn mine_single(
        &self,
        job: MiningJob,
        start_nonce: u64,
        event_tx: mpsc::UnboundedSender<MiningEvent>,
    ) -> Result<()> {
        let mut nonce = start_nonce;
        let mut last_report = Instant::now();
        let mut hashes_since_report: u64 = 0;

        while self.running.load(Ordering::Relaxed) {
            let hash = self.hasher.hash(&job, Nonce(nonce))?;

            if self.hasher.meets_target(&hash, &job.target) {
                debug!(nonce, "Share found");
                let _ = event_tx.send(MiningEvent::Share {
                    job_id: job.job_id.clone(),
                    nonce,
                    hash,
                    ntime: job.ntime.clone(),
                    extranonce2_size: job.extranonce2_size,
                });
            }

            nonce = nonce.wrapping_add(1);
            self.total_hashes.fetch_add(1, Ordering::Relaxed);
            hashes_since_report += 1;

            if last_report.elapsed().as_millis() >= HASH_REPORT_INTERVAL_MS {
                let _ = event_tx.send(MiningEvent::HashReport(hashes_since_report));
                hashes_since_report = 0;
                last_report = Instant::now();
            }
        }

        if hashes_since_report > 0 {
            let _ = event_tx.send(MiningEvent::HashReport(hashes_since_report));
        }

        info!("Mining stopped");
        Ok(())
    }
}
