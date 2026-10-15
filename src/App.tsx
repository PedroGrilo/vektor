import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";
import { Crosshair, Gauge as GaugeIcon, History, MemoryStick, Minus, MonitorCog, Square, SlidersHorizontal, Trash2, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { inTauri } from "./api";
import { backend } from "./mock";
import { ToastProvider, usePoll } from "./ui";
import Dashboard from "./pages/Dashboard";
import Optimize from "./pages/Optimize";
import Cs2 from "./pages/Cs2";
import Restore from "./pages/Restore";
import Cleanup from "./pages/Cleanup";
import Memory from "./pages/Memory";
import Nvidia from "./pages/Nvidia";

export type PageId = "dashboard" | "optimize" | "cleanup" | "memory" | "nvidia" | "cs2" | "restore";

const NAV: { id: PageId; label: string; icon: typeof GaugeIcon; section?: string }[] = [
  { id: "dashboard", label: "Painel", icon: GaugeIcon, section: "Sistema" },
  { id: "optimize", label: "Otimizações", icon: SlidersHorizontal },
  { id: "cleanup", label: "Limpeza", icon: Trash2 },
  { id: "memory", label: "Memória", icon: MemoryStick },
  { id: "nvidia", label: "NVIDIA", icon: MonitorCog },
  { id: "cs2", label: "Counter-Strike 2", icon: Crosshair, section: "Jogo" },
  { id: "restore", label: "Restaurar", icon: History, section: "Segurança" },
];

function TitleBar() {
  const win = inTauri ? getCurrentWindow() : null;
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="brand" data-tauri-drag-region>
        <Logo />
        VEKTOR <small>v0.1.0</small>
      </div>
      <div className="controls">
        <button aria-label="Minimizar" onClick={() => win?.minimize()}><Minus size={15} /></button>
        <button aria-label="Maximizar" onClick={() => win?.toggleMaximize()}><Square size={12} /></button>
        <button aria-label="Fechar" className="close" onClick={() => win?.close()}><X size={16} /></button>
      </div>
    </header>
  );
}

export function Logo({ size = 20 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden>
      <defs>
        <linearGradient id="lg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#67e8f9" />
          <stop offset="1" stopColor="#a78bfa" />
        </linearGradient>
      </defs>
      <path d="M3 4h4.2L12 15.5 16.8 4H21l-7.4 16h-3.2L3 4Z" fill="url(#lg)" />
      <path d="M9.5 4h5L12 10 9.5 4Z" fill="#05070d" opacity=".55" />
    </svg>
  );
}

function SidebarStatus() {
  const stats = usePoll(() => backend.liveStats(), 2500);
  return (
    <div className="status-card">
      <div className="row between" style={{ marginBottom: 6 }}>
        <span className="label">Estado</span>
        {stats?.cs2Running ? <span className="chip on"><span className="dot" />CS2 ativo</span> : <span className="chip">em espera</span>}
      </div>
      <div>CPU <b>{stats ? `${stats.cpuUsage.toFixed(0)}%` : "—"}</b> · RAM <b>{stats ? `${stats.ramUsedGb.toFixed(1)} GB` : "—"}</b></div>
    </div>
  );
}

export default function App() {
  const [page, setPage] = useState<PageId>(() => (window.location.hash.slice(1) as PageId) || "dashboard");
  return (
    <ToastProvider>
      <div className="backdrop">
        <div className="grid" />
        <div className="orb a" />
        <div className="orb b" />
        <div className="scan" />
      </div>
      <div className="shell">
        <TitleBar />
        <div className="body">
          <nav className="sidebar">
            {NAV.map((n) => (
              <div key={n.id}>
                {n.section && <div className="section">{n.section}</div>}
                <button className={`nav-item ${page === n.id ? "active" : ""}`} onClick={() => setPage(n.id)}>
                  <n.icon size={18} />
                  {n.label}
                </button>
              </div>
            ))}
            <div className="spacer" />
            <SidebarStatus />
          </nav>
          <AnimatePresence mode="wait">
            <motion.main key={page} className="page"
              initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -6 }}
              transition={{ duration: 0.22, ease: "easeOut" }}>
              {page === "dashboard" && <Dashboard go={setPage} />}
              {page === "optimize" && <Optimize />}
              {page === "cleanup" && <Cleanup />}
              {page === "memory" && <Memory />}
              {page === "nvidia" && <Nvidia />}
              {page === "cs2" && <Cs2 />}
              {page === "restore" && <Restore />}
            </motion.main>
          </AnimatePresence>
        </div>
      </div>
    </ToastProvider>
  );
}
