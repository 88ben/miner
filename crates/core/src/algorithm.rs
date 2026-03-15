use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::types::{HardwareKind, MiningJob, Nonce};

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
            Self::EtcHash => vec![HardwareKind::GpuNvidia, HardwareKind::GpuAmd],
            Self::KawPow => vec![HardwareKind::GpuNvidia, HardwareKind::GpuAmd],
            Self::KHeavyHash => vec![HardwareKind::GpuNvidia, HardwareKind::GpuAmd],
            Self::Equihash => vec![HardwareKind::GpuNvidia, HardwareKind::GpuAmd],
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
pub trait Hasher: Send + Sync {
    fn algorithm(&self) -> Algorithm;

    fn init(&mut self) -> Result<()>;

    fn hash(&self, job: &MiningJob, nonce: Nonce) -> Result<Vec<u8>>;

    fn meets_target(&self, hash: &[u8], target: &[u8]) -> bool {
        hash.iter()
            .zip(target.iter())
            .all(|(h, t)| h <= t)
    }
}
