/** Shared motion helpers for Svelte transitions. */
import { cubicOut } from "svelte/easing";
import type { TransitionConfig } from "svelte/transition";

export function reducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** The app's standard ease-out, matching `--ease-out` closely enough to mix with CSS. */
export function easeOut(t: number): number {
  return 1 - Math.pow(1 - t, 4);
}

interface ShiftOptions {
  duration?: number;
  delay?: number;
  /** Start offset in px; positive comes from below. */
  y?: number;
  x?: number;
  /** Start scale. */
  scale?: number;
}

/** Fade with a small travel. */
export function shift(
  _node: Element,
  { duration = 260, delay = 0, y = 0, x = 0, scale = 1 }: ShiftOptions = {},
): TransitionConfig {
  if (reducedMotion()) return { duration: 0 };
  return {
    duration,
    delay,
    easing: easeOut,
    css: (t) =>
      `opacity:${t};transform:translate3d(${(1 - t) * x}px,${(1 - t) * y}px,0) scale(${scale + (1 - scale) * t})`,
  };
}

/** Plain opacity fade. */
export function fade(
  _node: Element,
  { duration = 160, delay = 0 }: { duration?: number; delay?: number } = {},
): TransitionConfig {
  if (reducedMotion()) return { duration: 0 };
  return { duration, delay, easing: cubicOut, css: (t) => `opacity:${t}` };
}
