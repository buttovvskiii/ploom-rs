//! An in-memory simulated Ploom, for tests and for development without hardware.
//!
//! ```
//! use ploom_core::transport::Connector;
//! use ploom_core::transport::fake::{FakeConnector, FakeDevice};
//!
//! # let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
//! # rt.block_on(async {
//! let device = FakeDevice::new("fake-1", "Ploom aura TEST");
//! let connector = FakeConnector::new([device.clone()]);
//! let link = connector.connect(None).await.unwrap();
//! link.transport.write(&[0x02, 0x79, 0x01]).await.unwrap();
//! assert!(device.state().vibrating);
//! # });
//! ```

use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use uuid::Uuid;

use super::{Advertisement, Connector, Link, NotificationStream, Transport, gatt};
use crate::error::{Error, Result};

/// What a request interceptor wants done with a written frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// Let the simulated firmware answer as usual.
    Default,
    /// Swallow the frame: no state change, no reply.
    Ignore,
    /// Send exactly these indications instead of the usual reply.
    Reply(Vec<Vec<u8>>),
}

type Interceptor = Box<dyn FnMut(&[u8]) -> Response + Send>;

/// Simulated device state.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools, reason = "independent simulated flags")]
pub struct FakeState {
    /// State of charge, percent.
    pub battery_pct: u8,
    /// Battery health, percent.
    pub battery_health_pct: u8,
    /// RSSI the device reports, dBm.
    pub rssi_dbm: i8,
    /// Raw lock byte (`0` locked, `1` unlocked, `0x11` feature off).
    pub lock: u8,
    /// Auto-Start Heating.
    pub auto_start: bool,
    /// The buzzer is on.
    pub vibrating: bool,
    /// The downloaded profile is active (`0x3D`).
    pub downloaded_active: bool,
    /// Number of the downloaded profile (`0x3E`).
    pub downloaded_profile: u8,
    /// `0x30` byte 4 flags (bit 5 in use, bit 6 preheating, …).
    pub status_flags: u8,
    /// Like the real firmware, ignore `E3` while heating.
    pub ignore_status_while_heating: bool,
    /// Calibration table returned by `A5`.
    pub masters: Vec<u16>,
    /// USB-C CC line readings (diagnostics type 7).
    pub usb_cc: (u16, u16),
    /// Enclosure temperature in centi-degrees (diagnostics type 9).
    pub enclosure_centi_c: u16,
    /// Lifetime heating-start counters (diagnostics type 12).
    pub heating_starts: [u16; 7],
    /// Unacknowledged `(session, charge)` logs.
    pub unacquired_logs: (u16, u16),
    /// Lifetime charge count (`0x40`).
    pub charge_count: u16,
    /// Number of upcoming writes to swallow (the device ignores writes for a while after
    /// subscribing).
    pub swallow_writes: u32,
    /// DIS hardware revision.
    pub hardware: String,
    /// DIS firmware revision.
    pub firmware: String,
    /// Profile number seen in the last uploaded `BB` frame, applied on `A4 01`.
    pub pending_profile: Option<u8>,
    /// Counter reported in `0x30` bytes 0..2.
    pub uptime_s: u16,
}

impl Default for FakeState {
    fn default() -> Self {
        let mut masters: Vec<u16> = (0..18).map(|i| 3000 + 11 * i).collect();
        masters[4] = 2500;
        masters[7] = 1200;
        masters[8] = 1190;
        Self {
            battery_pct: 80,
            battery_health_pct: 95,
            rssi_dbm: -55,
            lock: 1,
            auto_start: false,
            vibrating: false,
            downloaded_active: false,
            downloaded_profile: 0,
            status_flags: 0,
            ignore_status_while_heating: true,
            masters,
            usb_cc: (28, 28),
            enclosure_centi_c: 2650,
            heating_starts: [12, 3, 0, 1, 0, 0, 2],
            unacquired_logs: (6, 7),
            charge_count: 42,
            swallow_writes: 0,
            hardware: "U123456VL4U66CJ 170B".into(),
            firmware: "M1.0014_G4".into(),
            pending_profile: None,
            uptime_s: 1000,
        }
    }
}

impl FakeState {
    fn heating(&self) -> bool {
        self.status_flags & 0x60 != 0
    }

    fn status_frame(&self) -> Vec<u8> {
        let [u0, u1] = self.uptime_s.to_le_bytes();
        let source = u8::from(self.downloaded_active);
        let auto = u8::from(self.auto_start);
        vec![
            0x10,
            0x30,
            u0,
            u1,
            0,
            0,
            self.status_flags,
            self.lock,
            0,
            0,
            0,
            0,
            0,
            auto,
            0,
            source,
            0,
        ]
    }

    fn lock_frame(&self) -> Vec<u8> {
        let mut f = vec![0x0C, 0x9F, self.lock];
        f.extend([0; 10]);
        f
    }

    fn unacquired_frame(&self) -> Vec<u8> {
        let [s0, s1] = self.unacquired_logs.0.to_le_bytes();
        let [c0, c1] = self.unacquired_logs.1.to_le_bytes();
        vec![0x05, 0x35, s0, s1, c0, c1]
    }

    fn diag_frame(&self, kind: u8) -> Vec<u8> {
        let fields: Vec<u16> = match kind {
            7 => vec![1650, self.usb_cc.0, self.usb_cc.1, 2900, 3800, 2950, 0],
            9 => vec![self.enclosure_centi_c],
            12 => self.heating_starts.to_vec(),
            _ => vec![0xFFFF],
        };
        let mut payload = vec![kind, 0];
        payload.extend(fields.iter().flat_map(|v| v.to_le_bytes()));
        frame(0xF2, &payload)
    }

    fn respond(&mut self, f: &[u8]) -> Vec<Vec<u8>> {
        let (Some(&op), arg) = (f.get(1), f.get(2).copied().unwrap_or(0)) else {
            return Vec::new();
        };
        match op {
            0xE3 if self.ignore_status_while_heating && self.heating() => Vec::new(),
            0xE3 => vec![
                self.status_frame(),
                vec![0x02, 0x33, self.battery_health_pct],
                self.lock_frame(),
                vec![0x03, 0x3C, self.battery_pct, 0],
            ],
            0xFD => vec![vec![0x02, 0xFC, self.rssi_dbm.to_le_bytes()[0]]],
            0xA2 => vec![vec![0x02, 0x3D, u8::from(self.downloaded_active)]],
            0xA3 => vec![vec![0x08, 0x3E, 0, self.downloaded_profile, 0, 0, 0, 0, 0]],
            0xA4 => {
                self.downloaded_active = arg == 1;
                if arg == 1 {
                    if let Some(n) = self.pending_profile.take() {
                        self.downloaded_profile = n;
                    }
                }
                vec![vec![0x02, 0x3F, 0]]
            }
            0xA5 => self
                .masters
                .chunks(6)
                .zip(0x44..=0x46_u8)
                .map(|(chunk, op)| {
                    let payload: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
                    frame(op, &payload)
                })
                .collect(),
            0xBB => {
                self.pending_profile = Some(arg);
                Vec::new()
            }
            0xCC => vec![vec![0x02, 0x49, 0]],
            0xCE => vec![vec![0x02, 0x4B, 0]],
            0x79 => {
                self.vibrating = arg == 1;
                vec![vec![0x02, 0x7A, u8::from(!self.vibrating)]]
            }
            0x9E => {
                self.lock = u8::from(arg != 1);
                vec![self.lock_frame()]
            }
            0xCA => {
                self.auto_start = arg == 1;
                Vec::new()
            }
            0xF1 => vec![self.diag_frame(arg)],
            0xC1 => vec![
                self.unacquired_frame(),
                hex("1336b033000068010800000100010e00000000b095"),
                hex("13370e005d009000d000e6000901240157010000"),
                hex("1338000000000000000000000000000000000000"),
                hex("0f390000000000000000000000000000"),
                hex("133a1b01ad02a401620d9c0b0000000000000000"),
            ],
            0xC2 => {
                let mut summary = vec![0x0F, 0x40];
                summary.extend(self.charge_count.to_le_bytes());
                summary.extend([1, 0, 0, 0, self.battery_health_pct]);
                summary.extend([0; 7]);
                vec![
                    self.unacquired_frame(),
                    hex("133b030000002f010000fd0d960ed00be60b0000"),
                    summary,
                ]
            }
            _ => Vec::new(),
        }
    }
}

fn frame(op: u8, payload: &[u8]) -> Vec<u8> {
    let mut f = Vec::with_capacity(payload.len() + 2);
    f.push((payload.len() + 1) as u8);
    f.push(op);
    f.extend_from_slice(payload);
    f
}

fn hex(s: &str) -> Vec<u8> {
    ploom_protocol::frame::decode_hex(s).unwrap_or_default()
}

struct LinkSlot {
    generation: u64,
    sender: mpsc::UnboundedSender<Vec<u8>>,
}

struct Inner {
    id: String,
    name: String,
    advertised_rssi: Mutex<Option<i16>>,
    state: Mutex<FakeState>,
    link: Mutex<Option<LinkSlot>>,
    generation: AtomicU64,
    reachable: AtomicBool,
    connects: AtomicUsize,
    writes: Mutex<Vec<Vec<u8>>>,
    interceptor: Mutex<Option<Interceptor>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A simulated Ploom.
#[derive(Clone)]
pub struct FakeDevice {
    inner: Arc<Inner>,
}

impl fmt::Debug for FakeDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FakeDevice")
            .field("id", &self.inner.id)
            .field("name", &self.inner.name)
            .finish_non_exhaustive()
    }
}

impl FakeDevice {
    /// A reachable device with [`FakeState::default`].
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            inner: Arc::new(Inner {
                id: id.into(),
                name: name.into(),
                advertised_rssi: Mutex::new(Some(-60)),
                state: Mutex::new(FakeState::default()),
                link: Mutex::new(None),
                generation: AtomicU64::new(0),
                reachable: AtomicBool::new(true),
                connects: AtomicUsize::new(0),
                writes: Mutex::new(Vec::new()),
                interceptor: Mutex::new(None),
            }),
        }
    }

    /// Platform id.
    pub fn id(&self) -> &str {
        &self.inner.id
    }

    /// A copy of the current state.
    pub fn state(&self) -> FakeState {
        lock(&self.inner.state).clone()
    }

    /// Changes the state.
    pub fn update(&self, f: impl FnOnce(&mut FakeState)) {
        f(&mut lock(&self.inner.state));
    }

    /// Sets the advertisement RSSI (`None` hides it).
    pub fn set_advertised_rssi(&self, rssi: Option<i16>) {
        *lock(&self.inner.advertised_rssi) = rssi;
    }

    /// Installs a hook that sees every write before the simulated firmware.
    pub fn intercept(&self, f: impl FnMut(&[u8]) -> Response + Send + 'static) {
        *lock(&self.inner.interceptor) = Some(Box::new(f));
    }

    /// Removes the interceptor.
    pub fn clear_intercept(&self) {
        *lock(&self.inner.interceptor) = None;
    }

    /// Every frame written so far, across links.
    pub fn writes(&self) -> Vec<Vec<u8>> {
        lock(&self.inner.writes).clone()
    }

    /// Writes whose opcode is `op`.
    pub fn writes_with_opcode(&self, op: u8) -> Vec<Vec<u8>> {
        self.writes()
            .into_iter()
            .filter(|w| w.get(1) == Some(&op))
            .collect()
    }

    /// Forgets recorded writes.
    pub fn clear_writes(&self) {
        lock(&self.inner.writes).clear();
    }

    /// How many links have been established.
    pub fn connect_count(&self) -> usize {
        self.inner.connects.load(Ordering::SeqCst)
    }

    /// Whether a link is currently up.
    pub fn is_linked(&self) -> bool {
        lock(&self.inner.link).is_some()
    }

    /// Makes the device (un)discoverable, like waking it up or putting it to sleep.
    pub fn set_reachable(&self, reachable: bool) {
        self.inner.reachable.store(reachable, Ordering::SeqCst);
    }

    /// Sends a spontaneous indication (e.g. `04 31 04 00 00`).
    pub fn emit(&self, bytes: impl Into<Vec<u8>>) -> bool {
        lock(&self.inner.link)
            .as_ref()
            .is_some_and(|l| l.sender.send(bytes.into()).is_ok())
    }

    /// Simulates a session starting: sets the in-use flag, bumps the first lifetime heating-start
    /// counter and emits `0x31` smoking.
    pub fn start_session(&self) {
        self.update(|s| {
            s.status_flags = 0x20;
            s.heating_starts[0] = s.heating_starts[0].wrapping_add(1);
        });
        self.emit([0x04, 0x31, 0x04, 0x00, 0x00]);
    }

    /// Simulates a session ending: clears the flags and emits `0x31` standby.
    pub fn end_session(&self) {
        self.update(|s| s.status_flags = 0);
        self.emit([0x04, 0x31, 0x00, 0x00, 0x00]);
    }

    /// Drops the link as if the device went out of range.
    pub fn drop_link(&self) {
        lock(&self.inner.link).take();
    }

    fn open_link(&self) -> FakeTransport {
        let generation = self.inner.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let (sender, receiver) = mpsc::unbounded_channel();
        *lock(&self.inner.link) = Some(LinkSlot { generation, sender });
        self.inner.connects.fetch_add(1, Ordering::SeqCst);
        FakeTransport {
            device: self.clone(),
            generation,
            receiver: Mutex::new(Some(receiver)),
        }
    }

    fn handle_write(&self, generation: u64, data: &[u8]) -> Result<()> {
        let link = lock(&self.inner.link);
        let Some(slot) = link.as_ref().filter(|l| l.generation == generation) else {
            return Err(Error::NotConnected);
        };
        lock(&self.inner.writes).push(data.to_vec());

        let response = lock(&self.inner.interceptor)
            .as_mut()
            .map_or(Response::Default, |f| f(data));
        let replies = match response {
            Response::Ignore => Vec::new(),
            Response::Reply(replies) => replies,
            Response::Default => {
                let mut state = lock(&self.inner.state);
                if state.swallow_writes > 0 {
                    state.swallow_writes -= 1;
                    Vec::new()
                } else {
                    state.respond(data)
                }
            }
        };
        for r in replies {
            // The receiver only disappears together with the link.
            let _ = slot.sender.send(r);
        }
        Ok(())
    }
}

/// A link to a [`FakeDevice`].
pub struct FakeTransport {
    device: FakeDevice,
    generation: u64,
    receiver: Mutex<Option<mpsc::UnboundedReceiver<Vec<u8>>>>,
}

impl fmt::Debug for FakeTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FakeTransport")
            .field("device", &self.device.inner.id)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl FakeTransport {
    fn is_current(&self) -> bool {
        lock(&self.device.inner.link)
            .as_ref()
            .is_some_and(|l| l.generation == self.generation)
    }
}

#[async_trait]
impl Transport for FakeTransport {
    async fn write(&self, frame: &[u8]) -> Result<()> {
        self.device.handle_write(self.generation, frame)
    }

    async fn notifications(&self) -> Result<NotificationStream> {
        let mut receiver = lock(&self.receiver)
            .take()
            .ok_or_else(|| Error::Internal("notifications() already taken".into()))?;
        Ok(Box::pin(futures::stream::poll_fn(move |cx| {
            receiver.poll_recv(cx)
        })))
    }

    async fn read_characteristic(&self, uuid: Uuid) -> Result<Vec<u8>> {
        if !self.is_current() {
            return Err(Error::NotConnected);
        }
        let state = self.device.state();
        match uuid {
            u if u == gatt::DIS_HARDWARE_REVISION => Ok(state.hardware.into_bytes()),
            u if u == gatt::DIS_FIRMWARE_REVISION => Ok(state.firmware.into_bytes()),
            other => Err(Error::Bluetooth(format!(
                "characteristic {other} not found"
            ))),
        }
    }

    async fn disconnect(&self) -> Result<()> {
        let mut link = lock(&self.device.inner.link);
        if link
            .as_ref()
            .is_some_and(|l| l.generation == self.generation)
        {
            link.take();
        }
        Ok(())
    }

    async fn is_connected(&self) -> bool {
        self.is_current()
    }
}

/// Makes [`FakeDevice`]s discoverable and connectable.
#[derive(Debug, Default, Clone)]
pub struct FakeConnector {
    devices: Arc<Mutex<VecDeque<FakeDevice>>>,
}

impl FakeConnector {
    /// A connector that knows `devices`.
    pub fn new(devices: impl IntoIterator<Item = FakeDevice>) -> Self {
        Self {
            devices: Arc::new(Mutex::new(devices.into_iter().collect())),
        }
    }

    /// Adds a device.
    pub fn add(&self, device: FakeDevice) {
        lock(&self.devices).push_back(device);
    }

    fn reachable(&self) -> Vec<FakeDevice> {
        lock(&self.devices)
            .iter()
            .filter(|d| d.inner.reachable.load(Ordering::SeqCst))
            .cloned()
            .collect()
    }
}

#[async_trait]
impl Connector for FakeConnector {
    async fn scan(&self, window: Duration) -> Result<Vec<Advertisement>> {
        tokio::time::sleep(window).await;
        Ok(self
            .reachable()
            .into_iter()
            .map(|d| Advertisement {
                id: d.inner.id.clone(),
                name: Some(d.inner.name.clone()),
                rssi_dbm: *lock(&d.inner.advertised_rssi),
            })
            .collect())
    }

    async fn connect(&self, id: Option<&str>) -> Result<Link> {
        // Real discovery takes a while; keep the fake asynchronous too.
        tokio::time::sleep(Duration::from_millis(100)).await;
        let device = self
            .reachable()
            .into_iter()
            .find(|d| id.is_none_or(|id| d.inner.id == id))
            .ok_or_else(|| Error::DeviceNotFound {
                id: id.map(str::to_owned),
            })?;
        let transport = device.open_link();
        Ok(Link {
            id: device.inner.id.clone(),
            name: Some(device.inner.name.clone()),
            transport: Arc::new(transport),
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;

    #[tokio::test]
    async fn status_burst_and_link_drop() {
        let device = FakeDevice::new("a", "Ploom aura A");
        let link = FakeConnector::new([device.clone()])
            .connect(Some("a"))
            .await
            .unwrap();
        let mut rx = link.transport.notifications().await.unwrap();
        link.transport.write(&[2, 0xE3, 0]).await.unwrap();
        let ops: Vec<u8> = rx.by_ref().take(4).map(|f| f[1]).collect().await;
        assert_eq!(ops, [0x30, 0x33, 0x9F, 0x3C]);

        device.drop_link();
        assert!(rx.next().await.is_none(), "stream ends with the link");
        assert!(matches!(
            link.transport.write(&[2, 0xE3, 0]).await,
            Err(Error::NotConnected)
        ));
        assert!(!link.transport.is_connected().await);
    }

    #[tokio::test]
    async fn unreachable_devices_are_not_found() {
        let device = FakeDevice::new("a", "Ploom aura A");
        device.set_reachable(false);
        let connector = FakeConnector::new([device]);
        assert!(matches!(
            connector.connect(None).await,
            Err(Error::DeviceNotFound { id: None })
        ));
        assert!(connector.scan(Duration::ZERO).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn masters_are_split_across_three_replies() {
        let mut state = FakeState::default();
        let replies = state.respond(&[2, 0xA5, 0]);
        let ops: Vec<u8> = replies.iter().map(|r| r[1]).collect();
        assert_eq!(ops, [0x44, 0x45, 0x46]);
        let payloads: Vec<&[u8]> = replies.iter().map(|r| &r[2..]).collect();
        let masters = ploom_protocol::Masters::from_replies(&payloads).unwrap();
        assert_eq!(masters.values(), state.masters.as_slice());
    }
}
