import type { DeviceStatus, Snapshot } from "$lib/api";

const EMPTY_STATUS: DeviceStatus = {
  batteryPct: null,
  batteryHealthPct: null,
  rssiDbm: null,
  heatingMode: null,
  customProfile: null,
  activity: null,
  sessionStartedAtMs: null,
  charging: null,
  usbPlugged: null,
  lock: null,
  autoStart: null,
  vibrating: false,
  temperatureWarning: null,
  enclosureTempC: null,
  unacquiredLogs: null,
};

/** The latest `Snapshot` pushed by the backend, plus the derivations every page needs. */
class SnapshotStore {
  current = $state<Snapshot>({
    connection: "idle",
    connectionError: null,
    scanning: false,
    device: null,
    status: EMPTY_STATUS,
    operation: null,
    limit: {
      settings: { enabled: false, sticks: 10, cooldownMinutes: 120, windowHours: 24 },
      sticksUsed: 0,
      windowStartedAtMs: null,
      lockedUntilMs: null,
    },
  });

  readonly status = $derived(this.current.status);
  readonly connected = $derived(this.current.connection === "connected");
  readonly busy = $derived(this.current.operation !== null);
  readonly inSession = $derived(
    this.current.status.activity === "preheating" || this.current.status.activity === "heating",
  );

  set(next: Snapshot): void {
    this.current = next;
  }
}

export const snapshot = new SnapshotStore();
