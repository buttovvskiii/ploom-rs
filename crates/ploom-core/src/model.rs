//! Data transfer objects mirroring `app/ui/src/lib/api/types.ts`.

use serde::{Deserialize, Serialize};

pub use ploom_protocol::{DangerReason, DeviceColor, HeatingMode};

/// Connection lifecycle as shown by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    /// Not connected and not trying (initial state, or after `disconnect`).
    #[default]
    Idle,
    /// First connection attempt in progress.
    Connecting,
    /// Connected and polling.
    Connected,
    /// The link dropped or the device was not found; retrying with backoff.
    Reconnecting,
}

/// What the device is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Activity {
    /// Idle.
    Standby,
    /// Heating up before a session.
    Preheating,
    /// A session is in progress.
    Heating,
    /// On the charger.
    Charging,
}

/// Lock state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LockState {
    /// Locked.
    Locked,
    /// Unlocked.
    Unlocked,
    /// The lock feature itself is turned off.
    FeatureOff,
}

/// Everything the UI shows, pushed on every change.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// Connection lifecycle.
    pub connection: ConnectionState,
    /// Last connection-level problem, or `None`.
    pub connection_error: Option<ErrorCode>,
    /// A scan started by `scan()` is running.
    pub scanning: bool,
    /// The connected (or reconnecting) device.
    pub device: Option<DeviceIdentity>,
    /// Derived device status.
    pub status: DeviceStatus,
    /// Long-running operation in progress, or `None`.
    pub operation: Option<Operation>,
    /// The stick limit: settings, progress and a running cooldown.
    pub limit: LimitState,
}

/// Settings of the stick limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitSettings {
    /// Whether the limit is enforced.
    pub enabled: bool,
    /// Sticks allowed before the device is locked, `1..=40`.
    pub sticks: u32,
    /// How long the device stays locked, `5..=1440` minutes.
    pub cooldown_minutes: u32,
    /// The counter resets by itself this long after the first stick of a window when the limit was
    /// not reached, `1..=168` hours.
    pub window_hours: u32,
}

impl Default for LimitSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            sticks: 10,
            cooldown_minutes: 120,
            window_hours: 24,
        }
    }
}

/// The stick limit as shown by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitState {
    /// Current settings.
    pub settings: LimitSettings,
    /// Sticks counted in the current window.
    pub sticks_used: u32,
    /// When the current window began (first counted stick), Unix ms.
    pub window_started_at_ms: Option<u64>,
    /// End of the running cooldown, Unix ms; `None` when not cooling down.
    pub locked_until_ms: Option<u64>,
}

/// Identity of a device.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIdentity {
    /// Platform peripheral id (a UUID on macOS, a MAC address elsewhere).
    pub id: String,
    /// Advertised name.
    pub name: Option<String>,
    /// DIS hardware revision (`0x2A27`).
    pub hardware: Option<String>,
    /// DIS firmware revision (`0x2A28`).
    pub firmware: Option<String>,
    /// Body color decoded from the hardware string; `None` if unknown.
    pub color: Option<DeviceColor>,
}

/// Derived device status.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    /// State of charge, percent.
    pub battery_pct: Option<u8>,
    /// Battery state of health, percent.
    pub battery_health_pct: Option<u8>,
    /// Link RSSI as measured by the device.
    pub rssi_dbm: Option<i16>,
    /// Effective heating mode.
    pub heating_mode: Option<HeatingMode>,
    /// Set when the active profile is a custom one flashed by this app.
    pub custom_profile: Option<CustomProfile>,
    /// What the device is doing.
    pub activity: Option<Activity>,
    /// When the current heating session began; `None` when not heating.
    pub session_started_at_ms: Option<u64>,
    /// Charging (USB-C detection, `0x34`, or battery-trend inference).
    pub charging: Option<bool>,
    /// A USB-C charger is attached (diagnostics channel).
    pub usb_plugged: Option<bool>,
    /// Lock state.
    pub lock: Option<LockState>,
    /// Auto-Start Heating.
    pub auto_start: Option<bool>,
    /// Find My Device buzzer is on.
    pub vibrating: bool,
    /// Too warm to start heating (normal right after a session).
    pub temperature_warning: Option<bool>,
    /// Enclosure temperature, °C.
    pub enclosure_temp_c: Option<f64>,
    /// Logs not yet acknowledged on the device.
    pub unacquired_logs: Option<LogCounts>,
}

/// Counts of unacknowledged logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogCounts {
    /// Session logs.
    pub sessions: u16,
    /// Charge logs.
    pub charges: u16,
}

/// A custom heating profile (a preset with a modified hold phase).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProfile {
    /// Base preset; never `balanced`.
    pub base: HeatingMode,
    /// Hold-phase temperature, °C.
    pub hold_temp_c: u16,
    /// Hold-phase length, seconds.
    pub hold_secs: u16,
}

impl From<ploom_protocol::CustomProfile> for CustomProfile {
    fn from(p: ploom_protocol::CustomProfile) -> Self {
        Self {
            base: p.base(),
            hold_temp_c: p.hold_temp_c(),
            hold_secs: u16::from(p.hold_secs()),
        }
    }
}

/// A long-running operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Operation {
    /// Uploading a heating profile.
    FlashingProfile {
        /// What is being applied.
        target: ProfileTarget,
        /// Frames sent so far.
        done: u32,
        /// Frames in total.
        total: u32,
    },
}

/// One entry of the activity feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEvent {
    /// Monotonic id, unique per session.
    pub id: u64,
    /// When it happened, Unix ms.
    pub timestamp_ms: u64,
    /// What happened.
    #[serde(flatten)]
    pub kind: ActivityKind,
}

/// The payload of an [`ActivityEvent`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ActivityKind {
    /// A connection was established.
    Connected {
        /// Advertised name.
        device_name: Option<String>,
    },
    /// The link dropped unexpectedly; reconnecting.
    ConnectionLost,
    /// The user disconnected.
    Disconnected,
    /// The device changed activity.
    StateChanged {
        /// New activity.
        activity: Activity,
    },
    /// Charging started or stopped.
    ChargingChanged {
        /// Whether it is charging now.
        charging: bool,
    },
    /// The buzzer was turned on or off.
    VibrationChanged {
        /// Whether it is buzzing now.
        on: bool,
    },
    /// The lock state changed.
    LockChanged {
        /// New state.
        lock: LockState,
    },
    /// Auto-Start Heating changed.
    AutoStartChanged {
        /// New setting.
        enabled: bool,
    },
    /// A profile was applied.
    ProfileApplied {
        /// What was applied.
        target: ProfileTarget,
        /// The device confirmed the active profile afterwards.
        confirmed: bool,
    },
    /// The device pushed a session log (sent after a session ends).
    SessionLogReceived,
    /// The stick limit was reached and the device was locked.
    LimitReached {
        /// The limit that was reached.
        sticks: u32,
        /// When the device will be unlocked again, Unix ms.
        locked_until_ms: u64,
    },
    /// The cooldown ended and the device was released.
    CooldownEnded,
    /// The device was unlocked from elsewhere during a cooldown and has been locked again.
    Relocked,
    /// A raw frame was sent from the console.
    RawFrameSent {
        /// The frame, hex.
        hex: String,
        /// Number of indications received in the reply window.
        replies: u32,
    },
}

/// Static information about modes and limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    /// In display order: delicate, balanced, intense, batterySaver.
    pub modes: Vec<ModeInfo>,
    /// Custom-profile limits.
    pub custom: CustomLimits,
    /// Opcodes `send_raw` refuses without confirmation.
    pub dangerous_opcodes: Vec<DangerousOpcode>,
}

/// Limits for custom profiles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomLimits {
    /// `[min, max]` hold temperature, °C.
    pub temp_c: (u16, u16),
    /// `[min, max]` hold length, seconds.
    pub hold_secs: (u16, u16),
    /// Modes a custom profile can be based on.
    pub bases: Vec<HeatingMode>,
}

/// A dangerous opcode and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DangerousOpcode {
    /// The opcode.
    pub opcode: u8,
    /// Why it is dangerous.
    pub reason: DangerReason,
}

/// One heating mode.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeInfo {
    /// The mode.
    pub id: HeatingMode,
    /// Sticks per full charge.
    pub max_sticks: u8,
    /// Session length, minutes.
    pub minutes: u8,
    /// Intensity dots shown in the official app, 1..=3.
    pub heat: u8,
    /// Heating curve; `None` for balanced (factory profile, shape unknown).
    pub curve: Option<Vec<CurveStep>>,
    /// Index of the hold phase a custom profile modifies; `None` for balanced.
    pub hold_step_index: Option<u32>,
}

/// One step of a heating curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurveStep {
    /// Step length, seconds.
    pub duration_s: u16,
    /// Negative values are device flags (plot the absolute value); 0 = heater off.
    pub temperature_c: i16,
    /// 1000 means "no slope" (a flat step), otherwise a ramp coefficient.
    pub gradient: f64,
}

impl From<ploom_protocol::heating::CurveStep> for CurveStep {
    fn from(s: ploom_protocol::heating::CurveStep) -> Self {
        Self {
            duration_s: u16::from(s.duration_s),
            temperature_c: s.temperature_c,
            gradient: s.gradient,
        }
    }
}

/// What `apply_profile`/`preview_profile` should upload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProfileTarget {
    /// One of the four presets.
    Preset {
        /// The preset.
        mode: HeatingMode,
    },
    /// A preset with a modified hold phase.
    Custom {
        /// Base preset; must not be balanced.
        base: HeatingMode,
        /// Hold temperature, °C (clamped to the catalog limits).
        hold_temp_c: i64,
        /// Hold length, seconds (clamped to the catalog limits).
        hold_secs: i64,
    },
}

/// A device seen during a scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredDevice {
    /// Platform peripheral id.
    pub id: String,
    /// Advertised name.
    pub name: String,
    /// Advertisement RSSI.
    pub rssi_dbm: Option<i16>,
    /// This is the device we are connected to right now.
    pub connected: bool,
}

/// The frames a profile upload would send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePreview {
    /// `masters[7]` read from the device; `None` for balanced.
    pub calibration_key: Option<u16>,
    /// Frames in order.
    pub frames: Vec<PreviewFrame>,
}

/// One frame of a [`FramePreview`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFrame {
    /// The frame, lowercase hex separated by spaces.
    pub hex: String,
    /// Opcode of the acknowledgement it waits for.
    pub awaits_opcode: Option<u8>,
}

/// Result of `read_logs`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsReport {
    /// The oldest unacknowledged session log.
    pub session: Option<SessionLog>,
    /// The oldest unacknowledged charge log.
    pub charge: Option<ChargeLog>,
    /// Number of unacknowledged logs.
    pub unacquired: Option<LogCounts>,
}

/// A session (smoking) log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools, reason = "mirrors the wire contract")]
pub struct SessionLog {
    /// Device-internal counter at session start (not wall clock).
    pub start_counter: u32,
    /// Session length, seconds.
    pub duration_s: u16,
    /// Number of puffs.
    pub puff_count: u8,
    /// Seconds from session start for each puff, ascending.
    pub puff_offsets_s: Vec<u16>,
    /// Puff-count stop was enabled.
    pub puff_stop_enabled: bool,
    /// Auto-start was enabled.
    pub auto_start_enabled: bool,
    /// The lock feature was enabled.
    pub lock_enabled: bool,
}

/// A charge log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargeLog {
    /// Device counter when charging started.
    pub start_counter: u32,
    /// Device counter when charging ended.
    pub end_counter: u32,
    /// Battery voltage at start, mV.
    pub start_mv: u16,
    /// Battery voltage at end, mV.
    pub end_mv: u16,
    /// Lifetime charge count (`0x40`).
    pub total_charge_count: Option<u16>,
    /// State of health at the start of the charge, percent (`0x40`).
    pub state_of_health_pct: Option<u8>,
}

/// Result of `read_diagnostics`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    /// Enclosure temperature, °C.
    pub enclosure_temp_c: Option<f64>,
    /// A USB-C charger is attached.
    pub usb_plugged: Option<bool>,
    /// Raw USB-C CC line readings `[cc1, cc2]`.
    pub usb_cc: Option<(u16, u16)>,
    /// Raw ADC readings.
    pub adc: Option<Vec<AdcReading>>,
    /// Lifetime heating-start statistics.
    pub heating_starts: Option<Vec<HeatingStartCount>>,
    /// Lifetime charge count.
    pub charge_cycles: Option<u16>,
    /// Battery health, percent.
    pub battery_health_pct: Option<u8>,
    /// State of charge, percent.
    pub battery_pct: Option<u8>,
}

/// A named raw ADC value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdcReading {
    /// Channel name (`Vtemp`, `USBcc1`, …).
    pub name: String,
    /// Raw value.
    pub value: u16,
}

/// A lifetime heating-start counter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatingStartCount {
    /// Stable id the UI localizes (`coldStart`, `hotStart`, …).
    pub key: String,
    /// How many times.
    pub count: u16,
}

/// Result of `send_raw`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawExchange {
    /// The frame sent, hex.
    pub sent: String,
    /// Every indication received in the reply window.
    pub replies: Vec<RawReply>,
}

/// One indication in a [`RawExchange`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawReply {
    /// Opcode.
    pub opcode: u8,
    /// The whole frame, hex.
    pub hex: String,
    /// The decoded notification (an object with a `"type"` key).
    pub decoded: serde_json::Map<String, serde_json::Value>,
}

/// The shape every command rejects with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Stable code the UI localizes.
    pub code: ErrorCode,
    /// English technical detail for logs/tooltips.
    pub detail: String,
}

/// Stable error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    /// No device connected.
    NotConnected,
    /// Another operation (flashing) holds the device.
    Busy,
    /// Scan found nothing.
    DeviceNotFound,
    /// Adapter off or permission denied.
    BluetoothUnavailable,
    /// The device did not answer in time.
    Timeout,
    /// The device answered with a non-zero status.
    DeviceRejected,
    /// The calibration table is unusable.
    CalibrationOutOfRange,
    /// Bad caller input.
    InvalidInput,
    /// Resend with `confirmDangerous: true`.
    DangerousFrame,
    /// Refused because a stick-limit cooldown is running.
    CooldownActive,
    /// Anything else.
    Internal,
}

/// An indication as received, for debugging consoles (`ploom watch`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedFrame {
    /// When it arrived, Unix ms.
    pub timestamp_ms: u64,
    /// The raw frame.
    pub bytes: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn round_trip<T>(value: &T, expected: &serde_json::Value)
    where
        T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_value(value).unwrap();
        assert_eq!(&json, expected);
        let back: T = serde_json::from_value(json).unwrap();
        assert_eq!(&back, value);
    }

    #[test]
    fn snapshot_keys() {
        let snapshot = Snapshot {
            connection: ConnectionState::Reconnecting,
            connection_error: Some(ErrorCode::DeviceNotFound),
            scanning: false,
            device: Some(DeviceIdentity {
                id: "1A2B".into(),
                name: Some("Ploom aura 123456VL".into()),
                hardware: None,
                firmware: Some("M1.0014_G4".into()),
                color: None,
            }),
            status: DeviceStatus {
                battery_pct: Some(85),
                heating_mode: Some(HeatingMode::BatterySaver),
                custom_profile: Some(CustomProfile {
                    base: HeatingMode::Intense,
                    hold_temp_c: 290,
                    hold_secs: 150,
                }),
                activity: Some(Activity::Heating),
                lock: Some(LockState::FeatureOff),
                enclosure_temp_c: Some(31.5),
                unacquired_logs: Some(LogCounts {
                    sessions: 6,
                    charges: 7,
                }),
                ..DeviceStatus::default()
            },
            operation: Some(Operation::FlashingProfile {
                target: ProfileTarget::Preset {
                    mode: HeatingMode::Delicate,
                },
                done: 3,
                total: 38,
            }),
            limit: LimitState {
                settings: LimitSettings {
                    enabled: true,
                    sticks: 10,
                    cooldown_minutes: 120,
                    window_hours: 24,
                },
                sticks_used: 7,
                window_started_at_ms: Some(1_800_000_000_000),
                locked_until_ms: None,
            },
        };
        round_trip(
            &snapshot,
            &json!({
                "connection": "reconnecting",
                "connectionError": "deviceNotFound",
                "scanning": false,
                "device": {
                    "id": "1A2B",
                    "name": "Ploom aura 123456VL",
                    "hardware": null,
                    "firmware": "M1.0014_G4",
                    "color": null
                },
                "status": {
                    "batteryPct": 85,
                    "batteryHealthPct": null,
                    "rssiDbm": null,
                    "heatingMode": "batterySaver",
                    "customProfile": { "base": "intense", "holdTempC": 290, "holdSecs": 150 },
                    "activity": "heating",
                    "sessionStartedAtMs": null,
                    "charging": null,
                    "usbPlugged": null,
                    "lock": "featureOff",
                    "autoStart": null,
                    "vibrating": false,
                    "temperatureWarning": null,
                    "enclosureTempC": 31.5,
                    "unacquiredLogs": { "sessions": 6, "charges": 7 }
                },
                "operation": {
                    "kind": "flashingProfile",
                    "target": { "kind": "preset", "mode": "delicate" },
                    "done": 3,
                    "total": 38
                },
                "limit": {
                    "settings": {
                        "enabled": true,
                        "sticks": 10,
                        "cooldownMinutes": 120,
                        "windowHours": 24
                    },
                    "sticksUsed": 7,
                    "windowStartedAtMs": 1_800_000_000_000_u64,
                    "lockedUntilMs": null
                }
            }),
        );
    }

    #[test]
    fn limit_events_and_error_code() {
        round_trip(
            &ActivityEvent {
                id: 9,
                timestamp_ms: 5,
                kind: ActivityKind::LimitReached {
                    sticks: 10,
                    locked_until_ms: 77,
                },
            },
            &json!({ "id": 9, "timestampMs": 5, "type": "limitReached", "sticks": 10, "lockedUntilMs": 77 }),
        );
        for (kind, name) in [
            (ActivityKind::CooldownEnded, "cooldownEnded"),
            (ActivityKind::Relocked, "relocked"),
        ] {
            round_trip(
                &ActivityEvent {
                    id: 1,
                    timestamp_ms: 2,
                    kind,
                },
                &json!({ "id": 1, "timestampMs": 2, "type": name }),
            );
        }
        round_trip(&ErrorCode::CooldownActive, &json!("cooldownActive"));
    }

    #[test]
    fn activity_event_is_flat() {
        round_trip(
            &ActivityEvent {
                id: 7,
                timestamp_ms: 1_700_000_000_000,
                kind: ActivityKind::Connected {
                    device_name: Some("Ploom aura".into()),
                },
            },
            &json!({ "id": 7, "timestampMs": 1_700_000_000_000_u64, "type": "connected", "deviceName": "Ploom aura" }),
        );
        round_trip(
            &ActivityEvent {
                id: 1,
                timestamp_ms: 2,
                kind: ActivityKind::ConnectionLost,
            },
            &json!({ "id": 1, "timestampMs": 2, "type": "connectionLost" }),
        );
        round_trip(
            &ActivityEvent {
                id: 3,
                timestamp_ms: 4,
                kind: ActivityKind::ProfileApplied {
                    target: ProfileTarget::Custom {
                        base: HeatingMode::Delicate,
                        hold_temp_c: 250,
                        hold_secs: 200,
                    },
                    confirmed: true,
                },
            },
            &json!({
                "id": 3, "timestampMs": 4, "type": "profileApplied", "confirmed": true,
                "target": { "kind": "custom", "base": "delicate", "holdTempC": 250, "holdSecs": 200 }
            }),
        );
        round_trip(
            &ActivityEvent {
                id: 5,
                timestamp_ms: 6,
                kind: ActivityKind::RawFrameSent {
                    hex: "02 e3 00".into(),
                    replies: 4,
                },
            },
            &json!({ "id": 5, "timestampMs": 6, "type": "rawFrameSent", "hex": "02 e3 00", "replies": 4 }),
        );
        round_trip(
            &ActivityEvent {
                id: 8,
                timestamp_ms: 9,
                kind: ActivityKind::AutoStartChanged { enabled: false },
            },
            &json!({ "id": 8, "timestampMs": 9, "type": "autoStartChanged", "enabled": false }),
        );
    }

    #[test]
    fn catalog_and_diagnostics_keys() {
        round_trip(
            &CustomLimits {
                temp_c: (240, 320),
                hold_secs: (30, 255),
                bases: vec![HeatingMode::Delicate, HeatingMode::BatterySaver],
            },
            &json!({ "tempC": [240, 320], "holdSecs": [30, 255], "bases": ["delicate", "batterySaver"] }),
        );
        round_trip(
            &DangerousOpcode {
                opcode: 0xA1,
                reason: DangerReason::DisablesBluetooth,
            },
            &json!({ "opcode": 161, "reason": "disablesBluetooth" }),
        );
        round_trip(
            &Diagnostics {
                usb_cc: Some((456, 28)),
                heating_starts: Some(vec![HeatingStartCount {
                    key: "coldStart".into(),
                    count: 3,
                }]),
                ..Diagnostics::default()
            },
            &json!({
                "enclosureTempC": null, "usbPlugged": null, "usbCc": [456, 28], "adc": null,
                "heatingStarts": [{ "key": "coldStart", "count": 3 }],
                "chargeCycles": null, "batteryHealthPct": null, "batteryPct": null
            }),
        );
        round_trip(
            &FramePreview {
                calibration_key: Some(1200),
                frames: vec![PreviewFrame {
                    hex: "02 a4 01".into(),
                    awaits_opcode: Some(0x3F),
                }],
            },
            &json!({ "calibrationKey": 1200, "frames": [{ "hex": "02 a4 01", "awaitsOpcode": 63 }] }),
        );
    }

    #[test]
    fn error_codes() {
        round_trip(
            &CommandError {
                code: ErrorCode::CalibrationOutOfRange,
                detail: "x".into(),
            },
            &json!({ "code": "calibrationOutOfRange", "detail": "x" }),
        );
        for (code, s) in [
            (ErrorCode::NotConnected, "notConnected"),
            (ErrorCode::BluetoothUnavailable, "bluetoothUnavailable"),
            (ErrorCode::DangerousFrame, "dangerousFrame"),
            (ErrorCode::DeviceRejected, "deviceRejected"),
        ] {
            assert_eq!(serde_json::to_value(code).unwrap(), json!(s));
        }
    }

    #[test]
    fn logs_keys() {
        round_trip(
            &LogsReport {
                session: Some(SessionLog {
                    start_counter: 13_232,
                    duration_s: 360,
                    puff_count: 8,
                    puff_offsets_s: vec![14, 93],
                    puff_stop_enabled: false,
                    auto_start_enabled: true,
                    lock_enabled: false,
                }),
                charge: None,
                unacquired: Some(LogCounts {
                    sessions: 1,
                    charges: 2,
                }),
            },
            &json!({
                "session": {
                    "startCounter": 13232, "durationS": 360, "puffCount": 8,
                    "puffOffsetsS": [14, 93], "puffStopEnabled": false,
                    "autoStartEnabled": true, "lockEnabled": false
                },
                "charge": null,
                "unacquired": { "sessions": 1, "charges": 2 }
            }),
        );
    }
}
