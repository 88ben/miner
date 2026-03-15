use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, info};

use miner_core::algorithm::Hasher;
use miner_core::error::Result;
use miner_core::types::{MiningJob, Nonce};

pub struct MiningEngine {
    hasher: Box<dyn Hasher>,
    running: Arc<AtomicBool>,
    total_hashes: Arc<AtomicU64>,
}

pub struct FoundShare {
    pub job_id: String,
    pub nonce: u64,
    pub hash: Vec<u8>,
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

    /// Mine against a job, scanning nonces from `start_nonce`.
    /// Automatically chooses batch or single-nonce mode based on the hasher.
    pub fn mine(
        &self,
        job: MiningJob,
        start_nonce: u64,
        share_tx: mpsc::UnboundedSender<FoundShare>,
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
            self.mine_batch(job, start_nonce, batch_size, share_tx)
        } else {
            self.mine_single(job, start_nonce, share_tx)
        }
    }

    fn mine_batch(
        &self,
        job: MiningJob,
        start_nonce: u64,
        batch_size: u64,
        share_tx: mpsc::UnboundedSender<FoundShare>,
    ) -> Result<()> {
        let mut nonce = start_nonce;

        while self.running.load(Ordering::Relaxed) {
            let found = self.hasher.hash_batch(&job, nonce, batch_size)?;

            for hit in &found {
                debug!(nonce = hit.nonce, "Share found (batch)");
                let _ = share_tx.send(FoundShare {
                    job_id: job.job_id.clone(),
                    nonce: hit.nonce,
                    hash: hit.hash.clone(),
                });
            }

            self.total_hashes.fetch_add(batch_size, Ordering::Relaxed);
            nonce = nonce.wrapping_add(batch_size);
        }

        info!("Mining stopped");
        Ok(())
    }

    fn mine_single(
        &self,
        job: MiningJob,
        start_nonce: u64,
        share_tx: mpsc::UnboundedSender<FoundShare>,
    ) -> Result<()> {
        let mut nonce = start_nonce;

        while self.running.load(Ordering::Relaxed) {
            let hash = self.hasher.hash(&job, Nonce(nonce))?;

            if self.hasher.meets_target(&hash, &job.target) {
                debug!(nonce, "Share found");
                let _ = share_tx.send(FoundShare {
                    job_id: job.job_id.clone(),
                    nonce,
                    hash,
                });
            }

            nonce = nonce.wrapping_add(1);
            self.total_hashes.fetch_add(1, Ordering::Relaxed);
        }

        info!("Mining stopped");
        Ok(())
    }
}
