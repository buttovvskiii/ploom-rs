//! Bluetooth LE transport, device session and state for the Ploom Aura (GEN4L).
//!
//! ```no_run
//! use ploom_core::{SessionConfig, SessionHandle};
//! use ploom_core::model::ConnectionState;
//!
//! # async fn demo() -> Result<(), ploom_core::Error> {
//! let session = SessionHandle::start(&SessionConfig {
//!     data_dir: "/tmp/ploom".into(),
//!     auto_connect: false,
//! });
//! session.connect(None).await?;
//! let mut snapshots = session.subscribe_snapshot();
//! snapshots
//!     .wait_for(|s| s.connection == ConnectionState::Connected)
//!     .await
//!     .expect("session running");
//! println!("battery: {:?}", session.snapshot().status.battery_pct);
//! # Ok(())
//! # }
//! ```

#![warn(missing_docs)]

pub mod catalog;
pub mod device;
pub mod error;
pub mod model;
pub mod session;
pub mod transport;

pub use device::Device;
pub use error::{Error, Result};
pub use session::{SessionConfig, SessionHandle};
pub use transport::{Connector, Transport};
