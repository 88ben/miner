import { useEffect, useState } from "react";
import type { CoinEntry } from "../types";

interface EditWorkerDialogProps {
  entry: CoinEntry;
  index: number;
  onSave: (index: number, entry: CoinEntry) => void;
  onClose: () => void;
}

export function EditWorkerDialog({
  entry,
  index,
  onSave,
  onClose,
}: EditWorkerDialogProps) {
  const [form, setForm] = useState<CoinEntry>({ ...entry });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setForm({ ...entry });
  }, [entry]);

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.target === e.currentTarget) onClose();
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      onSave(index, form);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={handleBackdropClick}
    >
      <div className="w-full max-w-md rounded-xl bg-[var(--color-bg-card)] border border-[var(--color-border)] shadow-2xl">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-[var(--color-border)]">
          <h3 className="text-lg font-semibold">
            Edit {form.symbol} Worker
          </h3>
          <button
            onClick={onClose}
            className="text-[var(--color-text-secondary)] hover:text-white transition-colors text-xl leading-none"
          >
            &times;
          </button>
        </div>

        {/* Form */}
        <div className="px-6 py-5 space-y-4">
          <Field label="Symbol" value={form.symbol} disabled />

          <Field label="Algorithm" value={form.algorithm} disabled />

          <Field
            label="Wallet Address"
            value={form.wallet}
            onChange={(v) => setForm({ ...form, wallet: v })}
            placeholder="Your wallet address"
          />

          <Field
            label="Pool URL"
            value={form.pool.url}
            onChange={(v) =>
              setForm({ ...form, pool: { ...form.pool, url: v } })
            }
            placeholder="pool.example.com"
          />

          <div className="grid grid-cols-2 gap-3">
            <Field
              label="Pool Port"
              value={String(form.pool.port)}
              onChange={(v) =>
                setForm({
                  ...form,
                  pool: { ...form.pool, port: parseInt(v) || 0 },
                })
              }
              type="number"
            />
            <Field
              label="Threads"
              value={form.threads != null ? String(form.threads) : ""}
              onChange={(v) =>
                setForm({
                  ...form,
                  threads: v === "" ? null : parseInt(v) || null,
                })
              }
              type="number"
              placeholder="auto"
            />
          </div>

          <div className="flex items-center gap-3">
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={form.pool.tls}
                onChange={(e) =>
                  setForm({
                    ...form,
                    pool: { ...form.pool, tls: e.target.checked },
                  })
                }
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-[var(--color-bg-secondary)] rounded-full peer
                            peer-checked:bg-[var(--color-accent)] transition-colors
                            after:content-[''] after:absolute after:top-0.5 after:left-[2px]
                            after:bg-white after:rounded-full after:h-4 after:w-4
                            after:transition-all peer-checked:after:translate-x-full" />
            </label>
            <span className="text-sm text-[var(--color-text-secondary)]">
              TLS
            </span>
          </div>

          <div className="flex items-center gap-3">
            <label className="relative inline-flex items-center cursor-pointer">
              <input
                type="checkbox"
                checked={form.enabled}
                onChange={(e) =>
                  setForm({ ...form, enabled: e.target.checked })
                }
                className="sr-only peer"
              />
              <div className="w-9 h-5 bg-[var(--color-bg-secondary)] rounded-full peer
                            peer-checked:bg-[var(--color-success)] transition-colors
                            after:content-[''] after:absolute after:top-0.5 after:left-[2px]
                            after:bg-white after:rounded-full after:h-4 after:w-4
                            after:transition-all peer-checked:after:translate-x-full" />
            </label>
            <span className="text-sm text-[var(--color-text-secondary)]">
              Enabled
            </span>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-3 px-6 py-4 border-t border-[var(--color-border)]">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm rounded-lg border border-[var(--color-border)]
                       text-[var(--color-text-secondary)] hover:text-white hover:border-[var(--color-text-secondary)]
                       transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={saving}
            className="px-4 py-2 text-sm rounded-lg bg-[var(--color-accent)] text-white font-medium
                       hover:brightness-110 disabled:opacity-50 transition-all"
          >
            {saving ? "Saving..." : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  placeholder,
  disabled,
  type = "text",
}: {
  label: string;
  value: string;
  onChange?: (v: string) => void;
  placeholder?: string;
  disabled?: boolean;
  type?: string;
}) {
  return (
    <div>
      <label className="block text-xs uppercase tracking-wider text-[var(--color-text-secondary)] mb-1.5">
        {label}
      </label>
      <input
        type={type}
        value={value}
        onChange={(e) => onChange?.(e.target.value)}
        placeholder={placeholder}
        disabled={disabled}
        className="w-full px-3 py-2 text-sm rounded-lg
                   bg-[var(--color-bg-secondary)] border border-[var(--color-border)]
                   text-white placeholder-[var(--color-text-secondary)]
                   focus:outline-none focus:border-[var(--color-accent)] focus:ring-1 focus:ring-[var(--color-accent)]
                   disabled:opacity-50 disabled:cursor-not-allowed
                   transition-colors"
      />
    </div>
  );
}
