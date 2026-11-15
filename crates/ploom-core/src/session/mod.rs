//! The device session: connection lifecycle, polling, derived status and commands, exposed through
//! a cheap-to-clone [`SessionHandle`].

pub mod limit;
mod ops;
mod store;
mod supervisor;
mod tracker;

use std::collections::VecDeque;
use std::fmt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::{Notify, broadcast, mpsc, oneshot, watch};
use tokio::time::Instant;
use tracing::warn;

use crate::device::Device;
use crate::error::{Error, Result};
use crate::model::{
    Activity, ActivityEvent, ActivityKind, Catalog, ConnectionState, DeviceIdentity, DeviceStatus,
    Diagnostics, DiscoveredDevice, ErrorCode, FramePreview, LimitSettings, LogsReport, Operation,
    ProfileTarget, RawExchange, ReceivedFrame, Snapshot,
};
use crate::transport::Connector;

use self::limit::LimitRecord;
use self::store::Store;
use self::tracker::StatusTracker;

/// Time between the end of one status poll and the start of the next.
pub const POLL_INTERVAL: Duration = Duration::from_secs(3);
/// The buzzer is switched off automatically after this long.
pub const VIBRATION_AUTO_OFF: Duration = Duration::from_secs(30);
/// Number of activity events kept for late subscribers.
pub const ACTIVITY_HISTORY: usize = 100;
/// First reconnect delay; doubles up to [`RECONNECT_MAX`].
pub const RECONNECT_MIN: Duration = Duration::from_secs(2);
/// Longest reconnect delay.
pub const RECONNECT_MAX: Duration = Duration::from_secs(20);
/// How often the stick limit is re-evaluated when nothing else happens.
pub const LIMIT_TICK: Duration = Duration::from_secs(5);

/// Source of the current time as Unix milliseconds.
pub type Clock = Arc<dyn Fn() -> u64 + Send + Sync>;

/// How a [`SessionHandle`] is set up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfig {
    /// Directory for persisted state (custom profiles, last device).
    pub data_dir: PathBuf,
    /// Connect on start to the last device (or the first Ploom found).
    pub auto_connect: bool,
}

#[derive(Debug)]
enum Control {
    Connect(Option<String>),
    Disconnect(oneshot::Sender<()>),
}

/// Handle to a running session.
#[derive(Clone)]
pub struct SessionHandle {
    shared: Arc<Shared>,
    control: mpsc::UnboundedSender<Control>,
}

impl fmt::Debug for SessionHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionHandle")
            .field("snapshot", &*self.shared.snapshot.borrow())
            .finish_non_exhaustive()
    }
}

impl SessionHandle {
    /// Starts a session on the system's Bluetooth adapter.
    #[cfg(feature = "btleplug")]
    pub fn start(config: &SessionConfig) -> Self {
        Self::with_connector(
            config,
            Arc::new(crate::transport::btleplug::BtleplugConnector::new()),
        )
    }

    /// Starts a session on any [`Connector`].
    pub fn with_connector(config: &SessionConfig, connector: Arc<dyn Connector>) -> Self {
        Self::with_connector_and_clock(config, connector, Arc::new(now_ms))
    }

    /// Like [`with_connector`](Self::with_connector), with an explicit [`Clock`] for the stick
    /// limit instead of the system clock.
    pub fn with_connector_and_clock(
        config: &SessionConfig,
        connector: Arc<dyn Connector>,
        clock: Clock,
    ) -> Self {
        let store = Store::load(&config.data_dir);
        let last_device = store.last_device_id();
        let shared = Arc::new(Shared::new(connector, store, clock));
        let (control, rx) = mpsc::unbounded_channel();
        tokio::spawn(supervisor::run(Arc::clone(&shared), rx));
        if config.auto_connect {
            // The receiver is alive: the supervisor was just spawned.
            let _ = control.send(Control::Connect(last_device));
        }
        Self { shared, control }
    }

    /// The current snapshot.
    pub fn snapshot(&self) -> Snapshot {
        self.shared.snapshot.borrow().clone()
    }

    /// Receives every snapshot change.
    pub fn subscribe_snapshot(&self) -> watch::Receiver<Snapshot> {
        self.shared.snapshot.subscribe()
    }

    /// Receives activity events from now on; see also [`activity_history`](Self::activity_history).
    pub fn subscribe_activity(&self) -> broadcast::Receiver<ActivityEvent> {
        self.shared.activity.subscribe()
    }

    /// The last [`ACTIVITY_HISTORY`] events, oldest first.
    pub fn activity_history(&self) -> Vec<ActivityEvent> {
        lock(&self.shared.history).iter().cloned().collect()
    }

    /// Receives every raw indication from the device (for debug consoles).
    pub fn subscribe_frames(&self) -> broadcast::Receiver<ReceivedFrame> {
        self.shared.frames.subscribe()
    }

    /// Static modes, limits and dangerous opcodes.
    pub fn catalog(&self) -> Catalog {
        crate::catalog::catalog()
    }

    /// Scans for nearby devices (two passes; the device advertises in sparse bursts).
    pub async fn scan(&self) -> Result<Vec<DiscoveredDevice>> {
        self.shared.scan().await
    }

    /// Connects to `id`, or to the first Ploom found when `None`.
    pub fn connect(&self, id: Option<String>) -> impl Future<Output = Result<()>> + use<> {
        let sent = self
            .control
            .send(Control::Connect(id))
            .map_err(|_| Error::Internal("session supervisor has stopped".into()));
        std::future::ready(sent)
    }

    /// Disconnects and stays idle (no reconnect).
    pub async fn disconnect(&self) -> Result<()> {
        let (ack, done) = oneshot::channel();
        self.control
            .send(Control::Disconnect(ack))
            .map_err(|_| Error::Internal("session supervisor has stopped".into()))?;
        done.await
            .map_err(|_| Error::Internal("session supervisor has stopped".into()))
    }

    /// Turns the buzzer (Find My Device) on or off.
    pub async fn set_vibration(&self, on: bool) -> Result<()> {
        self.shared.set_vibration(on).await
    }

    /// Locks or unlocks the device.
    pub async fn set_lock(&self, locked: bool) -> Result<()> {
        self.shared.set_lock(locked).await
    }

    /// Changes the stick-limit settings (see [`limit`]).
    pub fn set_limit_settings(&self, settings: LimitSettings) -> Result<()> {
        self.shared.set_limit_settings(settings)
    }

    /// Turns Auto-Start Heating on or off.
    pub async fn set_auto_start(&self, enabled: bool) -> Result<()> {
        self.shared.set_auto_start(enabled).await
    }

    /// Applies a heating profile.
    pub async fn apply_profile(&self, target: ProfileTarget) -> Result<()> {
        self.shared.apply_profile(target).await
    }

    /// The frames [`apply_profile`](Self::apply_profile) would send.
    pub async fn preview_profile(&self, target: ProfileTarget) -> Result<FramePreview> {
        self.shared.preview_profile(target).await
    }

    /// Reads the oldest unacknowledged session and charge logs.
    pub async fn read_logs(&self) -> Result<LogsReport> {
        self.shared.read_logs().await
    }

    /// Reads sensors and lifetime statistics from the diagnostics channel.
    pub async fn read_diagnostics(&self) -> Result<Diagnostics> {
        self.shared.read_diagnostics().await
    }

    /// Sends a raw frame (`"02e300"`) and returns every indication received within 2.5 s.
    pub async fn send_raw(&self, hex: &str, confirm_dangerous: bool) -> Result<RawExchange> {
        self.shared.send_raw(hex, confirm_dangerous).await
    }

    /// Polls the device now and returns the resulting status.
    pub async fn refresh_status(&self) -> Result<DeviceStatus> {
        self.shared.refresh_status().await
    }
}

#[derive(Debug, Default)]
struct State {
    connection: ConnectionState,
    connection_error: Option<ErrorCode>,
    scanning: bool,
    device: Option<DeviceIdentity>,
    operation: Option<Operation>,
    tracker: StatusTracker,
    /// The stick limit of `limit_device`.
    limit: LimitRecord,
    /// Whose limit `limit` is: the connected device, else the last one.
    limit_device: Option<String>,
}

impl State {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            connection: self.connection,
            connection_error: self.connection_error,
            scanning: self.scanning,
            device: self.device.clone(),
            status: self.tracker.status(),
            operation: self.operation.clone(),
            limit: self.limit.state(),
        }
    }
}

struct Shared {
    connector: Arc<dyn Connector>,
    store: Mutex<Store>,
    state: Mutex<State>,
    snapshot: watch::Sender<Snapshot>,
    activity: broadcast::Sender<ActivityEvent>,
    history: Mutex<VecDeque<ActivityEvent>>,
    next_event_id: AtomicU64,
    frames: broadcast::Sender<ReceivedFrame>,
    /// The connected device, if any.
    link: Mutex<Option<Arc<Device>>>,
    /// One BLE conversation at a time.
    conversation: tokio::sync::Mutex<()>,
    vibration_generation: AtomicU64,
    /// Set while `read_logs` runs so its `0x36` is not reported as new.
    quiet_logs: AtomicBool,
    poll_count: AtomicU64,
    epoch: Instant,
    /// Wall clock used by the stick limit.
    clock: Clock,
    /// Wakes the limit task: status changed, settings changed.
    limit_wake: Notify,
    /// The device's lifetime counters were read on this connection.
    limit_synced: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

impl Shared {
    fn new(connector: Arc<dyn Connector>, store: Store, clock: Clock) -> Self {
        // Show the last device's limit until a device connects.
        let limit_device = store.last_device_id();
        let limit = limit_device
            .as_deref()
            .and_then(|id| store.limit(id))
            .unwrap_or_default();
        let state = State {
            limit,
            limit_device,
            ..State::default()
        };
        Self {
            connector,
            store: Mutex::new(store),
            snapshot: watch::channel(state.snapshot()).0,
            state: Mutex::new(state),
            activity: broadcast::channel(ACTIVITY_HISTORY).0,
            history: Mutex::new(VecDeque::with_capacity(ACTIVITY_HISTORY)),
            next_event_id: AtomicU64::new(1),
            frames: broadcast::channel(256).0,
            link: Mutex::new(None),
            conversation: tokio::sync::Mutex::new(()),
            vibration_generation: AtomicU64::new(0),
            quiet_logs: AtomicBool::new(false),
            poll_count: AtomicU64::new(0),
            epoch: Instant::now(),
            clock,
            limit_wake: Notify::new(),
            limit_synced: AtomicBool::new(false),
        }
    }

    fn mono_ms(&self) -> u64 {
        u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn update<R>(&self, f: impl FnOnce(&mut State) -> R) -> R {
        let mut events = Vec::new();
        let mut limit_changed = None;
        let mut modified = false;
        let result = {
            let mut state = lock(&self.state);
            let before = (state.tracker.status().activity, state.limit);
            let result = f(&mut state);
            // A heating session starting while connected is one stick.
            let after = state.tracker.status().activity;
            if session_started(before.0, after) {
                let now = (self.clock)();
                state.limit.session_started(now);
            }
            if state.limit != before.1 {
                limit_changed = state.limit_device.clone().map(|id| (id, state.limit));
            }
            let next = state.snapshot();
            // Published under the state lock so snapshots never go backwards.
            self.snapshot.send_if_modified(|current| {
                if *current == next {
                    return false;
                }
                events = status_events(&current.status, &next.status);
                *current = next;
                modified = true;
                true
            });
            result
        };
        if let Some((id, record)) = limit_changed {
            self.persist(|s| s.set_limit(&id, record));
        }
        for kind in events {
            self.emit(kind);
        }
        if modified {
            // Whatever changed may matter to the limit (session ended, lock
            // lifted elsewhere, settings).
            self.limit_wake.notify_one();
        }
        result
    }

    fn emit(&self, kind: ActivityKind) {
        let event = ActivityEvent {
            id: self.next_event_id.fetch_add(1, Ordering::Relaxed),
            timestamp_ms: now_ms(),
            kind,
        };
        {
            let mut history = lock(&self.history);
            if history.len() == ACTIVITY_HISTORY {
                history.pop_front();
            }
            history.push_back(event.clone());
        }
        // No subscribers is fine.
        let _ = self.activity.send(event);
    }

    fn device(&self) -> Result<Arc<Device>> {
        lock(&self.link).clone().ok_or(Error::NotConnected)
    }

    fn device_id(&self) -> Option<String> {
        lock(&self.state).device.as_ref().map(|d| d.id.clone())
    }

    fn set_link(&self, device: Option<Arc<Device>>) {
        *lock(&self.link) = device;
    }

    fn operation_active(&self) -> bool {
        lock(&self.state).operation.is_some()
    }

    fn is_heating(&self) -> bool {
        lock(&self.state).tracker.is_heating()
    }

    async fn converse(&self) -> Result<tokio::sync::MutexGuard<'_, ()>> {
        if self.operation_active() {
            return Err(Error::Busy("a heating profile is being flashed"));
        }
        Ok(self.conversation.lock().await)
    }

    fn persist(&self, f: impl FnOnce(&mut Store) -> std::io::Result<()>) {
        if let Err(e) = f(&mut lock(&self.store)) {
            warn!(error = %e, "cannot persist session state");
        }
    }
}

fn session_started(old: Option<Activity>, new: Option<Activity>) -> bool {
    matches!(old, Some(Activity::Standby | Activity::Charging))
        && matches!(new, Some(Activity::Preheating | Activity::Heating))
}

fn status_events(old: &DeviceStatus, new: &DeviceStatus) -> Vec<ActivityKind> {
    fn changed<T: Copy + PartialEq>(old: Option<T>, new: Option<T>) -> Option<T> {
        match (old, new) {
            (Some(a), Some(b)) if a != b => Some(b),
            _ => None,
        }
    }
    let mut out = Vec::new();
    // Charging transitions are reported only as `chargingChanged`;
    // `stateChanged` covers standby/preheating/heating.
    if let Some(activity) = changed(old.activity, new.activity) {
        let charging_edge = activity == Activity::Charging
            || (old.activity == Some(Activity::Charging) && activity == Activity::Standby);
        if !charging_edge {
            out.push(ActivityKind::StateChanged { activity });
        }
    }
    if let Some(charging) = changed(old.charging, new.charging) {
        out.push(ActivityKind::ChargingChanged { charging });
    }
    if old.vibrating != new.vibrating {
        out.push(ActivityKind::VibrationChanged { on: new.vibrating });
    }
    if let Some(lock) = changed(old.lock, new.lock) {
        out.push(ActivityKind::LockChanged { lock });
    }
    if let Some(enabled) = changed(old.auto_start, new.auto_start) {
        out.push(ActivityKind::AutoStartChanged { enabled });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::LockState;

    #[test]
    fn events_only_for_known_transitions() {
        let unknown = DeviceStatus::default();
        let standby = DeviceStatus {
            activity: Some(Activity::Standby),
            charging: Some(false),
            lock: Some(LockState::Unlocked),
            auto_start: Some(false),
            ..DeviceStatus::default()
        };
        assert!(status_events(&unknown, &standby).is_empty());
        let heating = DeviceStatus {
            activity: Some(Activity::Heating),
            lock: Some(LockState::Locked),
            auto_start: Some(true),
            vibrating: true,
            ..standby.clone()
        };
        assert_eq!(
            status_events(&standby, &heating),
            [
                ActivityKind::StateChanged {
                    activity: Activity::Heating
                },
                ActivityKind::VibrationChanged { on: true },
                ActivityKind::LockChanged {
                    lock: LockState::Locked
                },
                ActivityKind::AutoStartChanged { enabled: true },
            ]
        );
        let charging = DeviceStatus {
            activity: Some(Activity::Charging),
            charging: Some(true),
            ..standby.clone()
        };
        assert_eq!(
            status_events(&standby, &charging),
            [ActivityKind::ChargingChanged { charging: true }],
            "charging is never a stateChanged"
        );
        assert_eq!(
            status_events(&charging, &standby),
            [ActivityKind::ChargingChanged { charging: false }]
        );
        assert!(
            status_events(&heating, &unknown)
                .iter()
                .all(|e| matches!(e, ActivityKind::VibrationChanged { on: false }))
        );
    }
}
