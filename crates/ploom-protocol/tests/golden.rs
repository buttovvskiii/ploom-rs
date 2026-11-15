//! Golden tests against vectors produced by the reference client.

use std::time::Duration;

use ploom_protocol::calibration::{self, CalibrationError, Masters};
use ploom_protocol::frame::{decode_hex, to_hex};
use ploom_protocol::heating::{self, CustomProfile, HeatingMode, HeatingProfile};
use ploom_protocol::notification::{
    ChargeMode, DeviceState, LockStatus, Notification, ProfileSource, ShakeSetting,
};
use ploom_protocol::status::{StatusUpdate, StatusWindow};
use serde_json::{Map, Value, json};

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap()
}

fn masters_values(v: &Value) -> Vec<u16> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| u16::try_from(x.as_u64().unwrap()).unwrap())
        .collect()
}

fn reference_mode(name: &str) -> HeatingMode {
    match name {
        "DELICATE" => HeatingMode::Delicate,
        "BALANCED" => HeatingMode::Balanced,
        "INTENSE" => HeatingMode::Intense,
        "BATTERY_SAVER" => HeatingMode::BatterySaver,
        other => panic!("unknown reference mode {other}"),
    }
}

fn reference_mode_name(mode: HeatingMode) -> &'static str {
    match mode {
        HeatingMode::Delicate => "DELICATE",
        HeatingMode::Balanced => "BALANCED",
        HeatingMode::Intense => "INTENSE",
        HeatingMode::BatterySaver => "BATTERY_SAVER",
    }
}

#[test]
fn build_frames_matches_reference() {
    let cases = fixture("build_frames.json");
    let cases = cases.as_array().unwrap();
    assert!(cases.len() > 100, "fixture unexpectedly small");
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let mode = reference_mode(case["mode"].as_str().unwrap());
        let result = &case["result"];

        let masters = match Masters::new(masters_values(&case["masters"])) {
            Ok(m) => m,
            Err(e) => {
                let expected = result["error"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{name}: {e}"));
                let actual = match e {
                    CalibrationError::TooShort { .. } => "tooShort",
                    CalibrationError::ZeroDivisor => "zeroDivisor",
                    other => panic!("{name}: unexpected {other}"),
                };
                assert_eq!(actual, expected, "{name}");
                continue;
            }
        };

        let profile = if case["custom"].is_null() {
            HeatingProfile::preset(mode).unwrap()
        } else {
            let temp = case["custom"]["temp"].as_i64().unwrap();
            let secs = case["custom"]["secs"].as_i64().unwrap();
            HeatingProfile::custom(&CustomProfile::new(mode, temp, secs).unwrap())
        };

        match heating::build_profile_frames(&profile, &masters) {
            Ok(frames) => {
                let expected = result["frames"].as_array().unwrap_or_else(|| {
                    panic!(
                        "{name}: reference failed with {}, we succeeded",
                        result["error"]
                    )
                });
                assert_eq!(frames.len(), expected.len(), "{name}: frame count");
                for (i, (ours, theirs)) in frames.iter().zip(expected).enumerate() {
                    let bytes = decode_hex(theirs["hex"].as_str().unwrap()).unwrap();
                    assert_eq!(
                        ours.frame.as_bytes(),
                        bytes,
                        "{name}: frame {i}\n ours:   {}\n theirs: {}",
                        ours.frame,
                        to_hex(&bytes)
                    );
                    let awaits = theirs["awaits"].as_u64().map(|v| u8::try_from(v).unwrap());
                    assert_eq!(ours.awaits, awaits, "{name}: frame {i} awaited reply");
                }
            }
            Err(CalibrationError::OutOfRange { key }) => {
                assert_eq!(result["error"], "outOfRange", "{name}");
                assert_eq!(key, masters.calibration_key());
            }
            Err(e) => panic!("{name}: unexpected {e}"),
        }
    }
}

#[test]
fn get_res_matches_reference() {
    for set in fixture("get_res.json").as_array().unwrap() {
        let masters = Masters::new(masters_values(&set["masters"])).unwrap();
        for pair in set["results"].as_array().unwrap() {
            let temp = i32::try_from(pair[0].as_i64().unwrap()).unwrap();
            let expected = pair[1].as_i64().unwrap();
            assert_eq!(
                heating::get_res(temp, &masters),
                expected,
                "temp {temp}, masters {:?}",
                masters.values()
            );
        }
    }
}

#[test]
fn half_rounding_vectors_are_present() {
    // Guard against the fixture losing the cases that distinguish Dart's
    // round-half-away-from-zero from round-half-to-even.
    let sets = fixture("get_res.json");
    let crafted = sets
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["masters"][4] == 2695 && s["masters"][3] == 3001)
        .expect("half@270 masters");
    let at_270 = crafted["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p[0] == 270)
        .unwrap();
    assert_eq!(at_270[1], 3003, "3001 + 1.5 must round away from zero");
}

#[test]
fn leaflet_const_matches_reference() {
    for pair in fixture("leaflet.json").as_array().unwrap() {
        let key = u16::try_from(pair[0].as_u64().unwrap()).unwrap();
        let expected = pair[1].as_u64().map(|v| u8::try_from(v).unwrap());
        assert_eq!(
            calibration::leaflet_const(key),
            expected,
            "masters[7] = {key}"
        );
    }
}

#[test]
fn parse_matches_reference() {
    for case in fixture("parse.json").as_array().unwrap() {
        let hex = case["hex"].as_str().unwrap();
        let n = Notification::parse(&decode_hex(hex).unwrap()).unwrap();
        assert_eq!(
            Value::Object(reference_notification(&n)),
            case["decoded"],
            "frame {hex} decoded as {n:?}"
        );
    }
}

#[test]
fn every_reference_opcode_is_typed() {
    // Frames the reference decodes must not fall through to `Unknown`.
    for case in fixture("parse.json").as_array().unwrap() {
        let has_fields = !case["decoded"].as_object().unwrap().is_empty();
        let n = Notification::parse(&decode_hex(case["hex"].as_str().unwrap()).unwrap()).unwrap();
        if has_fields {
            assert!(!matches!(n, Notification::Unknown { .. }), "{n:?}");
        }
    }
}

#[test]
fn merge_window_matches_reference() {
    for case in fixture("merge_window.json").as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let mut window = StatusWindow::new(Duration::from_secs(15));
        let now = case["nowMs"].as_u64().unwrap();
        for f in case["frames"].as_array().unwrap() {
            let raw = decode_hex(f["hex"].as_str().unwrap()).unwrap();
            let n = Notification::parse(&raw).unwrap();
            let t = f["tMs"].as_u64().unwrap();
            assert!(t <= now, "{name}: fixture frame from the future");
            window.push(t, StatusUpdate::from_notification(&n));
        }
        let merged = window.merged(now);
        let mut ours = reference_status(&merged);
        if let Some(mode) = merged.effective_mode() {
            ours.insert("heating_mode".into(), json!(reference_mode_name(mode)));
        }
        assert_eq!(Value::Object(ours), case["merged"], "{name}");
    }
}

fn lock(l: LockStatus) -> Value {
    match l {
        LockStatus::Locked => json!("locked"),
        LockStatus::Unlocked => json!("unlocked"),
        LockStatus::FeatureOff => json!("feature_off"),
        LockStatus::Unknown(b) => json!(b),
    }
}

fn shake(s: ShakeSetting) -> Value {
    match s {
        ShakeSetting::Off => json!("off"),
        ShakeSetting::ShakeOn => json!("shakeOn"),
        ShakeSetting::TapOn => json!("tapOn"),
        ShakeSetting::ShakeOrTapOn => json!("shakeOrTapOn"),
        ShakeSetting::Disabled => json!("disabled"),
        ShakeSetting::Unknown(b) => json!(b),
    }
}

fn source(p: ProfileSource) -> Value {
    match p {
        ProfileSource::Factory => json!("factory"),
        ProfileSource::Downloaded => json!("downloaded"),
        ProfileSource::P2p => json!("p2p"),
        ProfileSource::Unknown(b) => json!(b),
    }
}

fn charge_mode(m: ChargeMode) -> Value {
    match m {
        ChargeMode::Mode0 => json!("mode0"),
        ChargeMode::Mode1 => json!("mode1"),
        ChargeMode::Mode2 => json!("mode2"),
        ChargeMode::Mode3 => json!("mode3"),
        ChargeMode::Mode4 => json!("mode4"),
        ChargeMode::Mode5 => json!("mode5"),
        ChargeMode::Other => json!("other"),
        ChargeMode::NoConnect => json!("noConnect"),
        ChargeMode::Unknown(b) => json!(b),
    }
}

fn state(s: DeviceState) -> Value {
    match s {
        DeviceState::Standby => json!("standby"),
        DeviceState::AutomaticHeating => json!("automaticHeating"),
        DeviceState::Preheating => json!("preheating"),
        DeviceState::Smoking => json!("smoking"),
        DeviceState::Charging => json!("charging"),
        DeviceState::TemperatureError => json!("temperatureError"),
        DeviceState::AlarmError => json!("alarmError"),
        DeviceState::Unknown(b) => json!(format!("unknown:0x{b:02x}")),
    }
}

fn reference_notification(n: &Notification) -> Map<String, Value> {
    let v = match n {
        Notification::Status(s) => {
            let mut v = json!({
                "uptime_s": s.uptime_s,
                "is_temperature_error": s.temperature_error,
                "is_in_use": s.in_use,
                "is_preheating": s.preheating,
                "lock": lock(s.lock),
                "shake": shake(s.shake),
                "auto_start_heating": s.auto_start,
                "end_by_puff": s.end_by_puff,
                "profile_source": source(s.profile_source),
                "last_cigarette_allowed": s.last_stick_allowed,
            });
            if s.charging_bit {
                v["is_charging"] = json!(true);
            }
            v
        }
        Notification::Configuration(c) => json!({
            "is_temperature_error": c.temperature_error,
            "lock": lock(c.lock),
            "shake": shake(c.shake),
            "auto_start_heating": c.auto_start,
            "end_by_puff": c.end_by_puff,
            "profile_source": source(c.profile_source),
            "last_cigarette_allowed": c.last_stick_allowed,
        }),
        Notification::StateChanged { state: s } => json!({ "state": state(*s) }),
        Notification::BatteryHealth { pct } => json!({ "battery_health_pct": pct }),
        Notification::ChargingStatus { mode } => {
            json!({ "charge_mode": charge_mode(*mode), "is_charging": mode.is_charging() })
        }
        Notification::UnacquiredLogs { sessions, charges } => {
            json!({ "unacquired_logs": [sessions, charges] })
        }
        Notification::SmokingLog(l) => json!({
            "smoking_start": l.start_counter,
            "session_time": l.duration_s,
            "total_times_smoked": l.puff_count,
            "puff_stop_enabled": l.puff_stop_enabled,
            "heating_start_condition": l.heating_start_condition,
            "is_auto_on_enabled": l.auto_start_enabled,
            "is_lock_function_enabled": l.lock_enabled,
            "unlock_method": l.unlock_method,
            "downloaded_profile": l.downloaded_profile,
            "total_time": l.total_time,
        }),
        Notification::PuffTimings(p) => {
            let puffs: Map<String, Value> = p
                .puffs
                .iter()
                .map(|(i, s)| (i.to_string(), json!(s)))
                .collect();
            json!({ "puffs": puffs })
        }
        Notification::SmokingEnergy(e) => json!({
            "energy_50ms": e.energy_50ms,
            "stpe0_segment_energy": e.step0_segment_energy,
            "mv_avg0_at_start": e.mv_avg0_at_start,
            "mv_avg2_at_start": e.mv_avg2_at_start,
            "energy_200ms": e.energy_200ms,
        }),
        Notification::ChargeLog(c) => json!({
            "charge_start": c.start_counter,
            "charge_end": c.end_counter,
            "mv_start": c.start_mv,
            "mv_end": c.end_mv,
            "temp_start": c.start_temp_raw,
            "temp_end": c.end_temp_raw,
        }),
        Notification::BatteryLevel { pct } => json!({ "battery_pct": pct }),
        Notification::ProfileActive { downloaded } => {
            json!({ "profile_downloaded_active": downloaded })
        }
        Notification::DownloadedProfile { number } => json!({
            "heating_mode_downloaded":
                reference_mode_name(HeatingMode::from_profile_number(*number)),
        }),
        Notification::ChargeLogSummary(s) => json!({
            "total_charge_count": s.total_charge_count,
            "charge_mode_raw": s.charge_mode_raw,
            "soh": s.state_of_health_pct,
        }),
        Notification::ErrorLog { payload, .. } => json!({ "error_log": to_hex(payload) }),
        Notification::Vibration { on } => json!({ "is_vibrating": on }),
        Notification::Lock { status } => json!({ "lock": lock(*status) }),
        Notification::Rssi { dbm } => json!({ "rssi_dbm": dbm }),
        // Decoded here but not by the reference.
        _ => json!({}),
    };
    match v {
        Value::Object(map) => map,
        _ => unreachable!(),
    }
}

fn reference_status(u: &StatusUpdate) -> Map<String, Value> {
    let mut m = Map::new();
    let mut put = |k: &str, v: Option<Value>| {
        if let Some(v) = v {
            m.insert(k.to_owned(), v);
        }
    };
    put("battery_pct", u.battery_pct.map(|v| json!(v)));
    put("battery_health_pct", u.battery_health_pct.map(|v| json!(v)));
    put("rssi_dbm", u.rssi_dbm.map(|v| json!(v)));
    put("is_vibrating", u.vibrating.map(|v| json!(v)));
    put("state", u.state.map(state));
    put("lock", u.lock.map(lock));
    put("charge_mode", u.charge_mode.map(charge_mode));
    put("is_charging", u.charging.map(|v| json!(v)));
    put(
        "profile_downloaded_active",
        u.profile_downloaded_active.map(|v| json!(v)),
    );
    put(
        "heating_mode_downloaded",
        u.downloaded_mode.map(|v| json!(reference_mode_name(v))),
    );
    put("uptime_s", u.uptime_s.map(|v| json!(v)));
    put(
        "is_temperature_error",
        u.temperature_error.map(|v| json!(v)),
    );
    put("is_in_use", u.in_use.map(|v| json!(v)));
    put("is_preheating", u.preheating.map(|v| json!(v)));
    put("shake", u.shake.map(shake));
    put("auto_start_heating", u.auto_start.map(|v| json!(v)));
    put("end_by_puff", u.end_by_puff.map(|v| json!(v)));
    put("profile_source", u.profile_source.map(source));
    put(
        "last_cigarette_allowed",
        u.last_stick_allowed.map(|v| json!(v)),
    );
    put(
        "unacquired_logs",
        u.unacquired_logs.map(|(s, c)| json!([s, c])),
    );
    m
}
