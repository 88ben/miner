use hasherkawpow_sys::hash_kawpow;
use tracing::debug;

use miner_core::algorithm::{Algorithm, Hasher};
use miner_core::error::{MinerError, Result};
use miner_core::types::{FoundNonce, MiningJob, Nonce};

pub struct KawPowHasher {
    _private: (),
}

impl KawPowHasher {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Hasher for KawPowHasher {
    fn algorithm(&self) -> Algorithm {
        Algorithm::KawPow
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn hash(&self, job: &MiningJob, nonce: Nonce) -> Result<Vec<u8>> {
        let header_hash: [u8; 32] = job
            .blob
            .as_slice()
            .try_into()
            .map_err(|_| MinerError::Hardware("KawPow requires 32-byte header hash".into()))?;

        let height = job
            .height
            .ok_or_else(|| MinerError::Hardware("KawPow requires block height".into()))?;

        let (result, _mix_hash) = hash_kawpow(&header_hash, &nonce.0, height as i32);
        Ok(result.to_vec())
    }

    fn hash_batch(
        &self,
        job: &MiningJob,
        start_nonce: u64,
        batch_size: u64,
    ) -> Result<Vec<FoundNonce>> {
        let header_hash: [u8; 32] = job
            .blob
            .as_slice()
            .try_into()
            .map_err(|_| MinerError::Hardware("KawPow requires 32-byte header hash".into()))?;

        let height = job
            .height
            .ok_or_else(|| MinerError::Hardware("KawPow requires block height".into()))?;

        let mut found = Vec::new();
        for i in 0..batch_size {
            let n = start_nonce.wrapping_add(i);
            let (result, _mix_hash) = hash_kawpow(&header_hash, &n, height as i32);
            if self.meets_target(&result, &job.target) {
                debug!(nonce = n, "KawPow share found");
                found.push(FoundNonce {
                    nonce: n,
                    hash: result.to_vec(),
                });
            }
        }
        Ok(found)
    }

    fn meets_target(&self, hash: &[u8], target: &[u8]) -> bool {
        for (h, t) in hash.iter().zip(target.iter()) {
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
        512
    }
}
