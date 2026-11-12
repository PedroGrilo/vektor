// App update state shared by the sidebar badge and the Settings page.
// The backend checks the signed `latest.json` published with each GitHub release.
import { useSyncExternalStore } from "react";
import { listen } from "@tauri-apps/api/event";
import { inTauri } from "./api";
import type { UpdateInfo, UpdateProgress } from "./api";
import { backend } from "./mock";

export type UpdatePhase = "idle" | "checking" | "up-to-date" | "available" | "installing" | "error";
export interface UpdateState {
  phase: UpdatePhase;
  info: UpdateInfo | null;
  /** 0..1 while downloading, null when the size is unknown */
  progress: number | null;
  error: string | null;
  /** ISO timestamp of the last successful check */
  checkedAt: string | null;
}

const RECHECK_MS = 6 * 60 * 60 * 1000;

let state: UpdateState = { phase: "idle", info: null, progress: null, error: null, checkedAt: null };
const subs = new Set<() => void>();
const set = (patch: Partial<UpdateState>) => {
  state = { ...state, ...patch };
  subs.forEach((s) => s());
};

export const useUpdate = () =>
  useSyncExternalStore(
    (cb) => {
      subs.add(cb);
      return () => subs.delete(cb);
    },
    () => state,
  );

export async function checkForUpdate(): Promise<UpdateState> {
  if (state.phase === "checking" || state.phase === "installing") return state;
  set({ phase: "checking", error: null });
  try {
    const info = await backend.updateCheck();
    set({ phase: info.available ? "available" : "up-to-date", info, checkedAt: new Date().toISOString() });
  } catch (e) {
    // Keep a previously found update visible if a later re-check fails (e.g. offline).
    set({ phase: state.info?.available ? "available" : "error", error: String(e) });
  }
  return state;
}

export async function installUpdate(): Promise<void> {
  if (state.phase === "installing") return;
  set({ phase: "installing", progress: 0, error: null });
  try {
    // Resolves only if the installer could not be started: on success the app exits.
    await backend.updateInstall();
  } catch (e) {
    set({ phase: "available", progress: null, error: String(e) });
    throw e;
  }
}

let started = false;
/** Check shortly after start and then every few hours (the app lives in the tray). */
export function startAutoUpdateChecks() {
  if (started) return;
  started = true;
  setTimeout(() => void checkForUpdate(), 4000);
  setInterval(() => void checkForUpdate(), RECHECK_MS);
}

if (inTauri) {
  void listen<UpdateProgress>("vektor://update-progress", (e) => {
    const { downloaded, total } = e.payload;
    set({ progress: total ? Math.min(1, downloaded / total) : null });
  });
}
