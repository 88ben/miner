interface StatCardProps {
  label: string;
  value: string;
  sub?: string;
  color?: string;
}

export function StatCard({ label, value, sub, color = "text-white" }: StatCardProps) {
  return (
    <div className="rounded-xl bg-[var(--color-bg-card)] border border-[var(--color-border)] p-5 flex flex-col gap-1">
      <span className="text-xs uppercase tracking-wider text-[var(--color-text-secondary)]">
        {label}
      </span>
      <span className={`text-2xl font-bold ${color}`}>{value}</span>
      {sub && (
        <span className="text-xs text-[var(--color-text-secondary)]">{sub}</span>
      )}
    </div>
  );
}
