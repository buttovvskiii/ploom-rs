/** What the 3D device should look like for a given snapshot. */
import type { HeatingMode, Snapshot } from "$lib/api";

/** A slow sinusoidal swell of a light's intensity. */
export interface Breathe {
  periodS: number;
  /** 0..1, the fraction of the intensity that rises and falls. */
  depth: number;
}

export interface Lamp {
  /** sRGB hex. */
  color: string;
  /** 0 = off, 1 = full. */
  intensity: number;
  breathe: Breathe | null;
}

export interface VisualTargets {
  /** 0 = awake, 1 = asleep (dimmed and desaturated). */
  dim: number;
  /** The small light near the top of the front. */
  status: Lamp;
  /** The heater, seen inside the stick opening. */
  glow: Lamp;
  /** 0..1 amplitude of the Find-device shake. */
  shake: number;
}

export interface Palette {
  modes: Record<HeatingMode, string>;
  charging: string;
  /** The heater's own color, blended with the mode tint for the glow. */
  heat: string;
  /** Neutral light, used in place of the shake when motion is reduced. */
  find: string;
}

const OFF: Lamp = { color: "#000000", intensity: 0, breathe: null };

const PREHEAT_BREATHE: Breathe = { periodS: 2.6, depth: 0.42 };
const HEAT_BREATHE: Breathe = { periodS: 4.4, depth: 0.22 };
const CHARGE_BREATHE: Breathe = { periodS: 4, depth: 0.55 };

export function visualTargets(
  snapshot: Snapshot,
  palette: Palette,
  reducedMotion: boolean,
): VisualTargets {
  const calm = (b: Breathe): Breathe | null => (reducedMotion ? null : b);
  if (snapshot.connection !== "connected") {
    return { dim: 1, status: OFF, glow: OFF, shake: 0 };
  }

  const s = snapshot.status;
  let status: Lamp = OFF;
  let glow: Lamp = OFF;

  if (s.activity === "preheating" || s.activity === "heating") {
    const tint = palette.modes[s.heatingMode ?? s.customProfile?.base ?? "balanced"];
    const preheat = s.activity === "preheating";
    const breathe = calm(preheat ? PREHEAT_BREATHE : HEAT_BREATHE);
    status = { color: tint, intensity: preheat ? 0.8 : 1, breathe };
    glow = { color: mixHex(palette.heat, tint, 0.35), intensity: preheat ? 0.6 : 1, breathe };
  } else if (s.activity === "charging" || s.charging === true) {
    status = { color: palette.charging, intensity: 0.75, breathe: calm(CHARGE_BREATHE) };
  }

  // With reduced motion the shake becomes a steady light, unless a state already owns it.
  let shake = 0;
  if (s.vibrating) {
    if (!reducedMotion) shake = 1;
    else if (status.intensity === 0)
      status = { color: palette.find, intensity: 0.7, breathe: null };
  }

  return { dim: 0, status, glow, shake };
}

/** Linear blend of two `#rrggbb` colors; falls back to `a` if either is malformed. */
export function mixHex(a: string, b: string, t: number): string {
  const pa = parseHex(a);
  const pb = parseHex(b);
  if (!pa || !pb) return a;
  const out = pa.map((v, i) => Math.round(v + ((pb[i] ?? v) - v) * t));
  return `#${out.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

function parseHex(value: string): number[] | null {
  const m = /^#?([0-9a-f]{6})$/i.exec(value.trim());
  if (!m?.[1]) return null;
  const n = parseInt(m[1], 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}
