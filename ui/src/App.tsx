import { useEffect, useState } from "react";
import { fetchCoins, fetchStats, stopAllWorkers } from "./api";
import { CoinBadge } from "./components/CoinBadge";
import { StatCard } from "./components/StatCard";
import { WorkerRow } from "./components/WorkerRow";
import type { Coin, StatsSnapshot } from "./types";
import { formatHashrate } from "./utils";

const POLL_INTERVAL = 2000;

export default function App() {
  const [stats, setStats] = useState<StatsSnapshot[]>([]);
  const [coins, setCoins] = useState<Coin[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [stopping, setStopping] = useState(false);

  useEffect(() => {
    fetchCoins().then(setCoins).catch(() => {});

    const poll = () => {
      fetchStats()
        .then((data) => {
          setStats(data);
          setError(null);
        })
        .catch(() => setError("Cannot reach miner API"));
    };

    poll();
    const interval = setInterval(poll, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, []);

  const totalHashrate = stats.reduce((sum, s) => sum + s.hashrate, 0);
  const totalAccepted = stats.reduce((sum, s) => sum + s.accepted_shares, 0);
  const totalRejected = stats.reduce((sum, s) => sum + s.rejected_shares, 0);

  const handleStopAll = async () => {
    setStopping(true);
    try {
      await stopAllWorkers();
    } finally {
      setStopping(false);
    }
  };

  return (
    <div className="min-h-screen p-6 max-w-6xl mx-auto">
      {/* Header */}
      <header className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Miner Dashboard</h1>
          <p className="text-[var(--color-text-secondary)] text-sm mt-1">
            Multi-cryptocurrency mining engine
          </p>
        </div>
        <button
          onClick={handleStopAll}
          disabled={stopping || stats.length === 0}
          className="px-4 py-2 rounded-lg bg-[var(--color-danger)] text-white font-medium text-sm
                     hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
        >
          {stopping ? "Stopping..." : "Stop All Workers"}
        </button>
      </header>

      {error && (
        <div className="mb-6 p-3 rounded-lg bg-[var(--color-danger)]/10 border border-[var(--color-danger)]/30 text-[var(--color-danger)] text-sm">
          {error}
        </div>
      )}

      {/* Summary Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
        <StatCard
          label="Total Hashrate"
          value={formatHashrate(totalHashrate)}
          color="text-[var(--color-accent)]"
        />
        <StatCard
          label="Active Workers"
          value={String(stats.length)}
        />
        <StatCard
          label="Accepted Shares"
          value={String(totalAccepted)}
          color="text-[var(--color-success)]"
        />
        <StatCard
          label="Rejected Shares"
          value={String(totalRejected)}
          sub={
            totalAccepted + totalRejected > 0
              ? `${((totalRejected / (totalAccepted + totalRejected)) * 100).toFixed(1)}% reject rate`
              : undefined
          }
          color="text-[var(--color-danger)]"
        />
      </div>

      {/* Workers Table */}
      <section className="rounded-xl bg-[var(--color-bg-card)] border border-[var(--color-border)] overflow-hidden mb-8">
        <div className="px-5 py-4 border-b border-[var(--color-border)]">
          <h2 className="text-lg font-semibold">Workers</h2>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="text-left text-[var(--color-text-secondary)] text-xs uppercase tracking-wider">
                <th className="py-3 px-4">Algorithm</th>
                <th className="py-3 px-4">Hashrate</th>
                <th className="py-3 px-4">Accepted</th>
                <th className="py-3 px-4">Rejected</th>
                <th className="py-3 px-4">Reject %</th>
                <th className="py-3 px-4">Uptime</th>
              </tr>
            </thead>
            <tbody>
              {stats.length === 0 ? (
                <tr>
                  <td colSpan={6} className="py-8 text-center text-[var(--color-text-secondary)]">
                    No active workers
                  </td>
                </tr>
              ) : (
                stats.map((s, i) => <WorkerRow key={i} stats={s} />)
              )}
            </tbody>
          </table>
        </div>
      </section>

      {/* Supported Coins */}
      <section className="rounded-xl bg-[var(--color-bg-card)] border border-[var(--color-border)] p-5">
        <h2 className="text-lg font-semibold mb-4">Supported Coins</h2>
        <div className="flex flex-wrap gap-3">
          {coins.map((c) => (
            <CoinBadge key={c.symbol} coin={c} />
          ))}
        </div>
      </section>
    </div>
  );
}
