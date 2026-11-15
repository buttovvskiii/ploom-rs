import { i18n } from "./index.svelte";

const cache = new Map<string, Intl.NumberFormat>();

function numberFormat(digits: number): Intl.NumberFormat {
  const key = `${i18n.tag}:${digits}`;
  let fmt = cache.get(key);
  if (!fmt) {
    fmt = new Intl.NumberFormat(i18n.tag, {
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
    });
    cache.set(key, fmt);
  }
  return fmt;
}

export function formatNumber(value: number, digits = 0): string {
  return numberFormat(digits).format(value);
}

export function formatTemp(celsius: number, digits = 1): string {
  return `${formatNumber(celsius, digits)} °C`;
}

/** "5 min 18 s" */
export function formatDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.round(totalSeconds));
  if (s < 60) return i18n.t.format.sec(s);
  return i18n.t.format.minSec(Math.floor(s / 60), s % 60);
}

/** "2 h 30 min" */
export function formatMinutes(totalMinutes: number): string {
  const m = Math.max(0, Math.round(totalMinutes));
  if (m < 60) return i18n.t.format.min(m);
  return i18n.t.format.hourMin(Math.floor(m / 60), m % 60);
}

/** "1:04:09" above an hour, "4:09" below. */
export function formatCountdown(totalSeconds: number): string {
  const s = Math.max(0, Math.ceil(totalSeconds));
  if (s < 3600) return formatClock(s);
  const rest = String(Math.floor((s % 3600) / 60)).padStart(2, "0");
  return `${Math.floor(s / 3600)}:${rest}:${String(s % 60).padStart(2, "0")}`;
}

/** "5:18" */
export function formatClock(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

export function formatRelative(timestampMs: number, nowMs: number): string {
  const diff = Math.round((timestampMs - nowMs) / 1000);
  if (Math.abs(diff) < 45) return i18n.t.format.justNow;
  const rtf = new Intl.RelativeTimeFormat(i18n.tag, { numeric: "auto", style: "long" });
  if (Math.abs(diff) < 3600) return rtf.format(Math.round(diff / 60), "minute");
  if (Math.abs(diff) < 86400) return rtf.format(Math.round(diff / 3600), "hour");
  return rtf.format(Math.round(diff / 86400), "day");
}

export function formatTime(timestampMs: number): string {
  return new Intl.DateTimeFormat(i18n.tag, { hour: "2-digit", minute: "2-digit" }).format(
    timestampMs,
  );
}

export function formatOpcode(opcode: number): string {
  return `0x${opcode.toString(16).padStart(2, "0").toUpperCase()}`;
}

/** "02e300" → "02 e3 00" */
export function spacedHex(hex: string): string {
  return hex.replace(/\s+/g, "").replace(/(..)(?=.)/g, "$1 ");
}
