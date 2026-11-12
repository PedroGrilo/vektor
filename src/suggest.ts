// Suggested standby-cleaner rules for a PC, from how much RAM it has.
import type { MemoryConfig } from "./api";

export type MemoryRules = Pick<MemoryConfig, "standbyThresholdMb" | "freeThresholdMb" | "intervalSec">;

const GB = 1024;

const STEP = 256; // the sliders move in 256 MB steps

/**
 * A formula, not a lookup table, so any amount of RAM gets an answer (12, 20,
 * 40 GB...). The free-memory limit is the value that matters: one eighth of the
 * installed RAM, rounded to a slider step, never below 1 GB (the service has
 * nothing to gain earlier) nor above 4 GB (beyond that it would mostly throw
 * away useful cache).
 * 8 GB -> 1024 MB, 12 GB -> 1536 MB, 16 GB -> 2048 MB, 32 GB or more -> 4096 MB.
 */
export function suggestedRules(totalMb: number): MemoryRules {
  // Start from the installed amount: Windows reports a little less, and that
  // small difference should not move the suggestion.
  const eighth = Math.round((nominalGb(totalMb) * GB) / 8 / STEP) * STEP;
  return {
    standbyThresholdMb: GB,
    freeThresholdMb: Math.min(4 * GB, Math.max(GB, eighth)),
    intervalSec: 5,
  };
}

/** Windows reports slightly less than the installed amount (39.7 GB for 40 GB). */
export function nominalGb(totalMb: number) {
  return Math.round(totalMb / GB);
}
