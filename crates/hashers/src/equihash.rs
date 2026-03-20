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

        let mut nonce_bytes = [0u8; NONCE_SIZE];

        // If we have extranonce1 from the pool, place it at the start of the nonce
        let en2_offset = if let Some(ref en1) = job.extranonce1 {
            let len = en1.len().min(NONCE_SIZE);
            nonce_bytes[..len].copy_from_slice(&en1[..len]);
            len
        } else {
            0
        };

        // Fill the rest with our extranonce2 (derived from the nonce counter)
        let en2_size = job.extranonce2_size.unwrap_or(NONCE_SIZE - en2_offset);
        let le = nonce.0.to_le_bytes();
        let copy_len = le.len().min(en2_size).min(NONCE_SIZE - en2_offset);
        nonce_bytes[en2_offset..en2_offset + copy_len].copy_from_slice(&le[..copy_len]);

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

        Ok(Vec::new())
    }

    fn meets_target(&self, hash: &[u8], _target: &[u8]) -> bool {
        !hash.is_empty()
    }

    fn preferred_batch_size(&self) -> u64 {
        16
    }
}
