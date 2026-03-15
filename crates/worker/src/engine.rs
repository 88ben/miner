use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, info};

use miner_core::algorithm::Hasher;
use miner_core::error::Result;
use miner_core::types::{MiningJob, Nonce};

/// A single mining thread/worker that hashes against a given job.
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

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Mine against a job, scanning nonces from `start_nonce`.
    /// Found shares are sent through the channel.
    pub fn mine(
        &self,
        job: MiningJob,
        start_nonce: u64,
        share_tx: mpsc::UnboundedSender<FoundShare>,
    ) -> Result<()> {
        self.running.store(true, Ordering::Relaxed);

        info!(
            algo = %self.hasher.algorithm(),
            job_id = %job.job_id,
            "Mining started"
        );

        let mut nonce = start_nonce;

        while self.running.load(Ordering::Relaxed) {
            let hash = self.hasher.hash(&job, Nonce(nonce))?;

            if self.hasher.meets_target(&hash, &job.target) {
                debug!(nonce, "Share found!");
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
