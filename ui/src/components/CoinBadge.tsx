import type { Coin } from "../types";

const COIN_COLORS: Record<string, string> = {
  XMR: "bg-orange-500/20 text-orange-400 border-orange-500/30",
  ETC: "bg-green-500/20 text-green-400 border-green-500/30",
  RVN: "bg-purple-500/20 text-purple-400 border-purple-500/30",
  KAS: "bg-cyan-500/20 text-cyan-400 border-cyan-500/30",
  ZEC: "bg-yellow-500/20 text-yellow-400 border-yellow-500/30",
};

interface CoinBadgeProps {
  coin: Coin;
}

export function CoinBadge({ coin }: CoinBadgeProps) {
  const colorClass = COIN_COLORS[coin.symbol] ?? "bg-gray-500/20 text-gray-400 border-gray-500/30";

  return (
    <div className={`inline-flex items-center gap-2 rounded-lg border px-3 py-2 ${colorClass}`}>
      <span className="font-bold text-sm">{coin.symbol}</span>
      <span className="text-xs opacity-75">{coin.name}</span>
      <span className="text-[10px] opacity-50 ml-1">{coin.algorithm}</span>
    </div>
  );
}
