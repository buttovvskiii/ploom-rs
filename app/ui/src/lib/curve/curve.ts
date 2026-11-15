/** Turning a `CurveStep[]` into something drawable. */
import type { CurveStep, ModeInfo } from "$lib/api";

export const STEP = 1000;

export interface CurvePoint {
  /** seconds from session start */
  t: number;
  /** °C; 0 is the baseline */
  c: number;
}

export function totalDuration(curve: readonly CurveStep[]): number {
  return curve.reduce((sum, s) => sum + s.durationS, 0);
}

export function curvePoints(curve: readonly CurveStep[]): CurvePoint[] {
  const points: CurvePoint[] = [{ t: 0, c: 0 }];
  let t = 0;
  let level = 0;
  for (const step of curve) {
    if (step.durationS <= 0) continue;
    const target = Math.abs(step.temperatureC);
    if (target === 0 || step.gradient !== STEP) {
      // Ramp (or cool down) to the target over the step.
      t += step.durationS;
      points.push({ t, c: target });
    } else {
      if (target !== level) points.push({ t, c: target });
      t += step.durationS;
      points.push({ t, c: target });
    }
    level = target;
  }
  if (level !== 0) points.push({ t, c: 0 });
  return points;
}

/** Linear interpolation of the plotted line at time `t`. */
export function levelAt(points: readonly CurvePoint[], t: number): number {
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1];
    const b = points[i];
    if (!a || !b) continue;
    if (t <= b.t) {
      if (b.t === a.t) return b.c;
      return a.c + ((b.c - a.c) * (t - a.t)) / (b.t - a.t);
    }
  }
  return points.at(-1)?.c ?? 0;
}

/** [start, end] seconds of step `index`. */
export function stepRange(curve: readonly CurveStep[], index: number): [number, number] {
  let start = 0;
  for (let i = 0; i < index; i++) start += curve[i]?.durationS ?? 0;
  return [start, start + (curve[index]?.durationS ?? 0)];
}

/** Temperature domain for plotting: headroom above, room below the lowest non-zero level. */
export function domainFor(curves: readonly (readonly CurveStep[])[]): [number, number] {
  const temps = curves.flatMap((c) => c.map((s) => Math.abs(s.temperatureC)).filter((v) => v > 0));
  if (temps.length === 0) return [150, 350];
  const lo = Math.min(...temps);
  const hi = Math.max(...temps);
  return [Math.floor((lo - 50) / 50) * 50, Math.ceil((hi + 15) / 25) * 25];
}

/** The base preset's curve with its hold step replaced. */
export function customCurve(
  mode: ModeInfo,
  holdTempC: number,
  holdSecs: number,
): CurveStep[] | null {
  if (!mode.curve) return null;
  return mode.curve.map((s, i) =>
    i === mode.holdStepIndex ? { ...s, temperatureC: holdTempC, durationS: holdSecs } : { ...s },
  );
}

export interface XY {
  x: number;
  y: number;
}

export interface SmoothPath {
  /** SVG path data. */
  d: string;
  /** The path sampled densely, in drawing order, for reading `y` at an `x`. */
  samples: XY[];
}

const CORNER_SAMPLES = 12;

/**
 * Draws a polyline (in screen space) with every corner rounded into a quadratic Bézier whose
 * control point is the corner itself.
 */
export function smoothPath(points: readonly XY[], radius: number, ease = 0): SmoothPath {
  const pts = easeSteps(
    points.filter((p, i) => {
      const prev = points[i - 1];
      return !prev || Math.hypot(p.x - prev.x, p.y - prev.y) > 0.01;
    }),
    ease,
  );
  const first = pts[0];
  if (!first) return { d: "", samples: [] };

  const f = (n: number) => n.toFixed(2);
  let d = `M${f(first.x)} ${f(first.y)}`;
  const samples: XY[] = [first];

  for (let i = 1; i < pts.length; i++) {
    const prev = pts[i - 1] as XY;
    const corner = pts[i] as XY;
    const next = pts[i + 1];
    if (!next) {
      d += `L${f(corner.x)} ${f(corner.y)}`;
      samples.push(corner);
      break;
    }
    const inLen = Math.hypot(corner.x - prev.x, corner.y - prev.y);
    const outLen = Math.hypot(next.x - corner.x, next.y - corner.y);
    const r = Math.min(radius, inLen / 2, outLen / 2);
    const entry = {
      x: corner.x + ((prev.x - corner.x) / inLen) * r,
      y: corner.y + ((prev.y - corner.y) / inLen) * r,
    };
    const exit = {
      x: corner.x + ((next.x - corner.x) / outLen) * r,
      y: corner.y + ((next.y - corner.y) / outLen) * r,
    };
    d += `L${f(entry.x)} ${f(entry.y)}Q${f(corner.x)} ${f(corner.y)} ${f(exit.x)} ${f(exit.y)}`;
    samples.push(entry);
    for (let s = 1; s <= CORNER_SAMPLES; s++) {
      const u = s / CORNER_SAMPLES;
      const a = (1 - u) * (1 - u);
      const b = 2 * u * (1 - u);
      const c = u * u;
      samples.push({
        x: a * entry.x + b * corner.x + c * exit.x,
        y: a * entry.y + b * corner.y + c * exit.y,
      });
    }
  }
  return { d, samples };
}

/** Spreads every vertical segment of a polyline over up to `ease` px. */
function easeSteps(points: readonly XY[], ease: number): XY[] {
  const out = points.map((p) => ({ ...p }));
  if (ease <= 0) return out;
  for (let i = 0; i + 1 < points.length; i++) {
    const a = points[i] as XY;
    const b = points[i + 1] as XY;
    if (Math.abs(b.x - a.x) > 0.01) continue;
    const before = points[i - 1];
    const after = points[i + 2];
    const left = before ? Math.min(ease, (a.x - before.x) / 3) : 0;
    const right = after ? Math.min(ease, (after.x - b.x) / 3) : 0;
    (out[i] as XY).x = a.x - left;
    (out[i + 1] as XY).x = b.x + right;
  }
  return out;
}

/** `y` of a sampled path at `x` (samples ordered by non-decreasing `x`). */
export function yAtX(samples: readonly XY[], x: number): number {
  for (let i = 1; i < samples.length; i++) {
    const a = samples[i - 1] as XY;
    const b = samples[i] as XY;
    if (x <= b.x) {
      if (b.x - a.x < 1e-6) return b.y;
      return a.y + ((b.y - a.y) * (x - a.x)) / (b.x - a.x);
    }
  }
  return samples.at(-1)?.y ?? 0;
}
