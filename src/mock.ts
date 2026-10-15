// In-browser preview data so the UI can be designed with `bun run dev` outside Tauri.
import { api, inTauri } from "./api";
import type { BenchStatus, ConfigPreset, Cs2Config, GpuStatus, NvPreset, TweakInfo } from "./api";

const wait = <T,>(v: T, ms = 350) => new Promise<T>((r) => setTimeout(() => r(structuredClone(v)), ms));

let tweaks: TweakInfo[] = [
  { id: "power-plan", category: "power", title: "Plano de energia Vektor", description: "Plano de desempenho máximo: sem estacionamento de núcleos nem suspensão seletiva de USB.", risk: "low", requiresReboot: false, applied: true, available: true, reason: null },
  { id: "game-mode", category: "system", title: "Modo de Jogo do Windows", description: "Dá prioridade ao jogo em primeiro plano e suspende atualizações em segundo plano.", risk: "low", requiresReboot: false, applied: false, available: true, reason: null },
  { id: "game-dvr-off", category: "system", title: "Desligar gravação Game DVR", description: "Remove a captura em segundo plano da Xbox Game Bar, que consome GPU.", risk: "low", requiresReboot: false, applied: false, available: true, reason: null },
  { id: "hags", category: "gpu", title: "Agendamento de GPU por hardware", description: "Reduz a latência de submissão de frames em GPUs suportadas.", risk: "medium", requiresReboot: true, applied: false, available: true, reason: null },
  { id: "mouse-accel-off", category: "input", title: "Desligar aceleração do rato", description: "Movimento 1:1, essencial para pontaria consistente.", risk: "low", requiresReboot: false, applied: true, available: true, reason: null },
  { id: "network-throttling-off", category: "network", title: "Sem limitação de rede multimédia", description: "Desativa o NetworkThrottlingIndex do MMCSS.", risk: "low", requiresReboot: false, applied: false, available: true, reason: null },
  { id: "cs2-priority", category: "game", title: "CS2 com prioridade alta", description: "O Windows arranca sempre o cs2.exe com prioridade de CPU alta.", risk: "low", requiresReboot: false, applied: false, available: true, reason: null },
  { id: "cs2-fso-off", category: "game", title: "Sem otimizações de ecrã inteiro (CS2)", description: "Força ecrã inteiro exclusivo verdadeiro.", risk: "low", requiresReboot: false, applied: false, available: false, reason: "CS2 não encontrado" },
  { id: "telemetry-off", category: "privacy", title: "Desligar telemetria (DiagTrack)", description: "Para o serviço de telemetria do Windows.", risk: "low", requiresReboot: false, applied: false, available: true, reason: null },
];

const presetConfig = (fps: string): Cs2Config => ({
  convars: { fps_max: fps, engine_low_latency_sleep_after_client_tick: "true", r_fullscreen_gamma: "2.2" },
  binds: { mwheeldown: "+jump" },
  video: { width: 1920, height: 1080, refreshRate: 240, values: { "setting.msaa_samples": "0" } },
  reloadKey: "F5",
});
const presets: ConfigPreset[] = [
  { id: "max-fps", name: "FPS Máximo", description: "Tudo no mínimo, sombras essenciais ligadas.", tag: "fps", config: presetConfig("0") },
  { id: "competitive", name: "Competitivo", description: "Equilíbrio entre visibilidade e fluidez.", tag: "competitive", config: presetConfig("400") },
  { id: "visibility", name: "Visibilidade", description: "Gamma e contraste pensados para ver inimigos.", tag: "visibility", config: presetConfig("300") },
  { id: "low-end", name: "PC Modesto", description: "Para portáteis e GPUs integradas.", tag: "lowend", config: presetConfig("0") },
];

let bench: BenchStatus = { state: "idle", avgFps: null, p1Fps: null, message: null };
let gpu: GpuStatus = {
  vendor: "nvidia", nvapiAvailable: true, adlAvailable: false, vibrance: 50, vibranceAuto: false, nvidiaProfileApplied: false,
  displays: [{ device: "\\\\.\\DISPLAY1", name: "AOC 24G2", primary: true, current: { width: 1920, height: 1080, refresh: 144 },
    modes: [{ width: 1920, height: 1080, refresh: 144 }, { width: 1280, height: 960, refresh: 144 }, { width: 1440, height: 1080, refresh: 144 }, { width: 1024, height: 768, refresh: 144 }] }],
};

const mock: typeof api = {
  systemInfo: () => wait({ osName: "Windows 11 Pro", osBuild: "26100", hostname: "CLIENTE-PC", cpu: { name: "AMD Ryzen 7 7800X3D 8-Core Processor", vendor: "amd", cores: 8, threads: 16, zenGen: 4 }, gpus: [{ name: "NVIDIA GeForce RTX 4070", vendor: "nvidia", driverVersion: "32.0.15.6614", vramMb: 12288 }], ramTotalGb: 32, isAdmin: true }),
  liveStats: () => wait({ cpuUsage: 8 + Math.random() * 20, ramUsedGb: 11.4 + Math.random(), ramTotalGb: 32, processCount: 212, cs2Running: false }, 120),
  listTweaks: () => wait(tweaks),
  applyTweak: (id) => { tweaks = tweaks.map((t) => (t.id === id ? { ...t, applied: true } : t)); return wait(tweaks.find((t) => t.id === id)!, 600); },
  revertTweak: (id) => { tweaks = tweaks.map((t) => (t.id === id ? { ...t, applied: false } : t)); return wait(tweaks.find((t) => t.id === id)!, 500); },
  revertAll: () => { tweaks = tweaks.map((t) => ({ ...t, applied: false })); return wait(undefined, 800); },
  cleanShaderCache: () => wait({ freedBytes: 1_284_000_000, filesRemoved: 4210, paths: ["D3DSCache", "NVIDIA\\DXCache", "shadercache\\730"] }, 1200),
  listBackgroundApps: () => wait([{ name: "Discord.exe", pid: 1, memoryMb: 412, label: "Discord" }, { name: "chrome.exe", pid: 2, memoryMb: 1830, label: "Google Chrome" }, { name: "Spotify.exe", pid: 3, memoryMb: 260, label: "Spotify" }]),
  killBackgroundApps: (names) => wait(names.length, 700),
  cs2Detect: () => wait({ steamPath: "C:\\Program Files (x86)\\Steam", gamePath: "D:\\SteamLibrary\\steamapps\\common\\Counter-Strike Global Offensive", installed: true, running: false, users: [{ steamId64: "76561198000000000", accountId: 39734272, personaName: "cliente", mostRecent: true }], launchOptions: "-novid" }),
  cs2Presets: () => wait(presets),
  cs2ReadConfig: () => wait(presets[1].config),
  cs2ApplyConfig: () => wait(undefined, 700),
  cs2Launch: () => wait(undefined),
  cs2RecommendedLaunchOptions: () => wait("-novid -nojoy +exec vektor"),
  cs2BenchmarkStart: () => { bench = { state: "running", avgFps: null, p1Fps: null, message: "A correr benchmark…" }; setTimeout(() => { bench = { state: "done", avgFps: 612.4, p1Fps: 248.9, message: null }; }, 5000); return wait(undefined); },
  cs2BenchmarkPoll: () => wait(bench, 100),
  gpuStatus: () => wait(gpu),
  gpuApplyNvidiaProfile: (_p: NvPreset) => { gpu = { ...gpu, nvidiaProfileApplied: true }; return wait(undefined, 700); },
  gpuSetVibrance: (level) => { gpu = { ...gpu, vibrance: level }; return wait(undefined, 100); },
  gpuVibranceAuto: (enabled) => { gpu = { ...gpu, vibranceAuto: enabled }; return wait(undefined, 100); },
  gpuSetDisplayMode: () => wait(undefined, 600),
  ryzenStatus: () => wait({ supported: false, reason: "Não disponível", codename: null, pawnioInstalled: false, currentOffset: null, appliedOffset: null, minOffset: 0, maxOffset: 0 }),
  ryzenApplyCurve: () => Promise.reject("Não disponível"),
  ryzenClearCurve: () => Promise.reject("Não disponível"),
};

export const backend: typeof api = inTauri ? api : mock;
