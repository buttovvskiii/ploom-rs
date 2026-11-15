import type { DeviceIdentity } from "$lib/api";
import {
  DEFAULT_FINISH,
  detectedFinish,
  isFinish,
  type FinishId,
} from "$lib/features/device3d/finishes";

export type Section =
  "overview" | "heating" | "limit" | "history" | "diagnostics" | "devices" | "console";

const SECTIONS: readonly Section[] = [
  "overview",
  "heating",
  "limit",
  "history",
  "diagnostics",
  "devices",
  "console",
];
const STORAGE_KEY = "ploom.section";
/** The last finish shown with a device, for while none is connected. */
const LAST_FINISH_KEY = "ploom.finish";

function load(): Section {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return SECTIONS.find((s) => s === value) ?? "overview";
  } catch {
    return "overview";
  }
}

function loadLastFinish(): FinishId {
  try {
    const value = localStorage.getItem(LAST_FINISH_KEY);
    return isFinish(value) ? value : DEFAULT_FINISH;
  } catch {
    return DEFAULT_FINISH;
  }
}

function store(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Not persisted; the choice still applies for this run.
  }
}

class Prefs {
  section = $state<Section>(load());
  private lastFinish = $state<FinishId>(loadLastFinish());

  go(section: Section): void {
    this.section = section;
    store(STORAGE_KEY, section);
  }

  /** The device's own color; with no device, the last one shown. */
  finishFor(device: DeviceIdentity | null | undefined): FinishId {
    if (!device) return this.lastFinish;
    return detectedFinish(device) ?? "neutral";
  }

  /** Keep the finish on screen for when the device disconnects. */
  rememberFinish(finish: FinishId): void {
    if (this.lastFinish === finish) return;
    this.lastFinish = finish;
    store(LAST_FINISH_KEY, finish);
  }
}

export const prefs = new Prefs();
