use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::algorithm::Algorithm;

const HASHRATE_WINDOW: Duration = Duration::from_secs(60);

/// Tracks per-algorithm mining statistics.
#[derive(Debug)]
pub struct MinerStats {
    pub algorithm: Algorithm,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    hash_timestamps: VecDeque<(Instant, u64)>,
    started_at: Instant,
}

impl MinerStats {
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            algorithm,
            accepted_shares: 0,
            rejected_shares: 0,
            hash_timestamps: VecDeque::new(),
            started_at: Instant::now(),
        }
    }

    pub fn record_hashes(&mut self, count: u64) {
        let now = Instant::now();
        self.hash_timestamps.push_back((now, count));
        self.prune_old_entries(now);
    }

    pub fn record_share(&mut self, accepted: bool) {
        if accepted {
            self.accepted_shares += 1;
        } else {
            self.rejected_shares += 1;
        }
    }

    /// Current hashrate in H/s averaged over the last 60 seconds.
    pub fn hashrate(&self) -> f64 {
        let now = Instant::now();
        let cutoff = now - HASHRATE_WINDOW;

        let total: u64 = self
            .hash_timestamps
            .iter()
            .filter(|(t, _)| *t >= cutoff)
            .map(|(_, c)| c)
            .sum();

        let elapsed = now.duration_since(self.started_at).min(HASHRATE_WINDOW);
        if elapsed.as_secs_f64() == 0.0 {
            return 0.0;
        }

        total as f64 / elapsed.as_secs_f64()
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    fn prune_old_entries(&mut self, now: Instant) {
        let cutoff = now - HASHRATE_WINDOW;
        while self
            .hash_timestamps
            .front()
            .map_or(false, |(t, _)| *t < cutoff)
        {
            self.hash_timestamps.pop_front();
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsSnapshot {
    pub algorithm: String,
    pub hashrate: f64,
    pub accepted_shares: u64,
    pub rejected_shares: u64,
    pub uptime_secs: u64,
}

impl From<&MinerStats> for StatsSnapshot {
    fn from(stats: &MinerStats) -> Self {
        Self {
            algorithm: stats.algorithm.name().to_string(),
            hashrate: stats.hashrate(),
            accepted_shares: stats.accepted_shares,
            rejected_shares: stats.rejected_shares,
            uptime_secs: stats.uptime().as_secs(),
        }
    }
}
