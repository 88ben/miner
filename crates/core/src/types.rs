use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HardwareKind {
    Cpu,
    GpuNvidia,
    GpuAmd,
}

impl std::fmt::Display for HardwareKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu => write!(f, "CPU"),
            Self::GpuNvidia => write!(f, "GPU (NVIDIA/CUDA)"),
            Self::GpuAmd => write!(f, "GPU (AMD/OpenCL)"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareResult {
    pub accepted: bool,
    pub difficulty: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningJob {
    pub job_id: String,
    pub blob: Vec<u8>,
    pub target: Vec<u8>,
    pub difficulty: f64,
    pub height: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nonce(pub u64);

#[derive(Debug, Clone)]
pub struct FoundNonce {
    pub nonce: u64,
    pub hash: Vec<u8>,
}
