use thiserror::Error;

#[derive(Error, Debug)]
pub enum MinerError {
    #[error("Algorithm `{0}` is not supported")]
    UnsupportedAlgorithm(String),

    #[error("Connection to pool failed: {0}")]
    PoolConnection(String),

    #[error("Stratum protocol error: {0}")]
    Stratum(String),

    #[error("Worker `{0}` failed: {1}")]
    Worker(String, String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Hardware error: {0}")]
    Hardware(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, MinerError>;
