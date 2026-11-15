//! Named opcodes.

/// Opcodes of requests written to TX.
pub mod request {
    /// `requestDeviceStatusNotificationRetransmission`: resend the full status (`0x30`, `0x33`,
    /// `0x9F`, `0x3C`).
    pub const STATUS: u8 = 0xE3;
    /// `requestRssi`: link RSSI as seen by the device (`0xFC`).
    pub const RSSI: u8 = 0xFD;
    /// `ReqSettingProfileNumber`: whether the downloaded profile is active (`0x3D`).
    pub const PROFILE_ACTIVE: u8 = 0xA2;
    /// `ReqDownloadProfileNumber`: which profile is downloaded (`0x3E`).
    pub const DOWNLOADED_PROFILE: u8 = 0xA3;
    /// `ReqSendProfileNumber`: `00` activates the factory profile, `01` the downloaded one
    /// (`0x3F`).
    pub const ACTIVATE_PROFILE: u8 = 0xA4;
    /// `_getMasters`: calibration tables (`0x44`, `0x45`, `0x46`).
    pub const MASTERS: u8 = 0xA5;
    /// First heating-curve step (`A7` … `BA`, one per step).
    pub const CURVE_STEP_FIRST: u8 = 0xA7;
    /// `requestSmokingLogs`: oldest unacknowledged session log (`0x35`, `0x36`…`0x3A`).
    pub const SMOKING_LOGS: u8 = 0xC1;
    /// `requestChargingLogs`: oldest unacknowledged charge log (`0x35`, `0x3B`, `0x40`).
    pub const CHARGING_LOGS: u8 = 0xC2;
    /// `requestLogsDelete`: **destructive**, acknowledges (erases) the logs.
    pub const DELETE_LOGS: u8 = 0xC4;
    /// `requestLatestErrorLogs` (`0xF4`).
    pub const ERROR_LOGS: u8 = 0xF3;
    /// `requestAdditionalErrorLogs`: the diagnostics channel (`0xF2`).
    pub const DIAGNOSTICS: u8 = 0xF1;
    /// `toggleDeviceVibration`: buzz and blink (Find My Device).
    pub const VIBRATION: u8 = 0x79;
    /// `toggleDeviceLock`.
    pub const LOCK: u8 = 0x9E;
    /// `toggleDeviceAutoStartHeating`.
    pub const AUTO_START: u8 = 0xCA;
    /// `requestDisableBluetooth`: **dangerous**, turns the radio off until re-enabled on the device
    /// itself.
    pub const DISABLE_BLUETOOTH: u8 = 0xA1;
    /// `_startFirmwareUpdate`: **dangerous**, enters firmware-update mode.
    pub const FIRMWARE_UPDATE: u8 = 0xFF;
}

/// Opcodes of indications received on RX.
pub mod reply {
    /// `DeviceConfigurationInformationDTO`.
    pub const CONFIGURATION: u8 = 0x18;
    /// `DeviceStatusConfigurationDTO`, the main status frame.
    pub const STATUS: u8 = 0x30;
    /// `GetDeviceStatusChangeCommand`, sent spontaneously on state changes.
    pub const STATE_CHANGE: u8 = 0x31;
    /// `GetBatteryDegradationCommand`: battery health in percent.
    pub const BATTERY_HEALTH: u8 = 0x33;
    /// `GetChargingStatusCommand`, sent spontaneously while charging.
    pub const CHARGING_STATUS: u8 = 0x34;
    /// `UnacquiredLogsNumberNotification`.
    pub const UNACQUIRED_LOGS: u8 = 0x35;
    /// `SmokingLogTransmission1`.
    pub const SMOKING_LOG: u8 = 0x36;
    /// `SmokingLogTransmission2`: puffs 1–9.
    pub const PUFFS_1: u8 = 0x37;
    /// `SmokingLogTransmission3`: puffs 10–18.
    pub const PUFFS_2: u8 = 0x38;
    /// `SmokingLogTransmission4`: puffs 19–25.
    pub const PUFFS_3: u8 = 0x39;
    /// `SmokingLogTransmission5`: session energy.
    pub const SMOKING_ENERGY: u8 = 0x3A;
    /// `ChargeLogTransmission1`.
    pub const CHARGE_LOG: u8 = 0x3B;
    /// `GetBatteryLevelCommand`: state of charge in percent.
    pub const BATTERY_LEVEL: u8 = 0x3C;
    /// `GetSettingProfile`: 0 = factory profile active, 1 = downloaded.
    pub const PROFILE_ACTIVE: u8 = 0x3D;
    /// Downloaded profile number in byte 1.
    pub const DOWNLOADED_PROFILE: u8 = 0x3E;
    /// Acknowledgement of `ReqSendProfileNumber` (`A4`).
    pub const PROFILE_ACTIVATED: u8 = 0x3F;
    /// `ChargeLogTransmission2`.
    pub const CHARGE_LOG_SUMMARY: u8 = 0x40;
    /// Calibration table, part 1 of 3.
    pub const MASTERS_1: u8 = 0x44;
    /// Calibration table, part 2 of 3.
    pub const MASTERS_2: u8 = 0x45;
    /// Calibration table, part 3 of 3.
    pub const MASTERS_3: u8 = 0x46;
    /// Acknowledgement of `ReqAutoStartCommand2` (`CC`).
    pub const AUTO_START_ACK_1: u8 = 0x49;
    /// Acknowledgement of `ReqAutoStartCommand4` (`CE`).
    pub const AUTO_START_ACK_2: u8 = 0x4B;
    /// `ErrorLogTransmission`.
    pub const ERROR_LOG: u8 = 0x50;
    /// `GetVibrationStatus` (inverted: `0` means vibrating).
    pub const VIBRATION: u8 = 0x7A;
    /// `GetLockPatternNotification`.
    pub const LOCK: u8 = 0x9F;
    /// Diagnostics channel reply `[type][seq][data…]`.
    pub const DIAGNOSTICS: u8 = 0xF2;
    /// `SendLatestErrorLog`.
    pub const LATEST_ERROR_LOG: u8 = 0xF4;
    /// `GetDeviceRSSI`: signed dBm.
    pub const RSSI: u8 = 0xFC;
}

/// Why an opcode must not be sent without explicit confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum DangerReason {
    /// `C4` acknowledges logs, which permanently erases them from the device.
    ErasesLogs,
    /// `A1` turns Bluetooth off; it can only be re-enabled on the device.
    DisablesBluetooth,
    /// `FF` puts the device into firmware-update mode.
    EntersFirmwareUpdate,
}

/// Opcodes that are refused unless the caller explicitly confirms them.
pub const DANGEROUS_OPCODES: [(u8, DangerReason); 3] = [
    (request::DELETE_LOGS, DangerReason::ErasesLogs),
    (request::DISABLE_BLUETOOTH, DangerReason::DisablesBluetooth),
    (request::FIRMWARE_UPDATE, DangerReason::EntersFirmwareUpdate),
];

/// Returns why `opcode` is dangerous, or `None` if it is safe to send.
pub fn danger(opcode: u8) -> Option<DangerReason> {
    DANGEROUS_OPCODES
        .iter()
        .find(|(op, _)| *op == opcode)
        .map(|&(_, reason)| reason)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dangerous_opcodes() {
        assert_eq!(danger(0xC4), Some(DangerReason::ErasesLogs));
        assert_eq!(danger(0xA1), Some(DangerReason::DisablesBluetooth));
        assert_eq!(danger(0xFF), Some(DangerReason::EntersFirmwareUpdate));
        assert_eq!(danger(0xE3), None);
        assert_eq!(danger(0xC1), None);
    }
}
