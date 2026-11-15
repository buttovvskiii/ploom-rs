//! Request/response on top of a [`Transport`].

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use futures::StreamExt;
use ploom_protocol::calibration::MASTERS_REPLIES;
use ploom_protocol::diagnostics::{DiagKind, DiagReply};
use ploom_protocol::heating::FRAME_DELAY_MS;
use ploom_protocol::opcode::reply;
use ploom_protocol::{Command, Frame, Masters};
use tokio::sync::{broadcast, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep, timeout, timeout_at};
use tracing::{debug, trace, warn};

use crate::error::{Error, Result};
use crate::transport::{Transport, gatt};

/// Default time to wait for a reply (`sendCommandWithResponse` in the app).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(6);

/// `wait_ready`: how many times `RequestStatus` is sent.
pub const READY_ATTEMPTS: u32 = 6;
/// `wait_ready`: how long each attempt waits for the first indication.
pub const READY_ATTEMPT_TIMEOUT: Duration = Duration::from_millis(1200);

const BROADCAST_CAPACITY: usize = 256;

/// Device Information strings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Hardware revision (`0x2A27`), e.g. `U123456VL4U66CJ 170B`.
    pub hardware: Option<String>,
    /// Firmware revision (`0x2A28`), e.g. `M1.0014_G4`.
    pub firmware: Option<String>,
}

type Waiters = HashMap<u8, Vec<(u64, oneshot::Sender<Vec<u8>>)>>;

#[derive(Debug)]
struct Shared {
    waiters: Mutex<Waiters>,
    next_waiter: AtomicU64,
    incoming: broadcast::Sender<Vec<u8>>,
    closed: watch::Sender<bool>,
}

impl Shared {
    fn waiters(&self) -> MutexGuard<'_, Waiters> {
        self.waiters.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn dispatch(&self, bytes: Vec<u8>) {
        trace!(frame = %ploom_protocol::frame::to_hex(&bytes), "rx");
        if let [_, opcode, payload @ ..] = bytes.as_slice() {
            // Every waiter for this opcode gets the first matching reply.
            if let Some(waiters) = self.waiters().remove(opcode) {
                for (_, tx) in waiters {
                    let _ = tx.send(payload.to_vec());
                }
            }
        }
        // No subscribers is fine.
        let _ = self.incoming.send(bytes);
    }

    fn close(&self) {
        // Dropping the senders fails every pending request with NotConnected.
        self.waiters().clear();
        self.closed.send_replace(true);
    }
}

struct WaiterGuard<'a> {
    shared: &'a Shared,
    opcode: u8,
    id: u64,
}

impl Drop for WaiterGuard<'_> {
    fn drop(&mut self) {
        let mut waiters = self.shared.waiters();
        if let Some(list) = waiters.get_mut(&self.opcode) {
            list.retain(|(id, _)| *id != self.id);
            if list.is_empty() {
                waiters.remove(&self.opcode);
            }
        }
    }
}

/// A connected device: request/response, exchanges and diagnostics.
pub struct Device {
    transport: Arc<dyn Transport>,
    shared: Arc<Shared>,
    reader: JoinHandle<()>,
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("transport", &self.transport)
            .field("closed", &self.is_closed())
            .finish_non_exhaustive()
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.reader.abort();
    }
}

impl Device {
    /// Subscribes to the transport's indications and starts the reader task.
    pub async fn open(transport: Arc<dyn Transport>) -> Result<Self> {
        let mut stream = transport.notifications().await?;
        let (incoming, _) = broadcast::channel(BROADCAST_CAPACITY);
        let shared = Arc::new(Shared {
            waiters: Mutex::new(HashMap::new()),
            next_waiter: AtomicU64::new(0),
            incoming,
            closed: watch::channel(false).0,
        });
        let reader_shared = Arc::clone(&shared);
        let reader = tokio::spawn(async move {
            while let Some(bytes) = stream.next().await {
                reader_shared.dispatch(bytes);
            }
            debug!("indication stream ended: link lost");
            reader_shared.close();
        });
        Ok(Self {
            transport,
            shared,
            reader,
        })
    }

    /// Receives every indication from now on.
    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.shared.incoming.subscribe()
    }

    /// Whether the link has been lost.
    pub fn is_closed(&self) -> bool {
        *self.shared.closed.borrow()
    }

    /// Resolves when the link is lost.
    pub async fn closed(&self) {
        let mut rx = self.shared.closed.subscribe();
        // Err means the sender is gone, which only happens with the device.
        let _ = rx.wait_for(|closed| *closed).await;
    }

    /// Writes a frame (with response) without waiting for any indication.
    pub async fn send(&self, frame: &Frame) -> Result<()> {
        if self.is_closed() {
            return Err(Error::NotConnected);
        }
        trace!(frame = %frame, "tx");
        self.transport.write(frame.as_bytes()).await
    }

    /// Writes `frame` and returns the payload of the first indication with opcode `expect`.
    pub async fn request(&self, frame: &Frame, expect: u8, wait: Duration) -> Result<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        let id = self.shared.next_waiter.fetch_add(1, Ordering::Relaxed);
        self.shared
            .waiters()
            .entry(expect)
            .or_default()
            .push((id, tx));
        let _guard = WaiterGuard {
            shared: &self.shared,
            opcode: expect,
            id,
        };

        self.send(frame).await?;
        match timeout(wait, rx).await {
            Ok(Ok(payload)) => Ok(payload),
            Ok(Err(_)) => Err(Error::NotConnected),
            Err(_) => Err(Error::Timeout {
                request: frame.opcode(),
                expected: expect,
                timeout_ms: u64::try_from(wait.as_millis()).unwrap_or(u64::MAX),
            }),
        }
    }

    /// Sends `command` and waits for its reply (see [`Command::expected_reply`]); commands without
    /// a single reply are just sent and return an empty payload.
    pub async fn command(&self, command: Command, wait: Duration) -> Result<Vec<u8>> {
        match command.expected_reply() {
            Some(expect) => self.request(&command.frame(), expect, wait).await,
            None => self.send(&command.frame()).await.map(|()| Vec::new()),
        }
    }

    /// Writes `frame` and collects every indication received during `window`.
    pub async fn exchange(&self, frame: &Frame, window: Duration) -> Result<Vec<Vec<u8>>> {
        let mut rx = self.subscribe();
        self.send(frame).await?;
        let deadline = Instant::now() + window;
        let mut out = Vec::new();
        loop {
            match timeout_at(deadline, rx.recv()).await {
                Ok(Ok(bytes)) => out.push(bytes),
                Ok(Err(broadcast::error::RecvError::Lagged(n))) => {
                    warn!(missed = n, "exchange lagged behind indications");
                }
                Ok(Err(broadcast::error::RecvError::Closed)) | Err(_) => break,
            }
        }
        Ok(out)
    }

    /// After subscribing, the device swallows writes for a variable time.
    pub async fn wait_ready(&self) -> bool {
        let mut rx = self.subscribe();
        let request = Command::RequestStatus.frame();
        for attempt in 1..=READY_ATTEMPTS {
            if let Err(e) = self.send(&request).await {
                debug!(attempt, error = %e, "ready probe write failed");
            }
            match timeout(READY_ATTEMPT_TIMEOUT, rx.recv()).await {
                Ok(Ok(_) | Err(broadcast::error::RecvError::Lagged(_))) => {
                    debug!(attempt, "device ready");
                    return true;
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => return false,
                Err(_) => {}
            }
            if self.is_closed() {
                return false;
            }
        }
        false
    }

    /// Reads one diagnostics record; a reply for another type is treated as no answer.
    pub async fn read_diag(&self, kind: DiagKind, wait: Duration) -> Result<Option<DiagReply>> {
        let frame = Command::RequestDiagnostic(kind).frame();
        match self.request(&frame, reply::DIAGNOSTICS, wait).await {
            Ok(payload) => {
                let reply = DiagReply::parse(&payload).filter(|r| r.kind == kind.code());
                if reply.is_none() {
                    debug!(requested = kind.code(), got = ?payload.first(), "diagnostics type mismatch");
                }
                Ok(reply)
            }
            Err(Error::Timeout { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Reads the calibration table: `A5` three times, answered by `0x44`, `0x45` and `0x46`.
    pub async fn read_masters(&self) -> Result<Masters> {
        let frame = Command::RequestMasters.frame();
        let mut parts = Vec::with_capacity(MASTERS_REPLIES.len());
        for expect in MASTERS_REPLIES {
            sleep(Duration::from_millis(FRAME_DELAY_MS)).await;
            parts.push(self.request(&frame, expect, DEFAULT_TIMEOUT).await?);
        }
        Ok(Masters::from_replies(&parts)?)
    }

    /// Reads the Device Information strings; unreadable ones are `None`.
    pub async fn read_info(&self) -> DeviceInfo {
        let read = |uuid| async move {
            match self.transport.read_characteristic(uuid).await {
                Ok(bytes) => Some(
                    String::from_utf8_lossy(&bytes)
                        .trim_end_matches('\0')
                        .to_owned(),
                ),
                Err(e) => {
                    debug!(%uuid, error = %e, "DIS read failed");
                    None
                }
            }
        };
        DeviceInfo {
            hardware: read(gatt::DIS_HARDWARE_REVISION).await,
            firmware: read(gatt::DIS_FIRMWARE_REVISION).await,
        }
    }

    /// Closes the link.
    pub async fn disconnect(&self) -> Result<()> {
        self.transport.disconnect().await
    }

    /// Number of registered reply waiters (for leak checks in tests).
    #[doc(hidden)]
    pub fn pending_waiters(&self) -> usize {
        self.shared.waiters().values().map(Vec::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Connector;
    use crate::transport::fake::{FakeConnector, FakeDevice, Response};

    async fn open() -> (FakeDevice, Device) {
        let fake = FakeDevice::new("dev", "Ploom aura T");
        let link = FakeConnector::new([fake.clone()])
            .connect(None)
            .await
            .unwrap();
        (fake, Device::open(link.transport).await.unwrap())
    }

    #[tokio::test(start_paused = true)]
    async fn request_resolves_with_payload() {
        let (_fake, dev) = open().await;
        let payload = dev
            .command(Command::RequestStatus, DEFAULT_TIMEOUT)
            .await
            .unwrap();
        assert_eq!(payload, [80, 0]);
        assert_eq!(dev.pending_waiters(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_cleans_up_waiter() {
        let (fake, dev) = open().await;
        fake.intercept(|_| Response::Ignore);
        let err = dev
            .request(&Command::RequestRssi.frame(), 0xFC, Duration::from_secs(1))
            .await;
        assert!(
            matches!(
                err,
                Err(Error::Timeout {
                    request: 0xFD,
                    expected: 0xFC,
                    timeout_ms: 1000
                })
            ),
            "{err:?}"
        );
        assert_eq!(dev.pending_waiters(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn cancelled_request_cleans_up_waiter() {
        let (fake, dev) = open().await;
        fake.intercept(|_| Response::Ignore);
        let frame = Command::RequestRssi.frame();
        let fut = dev.request(&frame, 0xFC, Duration::from_secs(60));
        let res = timeout(Duration::from_millis(10), fut).await;
        assert!(res.is_err(), "request should still be pending");
        assert_eq!(dev.pending_waiters(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn link_loss_fails_pending_requests() {
        let (fake, dev) = open().await;
        fake.intercept(|_| Response::Ignore);
        let frame = Command::RequestRssi.frame();
        let request = dev.request(&frame, 0xFC, Duration::from_secs(60));
        let drop_later = async {
            sleep(Duration::from_millis(50)).await;
            fake.drop_link();
        };
        let (res, ()) = tokio::join!(request, drop_later);
        assert!(matches!(res, Err(Error::NotConnected)), "{res:?}");
        dev.closed().await;
        assert!(dev.is_closed());
        assert_eq!(dev.pending_waiters(), 0);
        assert!(matches!(
            dev.send(&Command::RequestStatus.frame()).await,
            Err(Error::NotConnected)
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn wait_ready_retries_until_first_indication() {
        let (fake, dev) = open().await;
        fake.update(|s| s.swallow_writes = 3);
        assert!(dev.wait_ready().await);
        assert_eq!(fake.writes_with_opcode(0xE3).len(), 4);
    }

    #[tokio::test(start_paused = true)]
    async fn wait_ready_gives_up() {
        let (fake, dev) = open().await;
        fake.intercept(|_| Response::Ignore);
        let started = Instant::now();
        assert!(!dev.wait_ready().await);
        assert_eq!(fake.writes().len(), READY_ATTEMPTS as usize);
        assert!(started.elapsed() >= READY_ATTEMPT_TIMEOUT * READY_ATTEMPTS);
    }

    #[tokio::test(start_paused = true)]
    async fn read_diag_checks_type_echo() {
        let (fake, dev) = open().await;
        let ok = dev
            .read_diag(DiagKind::EnclosureTemp, DEFAULT_TIMEOUT)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ok.enclosure_temp_c(), Some(26.5));

        // A late ADC reply arriving while we wait for the temperature.
        fake.intercept(|f| {
            if f[1] == 0xF1 {
                Response::Reply(vec![vec![0x06, 0xF2, 0x07, 0x00, 0x01, 0x02, 0x03]])
            } else {
                Response::Default
            }
        });
        let stale = dev
            .read_diag(DiagKind::EnclosureTemp, DEFAULT_TIMEOUT)
            .await
            .unwrap();
        assert_eq!(stale, None);

        fake.intercept(|_| Response::Ignore);
        let silent = dev
            .read_diag(DiagKind::Adc, Duration::from_millis(800))
            .await
            .unwrap();
        assert_eq!(silent, None);
        assert_eq!(dev.pending_waiters(), 0);
    }

    #[tokio::test(start_paused = true)]
    async fn exchange_collects_window() {
        let (_fake, dev) = open().await;
        let replies = dev
            .exchange(
                &Command::RequestChargingLogs.frame(),
                Duration::from_secs(2),
            )
            .await
            .unwrap();
        let ops: Vec<u8> = replies.iter().map(|r| r[1]).collect();
        assert_eq!(ops, [0x35, 0x3B, 0x40]);
    }

    #[tokio::test(start_paused = true)]
    async fn masters_and_info() {
        let (fake, dev) = open().await;
        let masters = dev.read_masters().await.unwrap();
        assert_eq!(masters.values(), fake.state().masters.as_slice());
        assert_eq!(fake.writes_with_opcode(0xA5).len(), 3);
        let info = dev.read_info().await;
        assert_eq!(info.firmware.as_deref(), Some("M1.0014_G4"));
        assert_eq!(info.hardware.as_deref(), Some("U123456VL4U66CJ 170B"));
    }
}
