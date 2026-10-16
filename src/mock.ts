// In-browser preview data so the UI can be designed with `bun run dev` outside Tauri.
import { api, inTauri } from "./api";
import type { AppSettings, BenchRecord, BenchStatus, CleanTarget, ConfigPreset, Cs2Config, DisplayInfo, MemoryConfig, MemoryStatus, NvidiaStatus, SessionConfig, SessionStatus, TweakInfo, VibranceInfo } from "./api";

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

let benchSt: BenchStatus = { state: "idle", avgFps: null, p1Fps: null, message: null };
const MB = 1024 * 1024;
const cleanTargets: CleanTarget[] = [
  { id: "user-temp", title: "Temporários do utilizador", description: "%TEMP% — ficheiros com mais de 24 h.", recommended: true, warning: null, sizeBytes: 2140 * MB, files: 8312, available: true },
  { id: "windows-temp", title: "Temporários do Windows", description: "C:\\Windows\\Temp", recommended: true, warning: null, sizeBytes: 380 * MB, files: 912, available: true },
  { id: "shader-cache-drivers", title: "Cache de shaders das drivers", description: "DirectX, NVIDIA, AMD e Intel.", recommended: true, warning: "Os jogos recompilam shaders no próximo arranque.", sizeBytes: 1210 * MB, files: 2400, available: true },
  { id: "wer", title: "Relatórios de erros (WER)", description: "Relatórios de falhas enviados à Microsoft.", recommended: true, warning: null, sizeBytes: 96 * MB, files: 140, available: true },
  { id: "crash-dumps", title: "Dumps de falhas", description: "Minidumps e MEMORY.DMP.", recommended: true, warning: null, sizeBytes: 812 * MB, files: 12, available: true },
  { id: "recycle-bin", title: "Reciclagem", description: "Esvazia a reciclagem.", recommended: false, warning: "Permanente — não é possível recuperar.", sizeBytes: 3400 * MB, files: 210, available: true },
  { id: "prefetch", title: "Prefetch", description: "Dados de arranque rápido de apps.", recommended: false, warning: "Não dá FPS; as apps abrem mais devagar até o Windows reconstruir.", sizeBytes: 42 * MB, files: 256, available: true },
];
let memCfg: MemoryConfig = { enabled: true, standbyThresholdMb: 1024, freeThresholdMb: 1024, intervalSec: 5 };
let mem: MemoryStatus = { serviceInstalled: false, serviceRunning: false, config: memCfg, totalMb: 32768, freeMb: 812, standbyMb: 14320, modifiedMb: 210, purgeCount: 0, lastPurge: null };
let nv: NvidiaStatus = { nvidiaGpu: true, inspectorAvailable: true, applied: false, cs2Applied: false, inspectorVersion: "3.0.2.1" };
let sessCfg: SessionConfig = { enabled: true, closeApps: ["Discord.exe", "chrome.exe"], reopenApps: true, affinity: "auto", affinityMask: null, resolution: { width: 1280, height: 960, refresh: 144 }, vibrance: 70 };
let sess: SessionStatus = { active: false, since: null, cs2Pid: null, actions: [], config: sessCfg };
const displays: DisplayInfo[] = [{ device: "\\\\.\\DISPLAY1", name: "AOC 24G2", primary: true, current: { width: 1920, height: 1080, refresh: 144 },
  modes: [{ width: 1920, height: 1080, refresh: 144 }, { width: 1920, height: 1080, refresh: 60 }, { width: 1440, height: 1080, refresh: 144 }, { width: 1280, height: 960, refresh: 144 }, { width: 1024, height: 768, refresh: 144 }] }];
let vib: VibranceInfo = { supported: true, vendor: "nvidia", current: 50 };
let bench: BenchRecord[] = [
  { id: "3", date: "2026-10-16T18:40:00Z", avgFps: 612.4, p1Fps: 248.9, appliedTweaks: 14, sessionActive: true, note: "Depois do Vektor" },
  { id: "2", date: "2026-10-15T21:10:00Z", avgFps: 548.1, p1Fps: 201.3, appliedTweaks: 6, sessionActive: false, note: null },
  { id: "1", date: "2026-10-15T20:02:00Z", avgFps: 503.7, p1Fps: 176.0, appliedTweaks: 0, sessionActive: false, note: "Antes" },
];
let settings: AppSettings = { autostart: false, startMinimized: false, closeToTray: true };

const mock: typeof api = {
  systemInfo: () => wait({ osName: "Windows 11 Pro", osBuild: "26100", hostname: "CLIENTE-PC", cpu: { name: "AMD Ryzen 7 7800X3D 8-Core Processor", vendor: "amd", cores: 8, threads: 16, zenGen: 4 }, gpus: [{ name: "NVIDIA GeForce RTX 4070", vendor: "nvidia", driverVersion: "32.0.15.6614", vramMb: 12288 }], ramTotalGb: 32, isAdmin: true }),
  liveStats: () => wait({ cpuUsage: 8 + Math.random() * 20, ramUsedGb: 11.4 + Math.random(), ramTotalGb: 32, processCount: 212, cs2Running: false }, 120),
  listTweaks: () => wait(tweaks),
  applyTweak: (id) => { tweaks = tweaks.map((t) => (t.id === id ? { ...t, applied: true } : t)); return wait(tweaks.find((t) => t.id === id)!, 600); },
  revertTweak: (id) => { tweaks = tweaks.map((t) => (t.id === id ? { ...t, applied: false } : t)); return wait(tweaks.find((t) => t.id === id)!, 500); },
  revertAll: () => { tweaks = tweaks.map((t) => ({ ...t, applied: false })); return wait(undefined, 800); },
  cleanScan: () => wait(cleanTargets, 900),
  cleanRun: (ids) => wait({ freedBytes: cleanTargets.filter((t) => ids.includes(t.id)).reduce((a, t) => a + t.sizeBytes, 0), filesRemoved: 4210, paths: ids }, 1500),
  listBackgroundApps: () => wait([{ name: "Discord.exe", pid: 1, memoryMb: 412, label: "Discord" }, { name: "chrome.exe", pid: 2, memoryMb: 1830, label: "Google Chrome" }, { name: "Spotify.exe", pid: 3, memoryMb: 260, label: "Spotify" }]),
  killBackgroundApps: (names) => wait(names.length, 700),
  cs2Detect: () => wait({ steamPath: "C:\\Program Files (x86)\\Steam", gamePath: "D:\\SteamLibrary\\steamapps\\common\\Counter-Strike Global Offensive", installed: true, running: false, users: [{ steamId64: "76561198000000000", accountId: 39734272, personaName: "cliente", mostRecent: true }], launchOptions: "-novid", updateRequired: false, buildId: "20481234" }),
  cs2Presets: () => wait(presets),
  cs2ReadConfig: () => wait(presets[1].config),
  cs2ApplyConfig: () => wait(undefined, 700),
  cs2Launch: () => wait(undefined),
  cs2RecommendedLaunchOptions: () => wait("-novid -nojoy +exec vektor"),
  cs2BenchmarkStart: () => { benchSt = { state: "running", avgFps: null, p1Fps: null, message: "A correr benchmark…" }; setTimeout(() => { benchSt = { state: "done", avgFps: 612.4, p1Fps: 248.9, message: null }; }, 5000); return wait(undefined); },
  cs2BenchmarkPoll: () => wait(benchSt, 100),
  memoryStatus: () => wait({ ...mem, freeMb: 700 + Math.random() * 300, standbyMb: 14000 + Math.random() * 800 }, 150),
  memorySetConfig: (c) => { memCfg = c; mem = { ...mem, config: c }; return wait(mem); },
  memoryInstallService: () => { mem = { ...mem, serviceInstalled: true, serviceRunning: true }; return wait(mem, 800); },
  memoryUninstallService: () => { mem = { ...mem, serviceInstalled: false, serviceRunning: false }; return wait(mem, 600); },
  memoryPurgeNow: () => { mem = { ...mem, freeMb: 14800, standbyMb: 320 }; return wait(mem, 500); },
  nvidiaStatus: () => wait(nv),
  nvidiaApplyGlobal: () => { nv = { ...nv, applied: true }; return wait(nv, 1500); },
  nvidiaRevertGlobal: () => { nv = { ...nv, applied: false }; return wait(nv, 1200); },
  nvidiaApplyCs2: () => { nv = { ...nv, cs2Applied: true }; return wait(nv, 1500); },
  nvidiaRevertCs2: () => { nv = { ...nv, cs2Applied: false }; return wait(nv, 1200); },
  sessionStatus: () => wait(sess, 150),
  sessionSetConfig: (c) => { sessCfg = c; sess = { ...sess, config: c }; return wait(sess); },
  sessionTopology: () => wait({ logicalCount: 16, groups: [{ id: 0, l3Mb: 96, mask: "0xFFFF", logicalCount: 16 }], recommendedMask: "0xFFFC", recommendedLabel: "Todos menos o núcleo 0" }),
  sessionStop: () => { sess = { ...sess, active: false, actions: [] }; return wait(sess); },
  displayList: () => wait(displays),
  vibranceInfo: () => wait(vib),
  vibrancePreview: (level) => { vib = { ...vib, current: level }; return wait(vib, 80); },
  vibranceReset: () => { vib = { ...vib, current: 50 }; return wait(vib); },
  benchHistory: () => wait(bench),
  benchDelete: (id) => { bench = bench.filter((b) => b.id !== id); return wait(bench); },
  benchSetNote: (id, note) => { bench = bench.map((b) => (b.id === id ? { ...b, note } : b)); return wait(bench); },
  appSettings: () => wait(settings),
  appSetSettings: (s2) => { settings = s2; return wait(settings, 400); },
  exportDiagnostics: () => wait("C:\\Users\\cliente\\Desktop\\Vektor-diagnostico-20261016.zip", 900),
};

export const backend: typeof api = inTauri ? api : mock;
