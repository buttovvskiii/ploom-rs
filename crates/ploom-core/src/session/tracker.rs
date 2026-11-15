//! Derives the UI's [`DeviceStatus`] from indications and side channels.

use ploom_protocol::notification::{DeviceState, LockStatus, Notification};
use ploom_protocol::status::{StatusUpdate, StatusWindow};

use crate::model::{Activity, CustomProfile, DeviceStatus, LockState, LogCounts};

const CHARGE_TREND_POINTS: u8 = 2;

#[derive(Debug, Default)]
pub(crate) struct StatusTracker {
    window: StatusWindow,
    latest: StatusUpdate,
    usb_plugged: Option<bool>,
    enclosure_temp_c: Option<f64>,
    battery_baseline: Option<u8>,
    charging_trend: bool,
    session_started_at_ms: Option<u64>,
    custom_profile: Option<CustomProfile>,
}

impl StatusTracker {
    pub(crate) fn new(custom: Option<CustomProfile>) -> Self {
        Self {
            custom_profile: custom,
            ..Self::default()
        }
    }

    pub(crate) fn ingest(&mut self, mono_ms: u64, wall_ms: u64, n: &Notification) {
        let update = StatusUpdate::from_notification(n);
        if update.is_empty() {
            return;
        }
        self.window.push(mono_ms, update);
        let merged = self.window.merged(mono_ms);
        self.latest.apply(&merged);
        match n {
            Notification::StateChanged {
                state: DeviceState::Standby,
            } => self.reset_trend(),
            Notification::ChargingStatus { mode } if !mode.is_charging() => self.reset_trend(),
            _ => {}
        }
        self.track_session(wall_ms);
    }

    pub(crate) fn set_usb_plugged(&mut self, plugged: bool) {
        self.usb_plugged = Some(plugged);
        if !plugged {
            self.latest.charging = Some(false);
            self.reset_trend();
        }
    }

    pub(crate) fn set_enclosure_temp_c(&mut self, celsius: f64) {
        self.enclosure_temp_c = Some(celsius);
    }

    pub(crate) fn after_poll(&mut self, wall_ms: u64) {
        if let Some(pct) = self.latest.battery_pct {
            if self.latest.charging == Some(true) {
                self.battery_baseline = Some(pct);
            } else {
                match self.battery_baseline {
                    None => self.battery_baseline = Some(pct),
                    Some(base) if pct >= base.saturating_add(CHARGE_TREND_POINTS) => {
                        self.charging_trend = true;
                    }
                    Some(base) if pct < base => {
                        self.battery_baseline = Some(pct);
                        self.charging_trend = false;
                    }
                    Some(_) => {}
                }
            }
        }
        self.track_session(wall_ms);
    }

    pub(crate) fn set_custom_profile(&mut self, custom: Option<CustomProfile>) {
        self.custom_profile = custom;
    }

    pub(crate) fn is_heating(&self) -> bool {
        self.latest.is_heating()
    }

    pub(crate) fn battery(&self) -> (Option<u8>, Option<u8>) {
        (self.latest.battery_pct, self.latest.battery_health_pct)
    }

    fn reset_trend(&mut self) {
        self.charging_trend = false;
        self.battery_baseline = self.latest.battery_pct;
    }

    fn track_session(&mut self, wall_ms: u64) {
        if self.is_heating() {
            self.session_started_at_ms.get_or_insert(wall_ms);
        } else {
            self.session_started_at_ms = None;
        }
    }

    fn charging(&self) -> Option<bool> {
        if let Some(plugged) = self.usb_plugged {
            return Some(plugged);
        }
        if self.latest.charging.is_some() || self.charging_trend {
            return Some(self.latest.charging == Some(true) || self.charging_trend);
        }
        self.latest.battery_pct.map(|_| false)
    }

    fn activity(&self) -> Option<Activity> {
        let charging = self.charging();
        if self.latest.preheating == Some(true) {
            Some(Activity::Preheating)
        } else if self.latest.in_use == Some(true) {
            Some(Activity::Heating)
        } else if charging == Some(true) {
            Some(Activity::Charging)
        } else if self.latest.preheating.is_some()
            || self.latest.in_use.is_some()
            || charging.is_some()
        {
            Some(Activity::Standby)
        } else {
            None
        }
    }

    pub(crate) fn status(&self) -> DeviceStatus {
        let heating_mode = self.latest.effective_mode();
        DeviceStatus {
            battery_pct: self.latest.battery_pct,
            battery_health_pct: self.latest.battery_health_pct,
            rssi_dbm: self.latest.rssi_dbm.map(i16::from),
            heating_mode,
            custom_profile: self.custom_profile.filter(|c| Some(c.base) == heating_mode),
            activity: self.activity(),
            session_started_at_ms: self.session_started_at_ms,
            charging: self.charging(),
            usb_plugged: self.usb_plugged,
            lock: self.latest.lock.and_then(|l| match l {
                LockStatus::Locked => Some(LockState::Locked),
                LockStatus::Unlocked => Some(LockState::Unlocked),
                LockStatus::FeatureOff => Some(LockState::FeatureOff),
                LockStatus::Unknown(_) => None,
            }),
            auto_start: self.latest.auto_start,
            vibrating: self.latest.vibrating.unwrap_or(false),
            temperature_warning: self.latest.temperature_error,
            enclosure_temp_c: self.enclosure_temp_c,
            unacquired_logs: self
                .latest
                .unacquired_logs
                .map(|(sessions, charges)| LogCounts { sessions, charges }),
        }
    }
}

#[cfg(test)]
mod tests {
    use ploom_protocol::HeatingMode;
    use ploom_protocol::frame::decode_hex;

    use super::*;

    const IDLE_30: &str = "10304314000000000000ff020801000100";
    const SMOKING_30: &str = "10304314000020010000000000010001 00";

    fn feed(t: &mut StatusTracker, at: u64, hex: &str) {
        let n = Notification::parse(&decode_hex(hex).unwrap()).unwrap();
        t.ingest(at, 1_000_000 + at, &n);
    }

    #[test]
    fn empty_until_reported() {
        let t = StatusTracker::new(None);
        assert_eq!(t.status(), DeviceStatus::default());
    }

    #[test]
    fn state_change_precedence_and_session_start() {
        let mut t = StatusTracker::new(None);
        feed(&mut t, 0, IDLE_30);
        assert_eq!(t.status().activity, Some(Activity::Standby));
        feed(&mut t, 1_000, "0431020000");
        assert_eq!(t.status().activity, Some(Activity::Preheating));
        assert_eq!(t.status().session_started_at_ms, Some(1_001_000));
        feed(&mut t, 2_000, "0431060000"); // several bits: highest wins
        let s = t.status();
        assert_eq!(s.activity, Some(Activity::Heating));
        assert_eq!(
            s.session_started_at_ms,
            Some(1_001_000),
            "start time is kept"
        );
        feed(&mut t, 3_000, "033c5500"); // unrelated frame re-merges the window
        assert_eq!(
            t.status().activity,
            Some(Activity::Heating),
            "stale idle 0x30 must not win"
        );
        feed(&mut t, 4_000, "0431000000");
        assert_eq!(t.status().activity, Some(Activity::Standby));
        assert_eq!(t.status().session_started_at_ms, None);
    }

    #[test]
    fn heating_status_frame_starts_session() {
        let mut t = StatusTracker::new(None);
        feed(&mut t, 0, SMOKING_30);
        assert!(t.is_heating());
        assert_eq!(t.status().activity, Some(Activity::Heating));
    }

    #[test]
    fn usb_detection_wins() {
        let mut t = StatusTracker::new(None);
        feed(&mut t, 0, "06340003000000");
        assert_eq!(t.status().charging, Some(true));
        t.set_usb_plugged(false);
        assert_eq!(t.status().charging, Some(false));
        assert_eq!(t.status().activity, Some(Activity::Standby));
        // An old 0x34 still in the window cannot override USB detection.
        feed(&mut t, 100, "033c5500");
        assert_eq!(t.status().charging, Some(false));
        t.set_usb_plugged(true);
        assert_eq!(t.status().charging, Some(true));
        assert_eq!(t.status().activity, Some(Activity::Charging));
    }

    #[test]
    fn charger_events_without_usb_reading() {
        let mut t = StatusTracker::new(None);
        feed(&mut t, 0, "033c3200");
        assert_eq!(t.status().charging, Some(false));
        feed(&mut t, 10, "06340003000000");
        assert_eq!(t.status().charging, Some(true));
        // 0x30 without the charging bit must not clear it.
        feed(&mut t, 20, IDLE_30);
        assert_eq!(t.status().charging, Some(true));
        feed(&mut t, 30, "06340007000000");
        assert_eq!(t.status().charging, Some(false));
    }

    #[test]
    fn battery_trend_infers_charging() {
        let mut t = StatusTracker::new(None);
        feed(&mut t, 0, "033c3200"); // 50 %
        t.after_poll(0);
        feed(&mut t, 10, "033c3300"); // +1: rebound, not charging
        t.after_poll(10);
        assert_eq!(t.status().charging, Some(false));
        feed(&mut t, 20, "033c3400"); // +2
        t.after_poll(20);
        assert_eq!(t.status().charging, Some(true));
        assert_eq!(t.status().activity, Some(Activity::Charging));
        // Standby resets the guess and the baseline.
        feed(&mut t, 30, "0431000000");
        assert_eq!(t.status().charging, Some(false));
        feed(&mut t, 40, "033c3500");
        t.after_poll(40);
        assert_eq!(t.status().charging, Some(false), "baseline moved to 52");
    }

    #[test]
    fn falling_battery_moves_baseline() {
        let mut t = StatusTracker::new(None);
        feed(&mut t, 0, "033c3200");
        t.after_poll(0);
        feed(&mut t, 10, "033c3000");
        t.after_poll(10);
        feed(&mut t, 20, "033c3200");
        t.after_poll(20);
        assert_eq!(
            t.status().charging,
            Some(true),
            "50 is +2 over the new baseline 48"
        );
    }

    #[test]
    fn custom_profile_only_while_base_mode_reported() {
        let custom = CustomProfile {
            base: HeatingMode::Intense,
            hold_temp_c: 290,
            hold_secs: 120,
        };
        let mut t = StatusTracker::new(Some(custom));
        assert_eq!(t.status().custom_profile, None, "mode unknown");
        feed(&mut t, 0, "023d01");
        feed(&mut t, 10, "083e00020000000000");
        assert_eq!(t.status().heating_mode, Some(HeatingMode::Intense));
        assert_eq!(t.status().custom_profile, Some(custom));
        feed(&mut t, 20, "023d00");
        assert_eq!(t.status().heating_mode, Some(HeatingMode::Balanced));
        assert_eq!(t.status().custom_profile, None);
    }

    #[test]
    fn lock_vibration_and_misc() {
        let mut t = StatusTracker::new(None);
        feed(&mut t, 0, "029f11");
        feed(&mut t, 0, "027a00");
        feed(&mut t, 0, "02fcc4");
        feed(&mut t, 0, "053506000700");
        t.set_enclosure_temp_c(31.5);
        let s = t.status();
        assert_eq!(s.lock, Some(LockState::FeatureOff));
        assert!(s.vibrating);
        assert_eq!(s.rssi_dbm, Some(-60));
        assert_eq!(
            s.unacquired_logs,
            Some(LogCounts {
                sessions: 6,
                charges: 7
            })
        );
        assert_eq!(s.enclosure_temp_c, Some(31.5));
        feed(&mut t, 0, "029f05");
        assert_eq!(
            t.status().lock,
            None,
            "an unknown lock byte reads as unknown"
        );
    }
}
