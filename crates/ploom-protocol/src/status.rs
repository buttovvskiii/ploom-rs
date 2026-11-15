//! Folding indications into a device status.

use std::collections::VecDeque;
use std::time::Duration;

use crate::heating::HeatingMode;
use crate::notification::{
    ChargeMode, DeviceState, LockStatus, Notification, ProfileSource, ShakeSetting,
};

/// Default window length: indications older than this are not merged.
pub const DEFAULT_WINDOW: Duration = Duration::from_secs(15);

macro_rules! status_update {
    ($( $(#[$doc:meta])* $field:ident : $ty:ty ),+ $(,)?) => {
        /// A partial device status: every field is `None` until some indication reports it.
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
            serde(rename_all = "camelCase")
        )]
        pub struct StatusUpdate {
            $( $(#[$doc])* pub $field: Option<$ty>, )+
        }

        impl StatusUpdate {
            /// Overwrites every field that `newer` knows.
            pub fn apply(&mut self, newer: &Self) {
                $( if newer.$field.is_some() { self.$field = newer.$field; } )+
            }

            /// Whether no field is known.
            pub fn is_empty(&self) -> bool {
                true $( && self.$field.is_none() )+
            }
        }
    };
}

status_update! {
    /// `0x3C` battery level, percent.
    battery_pct: u8,
    /// `0x33` battery health, percent.
    battery_health_pct: u8,
    /// `0xFC` RSSI, dBm.
    rssi_dbm: i8,
    /// `0x7A` vibrating.
    vibrating: bool,
    /// `0x31` last reported state.
    state: DeviceState,
    /// `0x30`/`0x18`/`0x9F` lock status.
    lock: LockStatus,
    /// `0x34` charger mode.
    charge_mode: ChargeMode,
    /// Charging as reported by the device (`0x34`, `0x31`, `0x30` bit 4).
    charging: bool,
    /// `0x3D`: the downloaded profile is active.
    profile_downloaded_active: bool,
    /// `0x3E`: the mode of the downloaded profile.
    downloaded_mode: HeatingMode,
    /// `0x30` device counter, seconds.
    uptime_s: u16,
    /// Temperature error / device too warm.
    temperature_error: bool,
    /// A session is in progress.
    in_use: bool,
    /// Preheating.
    preheating: bool,
    /// Shake/tap-to-wake setting.
    shake: ShakeSetting,
    /// Auto-Start Heating.
    auto_start: bool,
    /// End session by puff count.
    end_by_puff: bool,
    /// Active profile slot.
    profile_source: ProfileSource,
    /// One more stick is allowed on the remaining battery.
    last_stick_allowed: bool,
    /// `0x35` unacknowledged `(session, charge)` logs.
    unacquired_logs: (u16, u16),
}

impl StatusUpdate {
    /// The status fields carried by one indication.
    pub fn from_notification(n: &Notification) -> Self {
        let mut u = Self::default();
        match n {
            Notification::Status(s) => {
                u.uptime_s = Some(s.uptime_s);
                u.temperature_error = Some(s.temperature_error);
                u.in_use = Some(s.in_use);
                u.preheating = Some(s.preheating);
                u.lock = Some(s.lock);
                u.shake = Some(s.shake);
                u.auto_start = Some(s.auto_start);
                u.end_by_puff = Some(s.end_by_puff);
                u.profile_source = Some(s.profile_source);
                u.last_stick_allowed = Some(s.last_stick_allowed);
                // Bit 4 only ever confirms charging; it is clear while charging.
                if s.charging_bit {
                    u.charging = Some(true);
                }
            }
            Notification::Configuration(c) => {
                u.temperature_error = Some(c.temperature_error);
                u.lock = Some(c.lock);
                u.shake = Some(c.shake);
                u.auto_start = Some(c.auto_start);
                u.end_by_puff = Some(c.end_by_puff);
                u.profile_source = Some(c.profile_source);
                u.last_stick_allowed = Some(c.last_stick_allowed);
            }
            Notification::StateChanged { state } => {
                u.state = Some(*state);
                u.apply(&Self::state_flags(*state));
            }
            Notification::BatteryHealth { pct } => u.battery_health_pct = Some(*pct),
            Notification::ChargingStatus { mode } => {
                u.charge_mode = Some(*mode);
                u.charging = Some(mode.is_charging());
            }
            Notification::UnacquiredLogs { sessions, charges } => {
                u.unacquired_logs = Some((*sessions, *charges));
            }
            Notification::BatteryLevel { pct } => u.battery_pct = Some(*pct),
            Notification::ProfileActive { downloaded } => {
                u.profile_downloaded_active = Some(*downloaded);
            }
            Notification::DownloadedProfile { number } => {
                u.downloaded_mode = Some(HeatingMode::from_profile_number(*number));
            }
            Notification::Vibration { on } => u.vibrating = Some(*on),
            Notification::Lock { status } => u.lock = Some(*status),
            Notification::Rssi { dbm } => u.rssi_dbm = Some(*dbm),
            _ => {}
        }
        u
    }

    /// The `0x30`-style flags implied by a `0x31` state change.
    pub fn state_flags(state: DeviceState) -> Self {
        let mut u = Self::default();
        match state {
            DeviceState::Standby => {
                u.preheating = Some(false);
                u.in_use = Some(false);
                u.charging = Some(false);
            }
            DeviceState::AutomaticHeating | DeviceState::Preheating => {
                u.preheating = Some(true);
                u.in_use = Some(false);
            }
            DeviceState::Smoking => {
                u.in_use = Some(true);
                u.preheating = Some(false);
            }
            DeviceState::Charging => u.charging = Some(true),
            DeviceState::TemperatureError | DeviceState::AlarmError => {
                u.temperature_error = Some(true);
            }
            DeviceState::Unknown(_) => {}
        }
        u
    }

    /// The heating mode actually in effect, or `None` while unknown.
    pub fn effective_mode(&self) -> Option<HeatingMode> {
        let downloaded = match self.profile_downloaded_active {
            Some(active) => active,
            None => matches!(
                self.profile_source?,
                ProfileSource::Downloaded | ProfileSource::P2p
            ),
        };
        if downloaded {
            self.downloaded_mode
        } else {
            Some(HeatingMode::Balanced)
        }
    }

    /// Whether the heater is on (in use or preheating).
    pub fn is_heating(&self) -> bool {
        self.in_use == Some(true) || self.preheating == Some(true)
    }
}

/// A sliding time window of status updates.
#[derive(Debug, Clone)]
pub struct StatusWindow {
    age_ms: u64,
    entries: VecDeque<(u64, StatusUpdate)>,
}

impl Default for StatusWindow {
    fn default() -> Self {
        Self::new(DEFAULT_WINDOW)
    }
}

impl StatusWindow {
    /// An empty window keeping updates for `age`.
    pub fn new(age: Duration) -> Self {
        Self {
            age_ms: u64::try_from(age.as_millis()).unwrap_or(u64::MAX),
            entries: VecDeque::new(),
        }
    }

    /// Records an update received at `at_ms` and drops expired ones.
    pub fn push(&mut self, at_ms: u64, update: StatusUpdate) {
        if !update.is_empty() {
            self.entries.push_back((at_ms, update));
        }
        let cutoff = at_ms.saturating_sub(self.age_ms);
        while self.entries.front().is_some_and(|&(t, _)| t < cutoff) {
            self.entries.pop_front();
        }
    }

    /// Folds every update received at or after `now_ms - age`, in arrival order.
    pub fn merged(&self, now_ms: u64) -> StatusUpdate {
        let cutoff = now_ms.saturating_sub(self.age_ms);
        let mut out = StatusUpdate::default();
        for (_, u) in self.entries.iter().filter(|&&(t, _)| t >= cutoff) {
            out.apply(u);
        }
        out
    }

    /// Number of stored updates.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the window holds no update.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Forgets every update.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::decode_hex;

    fn update(hex: &str) -> StatusUpdate {
        StatusUpdate::from_notification(&Notification::parse(&decode_hex(hex).unwrap()).unwrap())
    }

    const IDLE_30: &str = "10 30 43 14 00 00 00 00 00 00 ff 02 08 01 00 01 00";

    #[test]
    fn state_change_overrides_older_status() {
        let mut w = StatusWindow::default();
        w.push(1_000, update(IDLE_30));
        w.push(2_000, update("04 31 04 00 00"));
        let m = w.merged(2_000);
        assert_eq!(m.in_use, Some(true));
        assert_eq!(m.preheating, Some(false));
        assert_eq!(m.state, Some(DeviceState::Smoking));
        assert!(m.is_heating());
    }

    #[test]
    fn newer_status_wins_over_older_state_change() {
        let mut w = StatusWindow::default();
        w.push(1_000, update("04 31 04 00 00"));
        w.push(2_000, update(IDLE_30));
        assert_eq!(w.merged(2_000).in_use, Some(false));
    }

    #[test]
    fn expired_updates_are_ignored() {
        let mut w = StatusWindow::new(Duration::from_secs(15));
        w.push(1_000, update("02 3c 50"));
        w.push(10_000, update("02 33 5f"));
        assert_eq!(w.merged(16_000).battery_pct, Some(80));
        assert_eq!(w.merged(16_001).battery_pct, None);
        w.push(16_001, update("02 fc c4"));
        assert_eq!(w.len(), 2, "pushing prunes expired entries");
        w.clear();
        assert!(w.is_empty());
    }

    #[test]
    fn charging_bit_only_confirms() {
        let mut flags = decode_hex(IDLE_30).unwrap();
        flags[6] = 0x10;
        let charging =
            StatusUpdate::from_notification(&Notification::parse(&flags).unwrap()).charging;
        assert_eq!(charging, Some(true));
        assert_eq!(update(IDLE_30).charging, None);
    }

    #[test]
    fn effective_mode_rules() {
        let mut u = StatusUpdate::default();
        assert_eq!(u.effective_mode(), None);
        u.profile_source = Some(ProfileSource::Factory);
        assert_eq!(u.effective_mode(), Some(HeatingMode::Balanced));
        u.profile_source = Some(ProfileSource::Downloaded);
        assert_eq!(
            u.effective_mode(),
            None,
            "downloaded, but which one is unknown"
        );
        u.downloaded_mode = Some(HeatingMode::Intense);
        assert_eq!(u.effective_mode(), Some(HeatingMode::Intense));
        u.profile_downloaded_active = Some(false);
        assert_eq!(
            u.effective_mode(),
            Some(HeatingMode::Balanced),
            "0x3D wins over 0x30"
        );
    }

    #[test]
    fn empty_updates_are_not_stored() {
        let mut w = StatusWindow::default();
        w.push(0, update("02 99 00"));
        assert!(w.is_empty());
    }
}
