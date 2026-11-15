//! The crate's error type and its mapping to wire [`ErrorCode`]s.

use ploom_protocol::{CalibrationError, DangerReason, FrameError, ProfileError};

use crate::model::{CommandError, ErrorCode};

/// Errors returned by [`SessionHandle`](crate::SessionHandle), [`Device`](crate::Device) and the
/// transports.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// No device is connected, or the link dropped mid-operation.
    #[error("not connected to a device")]
    NotConnected,

    /// Another long-running operation (profile flashing, scanning) holds the device.
    #[error("device is busy: {0}")]
    Busy(&'static str),

    /// Scanning did not find the requested device (or any Ploom).
    #[error("{}", match .id {
        Some(id) => format!("device {id} not found; it may be asleep (press its button) or connected to another app"),
        None => "no Ploom device found; it may be asleep (press its button) or connected to another app".to_owned(),
    })]
    DeviceNotFound {
        /// The id that was looked for, if any.
        id: Option<String>,
    },

    /// The Bluetooth adapter is missing, powered off, or permission was denied.
    #[error("bluetooth unavailable: {0}")]
    BluetoothUnavailable(String),

    /// The device did not answer a request in time.
    #[error("no 0x{expected:02x} reply to 0x{request:02x} within {timeout_ms} ms")]
    Timeout {
        /// Opcode of the request.
        request: u8,
        /// Opcode of the awaited reply.
        expected: u8,
        /// How long we waited.
        timeout_ms: u64,
    },

    /// The device answered with a non-zero status or reported a different result than requested.
    #[error("device rejected 0x{opcode:02x}: {detail}")]
    DeviceRejected {
        /// The request that was rejected.
        opcode: u8,
        /// What the device answered.
        detail: String,
    },

    /// The device's calibration table cannot be used to build a profile.
    #[error(transparent)]
    Calibration(#[from] CalibrationError),

    /// A caller-supplied value is invalid.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// A raw frame could not be parsed.
    #[error("invalid frame: {0}")]
    InvalidFrame(#[from] FrameError),

    /// A raw frame uses a dangerous opcode and was not confirmed.
    #[error("opcode 0x{opcode:02x} is dangerous ({reason:?}); resend with confirmation")]
    DangerousFrame {
        /// The opcode.
        opcode: u8,
        /// Why it is dangerous.
        reason: DangerReason,
    },

    /// Refused because a stick-limit cooldown is running: unlocking the device or relaxing the
    /// limit has to wait until it ends.
    #[error("a stick-limit cooldown is running until {until_ms} (unix ms)")]
    CooldownActive {
        /// When the cooldown ends, Unix ms.
        until_ms: u64,
    },

    /// A Bluetooth stack error that maps to no more specific variant.
    #[error("bluetooth error: {0}")]
    Bluetooth(String),

    /// Reading or writing the session's data directory failed.
    #[error("storage error: {0}")]
    Storage(#[from] std::io::Error),

    /// A bug or an unexpected state; the message says which.
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// The stable code the UI localizes.
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::NotConnected => ErrorCode::NotConnected,
            Self::Busy(_) => ErrorCode::Busy,
            Self::DeviceNotFound { .. } => ErrorCode::DeviceNotFound,
            Self::BluetoothUnavailable(_) => ErrorCode::BluetoothUnavailable,
            Self::Timeout { .. } => ErrorCode::Timeout,
            Self::DeviceRejected { .. } => ErrorCode::DeviceRejected,
            Self::Calibration(_) => ErrorCode::CalibrationOutOfRange,
            Self::InvalidInput(_) | Self::InvalidFrame(_) => ErrorCode::InvalidInput,
            Self::DangerousFrame { .. } => ErrorCode::DangerousFrame,
            Self::CooldownActive { .. } => ErrorCode::CooldownActive,
            Self::Bluetooth(_) | Self::Storage(_) | Self::Internal(_) => ErrorCode::Internal,
        }
    }

    /// The `{ code, detail }` shape every command rejects with over IPC.
    pub fn to_command_error(&self) -> CommandError {
        CommandError {
            code: self.code(),
            detail: self.to_string(),
        }
    }
}

impl From<&Error> for CommandError {
    fn from(e: &Error) -> Self {
        e.to_command_error()
    }
}

impl From<Error> for CommandError {
    fn from(e: Error) -> Self {
        e.to_command_error()
    }
}

impl From<ProfileError> for Error {
    fn from(e: ProfileError) -> Self {
        match e {
            ProfileError::Calibration(c) => Self::Calibration(c),
            other => Self::InvalidInput(other.to_string()),
        }
    }
}

#[cfg(feature = "btleplug")]
impl From<btleplug::Error> for Error {
    fn from(e: btleplug::Error) -> Self {
        use btleplug::Error as B;
        match e {
            B::PermissionDenied => Self::BluetoothUnavailable("permission denied".into()),
            B::DeviceNotFound => Self::DeviceNotFound { id: None },
            B::NotConnected => Self::NotConnected,
            other => Self::Bluetooth(other.to_string()),
        }
    }
}

/// Shorthand for `Result<T, ploom_core::Error>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes() {
        let cases = [
            (Error::NotConnected, ErrorCode::NotConnected),
            (Error::Busy("flashing"), ErrorCode::Busy),
            (
                Error::DeviceNotFound { id: None },
                ErrorCode::DeviceNotFound,
            ),
            (
                Error::BluetoothUnavailable("off".into()),
                ErrorCode::BluetoothUnavailable,
            ),
            (
                Error::Timeout {
                    request: 0xE3,
                    expected: 0x3C,
                    timeout_ms: 1,
                },
                ErrorCode::Timeout,
            ),
            (
                Error::DeviceRejected {
                    opcode: 0xCC,
                    detail: "status 1".into(),
                },
                ErrorCode::DeviceRejected,
            ),
            (
                Error::Calibration(CalibrationError::OutOfRange { key: 9 }),
                ErrorCode::CalibrationOutOfRange,
            ),
            (Error::InvalidInput("x".into()), ErrorCode::InvalidInput),
            (
                Error::InvalidFrame(FrameError::InvalidHex),
                ErrorCode::InvalidInput,
            ),
            (
                Error::DangerousFrame {
                    opcode: 0xC4,
                    reason: DangerReason::ErasesLogs,
                },
                ErrorCode::DangerousFrame,
            ),
            (Error::Bluetooth("x".into()), ErrorCode::Internal),
            (Error::Internal("x".into()), ErrorCode::Internal),
        ];
        for (error, code) in cases {
            assert_eq!(error.code(), code, "{error}");
        }
    }

    #[test]
    fn command_error_carries_detail() {
        let e = Error::Timeout {
            request: 0xE3,
            expected: 0x3C,
            timeout_ms: 1500,
        };
        let c = CommandError::from(&e);
        assert_eq!(c.code, ErrorCode::Timeout);
        assert_eq!(c.detail, "no 0x3c reply to 0xe3 within 1500 ms");
    }

    #[test]
    fn profile_errors_map() {
        assert_eq!(
            Error::from(ProfileError::NoCurve).code(),
            ErrorCode::InvalidInput
        );
        let c = ProfileError::Calibration(CalibrationError::ZeroDivisor);
        assert_eq!(Error::from(c).code(), ErrorCode::CalibrationOutOfRange);
    }
}
