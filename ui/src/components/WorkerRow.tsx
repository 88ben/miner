import type { CoinEntry, StatsSnapshot } from "../types";
import { formatHashrate, formatUptime } from "../utils";
import { CoinIcon } from "./CoinIcon";
import { PowerButton, type PowerState } from "./PowerButton";

interface WorkerRowProps {
  coin: CoinEntry;
  stats?: StatsSnapshot;
  running: boolean;
  powerState: PowerState;
  powerError?: string;
  disabledReason?: string;
  onEdit: () => void;
  onToggle: () => void;
}

export function WorkerRow({
  coin,
  stats,
  running,
  powerState,
  powerError,
  disabledReason,
  onEdit,
  onToggle,
}: WorkerRowProps) {
  const total = stats ? stats.accepted_shares + stats.rejected_shares : 0;
  const rejectRate =
    total > 0 ? ((stats!.rejected_shares / total) * 100).toFixed(1) : "0.0";

  return (
    <tr className="border-b border-[var(--color-border)] hover:bg-[var(--color-bg-secondary)] transition-colors">
      <td className="py-3 px-4">
        <div className="flex items-center gap-3">
          <div className="relative flex-shrink-0">
            <CoinIcon symbol={coin.symbol} size={24} />
            <span
              className={`absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2 border-[var(--color-bg-card)] ${
                running
                  ? "bg-[var(--color-success)]"
                  : "bg-[var(--color-text-secondary)]"
              }`}
            />
          </div>
          <div className="flex flex-col">
            <span className="font-medium leading-tight">{coin.symbol}</span>
            <span className="text-[var(--color-text-secondary)] text-xs leading-tight">
              {coin.algorithm}
            </span>
          </div>
        </div>
      </td>
      <td className="py-3 px-4 text-[var(--color-accent)] font-mono font-bold">
        {stats ? formatHashrate(stats.hashrate) : "—"}
      </td>
      <td className="py-3 px-4 text-[var(--color-success)]">
        {stats ? stats.accepted_shares : "—"}
      </td>
      <td className="py-3 px-4 text-[var(--color-danger)]">
        {stats ? stats.rejected_shares : "—"}
      </td>
      <td className="py-3 px-4 text-[var(--color-text-secondary)]">
        {stats ? `${rejectRate}%` : "—"}
      </td>
      <td className="py-3 px-4 text-[var(--color-text-secondary)] font-mono text-sm">
        {stats ? formatUptime(stats.uptime_secs) : "—"}
      </td>
      <td className="py-3 px-4">
        <div className="flex items-center gap-1">
          <button
            onClick={onEdit}
            className="p-1.5 rounded-md text-[var(--color-text-secondary)] hover:text-white
                       hover:bg-[var(--color-bg-secondary)] transition-colors cursor-pointer"
            title={`Edit ${coin.symbol} worker`}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 20 20"
              fill="currentColor"
              className="w-4 h-4"
            >
              <path d="M2.695 14.763l-1.262 3.154a.5.5 0 00.65.65l3.155-1.262a4 4 0 001.343-.885L17.5 5.5a2.121 2.121 0 00-3-3L3.58 13.42a4 4 0 00-.885 1.343z" />
            </svg>
          </button>
          <PowerButton
            state={powerState}
            onClick={onToggle}
            symbol={coin.symbol}
            errorMsg={powerError}
            disabledReason={disabledReason}
          />
        </div>
      </td>
    </tr>
  );
}
