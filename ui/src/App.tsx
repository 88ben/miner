import { useEffect, useState, useCallback } from "react";
import {
  fetchConfig,
  fetchStats,
  fetchWorkerStatus,
  startWorker,
  stopWorker,
  stopAllWorkers,
  updateCoinEntry,
} from "./api";
import { EditWorkerDialog } from "./components/EditWorkerDialog";
import type { PowerState } from "./components/PowerButton";
import { StatCard } from "./components/StatCard";
import { WorkerRow } from "./components/WorkerRow";
import type { CoinEntry, MinerConfig, StatsSnapshot } from "./types";
import { formatHashrate } from "./utils";

const POLL_INTERVAL = 2000;

function isCoinConfigured(coin: CoinEntry): boolean {
  return (
    coin.wallet.length > 0 &&
    !coin.wallet.startsWith("YOUR_") &&
    coin.pool.url.length > 0 &&
    coin.pool.port > 0
  );
}

function getDisabledReason(coin: CoinEntry): string | undefined {
  if (!coin.wallet || coin.wallet.length === 0)
    return "No wallet address set";
  if (coin.wallet.startsWith("YOUR_"))
    return "Wallet address is still a placeholder";
  if (!coin.pool.url || coin.pool.url.length === 0)
    return "No pool URL set";
  if (!coin.pool.port || coin.pool.port <= 0)
    return "No pool port set";
  return undefined;
}

export default function App() {
  const [stats, setStats] = useState<StatsSnapshot[]>([]);
  const [config, setConfig] = useState<MinerConfig | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stopping, setStopping] = useState(false);
  const [editIndex, setEditIndex] = useState<number | null>(null);

  const [workerStatus, setWorkerStatus] = useState<Record<string, boolean>>({});
  const [powerStates, setPowerStates] = useState<Record<number, PowerState>>({});
  const [powerErrors, setPowerErrors] = useState<Record<number, string>>({});

  const refreshStatus = useCallback(async () => {
    try {
      const status = await fetchWorkerStatus();
      setWorkerStatus(status);
    } catch {
      // silently ignore — we'll retry on next poll
    }
  }, []);

  useEffect(() => {
    fetchConfig().then(setConfig).catch(() => {});
    refreshStatus();

    const poll = () => {
      fetchStats()
        .then((data) => {
          setStats(data);
          setError(null);
        })
        .catch(() => setError("Cannot reach miner API"));

      refreshStatus();
    };

    poll();
    const interval = setInterval(poll, POLL_INTERVAL);
    return () => clearInterval(interval);
  }, [refreshStatus]);

  const totalHashrate = stats.reduce((sum, s) => sum + s.hashrate, 0);
  const totalAccepted = stats.reduce((sum, s) => sum + s.accepted_shares, 0);
  const totalRejected = stats.reduce((sum, s) => sum + s.rejected_shares, 0);

  const handleStopAll = async () => {
    setStopping(true);
    try {
      await stopAllWorkers();
      await refreshStatus();
    } finally {
      setStopping(false);
    }
  };

  const handleSave = async (index: number, entry: CoinEntry) => {
    try {
      const updated = await updateCoinEntry(index, entry);
      setConfig(updated);
      setEditIndex(null);
    } catch {
      alert("Failed to save config. Check the backend logs.");
    }
  };

  const handleToggle = async (index: number, coin: CoinEntry) => {
    const running = workerStatus[coin.symbol] ?? false;

    if (running) {
      setPowerStates((p) => ({ ...p, [index]: "loading" }));
      try {
        await stopWorker(index);
        await refreshStatus();
        setPowerStates((p) => ({ ...p, [index]: "off" }));
        setPowerErrors((e) => {
          const copy = { ...e };
          delete copy[index];
          return copy;
        });
      } catch (err) {
        setPowerStates((p) => ({ ...p, [index]: "error" }));
        setPowerErrors((e) => ({
          ...e,
          [index]: err instanceof Error ? err.message : "Failed to stop worker",
        }));
      }
    } else {
      setPowerStates((p) => ({ ...p, [index]: "loading" }));
      try {
        const res = await startWorker(index);
        await refreshStatus();
        if (res.ok) {
          setPowerStates((p) => ({ ...p, [index]: "on" }));
          setPowerErrors((e) => {
            const copy = { ...e };
            delete copy[index];
            return copy;
          });
        } else {
          setPowerStates((p) => ({ ...p, [index]: "error" }));
          setPowerErrors((e) => ({
            ...e,
            [index]: res.error || "Unknown error",
          }));
        }
      } catch (err) {
        setPowerStates((p) => ({ ...p, [index]: "error" }));
        setPowerErrors((e) => ({
          ...e,
          [index]: err instanceof Error ? err.message : "Failed to start worker",
        }));
      }
    }
  };

  const findStatsForCoin = (coin: CoinEntry): StatsSnapshot | undefined => {
    return stats.find(
      (s) => s.algorithm.toLowerCase() === coin.algorithm.toLowerCase()
    );
  };

  const resolvePowerState = (index: number, coin: CoinEntry): PowerState => {
    const override = powerStates[index];
    if (override === "loading") return "loading";

    if (!isCoinConfigured(coin)) return "disabled";

    const running = workerStatus[coin.symbol] ?? false;

    if (override === "error") return "error";

    return running ? "on" : "off";
  };

  return (
    <div className="min-h-screen p-6 max-w-6xl mx-auto">
      {/* Header */}
      <header className="flex items-center justify-between mb-8">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">
            Miner Dashboard
          </h1>
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
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-8 mb-8">
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
                <th className="py-3 px-4">Coin</th>
                <th className="py-3 px-4">Hashrate</th>
                <th className="py-3 px-4">Accepted</th>
                <th className="py-3 px-4">Rejected</th>
                <th className="py-3 px-4">Reject %</th>
                <th className="py-3 px-4">Uptime</th>
                <th className="py-3 px-4 w-20"></th>
              </tr>
            </thead>
            <tbody>
              {config && config.coins.length > 0 ? (
                config.coins.map((coin, i) => (
                  <WorkerRow
                    key={coin.symbol}
                    coin={coin}
                    stats={findStatsForCoin(coin)}
                    running={workerStatus[coin.symbol] ?? false}
                    powerState={resolvePowerState(i, coin)}
                    powerError={powerErrors[i]}
                    disabledReason={getDisabledReason(coin)}
                    onEdit={() => setEditIndex(i)}
                    onToggle={() => handleToggle(i, coin)}
                  />
                ))
              ) : (
                <tr>
                  <td
                    colSpan={7}
                    className="py-8 text-center text-[var(--color-text-secondary)]"
                  >
                    No workers configured
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </section>

      {/* Edit Dialog */}
      {editIndex !== null && config && (
        <EditWorkerDialog
          entry={config.coins[editIndex]}
          index={editIndex}
          onSave={handleSave}
          onClose={() => setEditIndex(null)}
        />
      )}
    </div>
  );
}
