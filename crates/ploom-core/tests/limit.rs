//! The stick limit against the simulated device.

use std::sync::Arc;
use std::time::Duration;

use ploom_core::model::{
    ActivityKind, ConnectionState, ErrorCode, LimitSettings, LockState, Snapshot,
};
use ploom_core::session::Clock;
use ploom_core::transport::fake::{FakeConnector, FakeDevice};
use ploom_core::{SessionConfig, SessionHandle};
use tempfile::TempDir;
use tokio::time::{Instant, sleep, timeout};

const ID: &str = "fake-1";
const BASE_MS: u64 = 1_800_000_000_000;
const LOCK: [u8; 3] = [0x02, 0x9E, 0x01];
const UNLOCK: [u8; 3] = [0x02, 0x9E, 0x00];

fn virtual_clock() -> Clock {
    let epoch = Instant::now();
    Arc::new(move || BASE_MS + u64::try_from(epoch.elapsed().as_millis()).unwrap())
}

struct Harness {
    session: SessionHandle,
    fake: FakeDevice,
    connector: FakeConnector,
    clock: Clock,
    dir: TempDir,
}

impl Harness {
    fn session(connector: &FakeConnector, clock: &Clock, dir: &TempDir) -> SessionHandle {
        SessionHandle::with_connector_and_clock(
            &SessionConfig {
                data_dir: dir.path().to_owned(),
                auto_connect: false,
            },
            Arc::new(connector.clone()),
            Arc::clone(clock),
        )
    }

    async fn connect(&self) {
        self.session.connect(Some(ID.into())).await.unwrap();
        wait_for(&self.session, "first poll", |s| {
            s.connection == ConnectionState::Connected && s.status.usb_plugged.is_some()
        })
        .await;
    }

    async fn smoke(&self) {
        self.fake.start_session();
        sleep(Duration::from_secs(1)).await;
        self.fake.end_session();
        sleep(Duration::from_secs(1)).await;
    }

    fn lock_writes(&self) -> Vec<Vec<u8>> {
        self.fake.writes_with_opcode(0x9E)
    }

    fn events(&self) -> Vec<ActivityKind> {
        self.session
            .activity_history()
            .into_iter()
            .map(|e| e.kind)
            .collect()
    }
}

async fn wait_for(
    session: &SessionHandle,
    what: &str,
    pred: impl FnMut(&Snapshot) -> bool,
) -> Snapshot {
    let mut rx = session.subscribe_snapshot();
    match timeout(Duration::from_secs(300), rx.wait_for(pred)).await {
        Ok(Ok(s)) => s.clone(),
        _ => panic!(
            "timed out waiting for {what}; last snapshot: {:#?}",
            session.snapshot()
        ),
    }
}

fn settings(sticks: u32, cooldown_minutes: u32) -> LimitSettings {
    LimitSettings {
        enabled: true,
        sticks,
        cooldown_minutes,
        window_hours: 24,
    }
}

async fn limited(sticks: u32, cooldown_minutes: u32) -> Harness {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    let connector = FakeConnector::new([fake.clone()]);
    let clock = virtual_clock();
    let dir = tempfile::tempdir().unwrap();
    let session = Harness::session(&connector, &clock, &dir);
    let h = Harness {
        session,
        fake,
        connector,
        clock,
        dir,
    };
    h.connect().await;
    h.session
        .set_limit_settings(settings(sticks, cooldown_minutes))
        .unwrap();
    sleep(Duration::from_secs(5)).await;
    h
}

async fn reach_limit(h: &Harness, sticks: u32) -> u64 {
    for _ in 0..sticks {
        h.smoke().await;
    }
    let s = wait_for(&h.session, "cooldown", |s| {
        s.limit.locked_until_ms.is_some() && s.status.lock == Some(LockState::Locked)
    })
    .await;
    s.limit.locked_until_ms.unwrap()
}

#[tokio::test(start_paused = true)]
async fn off_by_default_and_counts_nothing() {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    let connector = FakeConnector::new([fake.clone()]);
    let clock = virtual_clock();
    let dir = tempfile::tempdir().unwrap();
    let session = Harness::session(&connector, &clock, &dir);
    let h = Harness {
        session,
        fake,
        connector,
        clock,
        dir,
    };
    h.connect().await;
    assert_eq!(
        h.session.snapshot().limit.settings,
        LimitSettings::default()
    );
    h.smoke().await;
    let limit = h.session.snapshot().limit;
    assert_eq!((limit.sticks_used, limit.locked_until_ms), (0, None));
    assert!(h.lock_writes().is_empty());
    assert!(
        h.fake.writes_with_opcode(0xF1).iter().all(|f| f[2] != 12),
        "the lifetime counters are not read while the limit is off"
    );
}

#[tokio::test(start_paused = true)]
async fn locks_after_the_session_that_reaches_the_limit_ends() {
    let h = limited(2, 30).await;
    h.smoke().await;
    assert_eq!(h.session.snapshot().limit.sticks_used, 1);
    assert!(h.session.snapshot().limit.window_started_at_ms.is_some());

    // Second stick: the limit is reached when it starts…
    h.fake.start_session();
    wait_for(&h.session, "second stick", |s| s.limit.sticks_used == 2).await;
    sleep(Duration::from_secs(20)).await;
    // …but nothing is sent while the session is running.
    assert!(h.lock_writes().is_empty(), "never lock mid-session");
    assert_eq!(h.session.snapshot().limit.locked_until_ms, None);

    let before = (h.clock)();
    h.fake.end_session();
    let s = wait_for(&h.session, "cooldown", |s| {
        s.limit.locked_until_ms.is_some() && s.status.lock == Some(LockState::Locked)
    })
    .await;
    assert_eq!(h.lock_writes(), [LOCK]);
    let until = s.limit.locked_until_ms.unwrap();
    assert!(
        (before + 30 * 60_000..before + 30 * 60_000 + 5_000).contains(&until),
        "cooldown ends 30 min after the lock: {until}"
    );
    assert!(h.events().contains(&ActivityKind::LimitReached {
        sticks: 2,
        locked_until_ms: until,
    }));
}

#[tokio::test(start_paused = true)]
async fn cooldown_refuses_unlock_and_relaxing_but_allows_stricter() {
    let h = limited(1, 30).await;
    let until = reach_limit(&h, 1).await;

    let e = h.session.set_lock(false).await.unwrap_err();
    assert_eq!(e.code(), ErrorCode::CooldownActive);
    assert!(e.to_string().contains(&until.to_string()), "{e}");
    assert_eq!(h.lock_writes(), [LOCK], "no unlock frame was sent");
    h.session.set_lock(true).await.unwrap();

    for relaxed in [
        LimitSettings {
            enabled: false,
            ..settings(1, 30)
        },
        settings(2, 30),
        settings(1, 29),
    ] {
        let e = h.session.set_limit_settings(relaxed).unwrap_err();
        assert_eq!(e.code(), ErrorCode::CooldownActive, "{relaxed:?}");
    }
    assert_eq!(h.session.snapshot().limit.settings, settings(1, 30));
    h.session.set_limit_settings(settings(1, 45)).unwrap();
    assert_eq!(h.session.snapshot().limit.settings, settings(1, 45));

    let e = h.session.set_limit_settings(settings(0, 45)).unwrap_err();
    assert_eq!(e.code(), ErrorCode::InvalidInput);
}

#[tokio::test(start_paused = true)]
async fn relocks_when_another_app_unlocks() {
    let h = limited(1, 30).await;
    reach_limit(&h, 1).await;

    // The official app (or anything else) lifts the lock.
    h.fake.update(|s| s.lock = 1);
    wait_for(&h.session, "relock", |s| {
        s.status.lock == Some(LockState::Locked)
    })
    .await;
    sleep(Duration::from_secs(10)).await;
    assert_eq!(h.lock_writes(), [LOCK, LOCK]);
    assert_eq!(h.fake.state().lock, 0, "locked again on the device");
    assert!(h.events().contains(&ActivityKind::Relocked));
    assert!(h.session.snapshot().limit.locked_until_ms.is_some());
}

#[tokio::test(start_paused = true)]
async fn unlocks_when_the_cooldown_expires() {
    let h = limited(1, 30).await;
    let until = reach_limit(&h, 1).await;

    sleep(Duration::from_secs(29 * 60)).await;
    assert_eq!(
        h.lock_writes(),
        [LOCK],
        "still locked a minute before the end"
    );

    let s = wait_for(&h.session, "cooldown end", |s| {
        s.limit.locked_until_ms.is_none() && s.status.lock == Some(LockState::Unlocked)
    })
    .await;
    assert!((h.clock)() >= until);
    assert!((h.clock)() < until + 15_000, "released promptly");
    assert_eq!(h.lock_writes(), [LOCK, UNLOCK]);
    assert_eq!(
        (s.limit.sticks_used, s.limit.window_started_at_ms),
        (0, None)
    );
    assert!(h.events().contains(&ActivityKind::CooldownEnded));

    // The user is in charge of the lock again, and a new window starts.
    h.session.set_lock(true).await.unwrap();
    h.session.set_lock(false).await.unwrap();
    h.smoke().await;
    assert_eq!(h.session.snapshot().limit.sticks_used, 1);
}

#[tokio::test(start_paused = true)]
async fn unlocks_on_reconnect_after_the_cooldown_expired() {
    let h = limited(1, 30).await;
    reach_limit(&h, 1).await;

    h.session.disconnect().await.unwrap();
    assert!(
        h.session.snapshot().limit.locked_until_ms.is_some(),
        "the cooldown is still shown while disconnected"
    );
    sleep(Duration::from_secs(45 * 60)).await;
    assert_eq!(h.lock_writes(), [LOCK]);
    assert_eq!(h.fake.state().lock, 0);

    h.connect().await;
    wait_for(&h.session, "release", |s| {
        s.limit.locked_until_ms.is_none() && s.status.lock == Some(LockState::Unlocked)
    })
    .await;
    assert_eq!(h.lock_writes(), [LOCK, UNLOCK]);
}

#[tokio::test(start_paused = true)]
async fn cooldown_survives_a_restart_and_is_enforced_on_connect() {
    let h = limited(1, 60).await;
    let until = reach_limit(&h, 1).await;
    let Harness {
        session,
        fake,
        connector,
        clock,
        dir,
    } = h;
    drop(session);
    sleep(Duration::from_secs(10)).await;

    // Unlocked elsewhere while the app was closed.
    fake.update(|s| s.lock = 1);
    let session = Harness::session(&connector, &clock, &dir);
    assert_eq!(
        session.snapshot().limit.locked_until_ms,
        Some(until),
        "loaded for the last device before connecting"
    );
    let h = Harness {
        session,
        fake,
        connector,
        clock,
        dir,
    };
    h.connect().await;
    wait_for(&h.session, "relock", |s| {
        s.status.lock == Some(LockState::Locked)
    })
    .await;
    assert_eq!(h.fake.state().lock, 0);
    assert_eq!(
        h.session.set_lock(false).await.unwrap_err().code(),
        ErrorCode::CooldownActive
    );
}

#[tokio::test(start_paused = true)]
async fn sticks_used_while_disconnected_are_counted_from_the_device() {
    let h = limited(4, 30).await;
    assert!(
        h.fake.writes_with_opcode(0xF1).iter().any(|f| f[2] == 12),
        "baseline read once the limit is on"
    );
    h.smoke().await;
    assert_eq!(h.session.snapshot().limit.sticks_used, 1);

    h.session.disconnect().await.unwrap();
    for _ in 0..2 {
        h.fake.start_session();
        h.fake.end_session();
    }
    h.connect().await;
    wait_for(&h.session, "away sticks", |s| s.limit.sticks_used == 3).await;
    sleep(Duration::from_secs(30)).await;
    assert_eq!(
        h.session.snapshot().limit.sticks_used,
        3,
        "the stick seen live is not counted twice"
    );

    reach_limit(&h, 1).await;
    assert_eq!(h.lock_writes(), [LOCK]);
}

#[tokio::test(start_paused = true)]
async fn a_silent_diagnostics_channel_is_tolerated() {
    let h = limited(2, 30).await;
    h.session.disconnect().await.unwrap();
    // The record comes back empty after reconnecting.
    h.fake.update(|s| s.heating_starts = [0; 7]);
    h.connect().await;
    sleep(Duration::from_secs(30)).await;
    assert_eq!(h.session.snapshot().limit.sticks_used, 0);
    reach_limit(&h, 2).await;
}

#[tokio::test(start_paused = true)]
async fn the_counter_resets_when_the_window_passes() {
    let h = limited(5, 30).await;
    h.smoke().await;
    h.smoke().await;
    assert_eq!(h.session.snapshot().limit.sticks_used, 2);
    sleep(Duration::from_secs(24 * 3600)).await;
    let s = wait_for(&h.session, "window reset", |s| s.limit.sticks_used == 0).await;
    assert_eq!(s.limit.window_started_at_ms, None);
    assert!(h.lock_writes().is_empty());
}
