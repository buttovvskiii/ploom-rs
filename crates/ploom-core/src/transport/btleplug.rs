//! [`Connector`]/[`Transport`] on top of [btleplug](https://docs.rs/btleplug).

use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use btleplug::api::{
    Central, CentralEvent, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::StreamExt;
use tokio::sync::OnceCell;
use tokio::time::{Instant, sleep};
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{
    Advertisement, Connector, Link, NotificationStream, SCAN_PASSES, Transport, gatt, is_ploom,
};
use crate::error::{Error, Result};

const DISCOVERY_WINDOW: Duration = Duration::from_secs(10);
const DISCOVERY_TICK: Duration = Duration::from_millis(250);

/// Connects through the first Bluetooth adapter of the system.
#[derive(Default)]
pub struct BtleplugConnector {
    adapter: OnceCell<Adapter>,
}

impl fmt::Debug for BtleplugConnector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BtleplugConnector").finish_non_exhaustive()
    }
}

impl BtleplugConnector {
    /// A connector that opens the system's first adapter on first use.
    pub fn new() -> Self {
        Self::default()
    }

    async fn adapter(&self) -> Result<&Adapter> {
        self.adapter
            .get_or_try_init(|| async {
                let manager = Manager::new().await.map_err(unavailable)?;
                manager
                    .adapters()
                    .await
                    .map_err(unavailable)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| Error::BluetoothUnavailable("no Bluetooth adapter found".into()))
            })
            .await
    }

    async fn ploom_advertisement(p: &Peripheral) -> Option<Advertisement> {
        let props = p.properties().await.ok().flatten()?;
        is_ploom(props.local_name.as_deref(), &props.services).then(|| Advertisement {
            id: p.id().to_string(),
            name: props.local_name,
            rssi_dbm: props.rssi,
        })
    }

    async fn discover(
        &self,
        window: Duration,
        wanted: impl Fn(&Peripheral, Option<&Advertisement>) -> bool + Send,
    ) -> Result<Option<(Peripheral, Option<String>)>> {
        let adapter = self.adapter().await?;
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(unavailable)?;
        let deadline = Instant::now() + window;
        let found = loop {
            let mut hit = None;
            for p in adapter.peripherals().await? {
                let ad = Self::ploom_advertisement(&p).await;
                if wanted(&p, ad.as_ref()) {
                    hit = Some((p, ad.and_then(|a| a.name)));
                    break;
                }
            }
            if hit.is_some() || Instant::now() >= deadline {
                break hit;
            }
            sleep(DISCOVERY_TICK).await;
        };
        if let Err(e) = adapter.stop_scan().await {
            debug!(error = %e, "stop_scan failed");
        }
        Ok(found)
    }
}

#[async_trait]
impl Connector for BtleplugConnector {
    async fn scan(&self, window: Duration) -> Result<Vec<Advertisement>> {
        let adapter = self.adapter().await?;
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(unavailable)?;
        sleep(window).await;
        if let Err(e) = adapter.stop_scan().await {
            debug!(error = %e, "stop_scan failed");
        }
        let mut out = Vec::new();
        for p in adapter.peripherals().await? {
            if let Some(ad) = Self::ploom_advertisement(&p).await {
                out.push(ad);
            }
        }
        Ok(out)
    }

    async fn connect(&self, id: Option<&str>) -> Result<Link> {
        let mut found = None;
        for pass in 1..=SCAN_PASSES {
            debug!(pass, ?id, "looking for device");
            found = self
                .discover(DISCOVERY_WINDOW, |p, ad| match id {
                    Some(id) => p.id().to_string() == id,
                    None => ad.is_some(),
                })
                .await?;
            if found.is_some() {
                break;
            }
        }
        let (peripheral, name) = found.ok_or_else(|| Error::DeviceNotFound {
            id: id.map(str::to_owned),
        })?;
        let id = peripheral.id().to_string();
        info!(%id, ?name, "connecting");

        if !peripheral.is_connected().await.unwrap_or(false) {
            peripheral.connect().await?;
        }
        match subscribe(&peripheral).await {
            Ok((tx, rx)) => {
                let adapter = self.adapter().await?.clone();
                let transport = BtleplugTransport {
                    adapter,
                    peripheral,
                    tx,
                    rx,
                };
                Ok(Link {
                    id,
                    name,
                    transport: std::sync::Arc::new(transport),
                })
            }
            Err(e) => {
                // The device holds a single connection; never leave it dangling.
                if let Err(e) = peripheral.disconnect().await {
                    warn!(error = %e, "disconnect after failed setup");
                }
                Err(e)
            }
        }
    }
}

async fn subscribe(p: &Peripheral) -> Result<(Characteristic, Characteristic)> {
    p.discover_services().await?;
    let chars = p.characteristics();
    let find = |uuid: Uuid| chars.iter().find(|c| c.uuid == uuid).cloned();
    let (Some(tx), Some(rx)) = (find(gatt::TX), find(gatt::RX)) else {
        return Err(Error::Bluetooth(
            "Ploom service not found on the device".into(),
        ));
    };
    p.subscribe(&rx).await?;
    Ok((tx, rx))
}

/// A connected peripheral.
pub struct BtleplugTransport {
    adapter: Adapter,
    peripheral: Peripheral,
    tx: Characteristic,
    rx: Characteristic,
}

impl fmt::Debug for BtleplugTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BtleplugTransport")
            .field("id", &self.peripheral.id())
            .finish_non_exhaustive()
    }
}

enum Event {
    Data(Vec<u8>),
    Lost,
}

#[async_trait]
impl Transport for BtleplugTransport {
    async fn write(&self, frame: &[u8]) -> Result<()> {
        self.peripheral
            .write(&self.tx, frame, WriteType::WithResponse)
            .await?;
        Ok(())
    }

    async fn notifications(&self) -> Result<NotificationStream> {
        let rx_uuid = self.rx.uuid;
        let id = self.peripheral.id();
        let data = self.peripheral.notifications().await?.filter_map(move |n| {
            futures::future::ready((n.uuid == rx_uuid).then_some(Event::Data(n.value)))
        });
        // btleplug does not end the notification stream on every platform
        // when the link drops, so watch the adapter for the disconnect.
        let lost = self.adapter.events().await?.filter_map(move |e| {
            futures::future::ready(
                matches!(&e, CentralEvent::DeviceDisconnected(p) if *p == id)
                    .then_some(Event::Lost),
            )
        });
        let merged = futures::stream::select(data, lost)
            .take_while(|e| futures::future::ready(matches!(e, Event::Data(_))))
            .filter_map(|e| {
                futures::future::ready(match e {
                    Event::Data(v) => Some(v),
                    Event::Lost => None,
                })
            });
        Ok(Box::pin(merged))
    }

    async fn read_characteristic(&self, uuid: Uuid) -> Result<Vec<u8>> {
        let chars = self.peripheral.characteristics();
        let c = chars
            .iter()
            .find(|c| c.uuid == uuid)
            .ok_or_else(|| Error::Bluetooth(format!("characteristic {uuid} not found")))?;
        Ok(self.peripheral.read(c).await?)
    }

    async fn disconnect(&self) -> Result<()> {
        self.peripheral.disconnect().await?;
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.peripheral.is_connected().await.unwrap_or(false)
    }
}

fn unavailable(e: btleplug::Error) -> Error {
    match e {
        btleplug::Error::PermissionDenied => {
            Error::BluetoothUnavailable("Bluetooth permission denied".into())
        }
        other => Error::BluetoothUnavailable(other.to_string()),
    }
}
