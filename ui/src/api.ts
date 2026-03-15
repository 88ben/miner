import type { Coin, StatsSnapshot } from "./types";

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

export async function stopAllWorkers(): Promise<void> {
  const res = await fetch(`${BASE}/api/workers/stop-all`, { method: "POST" });
  if (!res.ok) throw new Error(`Stop failed: ${res.status}`);
}
