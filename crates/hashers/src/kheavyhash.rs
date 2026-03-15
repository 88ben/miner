use sha3::{Digest, Keccak256};
use tracing::debug;

use miner_core::algorithm::{Algorithm, Hasher};
use miner_core::error::Result;
use miner_core::types::{FoundNonce, MiningJob, Nonce};

/// kHeavyHash: Kaspa's PoW algorithm.
/// hash = keccak256(matrix_multiply(keccak256(header || nonce)))
///
/// The matrix is a 64x64 matrix of u16 values derived from the block header.
pub struct KHeavyHashHasher {
    matrix: [[u16; 64]; 64],
    matrix_initialized: bool,
}

impl KHeavyHashHasher {
    pub fn new() -> Self {
        Self {
            matrix: [[0u16; 64]; 64],
            matrix_initialized: false,
        }
    }

    /// Set the mining matrix from the block header (pool provides this).
    pub fn set_matrix(&mut self, matrix_data: &[u8]) {
        // The matrix is 64x64 u16 values = 8192 bytes
        if matrix_data.len() >= 8192 {
            for row in 0..64 {
                for col in 0..64 {
                    let idx = (row * 64 + col) * 2;
                    self.matrix[row][col] =
                        u16::from_le_bytes([matrix_data[idx], matrix_data[idx + 1]]);
                }
            }
            self.matrix_initialized = true;
        }
    }

    /// Generate a default matrix from seed for testing/initial state.
    pub fn set_matrix_from_seed(&mut self, seed: &[u8; 32]) {
        let mut state = *seed;
        for row in 0..64 {
            for col in 0..64 {
                let hash = Keccak256::digest(&state);
                state.copy_from_slice(&hash);
                self.matrix[row][col] =
                    u16::from_le_bytes([state[0], state[1]]);
            }
        }
        self.matrix_initialized = true;
    }

    fn heavy_hash(&self, input: &[u8; 32]) -> [u8; 32] {
        // Step 1: first keccak
        let hash1 = Keccak256::digest(input);
        let hash1_bytes: [u8; 32] = hash1.into();

        // Step 2: matrix multiplication
        // Treat the 32-byte hash as 64 nibbles (4-bit values)
        let mut nibbles = [0u16; 64];
        for i in 0..32 {
            nibbles[i * 2] = (hash1_bytes[i] >> 4) as u16;
            nibbles[i * 2 + 1] = (hash1_bytes[i] & 0x0F) as u16;
        }

        let mut product = [0u32; 64];
        for i in 0..64 {
            let mut sum: u32 = 0;
            for j in 0..64 {
                sum = sum.wrapping_add(self.matrix[i][j] as u32 * nibbles[j] as u32);
            }
            product[i] = sum;
        }

        // Reduce back to nibbles and XOR with original
        let mut result_bytes = [0u8; 32];
        for i in 0..32 {
            let hi = ((product[i * 2] >> 10) as u8) & 0x0F;
            let lo = ((product[i * 2 + 1] >> 10) as u8) & 0x0F;
            result_bytes[i] = (hi << 4) | lo;
            result_bytes[i] ^= hash1_bytes[i];
        }

        // Step 3: second keccak
        let hash2 = Keccak256::digest(&result_bytes);
        hash2.into()
    }
}

impl Hasher for KHeavyHashHasher {
    fn algorithm(&self) -> Algorithm {
        Algorithm::KHeavyHash
    }

    fn init(&mut self) -> Result<()> {
        if !self.matrix_initialized {
            // Initialize with a default seed; will be updated when pool sends the matrix
            let seed = [0u8; 32];
            self.set_matrix_from_seed(&seed);
        }
        Ok(())
    }

    fn hash(&self, job: &MiningJob, nonce: Nonce) -> Result<Vec<u8>> {
        // Build header: blob + nonce
        let mut header = [0u8; 32];
        let copy_len = job.blob.len().min(32);
        header[..copy_len].copy_from_slice(&job.blob[..copy_len]);

        // Insert nonce into the header
        let nonce_bytes = nonce.0.to_le_bytes();
        let nonce_offset = 24.min(header.len() - 8);
        header[nonce_offset..nonce_offset + 8].copy_from_slice(&nonce_bytes);

        let result = self.heavy_hash(&header);
        Ok(result.to_vec())
    }

    fn hash_batch(
        &self,
        job: &MiningJob,
        start_nonce: u64,
        batch_size: u64,
    ) -> Result<Vec<FoundNonce>> {
        let mut found = Vec::new();
        for i in 0..batch_size {
            let n = start_nonce.wrapping_add(i);
            let hash = self.hash(job, Nonce(n))?;
            if self.meets_target(&hash, &job.target) {
                debug!(nonce = n, "kHeavyHash share found");
                found.push(FoundNonce { nonce: n, hash });
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
        2048
    }
}
