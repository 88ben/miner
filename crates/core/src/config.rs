use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::algorithm::Algorithm;
use crate::coin::PoolConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerConfig {
    pub worker_name: String,
    pub coins: Vec<CoinEntry>,
    pub api: ApiConfig,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinEntry {
    pub symbol: String,
    pub algorithm: Algorithm,
    pub wallet: String,
    pub pool: PoolConfig,
    pub threads: Option<usize>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

impl Default for MinerConfig {
    fn default() -> Self {
        Self {
            worker_name: hostname(),
            coins: Vec::new(),
            api: ApiConfig {
                enabled: true,
                host: "127.0.0.1".into(),
                port: 3030,
            },
            log_level: "info".into(),
        }
    }
}

impl MinerConfig {
    pub fn load(path: &PathBuf) -> crate::error::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&data)?;
        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> crate::error::Result<()> {
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(path, data)?;
        Ok(())
    }
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "miner-rig".into())
}
