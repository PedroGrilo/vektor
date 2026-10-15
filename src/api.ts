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
}

// ---------- tweaks ----------
export type TweakCategory = "power" | "gpu" | "system" | "network" | "input" | "privacy" | "game";
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
export interface NvidiaStatus {
  nvidiaGpu: boolean;
  inspectorAvailable: boolean;
  applied: boolean;
  inspectorVersion: string | null;
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
};

/** True when running inside Tauri (false in a plain browser preview). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
