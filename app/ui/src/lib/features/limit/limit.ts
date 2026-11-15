import type { LimitState } from "$lib/api";

export type LimitPhase =
  | "off"
  | "counting"
  | "reached" // at the limit, waiting for the session to end
  | "cooldown"
  | "pending"; // cooldown over, device not reached yet

export function limitPhase(limit: LimitState, nowMs: number): LimitPhase {
  if (limit.lockedUntilMs !== null) return nowMs < limit.lockedUntilMs ? "cooldown" : "pending";
  if (!limit.settings.enabled) return "off";
  return limit.sticksUsed >= limit.settings.sticks ? "reached" : "counting";
}

export function sticksLeft(limit: LimitState): number {
  return Math.max(0, limit.settings.sticks - limit.sticksUsed);
}

export const STICKS = { min: 1, max: 40 } as const;
export const COOLDOWN = { min: 5, max: 1440, presets: [30, 60, 120, 240] } as const;
export const WINDOW = { min: 1, max: 168, presets: [12, 24, 48] } as const;
