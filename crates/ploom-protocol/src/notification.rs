//! Decoding of indications received on RX.

use crate::diagnostics::DiagReply;
use crate::frame::FrameError;
use crate::heating::HeatingMode;
use crate::opcode::reply;

/// A decoded indication.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(
        tag = "type",
        rename_all = "camelCase",
        rename_all_fields = "camelCase"
    )
)]
#[non_exhaustive]
pub enum Notification {
    /// `0x18` `DeviceConfigurationInformationDTO`.
    Configuration(DeviceConfiguration),
    /// `0x30` `DeviceStatusConfigurationDTO`, the main status frame.
    Status(StatusReport),
    /// `0x31` `GetDeviceStatusChangeCommand`, sent spontaneously.
    StateChanged {
        /// The new state.
        state: DeviceState,
    },
    /// `0x33` battery health (state of health), percent.
    BatteryHealth {
        /// Health, 0..=100.
        pct: u8,
    },
    /// `0x34` charger status, sent spontaneously while charging (roughly at every 1 % step) and
    /// when the cable is removed.
    ChargingStatus {
        /// Charger mode from `s[1]`.
        mode: ChargeMode,
    },
    /// `0x35` number of logs not yet acknowledged with `C4`.
    UnacquiredLogs {
        /// Session logs, `s[0..2]`.
        sessions: u16,
        /// Charge logs, `s[2..4]`.
        charges: u16,
    },
    /// `0x36` session log, part 1.
    SmokingLog(SmokingLog),
    /// `0x37`/`0x38`/`0x39` puff timings.
    PuffTimings(PuffTimings),
    /// `0x3A` session energy.
    SmokingEnergy(SmokingEnergy),
    /// `0x3B` charge log, part 1.
    ChargeLog(ChargeLog),
    /// `0x3C` battery level, percent.
    BatteryLevel {
        /// State of charge, 0..=100.
        pct: u8,
    },
    /// `0x3D` `GetSettingProfile`: a flag, not a mode number.
    ProfileActive {
        /// `true` when the downloaded profile is active, `false` for the factory one (Balanced).
        downloaded: bool,
    },
    /// `0x3E` the downloaded profile's number, `s[1]`.
    DownloadedProfile {
        /// Profile number; see [`HeatingMode::from_profile_number`].
        number: u8,
    },
    /// `0x3F`/`0x49`/`0x4B` acknowledgement of a profile-upload frame.
    Ack {
        /// Which acknowledgement this is.
        opcode: u8,
        /// `0` on success.
        status: u8,
    },
    /// `0x40` charge log, part 2.
    ChargeLogSummary(ChargeLogSummary),
    /// `0x44`/`0x45`/`0x46` one part of the calibration table.
    Masters {
        /// Part index 0..=2.
        part: u8,
        /// Little-endian `u16` values.
        values: Vec<u16>,
    },
    /// `0x50`/`0xF4` an error log record, undecoded.
    ErrorLog {
        /// `0x50` or `0xF4`.
        opcode: u8,
        /// Raw record.
        payload: Vec<u8>,
    },
    /// `0x7A` vibration status.
    Vibration {
        /// Whether the device is buzzing.
        on: bool,
    },
    /// `0x9F` lock status.
    Lock {
        /// The lock state.
        status: LockStatus,
    },
    /// `0xF2` a diagnostics record.
    Diagnostic(DiagReply),
    /// `0xFC` link RSSI measured by the device.
    Rssi {
        /// Signed dBm.
        dbm: i8,
    },
    /// An opcode this crate does not decode, or a payload too short for its opcode.
    Unknown {
        /// Opcode.
        opcode: u8,
        /// Payload (after `[LEN][OPCODE]`).
        payload: Vec<u8>,
    },
}

impl Notification {
    /// Decodes a raw indication `[LEN][OPCODE][payload]`.
    pub fn parse(raw: &[u8]) -> Result<Self, FrameError> {
        match raw {
            [_, opcode, payload @ ..] => {
                Ok(decode(*opcode, payload).unwrap_or_else(|| Self::Unknown {
                    opcode: *opcode,
                    payload: payload.to_vec(),
                }))
            }
            _ => Err(FrameError::TooShort { len: raw.len() }),
        }
    }

    /// The opcode this notification was decoded from.
    pub fn opcode(&self) -> u8 {
        match self {
            Self::Configuration(_) => reply::CONFIGURATION,
            Self::Status(_) => reply::STATUS,
            Self::StateChanged { .. } => reply::STATE_CHANGE,
            Self::BatteryHealth { .. } => reply::BATTERY_HEALTH,
            Self::ChargingStatus { .. } => reply::CHARGING_STATUS,
            Self::UnacquiredLogs { .. } => reply::UNACQUIRED_LOGS,
            Self::SmokingLog(_) => reply::SMOKING_LOG,
            Self::PuffTimings(p) => p.opcode(),
            Self::SmokingEnergy(_) => reply::SMOKING_ENERGY,
            Self::ChargeLog(_) => reply::CHARGE_LOG,
            Self::BatteryLevel { .. } => reply::BATTERY_LEVEL,
            Self::ProfileActive { .. } => reply::PROFILE_ACTIVE,
            Self::DownloadedProfile { .. } => reply::DOWNLOADED_PROFILE,
            Self::ChargeLogSummary(_) => reply::CHARGE_LOG_SUMMARY,
            Self::Masters { part, .. } => reply::MASTERS_1 + part,
            Self::Diagnostic(_) => reply::DIAGNOSTICS,
            Self::Vibration { .. } => reply::VIBRATION,
            Self::Lock { .. } => reply::LOCK,
            Self::Rssi { .. } => reply::RSSI,
            Self::Ack { opcode, .. }
            | Self::ErrorLog { opcode, .. }
            | Self::Unknown { opcode, .. } => *opcode,
        }
    }
}

fn decode(opcode: u8, s: &[u8]) -> Option<Notification> {
    use Notification as N;
    Some(match opcode {
        reply::CONFIGURATION => N::Configuration(DeviceConfiguration::decode(s)?),
        reply::STATUS => N::Status(StatusReport::decode(s)?),
        reply::STATE_CHANGE => N::StateChanged {
            state: DeviceState::from_bits(*s.first()?),
        },
        reply::BATTERY_HEALTH => N::BatteryHealth { pct: *s.first()? },
        reply::CHARGING_STATUS => N::ChargingStatus {
            mode: ChargeMode::from_byte(*s.get(1)?),
        },
        reply::UNACQUIRED_LOGS => N::UnacquiredLogs {
            sessions: u16_at(s, 0)?,
            charges: u16_at(s, 2)?,
        },
        reply::SMOKING_LOG => N::SmokingLog(SmokingLog::decode(s)?),
        reply::PUFFS_1 | reply::PUFFS_2 | reply::PUFFS_3 => {
            N::PuffTimings(PuffTimings::decode(opcode, s))
        }
        reply::SMOKING_ENERGY => N::SmokingEnergy(SmokingEnergy::decode(s)?),
        reply::CHARGE_LOG => N::ChargeLog(ChargeLog::decode(s)?),
        reply::BATTERY_LEVEL => N::BatteryLevel { pct: *s.first()? },
        reply::PROFILE_ACTIVE => N::ProfileActive {
            downloaded: *s.first()? != 0,
        },
        reply::DOWNLOADED_PROFILE => N::DownloadedProfile { number: *s.get(1)? },
        reply::PROFILE_ACTIVATED | reply::AUTO_START_ACK_1 | reply::AUTO_START_ACK_2 => N::Ack {
            opcode,
            status: *s.first()?,
        },
        reply::CHARGE_LOG_SUMMARY => N::ChargeLogSummary(ChargeLogSummary::decode(s)?),
        reply::MASTERS_1 | reply::MASTERS_2 | reply::MASTERS_3 => N::Masters {
            part: opcode - reply::MASTERS_1,
            values: crate::calibration::decode_values(s).collect(),
        },
        reply::ERROR_LOG | reply::LATEST_ERROR_LOG => N::ErrorLog {
            opcode,
            payload: s.to_vec(),
        },
        reply::VIBRATION => N::Vibration {
            on: *s.first()? == 0,
        },
        reply::LOCK => N::Lock {
            status: LockStatus::from_byte(*s.first()?),
        },
        reply::DIAGNOSTICS => N::Diagnostic(DiagReply::parse(s)?),
        reply::RSSI => N::Rssi {
            dbm: i8::from_le_bytes([*s.first()?]),
        },
        _ => return None,
    })
}

fn u16_at(s: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*s.get(i)?, *s.get(i + 1)?]))
}

fn u32_at(s: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(s.get(i..i + 4)?.try_into().ok()?))
}

macro_rules! byte_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($(#[$vmeta:meta])* $variant:ident = $value:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[cfg_attr(
            feature = "serde",
            derive(serde::Serialize, serde::Deserialize),
            serde(rename_all = "camelCase")
        )]
        pub enum $name {
            $($(#[$vmeta])* $variant,)+
            /// A value this crate does not know.
            Unknown(u8),
        }

        impl $name {
            /// Decodes the wire byte.
            pub fn from_byte(b: u8) -> Self {
                match b {
                    $($value => Self::$variant,)+
                    other => Self::Unknown(other),
                }
            }
        }
    };
}

byte_enum! {
    /// Lock status (`0x30` `s[5]`, `0x18` `s[1]`, `0x9F` `s[0]`).
    LockStatus {
        /// The device is locked.
        Locked = 0x00,
        /// The device is unlocked.
        Unlocked = 0x01,
        /// The lock feature itself is turned off.
        FeatureOff = 0x11,
    }
}

byte_enum! {
    /// Shake/tap-to-wake setting.
    ShakeSetting {
        /// Off.
        Off = 0,
        /// Wake on shake.
        ShakeOn = 1,
        /// Wake on tap.
        TapOn = 2,
        /// Wake on shake or tap.
        ShakeOrTapOn = 3,
        /// Disabled.
        Disabled = 4,
    }
}

byte_enum! {
    /// Which profile slot is active (`0x30` `s[13]`).
    ProfileSource {
        /// The factory profile (Balanced).
        Factory = 0,
        /// A downloaded profile.
        Downloaded = 1,
        /// A peer-to-peer shared profile.
        P2p = 2,
    }
}

byte_enum! {
    /// Charger mode (`0x34` `s[1]`).
    ChargeMode {
        /// Charger mode 0.
        Mode0 = 0,
        /// Charger mode 1.
        Mode1 = 1,
        /// Charger mode 2.
        Mode2 = 2,
        /// Charger mode 3.
        Mode3 = 3,
        /// Charger mode 4.
        Mode4 = 4,
        /// Charger mode 5.
        Mode5 = 5,
        /// Some other charging mode.
        Other = 6,
        /// No charger connected.
        NoConnect = 7,
    }
}

impl ChargeMode {
    /// The app's rule: charging whenever the mode is not `noConnect`.
    pub fn is_charging(self) -> bool {
        self != Self::NoConnect
    }
}

/// Device state from `0x31`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum DeviceState {
    /// Idle (`0x00` or `0x40`).
    Standby,
    /// Auto-start heating kicked in (`0x01`).
    AutomaticHeating,
    /// Preheating after a button press (`0x02`).
    Preheating,
    /// A session is in progress (`0x04`).
    Smoking,
    /// Charging (`0x08`).
    Charging,
    /// Temperature error (`0x10`).
    TemperatureError,
    /// Alarm error (`0x20`).
    AlarmError,
    /// Any other bit pattern; carries the original byte.
    Unknown(u8),
}

impl DeviceState {
    /// Decodes the `0x31` bitmask; with several bits set the highest wins.
    pub fn from_bits(byte: u8) -> Self {
        let bit = match byte {
            0x00 | 0x01 | 0x02 | 0x04 | 0x08 | 0x10 | 0x20 | 0x40 => byte,
            // Non-zero here, so `ilog2` cannot panic.
            _ => 1 << byte.ilog2(),
        };
        match bit {
            0x00 | 0x40 => Self::Standby,
            0x01 => Self::AutomaticHeating,
            0x02 => Self::Preheating,
            0x04 => Self::Smoking,
            0x08 => Self::Charging,
            0x10 => Self::TemperatureError,
            0x20 => Self::AlarmError,
            _ => Self::Unknown(byte),
        }
    }

    /// Whether the heater is on in this state.
    pub fn is_heating(self) -> bool {
        matches!(
            self,
            Self::AutomaticHeating | Self::Preheating | Self::Smoking
        )
    }
}

/// `0x30` status report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors independent wire flags"
)]
pub struct StatusReport {
    /// `s[0..2]`: a device counter that increases by one per second.
    pub uptime_s: u16,
    /// `s[4]` bit 3.
    pub temperature_error: bool,
    /// `s[4]` bit 4.
    pub charging_bit: bool,
    /// `s[4]` bit 5: a session is in progress.
    pub in_use: bool,
    /// `s[4]` bit 6: preheating.
    pub preheating: bool,
    /// `s[5]`.
    pub lock: LockStatus,
    /// `s[6]`.
    pub shake: ShakeSetting,
    /// `s[11]`: Auto-Start Heating.
    pub auto_start: bool,
    /// `s[12]`: end the session after the maximum number of puffs.
    pub end_by_puff: bool,
    /// `s[13]`.
    pub profile_source: ProfileSource,
    /// `s[14] == 0`: one more stick is allowed on the remaining battery.
    pub last_stick_allowed: bool,
}

impl StatusReport {
    fn decode(s: &[u8]) -> Option<Self> {
        if s.len() < 15 {
            return None;
        }
        let flags = s[4];
        Some(Self {
            uptime_s: u16::from_le_bytes([s[0], s[1]]),
            temperature_error: flags & 0x08 != 0,
            charging_bit: flags & 0x10 != 0,
            in_use: flags & 0x20 != 0,
            preheating: flags & 0x40 != 0,
            lock: LockStatus::from_byte(s[5]),
            shake: ShakeSetting::from_byte(s[6]),
            auto_start: s[11] != 0,
            end_by_puff: s[12] != 0,
            profile_source: ProfileSource::from_byte(s[13]),
            last_stick_allowed: s[14] == 0,
        })
    }
}

/// `0x18` device configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors independent wire flags"
)]
pub struct DeviceConfiguration {
    /// `s[0]` bit 3.
    pub temperature_error: bool,
    /// `s[1]`.
    pub lock: LockStatus,
    /// `s[2]`.
    pub shake: ShakeSetting,
    /// `s[7]`.
    pub auto_start: bool,
    /// `s[8]`.
    pub end_by_puff: bool,
    /// `s[9]`.
    pub profile_source: ProfileSource,
    /// `s[10] == 0`.
    pub last_stick_allowed: bool,
}

impl DeviceConfiguration {
    fn decode(s: &[u8]) -> Option<Self> {
        if s.len() < 11 {
            return None;
        }
        Some(Self {
            temperature_error: s[0] & 0x08 != 0,
            lock: LockStatus::from_byte(s[1]),
            shake: ShakeSetting::from_byte(s[2]),
            auto_start: s[7] != 0,
            end_by_puff: s[8] != 0,
            profile_source: ProfileSource::from_byte(s[9]),
            last_stick_allowed: s[10] == 0,
        })
    }
}

/// `0x36` session log, part 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "mirrors independent wire flags"
)]
pub struct SmokingLog {
    /// `s[0..4]`: device counter at session start (not wall-clock time).
    pub start_counter: u32,
    /// `s[4..6]`: session length, seconds.
    pub duration_s: u16,
    /// `s[6]`: number of puffs.
    pub puff_count: u8,
    /// `s[7]`.
    pub puff_stop_enabled: bool,
    /// `s[8]`: how heating was started.
    pub heating_start_condition: u8,
    /// `s[9]`.
    pub auto_start_enabled: bool,
    /// `s[10]`.
    pub lock_enabled: bool,
    /// `s[11]`.
    pub unlock_method: u8,
    /// `s[12..16]`.
    pub downloaded_profile: u32,
    /// `s[16..18]`.
    pub total_time: u16,
}

impl SmokingLog {
    fn decode(s: &[u8]) -> Option<Self> {
        if s.len() < 18 {
            return None;
        }
        Some(Self {
            start_counter: u32_at(s, 0)?,
            duration_s: u16_at(s, 4)?,
            puff_count: s[6],
            puff_stop_enabled: s[7] != 0,
            heating_start_condition: s[8],
            auto_start_enabled: s[9] != 0,
            lock_enabled: s[10] != 0,
            unlock_method: s[11],
            downloaded_profile: u32_at(s, 12)?,
            total_time: u16_at(s, 16)?,
        })
    }
}

/// `0x37`/`0x38`/`0x39` puff timings: seconds from session start, one little-endian `u16` per puff
/// slot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct PuffTimings {
    /// 1-based number of the first puff slot in this frame (1, 10 or 19).
    pub first_puff: u8,
    /// `(puff number, seconds from session start)` for every non-empty slot.
    pub puffs: Vec<(u8, u16)>,
}

impl PuffTimings {
    fn decode(opcode: u8, s: &[u8]) -> Self {
        let first_puff = match opcode {
            reply::PUFFS_1 => 1,
            reply::PUFFS_2 => 10,
            _ => 19,
        };
        let puffs = s
            .chunks_exact(2)
            .enumerate()
            .map(|(i, pair)| (first_puff + i as u8, u16::from_le_bytes([pair[0], pair[1]])))
            .filter(|&(_, offset)| offset != 0)
            .collect();
        Self { first_puff, puffs }
    }

    fn opcode(&self) -> u8 {
        match self.first_puff {
            1 => reply::PUFFS_1,
            10 => reply::PUFFS_2,
            _ => reply::PUFFS_3,
        }
    }
}

/// `0x3A` session energy figures (raw device units).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct SmokingEnergy {
    /// `amountOfEnergyIn50msInterval`.
    pub energy_50ms: u16,
    /// `stpe0SegmentEnergyAmount`.
    pub step0_segment_energy: u16,
    /// `voltageOfAvg0AtTheStartOfSmoking`, mV.
    pub mv_avg0_at_start: u16,
    /// `voltageOfAvg2AtTheStartOfSmoking`, mV.
    pub mv_avg2_at_start: u16,
    /// `amountOfEnergyFor200msInterval`.
    pub energy_200ms: u16,
}

impl SmokingEnergy {
    fn decode(s: &[u8]) -> Option<Self> {
        Some(Self {
            energy_50ms: u16_at(s, 0)?,
            step0_segment_energy: u16_at(s, 2)?,
            mv_avg0_at_start: u16_at(s, 4)?,
            mv_avg2_at_start: u16_at(s, 6)?,
            energy_200ms: u16_at(s, 8)?,
        })
    }
}

/// `0x3B` charge log, part 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ChargeLog {
    /// `s[0..4]`: device counter when charging started.
    pub start_counter: u32,
    /// `s[4..8]`: device counter when charging ended.
    pub end_counter: u32,
    /// `s[8..10]`: battery voltage at start, mV.
    pub start_mv: u16,
    /// `s[10..12]`: battery voltage at end, mV.
    pub end_mv: u16,
    /// `s[12..14]`: battery temperature at start, raw.
    pub start_temp_raw: u16,
    /// `s[14..16]`: battery temperature at end, raw.
    pub end_temp_raw: u16,
}

impl ChargeLog {
    fn decode(s: &[u8]) -> Option<Self> {
        if s.len() < 16 {
            return None;
        }
        Some(Self {
            start_counter: u32_at(s, 0)?,
            end_counter: u32_at(s, 4)?,
            start_mv: u16_at(s, 8)?,
            end_mv: u16_at(s, 10)?,
            start_temp_raw: u16_at(s, 12)?,
            end_temp_raw: u16_at(s, 14)?,
        })
    }
}

/// `0x40` charge log, part 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct ChargeLogSummary {
    /// `s[0..2]`: lifetime charge count.
    pub total_charge_count: u16,
    /// `s[2..6]`: charging mode, raw.
    pub charge_mode_raw: u32,
    /// `s[6]`: battery state of health at the start of the charge, percent.
    pub state_of_health_pct: u8,
}

impl ChargeLogSummary {
    fn decode(s: &[u8]) -> Option<Self> {
        if s.len() < 7 {
            return None;
        }
        Some(Self {
            total_charge_count: u16_at(s, 0)?,
            charge_mode_raw: u32_at(s, 2)?,
            state_of_health_pct: s[6],
        })
    }
}

impl Notification {
    /// For [`Notification::DownloadedProfile`], the heating mode it denotes.
    pub fn downloaded_mode(&self) -> Option<HeatingMode> {
        match self {
            Self::DownloadedProfile { number } => Some(HeatingMode::from_profile_number(*number)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::decode_hex;

    fn parse(hex: &str) -> Notification {
        Notification::parse(&decode_hex(hex).unwrap()).unwrap()
    }

    #[test]
    fn rejects_frames_without_opcode() {
        assert_eq!(
            Notification::parse(&[1]),
            Err(FrameError::TooShort { len: 1 })
        );
        assert_eq!(
            Notification::parse(&[]),
            Err(FrameError::TooShort { len: 0 })
        );
    }

    #[test]
    fn live_status_frame() {
        let Notification::Status(s) = parse("10 30 43 14 00 00 00 00 00 00 ff 02 08 01 00 01 00")
        else {
            panic!("not a status frame");
        };
        assert_eq!(s.lock, LockStatus::Locked);
        assert!(s.auto_start);
        assert_eq!(s.profile_source, ProfileSource::Downloaded);
        assert!(!s.in_use && !s.preheating && !s.charging_bit);
        assert_eq!(s.uptime_s, 0x1443);
    }

    #[test]
    fn state_change_takes_highest_bit() {
        assert_eq!(DeviceState::from_bits(0x00), DeviceState::Standby);
        assert_eq!(DeviceState::from_bits(0x40), DeviceState::Standby);
        assert_eq!(DeviceState::from_bits(0x06), DeviceState::Smoking);
        assert_eq!(DeviceState::from_bits(0x03), DeviceState::Preheating);
        assert_eq!(DeviceState::from_bits(0x80), DeviceState::Unknown(0x80));
        assert_eq!(DeviceState::from_bits(0x81), DeviceState::Unknown(0x81));
        assert_eq!(DeviceState::from_bits(0x41), DeviceState::Standby);
    }

    #[test]
    fn short_payload_is_unknown() {
        assert_eq!(
            parse("01 3c"),
            Notification::Unknown {
                opcode: 0x3C,
                payload: vec![]
            }
        );
        assert_eq!(
            parse("02 3e 00"),
            Notification::Unknown {
                opcode: 0x3E,
                payload: vec![0]
            }
        );
    }

    #[test]
    fn vibration_is_inverted() {
        assert_eq!(parse("02 7a 00"), Notification::Vibration { on: true });
        assert_eq!(parse("02 7a 01"), Notification::Vibration { on: false });
    }

    #[test]
    fn opcode_round_trips() {
        for hex in [
            "02 3c 55",
            "02 37 0e 00",
            "03 38 00 00",
            "02 39 00",
            "03 45 01 00",
            "02 3f 00",
            "02 49 01",
            "02 99 00",
            "02 fc c4",
        ] {
            let raw = decode_hex(hex).unwrap();
            assert_eq!(Notification::parse(&raw).unwrap().opcode(), raw[1], "{hex}");
        }
    }

    #[test]
    fn rssi_is_signed() {
        assert_eq!(parse("02 fc c4"), Notification::Rssi { dbm: -60 });
    }

    #[test]
    fn downloaded_mode() {
        assert_eq!(
            parse("05 3e 00 02 00 00").downloaded_mode(),
            Some(HeatingMode::Intense)
        );
        assert_eq!(parse("02 3c 01").downloaded_mode(), None);
    }
}
