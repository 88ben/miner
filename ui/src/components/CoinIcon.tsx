const ICON_CDN = "https://cdn.jsdelivr.net/gh/spothq/cryptocurrency-icons@master/svg/color";

const SYMBOL_OVERRIDES: Record<string, string> = {
  KAS: "generic",
};

interface CoinIconProps {
  symbol: string;
  size?: number;
}

export function CoinIcon({ symbol, size = 20 }: CoinIconProps) {
  const key = symbol.toUpperCase();
  const file = SYMBOL_OVERRIDES[key] ?? symbol.toLowerCase();
  const src = `${ICON_CDN}/${file}.svg`;

  return (
    <img
      src={src}
      alt={symbol}
      width={size}
      height={size}
      className="rounded-full"
      onError={(e) => {
        const target = e.currentTarget;
        if (!target.dataset.fallback) {
          target.dataset.fallback = "1";
          target.src = `${ICON_CDN}/generic.svg`;
        }
      }}
    />
  );
}
