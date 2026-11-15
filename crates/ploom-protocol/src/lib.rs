//! Pure implementation of the Ploom Aura (GEN4L) Bluetooth LE protocol.
//!
//! ```
//! use ploom_protocol::{command::Command, notification::Notification};
//!
//! let request = Command::RequestStatus.frame();
//! assert_eq!(request.as_bytes(), [0x02, 0xE3, 0x00]);
//!
//! let reply = Notification::parse(&[0x03, 0x3C, 0x55, 0x00]).unwrap();
//! assert_eq!(reply, Notification::BatteryLevel { pct: 85 });
//! ```

#![warn(missing_docs)]

pub mod calibration;
pub mod color;
pub mod command;
pub mod diagnostics;
pub mod frame;
pub mod heating;
pub mod notification;
pub mod opcode;
pub mod status;

pub use calibration::{CalibrationError, Masters};
pub use color::DeviceColor;
pub use command::Command;
pub use frame::{Frame, FrameError};
pub use heating::{CustomProfile, HeatingMode, HeatingProfile, ProfileError, ProfileFrame};
pub use notification::Notification;
pub use opcode::DangerReason;
pub use status::{StatusUpdate, StatusWindow};
