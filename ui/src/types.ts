export interface StatsSnapshot {
  algorithm: string;
  hashrate: number;
  accepted_shares: number;
  rejected_shares: number;
  uptime_secs: number;
}

export interface Coin {
  name: string;
  symbol: string;
  algorithm: string;
  default_pool: PoolConfig | null;
}

export interface PoolConfig {
  url: string;
  port: number;
  tls: boolean;
}
