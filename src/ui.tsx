import { createContext, memo, useCallback, useContext, useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import { CircleAlert, CircleCheck } from "lucide-react";
import type { LiveStats, Risk } from "./api";
import { backend } from "./mock";
import { isVisible, onVisibility } from "./visibility";

// ---------- async data ----------

export function useLoad<T>(fn: () => Promise<T>, deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const fnRef = useRef(fn);
  fnRef.current = fn;
  const reload = useCallback(async () => {
    setLoading(true);
    try {
      setData(await fnRef.current());
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { void reload(); }, deps);
  return { data, error, loading, reload, setData };
}

/**
 * Runs `tick` every `ms` while the window is on screen. Polling stops when the
 * app is minimised or hidden in the tray (so Vektor costs nothing while the
 * game runs) and resumes with an immediate tick. Ticks never overlap.
 */
function startPolling(tick: () => Promise<void>, ms: number) {
  let timer: ReturnType<typeof setInterval> | null = null;
  let running = false;
  const run = async () => {
    if (running) return;
    running = true;
    try {
      await tick();
    } finally {
      running = false;
    }
  };
  const sync = () => {
    if (isVisible() && timer === null) {
      void run();
      timer = setInterval(run, ms);
    } else if (!isVisible() && timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  };
  const off = onVisibility(sync);
  sync();
  return () => {
    off();
    if (timer !== null) clearInterval(timer);
  };
}

export function usePoll<T>(fn: () => Promise<T>, ms: number) {
  const [data, setData] = useState<T | null>(null);
  const fnRef = useRef(fn);
  useEffect(() => { fnRef.current = fn; });
  useEffect(() => {
    let alive = true;
    const stop = startPolling(async () => {
      try {
        const v = await fnRef.current();
        if (alive) setData(v);
      } catch {
        /* keep last value */
      }
    }, ms);
    return () => { alive = false; stop(); };
  }, [ms]);
  return [data, setData] as const;
}

// One shared poll for CPU/RAM, however many components show it.
const live = {
  value: null as LiveStats | null,
  subs: new Set<() => void>(),
  stop: null as (() => void) | null,
};
function subscribeLive(cb: () => void) {
  live.subs.add(cb);
  if (!live.stop) {
    live.stop = startPolling(async () => {
      try {
        live.value = await backend.liveStats();
        live.subs.forEach((s) => s());
      } catch {
        /* keep last value */
      }
    }, 2000);
  }
  return () => {
    live.subs.delete(cb);
    if (live.subs.size === 0 && live.stop) {
      live.stop();
      live.stop = null;
    }
  };
}
export const useLiveStats = () => useSyncExternalStore(subscribeLive, () => live.value);

// ---------- toasts ----------

type Toast = { id: number; kind: "ok" | "err"; text: string };
const ToastCtx = createContext<(kind: Toast["kind"], text: string) => void>(() => {});
export const useToast = () => useContext(ToastCtx);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const push = useCallback((kind: Toast["kind"], text: string) => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t.slice(-3), { id, kind, text }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), kind === "err" ? 7000 : 3500);
  }, []);
  return (
    <ToastCtx.Provider value={push}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {toasts.map((t) => (
          <div key={t.id} className={`toast ${t.kind}`}>
            {t.kind === "ok" ? <CircleCheck size={18} /> : <CircleAlert size={18} />}
            <span>{t.text}</span>
          </div>
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

// ---------- primitives ----------

export function PageHead({ eyebrow, title, desc, right }: { eyebrow: string; title: string; desc?: string; right?: ReactNode }) {
  return (
    <div className="page-head">
      <div>
        <div className="eyebrow">{eyebrow}</div>
        <h1>{title}</h1>
        {desc && <p>{desc}</p>}
      </div>
      {right}
    </div>
  );
}

export function Toggle({ on, busy, disabled, onChange, label }: { on: boolean; busy?: boolean; disabled?: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button type="button" role="switch" aria-checked={on} aria-label={label}
      className={`toggle ${on ? "on" : ""} ${busy ? "busy" : ""}`}
      disabled={disabled || busy} onClick={() => onChange(!on)} />
  );
}

const riskLabel: Record<Risk, string> = { low: "risco baixo", medium: "risco médio", high: "risco alto" };
export const RiskChip = ({ risk }: { risk: Risk }) => <span className={`chip ${risk}`}>{riskLabel[risk]}</span>;

export const Gauge = memo(function Gauge({ value, max = 100, label, unit = "%", size = 148, color = "var(--accent)" }: { value: number; max?: number; label: string; unit?: string; size?: number; color?: string }) {
  const stroke = 8;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const sweep = 0.75; // 270° arc
  const pct = max > 0 ? Math.max(0, Math.min(1, value / max)) : 0;
  return (
    <div className="gauge" style={{ width: size, height: size }}>
      <svg width={size} height={size}>
        <circle className="track" cx={size / 2} cy={size / 2} r={r} fill="none" strokeWidth={stroke}
          strokeDasharray={`${c * sweep} ${c}`} strokeLinecap="round" />
        <circle className="fill" cx={size / 2} cy={size / 2} r={r} fill="none" stroke={color} strokeWidth={stroke}
          strokeDasharray={`${c * sweep * pct} ${c}`} strokeLinecap="round" />
      </svg>
      <div className="center">
        <div className="value">{value.toFixed(0)}<span className="faint" style={{ fontSize: 13 }}>{unit}</span></div>
        <div className="label">{label}</div>
      </div>
    </div>
  );
});

export const fmtBytes = (b: number) => {
  const u = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (b >= 1024 && i < u.length - 1) { b /= 1024; i++; }
  return `${b.toFixed(i ? 1 : 0)} ${u[i]}`;
};

/** MB as a compact GB/MB string, e.g. 14320 -> "14.0 GB". */
export const fmtMb = (mb: number) => (mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`);
