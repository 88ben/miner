import type { Coin, CoinEntry, MinerConfig, StatsSnapshot } from "./types";

const BASE = import.meta.env.VITE_API_URL ?? "http://127.0.0.1:3030";

export async function fetchStats(): Promise<StatsSnapshot[]> {
  const res = await fetch(`${BASE}/api/stats`);
  if (!res.ok) throw new Error(`Stats fetch failed: ${res.status}`);
  return res.json();
}

export async function fetchCoins(): Promise<Coin[]> {
  const res = await fetch(`${BASE}/api/coins`);
  if (!res.ok) throw new Error(`Coins fetch failed: ${res.status}`);
  return res.json();
}

export async function fetchConfig(): Promise<MinerConfig> {
  const res = await fetch(`${BASE}/api/config`);
  if (!res.ok) throw new Error(`Config fetch failed: ${res.status}`);
  return res.json();
}

export async function updateCoinEntry(
  index: number,
  entry: CoinEntry
): Promise<MinerConfig> {
  const res = await fetch(`${BASE}/api/config/coins/${index}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(entry),
  });
  if (!res.ok) throw new Error(`Update failed: ${res.status}`);
  return res.json();
}

export async function fetchWorkerStatus(): Promise<Record<string, boolean>> {
  const res = await fetch(`${BASE}/api/workers/status`);
  if (!res.ok) throw new Error(`Status fetch failed: ${res.status}`);
  return res.json();
}

export async function startWorker(
  index: number
): Promise<{ ok: boolean; error?: string; worker_id?: string }> {
  const res = await fetch(`${BASE}/api/workers/start/${index}`, {
    method: "POST",
  });
  if (!res.ok) throw new Error(`Start failed: ${res.status}`);
  return res.json();
}

export async function stopWorker(
  index: number
): Promise<{ ok: boolean; stopped?: boolean }> {
  const res = await fetch(`${BASE}/api/workers/stop/${index}`, {
    method: "POST",
  });
  if (!res.ok) throw new Error(`Stop failed: ${res.status}`);
  return res.json();
}

export async function stopAllWorkers(): Promise<void> {
  const res = await fetch(`${BASE}/api/workers/stop-all`, { method: "POST" });
  if (!res.ok) throw new Error(`Stop failed: ${res.status}`);
}
