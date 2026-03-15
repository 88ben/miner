use std::sync::Mutex;

use randomx_rs::{RandomXCache, RandomXDataset, RandomXFlag, RandomXVM};
use tracing::{debug, info};

use miner_core::algorithm::{Algorithm, Hasher};
use miner_core::error::{MinerError, Result};
use miner_core::types::{MiningJob, Nonce};

/// Wrapper to make the RandomX FFI types Send + Sync.
/// Safety: each RandomXVM is only used from one thread at a time (guarded by Mutex).
/// The underlying C library allocates/frees its own memory and the pointers are stable.
struct RxState {
    vm: RandomXVM,
}

// SAFETY: The RandomX C library's VM, cache, and dataset are heap-allocated
// and stable. We protect concurrent access with a Mutex.
unsafe impl Send for RxState {}
unsafe impl Sync for RxState {}

pub struct RandomXHasher {
    flags: RandomXFlag,
    state: Mutex<Option<RxState>>,
    seed_hash: Mutex<Vec<u8>>,
}

impl RandomXHasher {
    pub fn new() -> Self {
        let flags = RandomXFlag::get_recommended_flags();
        info!(flags = ?flags, "RandomX flags detected");
        Self {
            flags,
            state: Mutex::new(None),
            seed_hash: Mutex::new(Vec::new()),
        }
    }

    pub fn set_seed(&self, seed_hash: &[u8]) -> Result<()> {
        {
            let current = self.seed_hash.lock().unwrap();
            if current.as_slice() == seed_hash && self.state.lock().unwrap().is_some() {
                return Ok(());
            }
        }

        debug!("Initializing RandomX cache");
        let cache = RandomXCache::new(self.flags, seed_hash)
            .map_err(|e| MinerError::Hardware(format!("RandomX cache init failed: {e}")))?;

        debug!("Initializing RandomX dataset");
        let dataset = RandomXDataset::new(self.flags, cache, 0)
            .map_err(|e| MinerError::Hardware(format!("RandomX dataset init failed: {e}")))?;

        debug!("Allocating new RandomX cache for VM");
        let cache2 = RandomXCache::new(self.flags, seed_hash)
            .map_err(|e| MinerError::Hardware(format!("RandomX cache2 init failed: {e}")))?;

        debug!("Creating RandomX VM");
        let vm = RandomXVM::new(self.flags, Some(cache2), Some(dataset))
            .map_err(|e| MinerError::Hardware(format!("RandomX VM creation failed: {e}")))?;

        *self.state.lock().unwrap() = Some(RxState { vm });
        *self.seed_hash.lock().unwrap() = seed_hash.to_vec();

        info!("RandomX initialized with new seed");
        Ok(())
    }
}

impl Hasher for RandomXHasher {
    fn algorithm(&self) -> Algorithm {
        Algorithm::RandomX
    }

    fn init(&mut self) -> Result<()> {
        if self.state.lock().unwrap().is_none() {
            self.set_seed(&[0u8; 32])?;
        }
        Ok(())
    }

    fn hash(&self, job: &MiningJob, nonce: Nonce) -> Result<Vec<u8>> {
        let guard = self.state.lock().unwrap();
        let state = guard
            .as_ref()
            .ok_or_else(|| MinerError::Hardware("RandomX VM not initialized".into()))?;

        let mut input = job.blob.clone();
        if input.len() >= 43 {
            let nonce_bytes = (nonce.0 as u32).to_le_bytes();
            input[39..43].copy_from_slice(&nonce_bytes);
        } else {
            input.extend_from_slice(&nonce.0.to_le_bytes());
        }

        let hash = state
            .vm
            .calculate_hash(&input)
            .map_err(|e| MinerError::Hardware(format!("RandomX hash failed: {e}")))?;

        Ok(hash.to_vec())
    }

    fn meets_target(&self, hash: &[u8], target: &[u8]) -> bool {
        for (h, t) in hash.iter().rev().zip(target.iter().rev()) {
            if h < t {
                return true;
            }
            if h > t {
                return false;
            }
        }
        true
    }

    fn preferred_batch_size(&self) -> u64 {
        256
    }
}
