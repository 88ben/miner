use tracing::{debug, info};

use miner_core::algorithm::{Algorithm, Hasher};
use miner_core::error::Result;
use miner_core::types::{MiningJob, Nonce};

// Zcash Equihash parameters: n=200, k=9
const N: u32 = 200;
const K: u32 = 9;
const NONCE_SIZE: usize = 32;

pub struct EquihashSolver {
    _private: (),
}

impl EquihashSolver {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Hasher for EquihashSolver {
    fn algorithm(&self) -> Algorithm {
        Algorithm::Equihash
    }

    fn init(&mut self) -> Result<()> {
        info!("Equihash solver initialized (n={}, k={})", N, K);
        Ok(())
    }

    fn hash(&self, job: &MiningJob, nonce: Nonce) -> Result<Vec<u8>> {
        let input = job.blob.clone();

        // Build a nonce as a 32-byte LE array
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        let le = nonce.0.to_le_bytes();
        nonce_bytes[..le.len()].copy_from_slice(&le);

        // The tromp solver takes a closure that produces nonces to try.
        // We give it exactly one nonce.
        let mut used = false;
        let next_nonce = || {
            if !used {
                used = true;
                Some(nonce_bytes)
            } else {
                None
            }
        };

        let solutions = equihash_crate::tromp::solve_200_9(&input, next_nonce);

        if let Some(solution) = solutions.into_iter().next() {
            debug!(nonce = nonce.0, "Equihash solution found");

            if equihash_crate::is_valid_solution(N, K, &input, &nonce_bytes, &solution)
                .is_ok()
            {
                return Ok(solution);
            }
        }

        // No solution found for this nonce
        Ok(Vec::new())
    }

    fn meets_target(&self, hash: &[u8], _target: &[u8]) -> bool {
        // A non-empty solution is a valid share
        !hash.is_empty()
    }

    fn preferred_batch_size(&self) -> u64 {
        16
    }
}
