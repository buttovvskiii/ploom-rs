import type { CustomProfile, HeatingMode, ProfileTarget } from "$lib/api";
import { formatDuration } from "$lib/i18n/format";
import { i18n } from "$lib/i18n/index.svelte";

export function tintOf(mode: HeatingMode | null | undefined): string {
  return mode ? `var(--tint-${mode})` : "var(--label-3)";
}

export function modeName(mode: HeatingMode): string {
  return i18n.t.modes[mode];
}

export function customLabel(p: Pick<CustomProfile, "base" | "holdTempC" | "holdSecs">): string {
  return i18n.t.heating.custom.label(modeName(p.base), p.holdTempC, formatDuration(p.holdSecs));
}

export function targetLabel(target: ProfileTarget): string {
  return target.kind === "preset" ? modeName(target.mode) : customLabel(target);
}
