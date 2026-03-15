import type { StatsSnapshot } from "../types";
import { formatHashrate, formatUptime } from "../utils";

interface WorkerRowProps {
  stats: StatsSnapshot;
}

export function WorkerRow({ stats }: WorkerRowProps) {
  const total = stats.accepted_shares + stats.rejected_shares;
  const rejectRate = total > 0 ? ((stats.rejected_shares / total) * 100).toFixed(1) : "0.0";

  return (
    <tr className="border-b border-[var(--color-border)] hover:bg-[var(--color-bg-secondary)] transition-colors">
      <td className="py-3 px-4 font-medium">{stats.algorithm}</td>
      <td className="py-3 px-4 text-[var(--color-accent)] font-mono font-bold">
        {formatHashrate(stats.hashrate)}
      </td>
      <td className="py-3 px-4 text-[var(--color-success)]">{stats.accepted_shares}</td>
      <td className="py-3 px-4 text-[var(--color-danger)]">{stats.rejected_shares}</td>
      <td className="py-3 px-4 text-[var(--color-text-secondary)]">{rejectRate}%</td>
      <td className="py-3 px-4 text-[var(--color-text-secondary)] font-mono text-sm">
        {formatUptime(stats.uptime_secs)}
      </td>
    </tr>
  );
}
