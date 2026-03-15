use miner_core::algorithm::{Algorithm, Hasher};
use miner_core::error::{MinerError, Result};

/// Create the appropriate hasher for a given algorithm.
pub fn create_hasher(algorithm: Algorithm) -> Result<Box<dyn Hasher>> {
    match algorithm {
        #[cfg(feature = "randomx")]
        Algorithm::RandomX => {
            let mut h = crate::randomx::RandomXHasher::new();
            h.init()?;
            Ok(Box::new(h))
        }

        #[cfg(feature = "ethash")]
        Algorithm::EtcHash => {
            let mut h = crate::ethash::EthashHasher::new();
            h.init()?;
            Ok(Box::new(h))
        }

        #[cfg(feature = "kawpow")]
        Algorithm::KawPow => {
            let mut h = crate::kawpow::KawPowHasher::new();
            h.init()?;
            Ok(Box::new(h))
        }

        #[cfg(feature = "kheavyhash")]
        Algorithm::KHeavyHash => {
            let mut h = crate::kheavyhash::KHeavyHashHasher::new();
            h.init()?;
            Ok(Box::new(h))
        }

        #[cfg(feature = "equihash")]
        Algorithm::Equihash => {
            let mut h = crate::equihash::EquihashSolver::new();
            h.init()?;
            Ok(Box::new(h))
        }

        #[allow(unreachable_patterns)]
        _ => Err(MinerError::UnsupportedAlgorithm(format!(
            "{} (feature not enabled)",
            algorithm
        ))),
    }
}
