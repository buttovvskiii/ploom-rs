//! Builders for every request the client sends.

use crate::diagnostics::DiagKind;
use crate::frame::Frame;
use crate::opcode::{reply, request};

/// A single request frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Command {
    /// `02 e3 00`: resend the full status.
    RequestStatus,
    /// `02 fd 00`: RSSI as seen by the device.
    RequestRssi,
    /// `02 a2 00`: whether the downloaded profile is the active one.
    RequestProfileActive,
    /// `02 a3 00`: which profile is downloaded.
    RequestDownloadedProfile,
    /// `02 a4 0x`: activate the factory (`false`) or downloaded (`true`) profile.
    ActivateProfile {
        /// `true` activates the downloaded profile, `false` the factory one (which is how
        /// "Balanced" is selected).
        downloaded: bool,
    },
    /// `02 a5 00`: one part of the calibration tables (see
    /// [`crate::calibration::MASTERS_REPLIES`]).
    RequestMasters,
    /// `02 c1 00`: the **oldest** unacknowledged session log.
    RequestSmokingLogs,
    /// `02 c2 00`: the **oldest** unacknowledged charge log.
    RequestChargingLogs,
    /// `02 f3 00`: the latest error log.
    RequestErrorLogs,
    /// `04 f1 <type> ff ff`: one record of the diagnostics channel.
    RequestDiagnostic(DiagKind),
    /// `02 79 0x`: buzz and blink on/off.
    SetVibration(bool),
    /// `02 9e 0x`: lock (`true`) or unlock the device.
    SetLock(bool),
    /// `02 ca 0x`: Auto-Start Heating on/off.
    SetAutoStart(bool),
}

impl Command {
    /// Encodes the request.
    pub fn frame(self) -> Frame {
        match self {
            Self::RequestStatus => simple(request::STATUS),
            Self::RequestRssi => simple(request::RSSI),
            Self::RequestProfileActive => simple(request::PROFILE_ACTIVE),
            Self::RequestDownloadedProfile => simple(request::DOWNLOADED_PROFILE),
            Self::ActivateProfile { downloaded } => flag(request::ACTIVATE_PROFILE, downloaded),
            Self::RequestMasters => simple(request::MASTERS),
            Self::RequestSmokingLogs => simple(request::SMOKING_LOGS),
            Self::RequestChargingLogs => simple(request::CHARGING_LOGS),
            Self::RequestErrorLogs => simple(request::ERROR_LOGS),
            Self::RequestDiagnostic(kind) => {
                Frame::build(request::DIAGNOSTICS, &[kind.code(), 0xFF, 0xFF])
            }
            Self::SetVibration(on) => flag(request::VIBRATION, on),
            Self::SetLock(locked) => flag(request::LOCK, locked),
            Self::SetAutoStart(enabled) => flag(request::AUTO_START, enabled),
        }
    }

    /// The indication that answers this request, when there is exactly one to wait for.
    pub fn expected_reply(self) -> Option<u8> {
        match self {
            Self::RequestStatus => Some(reply::BATTERY_LEVEL),
            Self::RequestRssi => Some(reply::RSSI),
            Self::RequestProfileActive => Some(reply::PROFILE_ACTIVE),
            Self::RequestDownloadedProfile => Some(reply::DOWNLOADED_PROFILE),
            Self::ActivateProfile { .. } => Some(reply::PROFILE_ACTIVATED),
            Self::RequestDiagnostic(_) => Some(reply::DIAGNOSTICS),
            Self::RequestMasters
            | Self::RequestSmokingLogs
            | Self::RequestChargingLogs
            | Self::RequestErrorLogs
            | Self::SetVibration(_)
            | Self::SetLock(_)
            | Self::SetAutoStart(_) => None,
        }
    }
}

impl From<Command> for Frame {
    fn from(command: Command) -> Self {
        command.frame()
    }
}

/// The status poll: each request paired with the reply that proves it arrived.
pub const POLL: [(Command, u8); 4] = [
    (Command::RequestStatus, reply::BATTERY_LEVEL),
    (Command::RequestRssi, reply::RSSI),
    (Command::RequestProfileActive, reply::PROFILE_ACTIVE),
    (Command::RequestDownloadedProfile, reply::DOWNLOADED_PROFILE),
];

fn simple(opcode: u8) -> Frame {
    Frame::build(opcode, &[0])
}

fn flag(opcode: u8, on: bool) -> Frame {
    Frame::build(opcode, &[u8::from(on)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(c: Command) -> String {
        c.frame().to_hex()
    }

    #[test]
    fn frames_match_reference_client() {
        assert_eq!(hex(Command::RequestStatus), "02 e3 00");
        assert_eq!(hex(Command::RequestRssi), "02 fd 00");
        assert_eq!(hex(Command::RequestProfileActive), "02 a2 00");
        assert_eq!(hex(Command::RequestDownloadedProfile), "02 a3 00");
        assert_eq!(
            hex(Command::ActivateProfile { downloaded: false }),
            "02 a4 00"
        );
        assert_eq!(
            hex(Command::ActivateProfile { downloaded: true }),
            "02 a4 01"
        );
        assert_eq!(hex(Command::RequestMasters), "02 a5 00");
        assert_eq!(hex(Command::RequestSmokingLogs), "02 c1 00");
        assert_eq!(hex(Command::RequestChargingLogs), "02 c2 00");
        assert_eq!(hex(Command::RequestErrorLogs), "02 f3 00");
        assert_eq!(
            hex(Command::RequestDiagnostic(DiagKind::EnclosureTemp)),
            "04 f1 09 ff ff"
        );
        assert_eq!(hex(Command::SetVibration(true)), "02 79 01");
        assert_eq!(hex(Command::SetVibration(false)), "02 79 00");
        assert_eq!(hex(Command::SetLock(true)), "02 9e 01");
        assert_eq!(hex(Command::SetLock(false)), "02 9e 00");
        assert_eq!(hex(Command::SetAutoStart(true)), "02 ca 01");
        assert_eq!(hex(Command::SetAutoStart(false)), "02 ca 00");
    }

    #[test]
    fn expected_replies() {
        for (cmd, reply) in POLL {
            assert_eq!(cmd.expected_reply(), Some(reply));
        }
        assert_eq!(
            Command::ActivateProfile { downloaded: true }.expected_reply(),
            Some(0x3F)
        );
        assert_eq!(
            Command::RequestDiagnostic(DiagKind::Adc).expected_reply(),
            Some(0xF2)
        );
        assert_eq!(Command::SetLock(true).expected_reply(), None);
    }
}
