// Typed bridge to the Rust backend (src-tauri/src/commands.rs).
// This file is the API CONTRACT: Rust DTOs use #[serde(rename_all = "camelCase")]
// and must match these shapes exactly.

import { invoke } from "@tauri-apps/api/core";

// ---------- system ----------
export interface CpuInfo {
  name: string;
  vendor: "amd" | "intel" | "other";
  cores: number;
  threads: number;
  /** 2,3,4,5 for AMD Zen generations when detectable */
  zenGen: number | null;
}
export interface GpuInfo {
  name: string;
  vendor: "nvidia" | "amd" | "intel" | "other";
  driverVersion: string | null;
  vramMb: number | null;
}
export interface SystemInfo {
  osName: string;
  osBuild: string;
  hostname: string;
  cpu: CpuInfo;
  gpus: GpuInfo[];
  ramTotalGb: number;
  isAdmin: boolean;
}
export interface LiveStats {
  cpuUsage: number; // 0..100
  ramUsedGb: number;
  ramTotalGb: number;
  processCount: number;
  cs2Running: boolean;
  /** game of the active Game Mode session */
  activeGame: string | null;
}

// ---------- tweaks ----------
export type TweakCategory = "power" | "gpu" | "system" | "network" | "input" | "privacy" | "game" | "services";
export type Risk = "low" | "medium" | "high";
export interface TweakInfo {
  id: string;
  category: TweakCategory;
  title: string;
  description: string;
  risk: Risk;
  requiresReboot: boolean;
  applied: boolean;
  available: boolean;
  /** why not available, if available === false */
  reason: string | null;
  /** asked before applying ("Usas impressora?"); never in the one-click boost */
  question: string | null;
}
export interface CleanReport {
  freedBytes: number;
  filesRemoved: number;
  paths: string[];
}
export interface ProcInfo {
  name: string;
  pid: number;
  memoryMb: number;
  /** friendly label e.g. "Discord" */
  label: string;
}

// ---------- cs2 ----------
export interface SteamUser {
  steamId64: string;
  accountId: number;
  personaName: string;
  mostRecent: boolean;
}
export interface Cs2Install {
  steamPath: string | null;
  gamePath: string | null;
  installed: boolean;
  running: boolean;
  users: SteamUser[];
  /** current launch options for app 730 of the most recent user */
  launchOptions: string | null;
  /** Steam marked CS2 as needing an update (appmanifest StateFlags) */
  updateRequired: boolean;
  buildId: string | null;
}
export interface VideoSettings {
  width: number | null;
  height: number | null;
  refreshRate: number | null;
  /** raw cs2_video.txt key -> value overrides, e.g. "setting.msaa_samples": "0" */
  values: Record<string, string>;
}
export interface Cs2Config {
  /** convar -> value written to vektor.cfg, e.g. { "fps_max": "0" } */
  convars: Record<string, string>;
  /** key -> command, e.g. { "mwheeldown": "+jump" } */
  binds: Record<string, string>;
  video: VideoSettings;
  /** optional key that re-execs vektor.cfg in game */
  reloadKey: string | null;
}
export interface ConfigPreset {
  id: string;
  name: string;
  description: string;
  tag: "fps" | "competitive" | "visibility" | "lowend";
  config: Cs2Config;
}
export interface BenchStatus {
  state: "idle" | "waiting-for-game" | "running" | "done" | "failed";
  avgFps: number | null;
  p1Fps: number | null;
  message: string | null;
}

// ---------- cleanup ----------
export interface CleanTarget {
  id: string;
  title: string;
  description: string;
  /** included in the default selection */
  recommended: boolean;
  /** shown before cleaning, e.g. side effects */
  warning: string | null;
  sizeBytes: number;
  files: number;
  available: boolean;
}

// ---------- memory (standby list cleaner, ISLC-style) ----------
export interface MemoryConfig {
  enabled: boolean;
  /** purge when the standby list is at least this big (MB) */
  standbyThresholdMb: number;
  /** ...and free memory is below this (MB) */
  freeThresholdMb: number;
  /** polling interval in seconds */
  intervalSec: number;
}
export interface MemoryStatus {
  serviceInstalled: boolean;
  serviceRunning: boolean;
  config: MemoryConfig;
  totalMb: number;
  freeMb: number;
  standbyMb: number;
  modifiedMb: number;
  /** purges done by the service since it started */
  purgeCount: number;
  /** ISO timestamp of the last purge by the service */
  lastPurge: string | null;
  /** ISO timestamp of the service's last evaluation (heartbeat) */
  lastCheck: string | null;
  /** why the service could not read memory or purge, if it failed */
  lastError: string | null;
}

// ---------- nvidia (Profile Inspector, global "Base Profile") ----------
export interface NvGlobalOptions {
  /** Power management mode: prefer maximum performance */
  maxPerformancePower: boolean;
  /** Low latency mode On (max pre-rendered frames = 1) */
  lowLatency: boolean;
  /** Texture filtering quality: high performance (+ trilinear/aniso optimizations) */
  textureHighPerformance: boolean;
  /** Shader cache on with unlimited size */
  unlimitedShaderCache: boolean;
}
export interface NvCs2Options {
  lowLatency: boolean;
  maxPerformancePower: boolean;
  /** force V-Sync off for cs2.exe */
  vsyncOff: boolean;
  textureHighPerformance: boolean;
}
export interface NvidiaStatus {
  nvidiaGpu: boolean;
  inspectorAvailable: boolean;
  applied: boolean;
  /** CS2-specific profile applied */
  cs2Applied: boolean;
  inspectorVersion: string | null;
}

// ---------- game session ("modo jogo": boost while CS2 runs) ----------
export interface DisplayMode {
  width: number;
  height: number;
  refresh: number;
}
export interface DisplayInfo {
  /** e.g. "\\.\DISPLAY1" */
  device: string;
  name: string;
  primary: boolean;
  current: DisplayMode;
  modes: DisplayMode[];
}
export interface SessionGame {
  /** executable file name, lowercase */
  exe: string;
  name: string;
  /** apply resolution and vibrance to this game */
  display: boolean;
}
export interface SessionConfig {
  /** apply the session automatically whenever a watched game starts */
  enabled: boolean;
  /** process names (from the background-apps allowlist) closed during the game */
  closeApps: string[];
  /** reopen closed apps when the game exits */
  reopenApps: boolean;
  affinity: "off" | "auto" | "custom";
  /** hex mask like "0xFF" when affinity === "custom" */
  affinityMask: string | null;
  /** resolution for the primary display while playing; null = unchanged */
  resolution: DisplayMode | null;
  /** digital vibrance 0..100 while the game has focus; null = off (50 = driver default) */
  vibrance: number | null;
  /** games that start a session */
  games: SessionGame[];
}
export interface CcdGroup {
  id: number;
  l3Mb: number;
  mask: string;
  logicalCount: number;
}
export interface CpuTopology {
  logicalCount: number;
  groups: CcdGroup[];
  recommendedMask: string;
  /** PT-PT explanation, e.g. "CCD com 3D V-Cache" */
  recommendedLabel: string;
}
export interface SessionStatus {
  active: boolean;
  /** ISO timestamp */
  since: string | null;
  pid: number | null;
  /** name of the game of the current session */
  game: string | null;
  /** PT-PT list of what the current session did */
  actions: string[];
  config: SessionConfig;
}
export interface VibranceInfo {
  supported: boolean;
  vendor: "nvidia" | "amd" | "other";
  /** current level 0..100 on the primary display */
  current: number | null;
}

// ---------- game library ----------
export type GameSource = "steam" | "epic" | "gog" | "ubisoft" | "riot" | "manual";
export type GameTweakKind = "priority" | "gpu" | "fso";
export interface GameInfo {
  id: string;
  name: string;
  source: GameSource;
  installDir: string | null;
  /** chosen executable (full path) */
  exe: string | null;
  /** likely executables, best first */
  candidates: string[];
  tweaks: { priority: boolean; gpu: boolean; fso: boolean };
  /** watched by Game Mode */
  inSession: boolean;
  running: boolean;
}
export interface RunningProcess {
  name: string;
  exe: string;
  memoryMb: number;
}

// ---------- restore points & journal ----------
export interface RestorePoint {
  description: string;
  /** ISO timestamp */
  date: string;
}
export interface RestorePointResult {
  created: boolean;
  message: string;
  last: RestorePoint | null;
}
export interface Change {
  id: string;
  title: string;
}

// ---------- startup apps ----------
export interface StartupEntry {
  id: string;
  name: string;
  command: string;
  location: string;
  enabled: boolean;
  changedByVektor: boolean;
  protected: boolean;
  hint: string | null;
}

// ---------- network ----------
export interface DnsResult {
  /** provider id, or "current" */
  id: string;
  name: string;
  servers: string[];
  latencyMs: number | null;
  /** 0..1 share of queries answered */
  success: number;
}
export interface DnsStatus {
  current: string[];
  applied: string | null;
}

// ---------- benchmarks ----------
export interface BenchRecord {
  id: string;
  /** ISO timestamp */
  date: string;
  avgFps: number;
  p1Fps: number;
  /** tweaks applied at the time */
  appliedTweaks: number;
  sessionActive: boolean;
  note: string | null;
}

// ---------- app ----------
export interface AppSettings {
  /** start with Windows (scheduled task, elevated, no UAC prompt) */
  autostart: boolean;
  startMinimized: boolean;
  /** closing the window keeps Vektor in the tray (needed for the game session watcher) */
  closeToTray: boolean;
  /** look for new versions on GitHub at startup and every few hours */
  autoUpdate: boolean;
}

// ---------- updates (GitHub releases, signed) ----------
export interface UpdateInfo {
  currentVersion: string;
  available: boolean;
  /** new version when available */
  version: string | null;
  /** release notes (markdown) */
  notes: string | null;
  /** ISO timestamp of the release */
  date: string | null;
}
/** Payload of the `vektor://update-progress` event while downloading. */
export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

export const api = {
  systemInfo: () => invoke<SystemInfo>("system_info"),
  liveStats: () => invoke<LiveStats>("live_stats"),

  listTweaks: () => invoke<TweakInfo[]>("list_tweaks"),
  applyTweak: (id: string) => invoke<TweakInfo>("apply_tweak", { id }),
  revertTweak: (id: string) => invoke<TweakInfo>("revert_tweak", { id }),
  revertAll: () => invoke<void>("revert_all"),
  cleanScan: () => invoke<CleanTarget[]>("clean_scan"),
  cleanRun: (ids: string[]) => invoke<CleanReport>("clean_run", { ids }),
  listBackgroundApps: () => invoke<ProcInfo[]>("list_background_apps"),
  killBackgroundApps: (names: string[]) => invoke<number>("kill_background_apps", { names }),

  cs2Detect: () => invoke<Cs2Install>("cs2_detect"),
  cs2Presets: () => invoke<ConfigPreset[]>("cs2_presets"),
  cs2ReadConfig: (accountId: number) => invoke<Cs2Config>("cs2_read_config", { accountId }),
  cs2ApplyConfig: (accountId: number, config: Cs2Config) =>
    invoke<void>("cs2_apply_config", { accountId, config }),
  cs2Launch: (args: string[]) => invoke<void>("cs2_launch", { args }),
  cs2RecommendedLaunchOptions: () => invoke<string>("cs2_recommended_launch_options"),
  cs2BenchmarkStart: () => invoke<void>("cs2_benchmark_start"),
  cs2BenchmarkPoll: () => invoke<BenchStatus>("cs2_benchmark_poll"),

  memoryStatus: () => invoke<MemoryStatus>("memory_status"),
  memorySetConfig: (config: MemoryConfig) => invoke<MemoryStatus>("memory_set_config", { config }),
  memoryInstallService: () => invoke<MemoryStatus>("memory_install_service"),
  memoryUninstallService: () => invoke<MemoryStatus>("memory_uninstall_service"),
  memoryPurgeNow: () => invoke<MemoryStatus>("memory_purge_now"),

  nvidiaStatus: () => invoke<NvidiaStatus>("nvidia_status"),
  nvidiaApplyGlobal: (options: NvGlobalOptions) => invoke<NvidiaStatus>("nvidia_apply_global", { options }),
  nvidiaRevertGlobal: () => invoke<NvidiaStatus>("nvidia_revert_global"),
  nvidiaApplyCs2: (options: NvCs2Options) => invoke<NvidiaStatus>("nvidia_apply_cs2", { options }),
  nvidiaRevertCs2: () => invoke<NvidiaStatus>("nvidia_revert_cs2"),

  sessionStatus: () => invoke<SessionStatus>("session_status"),
  sessionSetConfig: (config: SessionConfig) => invoke<SessionStatus>("session_set_config", { config }),
  sessionTopology: () => invoke<CpuTopology>("session_topology"),
  sessionStop: () => invoke<SessionStatus>("session_stop"),
  displayList: () => invoke<DisplayInfo[]>("display_list"),
  vibranceInfo: () => invoke<VibranceInfo>("vibrance_info"),
  vibrancePreview: (level: number) => invoke<VibranceInfo>("vibrance_preview", { level }),
  vibranceReset: () => invoke<VibranceInfo>("vibrance_reset"),

  benchHistory: () => invoke<BenchRecord[]>("bench_history"),
  benchDelete: (id: string) => invoke<BenchRecord[]>("bench_delete", { id }),
  benchSetNote: (id: string, note: string) => invoke<BenchRecord[]>("bench_set_note", { id, note }),

  gamesList: (refresh: boolean) => invoke<GameInfo[]>("games_list", { refresh }),
  gamesSetExe: (id: string, exe: string) => invoke<GameInfo>("games_set_exe", { id, exe }),
  gamesAdd: (exe: string, name: string | null) => invoke<GameInfo>("games_add", { exe, name }),
  gamesRemove: (id: string) => invoke<void>("games_remove", { id }),
  gamesUnhideAll: () => invoke<void>("games_unhide_all"),
  gamesSetTweak: (id: string, kind: GameTweakKind, on: boolean) => invoke<GameInfo>("games_set_tweak", { id, kind, on }),
  gamesOptimize: (id: string, on: boolean) => invoke<GameInfo>("games_optimize", { id, on }),
  gamesSetSession: (id: string, on: boolean) => invoke<GameInfo>("games_set_session", { id, on }),
  gamesRunningProcesses: () => invoke<RunningProcess[]>("games_running_processes"),

  restorePointLast: () => invoke<RestorePoint | null>("restore_point_last"),
  restorePointCreate: (enableProtection: boolean) => invoke<RestorePointResult>("restore_point_create", { enableProtection }),
  listChanges: () => invoke<Change[]>("list_changes"),
  revertChange: (id: string) => invoke<void>("revert_change", { id }),

  startupList: () => invoke<StartupEntry[]>("startup_list"),
  startupSetEnabled: (id: string, enabled: boolean) => invoke<StartupEntry[]>("startup_set_enabled", { id, enabled }),

  dnsTest: () => invoke<DnsResult[]>("dns_test"),
  dnsStatus: () => invoke<DnsStatus>("dns_status"),
  dnsApply: (provider: string) => invoke<DnsStatus>("dns_apply", { provider }),
  dnsRevert: () => invoke<DnsStatus>("dns_revert"),

  appSettings: () => invoke<AppSettings>("app_settings"),
  appSetSettings: (settings: AppSettings) => invoke<AppSettings>("app_set_settings", { settings }),
  exportDiagnostics: () => invoke<string>("export_diagnostics"),

  updateCheck: () => invoke<UpdateInfo>("update_check"),
  /** Downloads and runs the installer; the app restarts on its own. */
  updateInstall: () => invoke<void>("update_install"),
};

/** True when running inside Tauri (false in a plain browser preview). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export const AUTHOR_URL = "https://github.com/PedroGrilo";
export const PROJECT_URL = "https://github.com/PedroGrilo/vektor";

/** Open a link in the user's browser. Only the URLs allowed in capabilities/default.json work in the app. */
export const openUrl = (url: string): Promise<void> => {
  if (inTauri) return invoke<void>("plugin:opener|open_url", { url });
  window.open(url, "_blank", "noopener");
  return Promise.resolve();
};
