// Tracks whether the window is actually on screen, so polling can stop while
// Vektor sits in the tray or is minimised.
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { inTauri } from "./api";

/** Emitted by the backend when the window is hidden to / restored from the tray. */
const VISIBLE_EVENT = "vektor://visible";

let inTray = false;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

export const isVisible = () => !document.hidden && !inTray;

export function onVisibility(fn: () => void) {
  listeners.add(fn);
  return () => {
    listeners.delete(fn);
  };
}

document.addEventListener("visibilitychange", emit);
if (inTauri) {
  void listen<boolean>(VISIBLE_EVENT, (e) => {
    inTray = !e.payload;
    emit();
  });
  // Started with --minimized: the window was hidden before this page could hear about it.
  void getCurrentWindow().isVisible().then((v) => {
    inTray = !v;
    emit();
  }).catch(() => {});
}
