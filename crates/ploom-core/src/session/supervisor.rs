//! The connection lifecycle state machine.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{sleep, timeout};
use tracing::{debug, info, warn};

use super::tracker::StatusTracker;
use super::{Control, LIMIT_TICK, POLL_INTERVAL, RECONNECT_MAX, RECONNECT_MIN, Shared, now_ms};
use crate::device::{Device, DeviceInfo};
use crate::error::Result;
use crate::model::{ActivityKind, ConnectionState, DeviceIdentity, ReceivedFrame};
use crate::transport::{Link, Transport};

const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(5);

struct Connection {
    id: String,
    name: Option<String>,
    device: Arc<Device>,
    /// Subscribed before `wait_ready`, so the readiness probe's replies count.
    frames: broadcast::Receiver<Vec<u8>>,
    info: DeviceInfo,
}

enum Exit {
    LinkLost,
    Disconnect(oneshot::Sender<()>),
    Switch(Option<String>),
    Shutdown,
}

enum Next {
    Retarget(Option<String>),
    Idle(oneshot::Sender<()>),
    Shutdown,
}

fn on_control(msg: Option<Control>) -> Next {
    match msg {
        None => Next::Shutdown,
        Some(Control::Connect(id)) => Next::Retarget(id),
        Some(Control::Disconnect(ack)) => Next::Idle(ack),
    }
}

fn backoff(failures: u32) -> Duration {
    RECONNECT_MIN
        .saturating_mul(1 << failures.min(5))
        .min(RECONNECT_MAX)
}

pub(super) async fn run(shared: Arc<Shared>, mut control: mpsc::UnboundedReceiver<Control>) {
    // `None`: idle. `Some(None)`: first Ploom found. `Some(Some(id))`: that device.
    let mut target: Option<Option<String>> = None;
    let mut failures: u32 = 0;
    // Whether the next attempt follows a failure or a lost link.
    let mut retrying = false;

    loop {
        let Some(want) = target.clone() else {
            match control.recv().await {
                None => return,
                Some(Control::Connect(id)) => {
                    target = Some(id);
                    failures = 0;
                    retrying = false;
                }
                Some(Control::Disconnect(ack)) => {
                    let _ = ack.send(());
                }
            }
            continue;
        };

        if retrying {
            shared.update(|st| st.connection = ConnectionState::Reconnecting);
            let delay = backoff(failures.saturating_sub(1));
            debug!(?delay, "waiting before reconnect");
            tokio::select! {
                () = sleep(delay) => {}
                msg = control.recv() => {
                    match on_control(msg) {
                        Next::Retarget(id) => { target = Some(id); failures = 0; retrying = false; }
                        Next::Idle(ack) => { go_idle(&shared, true); target = None; let _ = ack.send(()); }
                        Next::Shutdown => return,
                    }
                    continue;
                }
            }
        }

        shared.update(|st| {
            st.connection = if retrying {
                ConnectionState::Reconnecting
            } else {
                ConnectionState::Connecting
            };
        });
        let attempt = tokio::select! {
            result = establish(&shared, want.as_deref()) => result,
            msg = control.recv() => {
                match on_control(msg) {
                    Next::Retarget(id) => { target = Some(id); failures = 0; retrying = false; }
                    Next::Idle(ack) => { go_idle(&shared, true); target = None; let _ = ack.send(()); }
                    Next::Shutdown => return,
                }
                continue;
            }
        };

        let conn = match attempt {
            Ok(conn) => conn,
            Err(e) => {
                warn!(error = %e, target = ?want, "connection attempt failed");
                shared.update(|st| st.connection_error = Some(e.code()));
                failures = failures.saturating_add(1);
                retrying = true;
                continue;
            }
        };

        failures = 0;
        let id = conn.id.clone();
        match run_connected(&shared, conn, &mut control).await {
            Exit::LinkLost => {
                warn!(%id, "link lost; reconnecting");
                shared.emit(ActivityKind::ConnectionLost);
                target = Some(Some(id));
                failures = 1;
                retrying = true;
            }
            Exit::Disconnect(ack) => {
                info!(%id, "disconnected by user");
                go_idle(&shared, true);
                target = None;
                let _ = ack.send(());
            }
            Exit::Switch(next) => {
                info!(from = %id, to = ?next, "switching device");
                target = Some(next);
                retrying = false;
            }
            Exit::Shutdown => return,
        }
    }
}

fn go_idle(shared: &Shared, announce: bool) {
    let had_device = shared.update(|st| {
        let had = st.device.is_some() || st.connection != ConnectionState::Idle;
        st.connection = ConnectionState::Idle;
        st.connection_error = None;
        st.device = None;
        st.operation = None;
        st.tracker = StatusTracker::default();
        had
    });
    if announce && had_device {
        shared.emit(ActivityKind::Disconnected);
    }
}

struct LinkGuard {
    transport: Option<Arc<dyn Transport>>,
}

impl LinkGuard {
    fn disarm(mut self) {
        self.transport = None;
    }
}

impl Drop for LinkGuard {
    fn drop(&mut self) {
        if let Some(t) = self.transport.take() {
            // The device holds a single connection; never leave it dangling.
            tokio::spawn(async move {
                if let Err(e) = t.disconnect().await {
                    debug!(error = %e, "disconnect of abandoned link failed");
                }
            });
        }
    }
}

async fn establish(shared: &Shared, target: Option<&str>) -> Result<Connection> {
    info!(?target, "connecting");
    let Link {
        id,
        name,
        transport,
    } = shared.connector.connect(target).await?;
    let guard = LinkGuard {
        transport: Some(Arc::clone(&transport)),
    };
    let device = Arc::new(Device::open(transport).await?);
    let frames = device.subscribe();
    if !device.wait_ready().await {
        warn!(%id, "device connected but does not answer yet");
    }
    let info = device.read_info().await;
    guard.disarm();
    info!(%id, ?name, firmware = ?info.firmware, "connected");
    Ok(Connection {
        id,
        name,
        device,
        frames,
        info,
    })
}

struct Tasks(Vec<JoinHandle<()>>);

impl Drop for Tasks {
    fn drop(&mut self) {
        for t in &self.0 {
            t.abort();
        }
    }
}

async fn run_connected(
    shared: &Arc<Shared>,
    conn: Connection,
    control: &mut mpsc::UnboundedReceiver<Control>,
) -> Exit {
    let Connection {
        id,
        name,
        device,
        frames,
        info,
    } = conn;
    let (custom, stored_limit) = {
        let store = super::lock(&shared.store);
        (store.custom_profile(&id), store.limit(&id))
    };
    shared
        .limit_synced
        .store(false, std::sync::atomic::Ordering::SeqCst);
    shared.update(|st| {
        // A device seen for the first time starts from the settings shown
        // so far, with a clean counter.
        st.limit = stored_limit.unwrap_or(super::limit::LimitRecord {
            settings: st.limit.settings,
            ..Default::default()
        });
        st.limit_device = Some(id.clone());
        st.connection = ConnectionState::Connected;
        st.connection_error = None;
        st.device = Some(DeviceIdentity {
            id: id.clone(),
            name: name.clone(),
            hardware: info.hardware.clone(),
            firmware: info.firmware.clone(),
            color: info
                .hardware
                .as_deref()
                .and_then(ploom_protocol::color::decode),
        });
        st.tracker = StatusTracker::new(custom);
    });
    shared.persist(|s| s.set_last_device_id(&id));
    shared.set_link(Some(Arc::clone(&device)));
    shared.emit(ActivityKind::Connected { device_name: name });

    let tasks = Tasks(vec![
        tokio::spawn(ingest(Arc::clone(shared), frames)),
        tokio::spawn(poll_loop(Arc::clone(shared), Arc::clone(&device))),
        tokio::spawn(limit_loop(Arc::clone(shared), Arc::clone(&device))),
    ]);

    let exit = loop {
        tokio::select! {
            () = device.closed() => break Exit::LinkLost,
            msg = control.recv() => match msg {
                None => break Exit::Shutdown,
                Some(Control::Disconnect(ack)) => break Exit::Disconnect(ack),
                Some(Control::Connect(Some(other))) if other != id => {
                    break Exit::Switch(Some(other));
                }
                Some(Control::Connect(_)) => {}
            },
        }
    };

    drop(tasks);
    shared.set_link(None);
    if !matches!(exit, Exit::LinkLost) {
        match timeout(DISCONNECT_TIMEOUT, device.disconnect()).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => warn!(error = %e, "disconnect failed"),
            Err(_) => warn!("disconnect timed out"),
        }
    }
    exit
}

async fn ingest(shared: Arc<Shared>, mut frames: broadcast::Receiver<Vec<u8>>) {
    loop {
        match frames.recv().await {
            Ok(bytes) => shared.on_frame(bytes),
            Err(broadcast::error::RecvError::Lagged(n)) => warn!(missed = n, "status lagged"),
            Err(broadcast::error::RecvError::Closed) => return,
        }
    }
}

async fn poll_loop(shared: Arc<Shared>, device: Arc<Device>) {
    loop {
        if let Err(e) = shared.poll_once(&device).await {
            debug!(error = %e, "poll stopped");
            if device.is_closed() {
                return;
            }
        }
        sleep(POLL_INTERVAL).await;
    }
}

async fn limit_loop(shared: Arc<Shared>, device: Arc<Device>) {
    loop {
        if device.is_closed() {
            return;
        }
        shared.sync_lifetime_starts(&device).await;
        shared.limit_step(&device).await;
        tokio::select! {
            () = shared.limit_wake.notified() => {}
            () = sleep(LIMIT_TICK) => {}
        }
    }
}

impl Shared {
    fn on_frame(&self, bytes: Vec<u8>) {
        let wall = now_ms();
        let parsed = ploom_protocol::Notification::parse(&bytes);
        let _ = self.frames.send(ReceivedFrame {
            timestamp_ms: wall,
            bytes,
        });
        let Ok(n) = parsed else { return };
        let mono = self.mono_ms();
        self.update(|st| st.tracker.ingest(mono, wall, &n));
        if matches!(n, ploom_protocol::Notification::SmokingLog(_))
            && !self.quiet_logs.load(std::sync::atomic::Ordering::SeqCst)
        {
            self.emit(ActivityKind::SessionLogReceived);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_to_cap() {
        let secs: Vec<u64> = (0..8).map(|n| backoff(n).as_secs()).collect();
        assert_eq!(secs, [2, 4, 8, 16, 20, 20, 20, 20]);
        assert_eq!(backoff(u32::MAX), RECONNECT_MAX);
    }
}
