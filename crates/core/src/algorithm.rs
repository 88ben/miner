use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::types::{FoundNonce, HardwareKind, MiningJob, Nonce};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Algorithm {
    RandomX,
    EtcHash,
    KawPow,
    KHeavyHash,
    Equihash,
}

impl Algorithm {
    pub fn name(&self) -> &'static str {
        match self {
            Self::RandomX => "RandomX",
            Self::EtcHash => "ETCHash",
            Self::KawPow => "KawPow",
            Self::KHeavyHash => "kHeavyHash",
            Self::Equihash => "Equihash",
        }
    }

    pub fn supported_hardware(&self) -> Vec<HardwareKind> {
        match self {
            Self::RandomX => vec![HardwareKind::Cpu],
            Self::EtcHash => vec![HardwareKind::Cpu, HardwareKind::GpuNvidia, HardwareKind::GpuAmd],
            Self::KawPow => vec![HardwareKind::Cpu, HardwareKind::GpuNvidia, HardwareKind::GpuAmd],
            Self::KHeavyHash => vec![HardwareKind::Cpu, HardwareKind::GpuNvidia, HardwareKind::GpuAmd],
            Self::Equihash => vec![HardwareKind::Cpu],
        }
    }
}

impl std::fmt::Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for Algorithm {
    type Err = crate::error::MinerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "randomx" => Ok(Self::RandomX),
            "etchash" | "ethash" => Ok(Self::EtcHash),
            "kawpow" => Ok(Self::KawPow),
            "kheavyhash" => Ok(Self::KHeavyHash),
            "equihash" => Ok(Self::Equihash),
            _ => Err(crate::error::MinerError::UnsupportedAlgorithm(
                s.to_string(),
            )),
        }
    }
}

/// Trait that every mining algorithm backend must implement.
///
/// CPU hashers implement `hash()` for single-nonce operation.
/// GPU hashers override `hash_batch()` for massively parallel nonce scanning.
pub trait Hasher: Send + Sync {
    fn algorithm(&self) -> Algorithm;

    fn init(&mut self) -> Result<()>;

    /// Compute hash for a single nonce (CPU path).
    fn hash(&self, job: &MiningJob, nonce: Nonce) -> Result<Vec<u8>>;

    /// Batch-hash a range of nonces, returning only those that meet the target.
    /// GPU hashers should override this. The default falls back to single-nonce looping.
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
                found.push(FoundNonce { nonce: n, hash });
            }
        }
        Ok(found)
    }

    fn meets_target(&self, hash: &[u8], target: &[u8]) -> bool {
        hash.iter()
            .zip(target.iter())
            .all(|(h, t)| h <= t)
    }

    fn is_gpu(&self) -> bool {
        false
    }

    fn preferred_batch_size(&self) -> u64 {
        1
    }
}
