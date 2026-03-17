export type PowerState = "off" | "loading" | "on" | "disabled" | "error";

interface PowerButtonProps {
  state: PowerState;
  onClick: () => void;
  symbol: string;
  errorMsg?: string;
}

export function PowerButton({ state, onClick, symbol, errorMsg }: PowerButtonProps) {
  const config: Record<
    PowerState,
    { title: string; color: string; hoverColor: string; cursor: string }
  > = {
    off: {
      title: `Start ${symbol} miner`,
      color: "text-[var(--color-text-secondary)]",
      hoverColor: "hover:text-[var(--color-success)] hover:bg-[var(--color-success)]/10",
      cursor: "cursor-pointer",
    },
    loading: {
      title: `${symbol} miner starting...`,
      color: "text-[var(--color-accent)]",
      hoverColor: "",
      cursor: "cursor-wait",
    },
    on: {
      title: `Stop ${symbol} miner`,
      color: "text-[var(--color-success)]",
      hoverColor: "hover:text-[var(--color-danger)] hover:bg-[var(--color-danger)]/10",
      cursor: "cursor-pointer",
    },
    disabled: {
      title: `${symbol} not configured — set wallet & pool first`,
      color: "text-[var(--color-text-secondary)] opacity-30",
      hoverColor: "",
      cursor: "cursor-not-allowed",
    },
    error: {
      title: errorMsg || `${symbol} failed to start`,
      color: "text-[var(--color-danger)]",
      hoverColor: "hover:text-[var(--color-danger)] hover:bg-[var(--color-danger)]/10",
      cursor: "cursor-pointer",
    },
  };

  const c = config[state];
  const isClickable = state === "off" || state === "on" || state === "error";

  return (
    <button
      onClick={isClickable ? onClick : undefined}
      disabled={!isClickable}
      className={`p-1.5 rounded-md transition-colors ${c.color} ${c.hoverColor} ${c.cursor}
                  disabled:pointer-events-none`}
      title={c.title}
    >
      {state === "loading" ? <SpinnerIcon /> : null}
      {state === "off" ? <PowerOnIcon /> : null}
      {state === "on" ? <PowerOffIcon /> : null}
      {state === "disabled" ? <PowerDisabledIcon /> : null}
      {state === "error" ? <ErrorIcon /> : null}
    </button>
  );
}

function PowerOnIcon() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
      <path
        fillRule="evenodd"
        d="M10 1a.75.75 0 01.75.75v6.5a.75.75 0 01-1.5 0v-6.5A.75.75 0 0110 1zM5.404 4.343a.75.75 0 010 1.06 6.5 6.5 0 109.192 0 .75.75 0 111.06-1.06 8 8 0 11-11.312 0 .75.75 0 011.06 0z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function PowerOffIcon() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
      <path
        fillRule="evenodd"
        d="M10 1a.75.75 0 01.75.75v6.5a.75.75 0 01-1.5 0v-6.5A.75.75 0 0110 1zM5.404 4.343a.75.75 0 010 1.06 6.5 6.5 0 109.192 0 .75.75 0 111.06-1.06 8 8 0 11-11.312 0 .75.75 0 011.06 0z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function PowerDisabledIcon() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
      <path
        fillRule="evenodd"
        d="M10 1a.75.75 0 01.75.75v6.5a.75.75 0 01-1.5 0v-6.5A.75.75 0 0110 1zM5.404 4.343a.75.75 0 010 1.06 6.5 6.5 0 109.192 0 .75.75 0 111.06-1.06 8 8 0 11-11.312 0 .75.75 0 011.06 0z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function ErrorIcon() {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" className="w-4 h-4">
      <path
        fillRule="evenodd"
        d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-8-5a.75.75 0 01.75.75v4.5a.75.75 0 01-1.5 0v-4.5A.75.75 0 0110 5zm0 10a1 1 0 100-2 1 1 0 000 2z"
        clipRule="evenodd"
      />
    </svg>
  );
}

function SpinnerIcon() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      className="w-4 h-4 animate-spin"
    >
      <circle
        className="opacity-25"
        cx="12"
        cy="12"
        r="10"
        stroke="currentColor"
        strokeWidth="4"
      />
      <path
        className="opacity-75"
        fill="currentColor"
        d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
      />
    </svg>
  );
}
