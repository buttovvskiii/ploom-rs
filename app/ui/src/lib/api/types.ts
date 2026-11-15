/**
 * Wire contract between the Rust backend (`ploom-core` DTOs, exposed through `app/src-tauri`) and
 * the UI.
 */

export type HeatingMode = "delicate" | "balanced" | "intense" | "batterySaver";

export type ConnectionState =
  | "idle" // not connected and not trying (user pressed Disconnect)
  | "connecting" // first connection attempt in progress
  | "connected"
  | "reconnecting"; // link dropped or device not found; retrying with backoff

/** What the device is doing right now (derived, see docs/ARCHITECTURE.md). */
export type Activity = "standby" | "preheating" | "heating" | "charging";

export type LockState = "locked" | "unlocked" | "featureOff";

export interface Snapshot {
  connection: ConnectionState;
  /** Last connection-level problem, or null. */
  connectionError: ErrorCode | null;
  /** True while a BLE scan started by `scan()` is running. */
  scanning: boolean;
  device: DeviceIdentity | null;
  status: DeviceStatus;
  /** Long-running operation in progress (profile flashing), or null. */
  operation: Operation | null;
  /** The stick limit of the connected device (or the last one connected). */
  limit: LimitState;
}

/**
 * The stick limit: after `sticks` heating sessions the backend locks the device for
 * `cooldownMinutes`, then unlocks it again.
 */
export interface LimitSettings {
  enabled: boolean;
  /** Sticks allowed before the device is locked, 1..40. */
  sticks: number;
  /** How long the device stays locked, 5..1440. */
  cooldownMinutes: number;
  /** Hours after the first stick of a window until the counter resets, 1..168. */
  windowHours: number;
}

export interface LimitState {
  settings: LimitSettings;
  /** Sticks counted in the current window. */
  sticksUsed: number;
  /** When the current window began (first counted stick); null when empty. */
  windowStartedAtMs: number | null;
  /** End of the running cooldown. */
  lockedUntilMs: number | null;
}

export interface DeviceIdentity {
  /** Platform peripheral id (a UUID on macOS, a MAC address elsewhere). */
  id: string;
  name: string | null;
  hardware: string | null;
  firmware: string | null;
  /** Body color, decoded by the backend from the hardware string; null if unknown. */
  color: DeviceColor | null;
}

export interface DeviceColor {
  /** Raw color code from the hardware string, e.g. "L4U". */
  code: string;
  /** Stable id the UI maps to a finish: black, silver, gold, blue, glacier, teal. */
  id: string;
  /** Catalog name (English); the UI shows its own localized name for known ids. */
  name: string;
  /** Catalog base color, `#rrggbb`. */
  hex: string;
}

export interface DeviceStatus {
  batteryPct: number | null;
  batteryHealthPct: number | null;
  rssiDbm: number | null;
  /** Effective heating mode; null until the device has reported it. */
  heatingMode: HeatingMode | null;
  /** Set when the active profile is a custom one flashed by this app. */
  customProfile: CustomProfile | null;
  activity: Activity | null;
  /** When the current heating session began; null when not heating. */
  sessionStartedAtMs: number | null;
  charging: boolean | null;
  usbPlugged: boolean | null;
  lock: LockState | null;
  autoStart: boolean | null;
  vibrating: boolean;
  /** Device is too warm to start heating (normal right after a session). */
  temperatureWarning: boolean | null;
  enclosureTempC: number | null;
  unacquiredLogs: { sessions: number; charges: number } | null;
}

export interface CustomProfile {
  base: HeatingMode; // never "balanced"
  holdTempC: number;
  holdSecs: number;
}

export interface Operation {
  kind: "flashingProfile";
  target: ProfileTarget;
  done: number;
  total: number;
}

export type ActivityEvent = { id: number; timestampMs: number } & (
  | { type: "connected"; deviceName: string | null }
  | { type: "connectionLost" }
  | { type: "disconnected" }
  | { type: "stateChanged"; activity: Activity }
  | { type: "chargingChanged"; charging: boolean }
  | { type: "vibrationChanged"; on: boolean }
  | { type: "lockChanged"; lock: LockState }
  | { type: "autoStartChanged"; enabled: boolean }
  | { type: "profileApplied"; target: ProfileTarget; confirmed: boolean }
  | { type: "sessionLogReceived" }
  | { type: "limitReached"; sticks: number; lockedUntilMs: number }
  | { type: "cooldownEnded" }
  /** Unlocked from elsewhere during a cooldown; the backend locked it again. */
  | { type: "relocked" }
  | { type: "rawFrameSent"; hex: string; replies: number }
);

export interface Catalog {
  modes: ModeInfo[]; // in display order: delicate, balanced, intense, batterySaver
  custom: {
    tempC: [min: number, max: number];
    holdSecs: [min: number, max: number];
    bases: HeatingMode[];
  };
  dangerousOpcodes: { opcode: number; reason: DangerReason }[];
}

export type DangerReason = "erasesLogs" | "disablesBluetooth" | "entersFirmwareUpdate";

export interface ModeInfo {
  id: HeatingMode;
  maxSticks: number;
  minutes: number;
  /** Intensity dots shown in the official app, 1..3. */
  heat: 1 | 2 | 3;
  /** Heating curve; null for "balanced" (factory profile, shape unknown). */
  curve: CurveStep[] | null;
  /** Index of the hold phase a custom profile modifies; null for "balanced". */
  holdStepIndex: number | null;
}

export interface CurveStep {
  durationS: number;
  /** Negative values are device flags; plot Math.abs(). 0 = heater off. */
  temperatureC: number;
  /** 1000 means "no slope" (step), otherwise a ramp coefficient. */
  gradient: number;
}

export type ProfileTarget =
  | { kind: "preset"; mode: HeatingMode }
  | { kind: "custom"; base: HeatingMode; holdTempC: number; holdSecs: number };

export interface DiscoveredDevice {
  id: string;
  name: string;
  rssiDbm: number | null;
  /** This is the device we are connected to right now. */
  connected: boolean;
}

export interface FramePreview {
  /** masters[7] calibration value read from the device; null for balanced. */
  calibrationKey: number | null;
  frames: { hex: string; awaitsOpcode: number | null }[];
}

export interface LogsReport {
  session: SessionLog | null;
  charge: ChargeLog | null;
  unacquired: { sessions: number; charges: number } | null;
}

export interface SessionLog {
  /** Device-internal counter at session start (not wall clock). */
  startCounter: number;
  durationS: number;
  puffCount: number;
  /** Seconds from session start for each puff, ascending. */
  puffOffsetsS: number[];
  puffStopEnabled: boolean;
  autoStartEnabled: boolean;
  lockEnabled: boolean;
}

export interface ChargeLog {
  startCounter: number;
  endCounter: number;
  startMv: number;
  endMv: number;
  totalChargeCount: number | null;
  stateOfHealthPct: number | null;
}

export interface Diagnostics {
  enclosureTempC: number | null;
  usbPlugged: boolean | null;
  usbCc: [cc1: number, cc2: number] | null;
  adc: { name: string; value: number }[] | null;
  /**
   * Lifetime heating-start statistics, `key` is a stable id the UI localizes: coldStart, hotStart,
   * buttonInAutoWindow, buttonAfterTimeout, buttonAfterUnavailable,
   * buttonAfterTimeoutOrUnavailable, autoAfterUnavailable
   */
  heatingStarts: { key: string; count: number }[] | null;
  chargeCycles: number | null;
  batteryHealthPct: number | null;
  batteryPct: number | null;
}

export interface RawExchange {
  sent: string;
  replies: { opcode: number; hex: string; decoded: Record<string, unknown> }[];
}

/** Every command rejects with this shape. */
export interface CommandError {
  code: ErrorCode;
  /** English technical detail for logs/tooltips; never shown as the main text. */
  detail: string;
}

export type ErrorCode =
  | "notConnected"
  | "busy" // another operation (flashing) holds the device
  | "deviceNotFound"
  | "bluetoothUnavailable" // adapter off / permission denied
  | "timeout"
  | "deviceRejected" // device answered with a non-zero status
  | "calibrationOutOfRange"
  | "invalidInput"
  | "dangerousFrame" // resend with confirmDangerous: true
  | "cooldownActive" // unlocking / relaxing the limit must wait for the cooldown
  | "internal";

/** Command table. */
export interface Commands {
  get_snapshot: { args: Record<string, never>; result: Snapshot };
  get_catalog: { args: Record<string, never>; result: Catalog };
  get_activity: { args: Record<string, never>; result: ActivityEvent[] };
  scan: { args: Record<string, never>; result: DiscoveredDevice[] };
  /** id null = first Ploom found. */
  connect: { args: { id: string | null }; result: void };
  disconnect: { args: Record<string, never>; result: void };
  set_vibration: { args: { on: boolean }; result: void };
  set_lock: { args: { locked: boolean }; result: void };
  set_auto_start: { args: { enabled: boolean }; result: void };
  /** Works without a connection. */
  set_limit_settings: { args: { settings: LimitSettings }; result: void };
  /** Resolves when the device confirmed (or rejects). */
  apply_profile: { args: { target: ProfileTarget }; result: void };
  preview_profile: { args: { target: ProfileTarget }; result: FramePreview };
  read_logs: { args: Record<string, never>; result: LogsReport };
  read_diagnostics: { args: Record<string, never>; result: Diagnostics };
  send_raw: { args: { hex: string; confirmDangerous: boolean }; result: RawExchange };
}

export const EVENTS = {
  snapshot: "ploom://snapshot",
  activity: "ploom://activity",
} as const;
