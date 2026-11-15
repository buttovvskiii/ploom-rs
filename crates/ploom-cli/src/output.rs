//! Human-readable rendering.

use std::fmt::Write as _;

use ploom_core::model::{
    Activity, ActivityEvent, ActivityKind, DeviceStatus, Diagnostics, DiscoveredDevice,
    FramePreview, LimitState, LockState, LogsReport, ProfileTarget, RawExchange, Snapshot,
};
use ploom_protocol::HeatingMode;

const LABEL: usize = 22;

fn row(out: &mut String, label: &str, value: impl std::fmt::Display) {
    let _ = writeln!(out, "  {label:<LABEL$}{value}");
}

fn opt<T: std::fmt::Display>(v: Option<T>, unit: &str) -> String {
    v.map_or_else(|| "?".to_owned(), |v| format!("{v}{unit}"))
}

fn yes_no(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => "yes",
        Some(false) => "no",
        None => "?",
    }
}

fn activity(a: Option<Activity>) -> &'static str {
    match a {
        Some(Activity::Standby) => "standby",
        Some(Activity::Preheating) => "preheating",
        Some(Activity::Heating) => "heating",
        Some(Activity::Charging) => "charging",
        None => "?",
    }
}

fn lock(l: Option<LockState>) -> &'static str {
    match l {
        Some(LockState::Locked) => "locked",
        Some(LockState::Unlocked) => "unlocked",
        Some(LockState::FeatureOff) => "lock feature off",
        None => "?",
    }
}

pub(crate) fn mode(status: &DeviceStatus) -> String {
    match (status.heating_mode, status.custom_profile) {
        (Some(m), Some(c)) => {
            format!(
                "{} (custom: {} °C × {} s)",
                m.title(),
                c.hold_temp_c,
                c.hold_secs
            )
        }
        (Some(m), None) => m.title().to_owned(),
        (None, _) => "?".to_owned(),
    }
}

pub(crate) fn target(t: &ProfileTarget) -> String {
    match t {
        ProfileTarget::Preset { mode } => mode.title().to_owned(),
        ProfileTarget::Custom {
            base,
            hold_temp_c,
            hold_secs,
        } => {
            format!(
                "custom {} profile, hold {hold_temp_c} °C × {hold_secs} s",
                base.title()
            )
        }
    }
}

pub(crate) fn limit(l: &LimitState, now_ms: u64) -> String {
    let mut out = String::new();
    let s = l.settings;
    row(
        &mut out,
        "stick limit",
        if s.enabled { "on" } else { "off" },
    );
    row(&mut out, "sticks", s.sticks);
    row(&mut out, "cooldown", format!("{} min", s.cooldown_minutes));
    row(
        &mut out,
        "counter resets after",
        format!("{} h", s.window_hours),
    );
    if s.enabled {
        row(
            &mut out,
            "used",
            format!("{} of {}", l.sticks_used.min(s.sticks), s.sticks),
        );
    }
    match l.locked_until_ms {
        Some(until) if until > now_ms => {
            let mins = (until - now_ms).div_ceil(60_000);
            row(&mut out, "locked for", format!("{mins} more min"));
        }
        Some(_) => row(&mut out, "cooldown over", "unlocks on the next connection"),
        None => {}
    }
    out
}

pub(crate) fn status(s: &Snapshot) -> String {
    let mut out = String::new();
    if let Some(d) = &s.device {
        row(&mut out, "device", d.name.as_deref().unwrap_or("Ploom"));
        row(&mut out, "id", &d.id);
        row(&mut out, "firmware", d.firmware.as_deref().unwrap_or("?"));
        row(&mut out, "hardware", d.hardware.as_deref().unwrap_or("?"));
        if let Some(c) = &d.color {
            row(
                &mut out,
                "color",
                format!("{} [{}] {}", c.name, c.code, c.hex),
            );
        }
        out.push('\n');
    }
    let st = &s.status;
    row(&mut out, "battery", opt(st.battery_pct, "%"));
    row(&mut out, "battery health", opt(st.battery_health_pct, "%"));
    row(&mut out, "signal", opt(st.rssi_dbm, " dBm"));
    row(&mut out, "heating mode", mode(st));
    row(&mut out, "state", activity(st.activity));
    if let Some(started) = st.session_started_at_ms {
        row(&mut out, "session started", format!("{started} (unix ms)"));
    }
    row(&mut out, "charging", yes_no(st.charging));
    row(&mut out, "usb-c plugged", yes_no(st.usb_plugged));
    row(&mut out, "lock", lock(st.lock));
    row(&mut out, "auto-start heating", yes_no(st.auto_start));
    row(
        &mut out,
        "too warm to start",
        yes_no(st.temperature_warning),
    );
    row(
        &mut out,
        "enclosure",
        st.enclosure_temp_c
            .map_or("?".into(), |t| format!("{t:.1} °C")),
    );
    if let Some(l) = st.unacquired_logs {
        row(
            &mut out,
            "logs not yet read",
            format!("{} sessions, {} charges", l.sessions, l.charges),
        );
    }
    out
}

pub(crate) fn battery(st: &DeviceStatus) -> String {
    let mut out = String::new();
    row(&mut out, "battery", opt(st.battery_pct, "%"));
    row(&mut out, "battery health", opt(st.battery_health_pct, "%"));
    row(&mut out, "charging", yes_no(st.charging));
    out
}

fn bars(rssi: Option<i16>) -> String {
    let Some(rssi) = rssi else {
        return "    ".into();
    };
    let n = usize::try_from(((rssi + 90) / 10).clamp(0, 4)).unwrap_or(0);
    format!("{}{}", "▮".repeat(n), "▯".repeat(4 - n))
}

pub(crate) fn scan(devices: &[DiscoveredDevice]) -> String {
    if devices.is_empty() {
        return "  No Ploom found nearby. Wake the device with its button and try again.\n".into();
    }
    let width = devices
        .iter()
        .map(|d| d.name.chars().count())
        .max()
        .unwrap_or(0)
        .max(4);
    let mut out = format!("  {:<width$}  {:>8}        {}\n", "NAME", "SIGNAL", "ID");
    for d in devices {
        let _ = writeln!(
            out,
            "  {:<width$}  {:>8}  {}  {}{}",
            d.name,
            opt(d.rssi_dbm, " dBm"),
            bars(d.rssi_dbm),
            d.id,
            if d.connected { "  (connected)" } else { "" }
        );
    }
    out
}

pub(crate) fn preview(p: &FramePreview) -> String {
    let mut out = String::new();
    if let Some(key) = p.calibration_key {
        let _ = writeln!(out, "  calibration key masters[7] = {key}");
    }
    let _ = writeln!(out, "  {} frame(s):", p.frames.len());
    for f in &p.frames {
        let _ = write!(out, "    {}", f.hex);
        if let Some(op) = f.awaits_opcode {
            let _ = write!(out, "   -> awaits 0x{op:02x}");
        }
        out.push('\n');
    }
    out
}

pub(crate) fn logs(r: &LogsReport) -> String {
    let mut out = String::new();
    match &r.session {
        Some(s) => {
            let _ = writeln!(out, "  Oldest session log");
            row(&mut out, "started at counter", s.start_counter);
            row(&mut out, "duration", format!("{} s", s.duration_s));
            row(&mut out, "puffs", s.puff_count);
            let offsets: Vec<String> = s.puff_offsets_s.iter().map(|o| format!("{o}s")).collect();
            row(
                &mut out,
                "puff times",
                if offsets.is_empty() {
                    "-".into()
                } else {
                    offsets.join(" ")
                },
            );
            row(
                &mut out,
                "auto-start enabled",
                yes_no(Some(s.auto_start_enabled)),
            );
            row(&mut out, "lock enabled", yes_no(Some(s.lock_enabled)));
            row(
                &mut out,
                "puff stop enabled",
                yes_no(Some(s.puff_stop_enabled)),
            );
        }
        None => out.push_str("  No session log.\n"),
    }
    out.push('\n');
    match &r.charge {
        Some(c) => {
            let _ = writeln!(out, "  Oldest charge log");
            row(
                &mut out,
                "counter",
                format!("{} → {}", c.start_counter, c.end_counter),
            );
            row(
                &mut out,
                "voltage",
                format!("{} → {} mV", c.start_mv, c.end_mv),
            );
            row(&mut out, "charge cycles", opt(c.total_charge_count, ""));
            row(&mut out, "battery health", opt(c.state_of_health_pct, "%"));
        }
        None => out.push_str("  No charge log.\n"),
    }
    if let Some(u) = r.unacquired {
        let _ = writeln!(
            out,
            "\n  {} session and {} charge log(s) are queued on the device. It only ever\n  \
             returns the oldest one; the queue advances when logs are deleted (raw 02c400\n  \
             --force), which is irreversible.",
            u.sessions, u.charges
        );
    }
    out
}

pub(crate) fn diagnostics(d: &Diagnostics) -> String {
    let mut out = String::new();
    row(
        &mut out,
        "enclosure",
        d.enclosure_temp_c
            .map_or("? (sensor asleep)".into(), |t| format!("{t:.1} °C")),
    );
    let cc = d
        .usb_cc
        .map_or(String::new(), |(a, b)| format!("   (CC {a}/{b})"));
    row(
        &mut out,
        "usb-c plugged",
        format!("{}{cc}", yes_no(d.usb_plugged)),
    );
    row(&mut out, "battery", opt(d.battery_pct, "%"));
    row(&mut out, "battery health", opt(d.battery_health_pct, "%"));
    row(&mut out, "charge cycles", opt(d.charge_cycles, ""));
    if let Some(starts) = &d.heating_starts {
        out.push_str("\n  Heating starts (lifetime)\n");
        let total: u32 = starts.iter().map(|s| u32::from(s.count)).sum();
        for s in starts {
            row(&mut out, &format!("  {}", s.key), s.count);
        }
        row(&mut out, "  total", total);
    }
    if let Some(adc) = &d.adc {
        let raw: Vec<String> = adc
            .iter()
            .map(|a| format!("{}={}", a.name, a.value))
            .collect();
        let _ = writeln!(out, "\n  Raw ADC: {}", raw.join("  "));
    }
    out
}

pub(crate) fn raw(x: &RawExchange) -> String {
    let mut out = format!("  --> {}\n", x.sent);
    if x.replies.is_empty() {
        out.push_str("  (no reply)\n");
    }
    for r in &x.replies {
        let _ = writeln!(
            out,
            "  <-- {}   {}",
            r.hex,
            serde_json::Value::Object(r.decoded.clone())
        );
    }
    out
}

pub(crate) fn event(e: &ActivityEvent) -> String {
    let text = match &e.kind {
        ActivityKind::Connected { device_name } => {
            format!(
                "connected to {}",
                device_name.as_deref().unwrap_or("device")
            )
        }
        ActivityKind::ConnectionLost => "connection lost, reconnecting".into(),
        ActivityKind::Disconnected => "disconnected".into(),
        ActivityKind::StateChanged { activity: a } => format!("state: {}", activity(Some(*a))),
        ActivityKind::ChargingChanged { charging } => {
            if *charging {
                "charging started".into()
            } else {
                "charging stopped".into()
            }
        }
        ActivityKind::VibrationChanged { on } => {
            format!("buzzer {}", if *on { "on" } else { "off" })
        }
        ActivityKind::LockChanged { lock: l } => format!("lock: {}", lock(Some(*l))),
        ActivityKind::AutoStartChanged { enabled } => {
            format!("auto-start heating {}", if *enabled { "on" } else { "off" })
        }
        ActivityKind::ProfileApplied {
            target: t,
            confirmed,
        } => format!(
            "profile applied: {}{}",
            target(t),
            if *confirmed { "" } else { " (unconfirmed)" }
        ),
        ActivityKind::SessionLogReceived => "session log received".into(),
        ActivityKind::LimitReached {
            sticks,
            locked_until_ms,
        } => format!("stick limit of {sticks} reached; locked until {locked_until_ms} (unix ms)"),
        ActivityKind::CooldownEnded => "cooldown over; device released".into(),
        ActivityKind::Relocked => "unlocked elsewhere during a cooldown; locked again".into(),
        ActivityKind::RawFrameSent { hex, replies } => format!("raw {hex} ({replies} replies)"),
    };
    format!("· {text}")
}

pub(crate) fn flash_warning(t: &ProfileTarget) -> String {
    let is_factory = matches!(
        t,
        ProfileTarget::Preset {
            mode: HeatingMode::Balanced
        }
    );
    if is_factory {
        return String::new();
    }
    format!(
        "  Flashing {}: 38 frames with a heating curve are written to the device.\n  \
         To roll back, switch to any preset or `ploom mode balanced` (factory profile).\n",
        target(t)
    )
}

pub(crate) fn progress(done: u32, total: u32) -> String {
    const WIDTH: u32 = 30;
    let filled = (done.min(total) * WIDTH).checked_div(total).unwrap_or(0);
    format!(
        "  [{}{}] {done}/{total}",
        "#".repeat(filled as usize),
        "-".repeat((WIDTH - filled) as usize)
    )
}

#[cfg(test)]
mod tests {
    use ploom_core::model::{
        AdcReading, CustomProfile, DeviceIdentity, HeatingStartCount, LogCounts, PreviewFrame,
    };

    use super::*;

    #[test]
    fn limit_shows_progress_and_cooldown() {
        let mut l = LimitState::default();
        let text = limit(&l, 0);
        assert!(text.contains("stick limit           off"), "{text}");
        assert!(!text.contains("used"), "{text}");
        l.settings.enabled = true;
        l.sticks_used = 12;
        l.locked_until_ms = Some(10 * 60_000 + 1);
        let text = limit(&l, 0);
        assert!(text.contains("used                  10 of 10"), "{text}");
        assert!(text.contains("locked for            11 more min"), "{text}");
        assert!(limit(&l, 11 * 60_000).contains("unlocks on the next connection"));
    }

    #[test]
    fn status_is_aligned() {
        let s = Snapshot {
            device: Some(DeviceIdentity {
                id: "ABC".into(),
                name: Some("Ploom aura X".into()),
                hardware: None,
                firmware: Some("M1.0014_G4".into()),
                color: None,
            }),
            status: DeviceStatus {
                battery_pct: Some(80),
                heating_mode: Some(HeatingMode::Intense),
                custom_profile: Some(CustomProfile {
                    base: HeatingMode::Intense,
                    hold_temp_c: 290,
                    hold_secs: 150,
                }),
                activity: Some(Activity::Standby),
                lock: Some(LockState::Unlocked),
                enclosure_temp_c: Some(26.54),
                unacquired_logs: Some(LogCounts {
                    sessions: 1,
                    charges: 2,
                }),
                ..DeviceStatus::default()
            },
            ..Snapshot::default()
        };
        let text = status(&s);
        assert!(text.contains("  battery               80%\n"), "{text}");
        assert!(
            text.contains("  heating mode          Intense (custom: 290 °C × 150 s)\n"),
            "{text}"
        );
        assert!(text.contains("  enclosure             26.5 °C\n"), "{text}");
        assert!(text.contains("  charging              ?\n"), "{text}");
        assert!(
            text.contains("  firmware              M1.0014_G4\n"),
            "{text}"
        );
        for line in text.lines().filter(|l| !l.is_empty()) {
            let bytes = line.as_bytes();
            assert!(
                bytes[LABEL + 1] == b' ' && bytes[LABEL + 2] != b' ',
                "misaligned: {line:?}"
            );
        }
    }

    #[test]
    fn scan_table() {
        let text = scan(&[
            DiscoveredDevice {
                id: "1".into(),
                name: "Ploom aura A".into(),
                rssi_dbm: Some(-52),
                connected: true,
            },
            DiscoveredDevice {
                id: "2".into(),
                name: "Ploom aura LONGER".into(),
                rssi_dbm: None,
                connected: false,
            },
        ]);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "  NAME                 SIGNAL        ID");
        assert_eq!(
            lines[1],
            "  Ploom aura A        -52 dBm  ▮▮▮▯  1  (connected)"
        );
        assert_eq!(lines[2], "  Ploom aura LONGER         ?        2");
        assert!(scan(&[]).contains("No Ploom found"));
    }

    #[test]
    fn signal_bars() {
        assert_eq!(bars(Some(-40)), "▮▮▮▮");
        assert_eq!(bars(Some(-95)), "▯▯▯▯");
        assert_eq!(bars(Some(-75)), "▮▯▯▯");
    }

    #[test]
    fn preview_lists_awaited_replies() {
        let text = preview(&FramePreview {
            calibration_key: Some(1200),
            frames: vec![
                PreviewFrame {
                    hex: "02 d7 00".into(),
                    awaits_opcode: None,
                },
                PreviewFrame {
                    hex: "02 a4 01".into(),
                    awaits_opcode: Some(0x3F),
                },
            ],
        });
        assert!(text.contains("masters[7] = 1200"));
        assert!(text.contains("    02 a4 01   -> awaits 0x3f\n"));
    }

    #[test]
    fn diagnostics_text() {
        let text = diagnostics(&Diagnostics {
            usb_plugged: Some(true),
            usb_cc: Some((456, 28)),
            adc: Some(vec![AdcReading {
                name: "USBcc1".into(),
                value: 456,
            }]),
            heating_starts: Some(vec![
                HeatingStartCount {
                    key: "coldStart".into(),
                    count: 3,
                },
                HeatingStartCount {
                    key: "hotStart".into(),
                    count: 2,
                },
            ]),
            ..Diagnostics::default()
        });
        assert!(
            text.contains("usb-c plugged         yes   (CC 456/28)"),
            "{text}"
        );
        assert!(
            text.contains("enclosure             ? (sensor asleep)"),
            "{text}"
        );
        assert!(text.contains("  total               5"), "{text}");
        assert!(text.contains("Raw ADC: USBcc1=456"), "{text}");
    }

    #[test]
    fn progress_bar() {
        assert_eq!(progress(0, 38), format!("  [{}] 0/38", "-".repeat(30)));
        assert_eq!(progress(38, 38), format!("  [{}] 38/38", "#".repeat(30)));
        assert_eq!(progress(1, 0), format!("  [{}] 1/0", "-".repeat(30)));
    }

    #[test]
    fn events_and_warning() {
        let e = ActivityEvent {
            id: 1,
            timestamp_ms: 0,
            kind: ActivityKind::ProfileApplied {
                target: ProfileTarget::Preset {
                    mode: HeatingMode::BatterySaver,
                },
                confirmed: false,
            },
        };
        assert_eq!(event(&e), "· profile applied: Battery Saver (unconfirmed)");
        assert!(
            flash_warning(&ProfileTarget::Preset {
                mode: HeatingMode::Balanced
            })
            .is_empty()
        );
        assert!(
            flash_warning(&ProfileTarget::Preset {
                mode: HeatingMode::Delicate
            })
            .contains("38 frames")
        );
    }
}
