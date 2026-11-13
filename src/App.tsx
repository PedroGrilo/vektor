import { memo, useEffect, useState } from "react";
import { ArrowUpCircle, Crosshair, Gamepad2, Network as NetworkIcon, Power, Rocket, Settings as SettingsIcon, Gauge as GaugeIcon, History, MemoryStick, Minus, MonitorCog, Square, SlidersHorizontal, Trash2, X } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { inTauri } from "./api";
import { backend } from "./mock";
import { ToastProvider, useLiveStats } from "./ui";
import { startAutoUpdateChecks, useUpdate } from "./updates";
import Dashboard from "./pages/Dashboard";
import Optimize from "./pages/Optimize";
import Cs2 from "./pages/Cs2";
import Restore from "./pages/Restore";
import Cleanup from "./pages/Cleanup";
import Memory from "./pages/Memory";
import Nvidia from "./pages/Nvidia";
import GameMode from "./pages/GameMode";
import Games from "./pages/Games";
import StartupPage from "./pages/Startup";
import Network from "./pages/Network";
import SettingsPage from "./pages/Settings";

export type PageId = "dashboard" | "optimize" | "startup" | "cleanup" | "memory" | "network" | "nvidia" | "games" | "game" | "cs2" | "restore" | "settings";

const NAV: { id: PageId; label: string; icon: typeof GaugeIcon; section?: string }[] = [
  { id: "dashboard", label: "Painel", icon: GaugeIcon, section: "Sistema" },
  { id: "optimize", label: "Otimizações", icon: SlidersHorizontal },
  { id: "startup", label: "Arranque", icon: Power },
  { id: "cleanup", label: "Limpeza", icon: Trash2 },
  { id: "memory", label: "Memória", icon: MemoryStick },
  { id: "network", label: "Rede", icon: NetworkIcon },
  { id: "nvidia", label: "NVIDIA", icon: MonitorCog },
  { id: "games", label: "Jogos", icon: Gamepad2, section: "Jogo" },
  { id: "game", label: "Modo Jogo", icon: Rocket },
  { id: "cs2", label: "Counter-Strike 2", icon: Crosshair },
  { id: "restore", label: "Restaurar", icon: History, section: "Segurança" },
  { id: "settings", label: "Definições", icon: SettingsIcon },
];
const PAGE_IDS = new Set<string>(NAV.map((n) => n.id));

const TitleBar = memo(function TitleBar() {
  const win = inTauri ? getCurrentWindow() : null;
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="brand" data-tauri-drag-region>
        <Logo />
        VEKTOR <small>v{__APP_VERSION__}</small>
      </div>
      <div className="controls">
        <button aria-label="Minimizar" onClick={() => win?.minimize()}><Minus size={15} /></button>
        <button aria-label="Maximizar" onClick={() => win?.toggleMaximize()}><Square size={12} /></button>
        <button aria-label="Fechar" className="close" onClick={() => win?.close()}><X size={16} /></button>
      </div>
    </header>
  );
});

export function Logo({ size = 20 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" aria-hidden>
      <defs>
        <linearGradient id="lg" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#cfeee6" />
          <stop offset=".45" stopColor="#28c39d" />
          <stop offset="1" stopColor="#0f7a62" />
        </linearGradient>
      </defs>
      <path d="M3 4h4.2L12 15.5 16.8 4H21l-7.4 16h-3.2L3 4Z" fill="url(#lg)" />
      <path d="M9.5 4h5L12 10 9.5 4Z" fill="#0d0e10" opacity=".55" />
    </svg>
  );
}

// Isolated so the 2 s CPU/RAM tick re-renders these two lines, not the whole shell.
function SidebarStatus() {
  const stats = useLiveStats();
  return (
    <div className="status-card">
      <div className="row between" style={{ marginBottom: 6 }}>
        <span className="label">Estado</span>
        {stats?.activeGame ? <span className="chip on"><span className="dot" />{stats.activeGame}</span>
          : stats?.cs2Running ? <span className="chip on"><span className="dot" />CS2 ativo</span>
          : <span className="chip">em espera</span>}
      </div>
      <div>CPU <b>{stats ? `${stats.cpuUsage.toFixed(0)}%` : "—"}</b> · RAM <b>{stats ? `${stats.ramUsedGb.toFixed(1)} GB` : "—"}</b></div>
    </div>
  );
}

function UpdateBadge({ go }: { go: (p: PageId) => void }) {
  const up = useUpdate();
  if (up.phase !== "available" && up.phase !== "installing") return null;
  return (
    <button className="btn primary" style={{ marginBottom: 10 }} onClick={() => go("settings")}>
      <ArrowUpCircle size={15} />
      {up.phase === "installing" ? "A atualizar…" : `Atualizar para ${up.info?.version ?? ""}`}
    </button>
  );
}

const Sidebar = memo(function Sidebar({ page, go }: { page: PageId; go: (p: PageId) => void }) {
  return (
    <nav className="sidebar">
      {NAV.map((n) => (
        <div key={n.id}>
          {n.section && <div className="section">{n.section}</div>}
          <button className={`nav-item ${page === n.id ? "active" : ""}`} aria-current={page === n.id ? "page" : undefined} onClick={() => go(n.id)}>
            <n.icon size={18} />
            {n.label}
          </button>
        </div>
      ))}
      <div className="spacer" />
      <UpdateBadge go={go} />
      <SidebarStatus />
    </nav>
  );
});

function initialPage(): PageId {
  const hash = window.location.hash.slice(1);
  return PAGE_IDS.has(hash) ? (hash as PageId) : "dashboard";
}

export default function App() {
  const [page, setPage] = useState<PageId>(initialPage);

  useEffect(() => {
    // Respect the "check for updates" preference; default is on.
    backend.appSettings().then((s) => { if (s.autoUpdate) startAutoUpdateChecks(); }).catch(() => {});
  }, []);

  return (
    <ToastProvider>
      <div className="backdrop" />
      <div className="shell">
        <TitleBar />
        <div className="body">
          <Sidebar page={page} go={setPage} />
          {/* The key remounts the page so the CSS enter animation runs; no exit animation = instant navigation. */}
          <main key={page} className="page">
            {page === "dashboard" && <Dashboard go={setPage} />}
            {page === "optimize" && <Optimize />}
            {page === "cleanup" && <Cleanup />}
            {page === "memory" && <Memory />}
            {page === "nvidia" && <Nvidia />}
            {page === "game" && <GameMode go={setPage} />}
            {page === "games" && <Games />}
            {page === "startup" && <StartupPage />}
            {page === "network" && <Network />}
            {page === "cs2" && <Cs2 />}
            {page === "settings" && <SettingsPage />}
            {page === "restore" && <Restore />}
          </main>
        </div>
      </div>
    </ToastProvider>
  );
}
