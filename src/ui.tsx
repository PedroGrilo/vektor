import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { AnimatePresence, motion } from "motion/react";
import { CircleAlert, CircleCheck } from "lucide-react";
import type { Risk } from "./api";

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

export function usePoll<T>(fn: () => Promise<T>, ms: number) {
  const [data, setData] = useState<T | null>(null);
  useEffect(() => {
    let alive = true;
    const tick = async () => {
      try {
        const v = await fn();
        if (alive) setData(v);
      } catch {
        /* keep last value */
      }
    };
    void tick();
    const id = setInterval(tick, ms);
    return () => { alive = false; clearInterval(id); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ms]);
  return data;
}

// ---------- toasts ----------

type Toast = { id: number; kind: "ok" | "err"; text: string };
const ToastCtx = createContext<(kind: Toast["kind"], text: string) => void>(() => {});
export const useToast = () => useContext(ToastCtx);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const push = useCallback((kind: Toast["kind"], text: string) => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t, { id, kind, text }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), kind === "err" ? 7000 : 3500);
  }, []);
  return (
    <ToastCtx.Provider value={push}>
      {children}
      <div className="toasts">
        <AnimatePresence>
          {toasts.map((t) => (
            <motion.div key={t.id} className={`toast ${t.kind}`}
              initial={{ opacity: 0, x: 40 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 40 }}>
              {t.kind === "ok" ? <CircleCheck size={18} /> : <CircleAlert size={18} />}
              <span>{t.text}</span>
            </motion.div>
          ))}
        </AnimatePresence>
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

export function Gauge({ value, max = 100, label, unit = "%", size = 148, color = "var(--cyan)" }: { value: number; max?: number; label: string; unit?: string; size?: number; color?: string }) {
  const stroke = 8;
  const r = (size - stroke) / 2;
  const c = 2 * Math.PI * r;
  const sweep = 0.75; // 270° arc
  const pct = Math.max(0, Math.min(1, value / max));
  return (
    <div className="gauge" style={{ width: size, height: size }}>
      <svg width={size} height={size}>
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke="rgba(255,255,255,0.06)" strokeWidth={stroke}
          strokeDasharray={`${c * sweep} ${c}`} strokeLinecap="round" />
        <circle cx={size / 2} cy={size / 2} r={r} fill="none" stroke={color} strokeWidth={stroke}
          strokeDasharray={`${c * sweep * pct} ${c}`} strokeLinecap="round"
          style={{ transition: "stroke-dasharray .8s cubic-bezier(.2,.8,.2,1)", filter: `drop-shadow(0 0 6px ${color})` }} />
      </svg>
      <div className="center">
        <div className="value">{value.toFixed(0)}<span className="faint" style={{ fontSize: 13 }}>{unit}</span></div>
        <div className="label">{label}</div>
      </div>
    </div>
  );
}

export const fmtBytes = (b: number) => {
  const u = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (b >= 1024 && i < u.length - 1) { b /= 1024; i++; }
  return `${b.toFixed(i ? 1 : 0)} ${u[i]}`;
};
