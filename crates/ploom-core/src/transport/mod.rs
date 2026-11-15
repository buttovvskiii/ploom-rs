//! The Bluetooth boundary.

use std::fmt;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use uuid::Uuid;

use crate::error::Result;

#[cfg(feature = "btleplug")]
pub mod btleplug;
#[cfg(any(test, feature = "test-util"))]
pub mod fake;

/// GATT identifiers.
pub mod gatt {
    use uuid::Uuid;

    /// The Ploom service.
    pub const SERVICE: Uuid = Uuid::from_u128(0x5365_4010_a391_4a65_83fa_bc58_084a_ca28);
    /// TX: commands to the device.
    pub const TX: Uuid = Uuid::from_u128(0x5365_4011_a391_4a65_83fa_bc58_084a_ca28);
    /// RX: replies and telemetry, delivered as indications.
    pub const RX: Uuid = Uuid::from_u128(0x5365_4012_a391_4a65_83fa_bc58_084a_ca28);
    /// Device Information Service: Hardware Revision String (`0x2A27`).
    pub const DIS_HARDWARE_REVISION: Uuid =
        Uuid::from_u128(0x0000_2a27_0000_1000_8000_0080_5f9b_34fb);
    /// Device Information Service: Software Revision String (`0x2A28`), which carries the firmware
    /// version (e.g. `M1.0014_G4`).
    pub const DIS_FIRMWARE_REVISION: Uuid =
        Uuid::from_u128(0x0000_2a28_0000_1000_8000_0080_5f9b_34fb);
}

/// Advertised-name prefix of every Ploom Aura (`Ploom aura 123456VL`).
pub const NAME_PREFIX: &str = "Ploom aura";

/// Number of scan passes when looking for devices.
pub const SCAN_PASSES: u32 = 2;

/// Whether an advertisement looks like a Ploom: the name starts with [`NAME_PREFIX`]
/// (case-insensitively) or the Ploom service is advertised.
pub fn is_ploom(name: Option<&str>, services: &[Uuid]) -> bool {
    let named = name.is_some_and(|n| {
        n.get(..NAME_PREFIX.len())
            .is_some_and(|p| p.eq_ignore_ascii_case(NAME_PREFIX))
    });
    named || services.contains(&gatt::SERVICE)
}

/// Stream of raw indications from RX.
pub type NotificationStream = Pin<Box<dyn Stream<Item = Vec<u8>> + Send>>;

/// A connected link to one device.
#[async_trait]
pub trait Transport: Send + Sync + fmt::Debug {
    /// Writes a frame to TX with response; `CoreBluetooth` drops writes without one.
    async fn write(&self, frame: &[u8]) -> Result<()>;

    /// Subscribes to RX indications.
    async fn notifications(&self) -> Result<NotificationStream>;

    /// Reads a readable characteristic, e.g. [`gatt::DIS_FIRMWARE_REVISION`].
    async fn read_characteristic(&self, uuid: Uuid) -> Result<Vec<u8>>;

    /// Closes the link.
    async fn disconnect(&self) -> Result<()>;

    /// Whether the link is up.
    async fn is_connected(&self) -> bool;
}

/// A device seen while scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advertisement {
    /// Platform peripheral id (a UUID on macOS, a MAC address elsewhere).
    pub id: String,
    /// Advertised local name.
    pub name: Option<String>,
    /// Advertisement RSSI, dBm.
    pub rssi_dbm: Option<i16>,
}

/// A freshly established link.
#[derive(Debug, Clone)]
pub struct Link {
    /// Platform peripheral id.
    pub id: String,
    /// Advertised local name.
    pub name: Option<String>,
    /// The connected transport, already subscribed to RX.
    pub transport: Arc<dyn Transport>,
}

/// Discovers and connects to Ploom devices.
#[async_trait]
pub trait Connector: Send + Sync + fmt::Debug {
    /// Scans for `window` and returns every Ploom seen (see [`is_ploom`]).
    async fn scan(&self, window: Duration) -> Result<Vec<Advertisement>>;

    /// Connects to the device with platform id `id`, or to the first Ploom found when `id` is
    /// `None`, scanning up to [`SCAN_PASSES`] times.
    async fn connect(&self, id: Option<&str>) -> Result<Link>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ploom_detection() {
        assert!(is_ploom(Some("Ploom aura 123456VL"), &[]));
        assert!(is_ploom(Some("PLOOM AURA"), &[]));
        assert!(!is_ploom(Some("Ploom"), &[]));
        assert!(!is_ploom(Some("Plo"), &[]));
        assert!(
            !is_ploom(Some("Пloom aura"), &[]),
            "non-ASCII prefix must not panic"
        );
        assert!(!is_ploom(None, &[]));
        assert!(is_ploom(None, &[gatt::SERVICE]));
    }

    #[test]
    fn uuids() {
        assert_eq!(
            gatt::SERVICE.to_string(),
            "53654010-a391-4a65-83fa-bc58084aca28"
        );
        assert_eq!(gatt::TX.to_string(), "53654011-a391-4a65-83fa-bc58084aca28");
        assert_eq!(gatt::RX.to_string(), "53654012-a391-4a65-83fa-bc58084aca28");
        assert_eq!(
            gatt::DIS_HARDWARE_REVISION.to_string(),
            "00002a27-0000-1000-8000-00805f9b34fb"
        );
        assert_eq!(
            gatt::DIS_FIRMWARE_REVISION.to_string(),
            "00002a28-0000-1000-8000-00805f9b34fb"
        );
    }
}
