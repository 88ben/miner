use serde::{Deserialize, Serialize};

use crate::algorithm::Algorithm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub name: String,
    pub symbol: String,
    pub algorithm: Algorithm,
    pub default_pool: Option<PoolConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub url: String,
    pub port: u16,
    pub tls: bool,
}

impl PoolConfig {
    pub fn address(&self) -> String {
        let scheme = if self.tls { "stratum+ssl" } else { "stratum+tcp" };
        format!("{}://{}:{}", scheme, self.url, self.port)
    }
}

pub fn supported_coins() -> Vec<Coin> {
    vec![
        Coin {
            name: "Monero".into(),
            symbol: "XMR".into(),
            algorithm: Algorithm::RandomX,
            default_pool: None,
        },
        Coin {
            name: "Ethereum Classic".into(),
            symbol: "ETC".into(),
            algorithm: Algorithm::EtcHash,
            default_pool: None,
        },
        Coin {
            name: "Ravencoin".into(),
            symbol: "RVN".into(),
            algorithm: Algorithm::KawPow,
            default_pool: None,
        },
        Coin {
            name: "Kaspa".into(),
            symbol: "KAS".into(),
            algorithm: Algorithm::KHeavyHash,
            default_pool: None,
        },
        Coin {
            name: "Zcash".into(),
            symbol: "ZEC".into(),
            algorithm: Algorithm::Equihash,
            default_pool: None,
        },
    ]
}
