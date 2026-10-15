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

// ---------- gpu ----------
export interface DisplayMode {
  width: number;
  height: number;
  refresh: number;
}
export interface DisplayInfo {
  device: string; // "\\.\DISPLAY1"
  name: string;
  primary: boolean;
  current: DisplayMode;
  modes: DisplayMode[];
}
export interface GpuStatus {
  vendor: "nvidia" | "amd" | "intel" | "other";
  nvapiAvailable: boolean;
  adlAvailable: boolean;
  /** 0..100, null if unknown */
  vibrance: number | null;
  vibranceAuto: boolean;
  nvidiaProfileApplied: boolean;
  displays: DisplayInfo[];
}
export interface NvPreset {
  lowLatency: "off" | "on" | "ultra";
  maxPreRenderedFrames: number; // 1..4
  powerMode: "optimal" | "max-performance";
  vsync: "app" | "off";
  textureFiltering: "quality" | "performance" | "high-performance";
  shaderCache: "default" | "unlimited";
  threadedOptimization: "auto" | "on" | "off";
}

// ---------- ryzen ----------
export interface RyzenStatus {
  supported: boolean;
  reason: string | null;
  codename: string | null;
  pawnioInstalled: boolean;
  /** current all-core Curve Optimizer offset if readable */
  currentOffset: number | null;
  /** offset Vektor applied this boot (CO resets on reboot) */
  appliedOffset: number | null;
  minOffset: number; // e.g. -30
  maxOffset: number; // 0
}

export const api = {
  systemInfo: () => invoke<SystemInfo>("system_info"),
  liveStats: () => invoke<LiveStats>("live_stats"),

  listTweaks: () => invoke<TweakInfo[]>("list_tweaks"),
  applyTweak: (id: string) => invoke<TweakInfo>("apply_tweak", { id }),
  revertTweak: (id: string) => invoke<TweakInfo>("revert_tweak", { id }),
  revertAll: () => invoke<void>("revert_all"),
  cleanShaderCache: () => invoke<CleanReport>("clean_shader_cache"),
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

  gpuStatus: () => invoke<GpuStatus>("gpu_status"),
  gpuApplyNvidiaProfile: (preset: NvPreset) => invoke<void>("gpu_apply_nvidia_profile", { preset }),
  gpuSetVibrance: (level: number) => invoke<void>("gpu_set_vibrance", { level }),
  gpuVibranceAuto: (enabled: boolean, level: number) =>
    invoke<void>("gpu_vibrance_auto", { enabled, level }),
  gpuSetDisplayMode: (device: string, width: number, height: number, refresh: number) =>
    invoke<void>("gpu_set_display_mode", { device, width, height, refresh }),

  ryzenStatus: () => invoke<RyzenStatus>("ryzen_status"),
  ryzenApplyCurve: (offset: number) => invoke<RyzenStatus>("ryzen_apply_curve", { offset }),
  ryzenClearCurve: () => invoke<RyzenStatus>("ryzen_clear_curve"),
};

/** True when running inside Tauri (false in a plain browser preview). */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
