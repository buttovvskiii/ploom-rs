//! Session behaviour against the simulated device.

use std::sync::Arc;
use std::time::Duration;

use ploom_core::model::{
    Activity, ActivityKind, ConnectionState, CustomProfile, ErrorCode, HeatingMode, LockState,
    Operation, ProfileTarget, Snapshot,
};
use ploom_core::transport::fake::{FakeConnector, FakeDevice, Response};
use ploom_core::{SessionConfig, SessionHandle};
use ploom_protocol::heating::{HeatingProfile, build_profile_frames};
use ploom_protocol::{CustomProfile as ProtoCustom, Masters};
use tempfile::TempDir;
use tokio::time::{sleep, timeout};

const ID: &str = "fake-1";

struct Harness {
    session: SessionHandle,
    fake: FakeDevice,
    connector: FakeConnector,
    dir: TempDir,
}

fn start(
    devices: &[&FakeDevice],
    auto_connect: bool,
    dir: TempDir,
) -> (SessionHandle, FakeConnector, TempDir) {
    let connector = FakeConnector::new(devices.iter().map(|d| (*d).clone()));
    let config = SessionConfig {
        data_dir: dir.path().to_owned(),
        auto_connect,
    };
    (
        SessionHandle::with_connector(&config, Arc::new(connector.clone())),
        connector,
        dir,
    )
}

async fn wait_for(
    session: &SessionHandle,
    what: &str,
    pred: impl FnMut(&Snapshot) -> bool,
) -> Snapshot {
    let mut rx = session.subscribe_snapshot();
    match timeout(Duration::from_secs(120), rx.wait_for(pred)).await {
        Ok(Ok(s)) => s.clone(),
        _ => panic!(
            "timed out waiting for {what}; last snapshot: {:#?}",
            session.snapshot()
        ),
    }
}

async fn connected() -> Harness {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    let (session, connector, dir) = start(&[&fake], false, tempfile::tempdir().unwrap());
    session.connect(Some(ID.into())).await.unwrap();
    wait_for(&session, "first poll", |s| {
        s.connection == ConnectionState::Connected && s.status.usb_plugged.is_some()
    })
    .await;
    Harness {
        session,
        fake,
        connector,
        dir,
    }
}

fn events(session: &SessionHandle) -> Vec<ActivityKind> {
    session
        .activity_history()
        .into_iter()
        .map(|e| e.kind)
        .collect()
}

#[tokio::test(start_paused = true)]
async fn connects_and_derives_status() {
    let h = connected().await;
    let s = h.session.snapshot();
    let device = s.device.unwrap();
    assert_eq!(device.id, ID);
    assert_eq!(device.name.as_deref(), Some("Ploom aura TEST"));
    assert_eq!(device.firmware.as_deref(), Some("M1.0014_G4"));
    assert_eq!(device.hardware.as_deref(), Some("U123456VL4U66CJ 170B"));
    let color = device.color.expect("color decoded from hardware");
    assert_eq!(color.code, "L4U");
    assert_eq!(color.id, "glacier");
    assert_eq!(s.connection_error, None);

    let st = s.status;
    assert_eq!(st.battery_pct, Some(80));
    assert_eq!(st.battery_health_pct, Some(95));
    assert_eq!(st.rssi_dbm, Some(-55));
    assert_eq!(st.heating_mode, Some(HeatingMode::Balanced));
    assert_eq!(st.lock, Some(LockState::Unlocked));
    assert_eq!(st.auto_start, Some(false));
    assert_eq!(st.usb_plugged, Some(false));
    assert_eq!(st.charging, Some(false));
    assert_eq!(st.activity, Some(Activity::Standby));
    assert_eq!(st.enclosure_temp_c, Some(26.5));
    assert_eq!(st.temperature_warning, Some(false));
    assert!(!st.vibrating);

    assert_eq!(
        events(&h.session),
        [ActivityKind::Connected {
            device_name: Some("Ploom aura TEST".into())
        }]
    );
    // The readiness probe and the first poll each asked for status.
    assert!(h.fake.writes_with_opcode(0xE3).len() >= 2);
    assert!(
        h.dir.path().join("session.json").exists(),
        "last device is persisted"
    );
}

#[tokio::test(start_paused = true)]
async fn state_change_beats_stale_status_while_heating() {
    let h = connected().await;
    h.fake.start_session();
    let s = wait_for(&h.session, "heating", |s| {
        s.status.activity == Some(Activity::Heating)
    })
    .await;
    let started = s
        .status
        .session_started_at_ms
        .expect("session start recorded");

    // Several polls later the device still ignores E3 while heating; the
    // idle 0x30 from before the session (still in the window) must not win.
    sleep(Duration::from_secs(10)).await;
    let s = h.session.snapshot();
    assert_eq!(s.status.activity, Some(Activity::Heating));
    assert_eq!(s.status.session_started_at_ms, Some(started));

    // Multi-bit state: highest bit (smoking) wins.
    h.fake.emit([0x04, 0x31, 0x06, 0x00, 0x00]);
    sleep(Duration::from_millis(10)).await;
    assert_eq!(
        h.session.snapshot().status.activity,
        Some(Activity::Heating)
    );

    h.fake.end_session();
    let s = wait_for(&h.session, "standby", |s| {
        s.status.activity == Some(Activity::Standby)
    })
    .await;
    assert_eq!(s.status.session_started_at_ms, None);

    let states: Vec<_> = events(&h.session)
        .into_iter()
        .filter_map(|e| match e {
            ActivityKind::StateChanged { activity } => Some(activity),
            _ => None,
        })
        .collect();
    assert_eq!(states, [Activity::Heating, Activity::Standby]);
}

#[tokio::test(start_paused = true)]
async fn charging_from_usb_detection() {
    let h = connected().await;
    h.fake.update(|s| s.usb_cc = (456, 28));
    let s = wait_for(&h.session, "charging", |s| s.status.charging == Some(true)).await;
    assert_eq!(s.status.usb_plugged, Some(true));
    assert_eq!(s.status.activity, Some(Activity::Charging));

    h.fake.update(|s| s.usb_cc = (28, 28));
    wait_for(&h.session, "unplugged", |s| {
        s.status.charging == Some(false)
    })
    .await;

    let evs = events(&h.session);
    let charging: Vec<_> = evs
        .iter()
        .filter_map(|e| match e {
            ActivityKind::ChargingChanged { charging } => Some(*charging),
            _ => None,
        })
        .collect();
    assert_eq!(charging, [true, false]);
    assert!(
        !evs.iter()
            .any(|e| matches!(e, ActivityKind::StateChanged { .. })),
        "charging transitions are reported only as chargingChanged: {evs:?}"
    );
}

#[tokio::test(start_paused = true)]
async fn charging_from_device_events_without_usb_reading() {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    // Diagnostics channel silent: no USB-C reading available.
    fake.intercept(|f| {
        if f[1] == 0xF1 {
            Response::Ignore
        } else {
            Response::Default
        }
    });
    let (session, _connector, _dir) = start(&[&fake], false, tempfile::tempdir().unwrap());
    session.connect(None).await.unwrap();
    wait_for(&session, "battery", |s| s.status.battery_pct.is_some()).await;
    assert_eq!(session.snapshot().status.usb_plugged, None);

    fake.emit([0x06, 0x34, 0x00, 0x03, 0x00, 0x00, 0x00]);
    wait_for(&session, "charging", |s| s.status.charging == Some(true)).await;
    // Polls keep delivering 0x30 without the charging bit; that must not clear it.
    sleep(Duration::from_secs(10)).await;
    assert_eq!(session.snapshot().status.charging, Some(true));

    fake.emit([0x06, 0x34, 0x00, 0x07, 0x00, 0x00, 0x00]);
    wait_for(&session, "not charging", |s| {
        s.status.charging == Some(false)
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn charging_inferred_from_battery_rise() {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    fake.intercept(|f| {
        if f[1] == 0xF1 {
            Response::Ignore
        } else {
            Response::Default
        }
    });
    fake.update(|s| s.battery_pct = 50);
    let (session, _connector, _dir) = start(&[&fake], false, tempfile::tempdir().unwrap());
    session.connect(None).await.unwrap();
    wait_for(&session, "battery", |s| s.status.battery_pct == Some(50)).await;
    sleep(Duration::from_secs(4)).await;

    fake.update(|s| s.battery_pct = 51);
    wait_for(&session, "51%", |s| s.status.battery_pct == Some(51)).await;
    sleep(Duration::from_secs(4)).await;
    assert_eq!(
        session.snapshot().status.charging,
        Some(false),
        "+1 is rebound"
    );

    fake.update(|s| s.battery_pct = 52);
    wait_for(&session, "trend", |s| s.status.charging == Some(true)).await;
}

#[tokio::test(start_paused = true)]
async fn reconnects_after_link_loss() {
    let h = connected().await;
    h.fake.set_reachable(false);
    h.fake.drop_link();
    wait_for(&h.session, "reconnecting", |s| {
        s.connection == ConnectionState::Reconnecting
    })
    .await;
    assert!(
        matches!(h.session.set_lock(true).await, Err(e) if e.code() == ErrorCode::NotConnected)
    );

    // While unreachable, attempts fail with backoff and report why.
    let s = wait_for(&h.session, "connection error", |s| {
        s.connection_error.is_some()
    })
    .await;
    assert_eq!(s.connection_error, Some(ErrorCode::DeviceNotFound));
    assert_eq!(s.connection, ConnectionState::Reconnecting);
    assert_eq!(
        s.device.as_ref().map(|d| d.id.as_str()),
        Some(ID),
        "identity kept"
    );

    h.fake.set_reachable(true);
    let s = wait_for(&h.session, "reconnected", |s| {
        s.connection == ConnectionState::Connected
    })
    .await;
    assert_eq!(s.connection_error, None);
    assert_eq!(h.fake.connect_count(), 2);

    let evs = events(&h.session);
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, ActivityKind::ConnectionLost))
            .count(),
        1
    );
    assert_eq!(
        evs.iter()
            .filter(|e| matches!(e, ActivityKind::Connected { .. }))
            .count(),
        2
    );
}

#[tokio::test(start_paused = true)]
async fn backoff_grows_between_attempts() {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    fake.set_reachable(false);
    let (session, _connector, _dir) = start(&[&fake], false, tempfile::tempdir().unwrap());
    session.connect(Some(ID.into())).await.unwrap();
    let s = wait_for(&session, "first failure", |s| s.connection_error.is_some()).await;
    assert_eq!(s.connection, ConnectionState::Reconnecting);
    // Attempts at ~0, 2, 6, 14, 30, 50 s: after a minute there were at most 6.
    sleep(Duration::from_secs(60)).await;
    fake.set_reachable(true);
    wait_for(&session, "connected", |s| {
        s.connection == ConnectionState::Connected
    })
    .await;
    assert_eq!(fake.connect_count(), 1);
}

#[tokio::test(start_paused = true)]
async fn user_disconnect_stays_idle() {
    let h = connected().await;
    h.session.disconnect().await.unwrap();
    let s = h.session.snapshot();
    assert_eq!(s.connection, ConnectionState::Idle);
    assert_eq!(s.device, None);
    assert_eq!(s.status.battery_pct, None);
    assert!(!h.fake.is_linked());
    sleep(Duration::from_secs(120)).await;
    assert_eq!(
        h.fake.connect_count(),
        1,
        "no reconnect after a user disconnect"
    );
    assert_eq!(events(&h.session).last(), Some(&ActivityKind::Disconnected));
    h.session.disconnect().await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn disconnect_while_reconnecting() {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    fake.set_reachable(false);
    let (session, _connector, _dir) = start(&[&fake], false, tempfile::tempdir().unwrap());
    session.connect(None).await.unwrap();
    wait_for(&session, "failure", |s| {
        s.connection == ConnectionState::Reconnecting
    })
    .await;
    session.disconnect().await.unwrap();
    assert_eq!(session.snapshot().connection, ConnectionState::Idle);
    fake.set_reachable(true);
    sleep(Duration::from_secs(60)).await;
    assert_eq!(fake.connect_count(), 0);
}

#[tokio::test(start_paused = true)]
async fn switching_devices() {
    let a = FakeDevice::new("a", "Ploom aura A");
    let b = FakeDevice::new("b", "Ploom aura B");
    b.update(|s| s.battery_pct = 33);
    let (session, _connector, _dir) = start(&[&a, &b], false, tempfile::tempdir().unwrap());
    session.connect(Some("a".into())).await.unwrap();
    wait_for(&session, "a", |s| s.status.battery_pct == Some(80)).await;
    session.connect(Some("b".into())).await.unwrap();
    let s = wait_for(&session, "b", |s| s.status.battery_pct == Some(33)).await;
    assert_eq!(s.device.unwrap().id, "b");
    assert!(!a.is_linked());
    assert!(b.is_linked());
}

#[tokio::test(start_paused = true)]
async fn auto_connect_uses_last_device() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("session.json"),
        r#"{"version":1,"lastDeviceId":"b","customProfiles":{}}"#,
    )
    .unwrap();
    let a = FakeDevice::new("a", "Ploom aura A");
    let b = FakeDevice::new("b", "Ploom aura B");
    let (session, _connector, _dir) = start(&[&a, &b], true, dir);
    let s = wait_for(&session, "connected", |s| {
        s.connection == ConnectionState::Connected
    })
    .await;
    assert_eq!(s.device.unwrap().id, "b");
    assert_eq!(a.connect_count(), 0);
}

#[tokio::test(start_paused = true)]
async fn dropping_the_last_handle_disconnects() {
    let h = connected().await;
    let fake = h.fake.clone();
    drop(h);
    sleep(Duration::from_secs(1)).await;
    assert!(!fake.is_linked());
}

#[tokio::test(start_paused = true)]
async fn flashing_reports_progress_and_persists_custom_profile() {
    let h = connected().await;
    let mut rx = h.session.subscribe_snapshot();
    let progress = tokio::spawn(async move {
        let mut seen = Vec::new();
        while rx.changed().await.is_ok() {
            if let Some(Operation::FlashingProfile { done, total, .. }) =
                rx.borrow_and_update().operation.clone()
            {
                seen.push((done, total));
            }
        }
        seen
    });

    h.fake.clear_writes();
    let target = ProfileTarget::Custom {
        base: HeatingMode::Intense,
        hold_temp_c: 999,
        hold_secs: 150,
    };
    h.session.apply_profile(target).await.unwrap();

    // Byte-for-byte the upload computed from the device's calibration.
    let custom = ProtoCustom::new(HeatingMode::Intense, 999, 150).unwrap();
    let masters = Masters::new(h.fake.state().masters).unwrap();
    let expected: Vec<Vec<u8>> = build_profile_frames(&HeatingProfile::custom(&custom), &masters)
        .unwrap()
        .into_iter()
        .map(|f| f.frame.into_bytes())
        .collect();
    let uploaded: Vec<Vec<u8>> = h
        .fake
        .writes()
        .into_iter()
        .filter(|w| (0xA7..=0xD8).contains(&w[1]) || [0xFB, 0xA4].contains(&w[1]))
        .collect();
    assert_eq!(uploaded, expected);
    assert_eq!(h.fake.state().downloaded_profile, 2);

    let normalized = ProfileTarget::Custom {
        base: HeatingMode::Intense,
        hold_temp_c: 320,
        hold_secs: 150,
    };
    assert!(events(&h.session).contains(&ActivityKind::ProfileApplied {
        target: normalized,
        confirmed: true
    }));
    assert_eq!(h.session.snapshot().operation, None);

    let expected_custom = CustomProfile {
        base: HeatingMode::Intense,
        hold_temp_c: 320,
        hold_secs: 150,
    };
    let s = wait_for(&h.session, "custom reported", |s| {
        s.status.custom_profile.is_some()
    })
    .await;
    assert_eq!(s.status.heating_mode, Some(HeatingMode::Intense));
    assert_eq!(s.status.custom_profile, Some(expected_custom));
    let saved = std::fs::read_to_string(h.dir.path().join("session.json")).unwrap();
    assert!(saved.contains("\"holdTempC\": 320"), "{saved}");

    drop(h.session);
    let seen = progress.await.unwrap();
    assert!(seen.iter().all(|&(_, total)| total == 38));
    assert_eq!(seen.last(), Some(&(38, 38)));
    assert!(
        seen.windows(2).all(|w| w[0].0 <= w[1].0),
        "progress is monotonic: {seen:?}"
    );
    assert!(seen.len() > 10, "intermediate progress published: {seen:?}");
}

#[tokio::test(start_paused = true)]
async fn balanced_is_one_frame_and_clears_custom_profile() {
    let h = connected().await;
    h.session
        .apply_profile(ProfileTarget::Custom {
            base: HeatingMode::Delicate,
            hold_temp_c: 250,
            hold_secs: 200,
        })
        .await
        .unwrap();
    wait_for(&h.session, "custom", |s| s.status.custom_profile.is_some()).await;

    h.fake.clear_writes();
    h.session
        .apply_profile(ProfileTarget::Preset {
            mode: HeatingMode::Balanced,
        })
        .await
        .unwrap();
    assert_eq!(h.fake.writes_with_opcode(0xA4), [vec![0x02, 0xA4, 0x00]]);
    assert!(
        h.fake.writes_with_opcode(0xA5).is_empty(),
        "no calibration read for balanced"
    );
    let s = wait_for(&h.session, "balanced", |s| {
        s.status.heating_mode == Some(HeatingMode::Balanced)
    })
    .await;
    assert_eq!(s.status.custom_profile, None);
    let saved = std::fs::read_to_string(h.dir.path().join("session.json")).unwrap();
    assert!(!saved.contains("holdTempC"), "{saved}");
}

#[tokio::test(start_paused = true)]
async fn device_rejection_aborts_flashing() {
    let h = connected().await;
    h.fake.intercept(|f| {
        if f[1] == 0xCC {
            Response::Reply(vec![vec![0x02, 0x49, 0x01]])
        } else {
            Response::Default
        }
    });
    h.fake.clear_writes();
    let err = h
        .session
        .apply_profile(ProfileTarget::Preset {
            mode: HeatingMode::Delicate,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::DeviceRejected);
    assert!(err.to_string().contains("0xcc"), "{err}");
    assert_eq!(h.session.snapshot().operation, None);
    assert!(
        h.fake.writes_with_opcode(0xA4).is_empty(),
        "never activated"
    );
    assert!(
        h.fake.writes_with_opcode(0xCD).is_empty(),
        "stopped at the refusal"
    );
    assert!(
        !events(&h.session)
            .iter()
            .any(|e| matches!(e, ActivityKind::ProfileApplied { .. }))
    );
}

#[tokio::test(start_paused = true)]
async fn verification_mismatch_is_rejected() {
    let h = connected().await;
    h.fake.intercept(|f| {
        if f[1] == 0xA3 {
            Response::Reply(vec![vec![0x08, 0x3E, 0, 3, 0, 0, 0, 0, 0]])
        } else {
            Response::Default
        }
    });
    let err = h
        .session
        .apply_profile(ProfileTarget::Preset {
            mode: HeatingMode::Intense,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::DeviceRejected);
}

#[tokio::test(start_paused = true)]
async fn unanswered_verification_is_unconfirmed() {
    let h = connected().await;
    h.fake.intercept(|f| {
        if f[1] == 0xA3 {
            Response::Ignore
        } else {
            Response::Default
        }
    });
    h.session
        .apply_profile(ProfileTarget::Preset {
            mode: HeatingMode::Intense,
        })
        .await
        .unwrap();
    assert!(events(&h.session).contains(&ActivityKind::ProfileApplied {
        target: ProfileTarget::Preset {
            mode: HeatingMode::Intense
        },
        confirmed: false
    }));
}

#[tokio::test(start_paused = true)]
async fn calibration_out_of_range_writes_nothing() {
    let h = connected().await;
    h.fake.update(|s| s.masters[7] = 1000);
    h.fake.clear_writes();
    let err = h
        .session
        .apply_profile(ProfileTarget::Preset {
            mode: HeatingMode::Delicate,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::CalibrationOutOfRange);
    assert!(h.fake.writes().iter().all(|w| w[1] != 0xA7 && w[1] != 0xA4));
}

#[tokio::test(start_paused = true)]
async fn commands_are_refused_while_flashing() {
    let h = connected().await;
    // Stall the upload on its last acknowledgement.
    h.fake.intercept(|f| {
        if f[1] == 0xCE {
            Response::Ignore
        } else {
            Response::Default
        }
    });
    let session = h.session.clone();
    let flash = tokio::spawn(async move {
        session
            .apply_profile(ProfileTarget::Preset {
                mode: HeatingMode::Delicate,
            })
            .await
    });
    wait_for(&h.session, "flashing", |s| s.operation.is_some()).await;

    for err in [
        h.session.set_lock(true).await.unwrap_err(),
        h.session.read_logs().await.unwrap_err(),
        h.session.send_raw("02e300", false).await.unwrap_err(),
        h.session
            .apply_profile(ProfileTarget::Preset {
                mode: HeatingMode::Balanced,
            })
            .await
            .unwrap_err(),
    ] {
        assert_eq!(err.code(), ErrorCode::Busy, "{err}");
    }

    let err = flash.await.unwrap().unwrap_err();
    assert_eq!(err.code(), ErrorCode::Timeout);
    assert_eq!(
        h.session.snapshot().operation,
        None,
        "operation cleared after failure"
    );
    h.session.set_lock(true).await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn heating_device_refuses_flashing_and_diagnostics() {
    let h = connected().await;
    h.fake.start_session();
    wait_for(&h.session, "heating", |s| {
        s.status.activity == Some(Activity::Heating)
    })
    .await;
    h.fake.clear_writes();
    let err = h
        .session
        .apply_profile(ProfileTarget::Preset {
            mode: HeatingMode::Intense,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::Busy);
    let err = h.session.read_diagnostics().await.unwrap_err();
    assert_eq!(err.code(), ErrorCode::Busy);
    assert!(err.to_string().contains("heating"), "{err}");
    assert!(
        h.fake.writes_with_opcode(0xA5).is_empty() && h.fake.writes_with_opcode(0xF1).is_empty()
    );
}

#[tokio::test(start_paused = true)]
async fn dangerous_raw_frames_need_confirmation() {
    let h = connected().await;
    for hex in ["02c400", "02 a1 00", "02FF00"] {
        let err = h.session.send_raw(hex, false).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::DangerousFrame, "{hex}");
    }
    assert!(h.fake.writes_with_opcode(0xC4).is_empty());
    assert!(h.fake.writes_with_opcode(0xA1).is_empty());
    assert!(h.fake.writes_with_opcode(0xFF).is_empty());

    h.session.send_raw("02c400", true).await.unwrap();
    assert_eq!(h.fake.writes_with_opcode(0xC4).len(), 1);
}

#[tokio::test(start_paused = true)]
async fn raw_exchange_decodes_replies() {
    let h = connected().await;
    let x = h.session.send_raw("02 e3 00", false).await.unwrap();
    assert_eq!(x.sent, "02 e3 00");
    let ops: Vec<u8> = x.replies.iter().map(|r| r.opcode).collect();
    assert_eq!(ops, [0x30, 0x33, 0x9F, 0x3C]);
    assert_eq!(x.replies[3].decoded["type"], "batteryLevel");
    assert_eq!(x.replies[0].decoded["type"], "status");
    assert!(events(&h.session).contains(&ActivityKind::RawFrameSent {
        hex: "02 e3 00".into(),
        replies: 4
    }));

    for bad in ["zz", "02e3", "03e300", ""] {
        let err = h.session.send_raw(bad, false).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::InvalidInput, "{bad}");
    }
}

#[tokio::test(start_paused = true)]
async fn not_connected_errors() {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    let (session, _connector, _dir) = start(&[&fake], false, tempfile::tempdir().unwrap());
    assert_eq!(session.snapshot().connection, ConnectionState::Idle);
    assert_eq!(
        session.set_lock(true).await.unwrap_err().code(),
        ErrorCode::NotConnected
    );
    assert_eq!(
        session.read_logs().await.unwrap_err().code(),
        ErrorCode::NotConnected
    );
    assert_eq!(
        session.refresh_status().await.unwrap_err().code(),
        ErrorCode::NotConnected
    );
    // Validation happens before the connection check.
    assert_eq!(
        session.send_raw("02c400", false).await.unwrap_err().code(),
        ErrorCode::DangerousFrame
    );
    let custom_on_balanced = ProfileTarget::Custom {
        base: HeatingMode::Balanced,
        hold_temp_c: 280,
        hold_secs: 60,
    };
    assert_eq!(
        session
            .apply_profile(custom_on_balanced)
            .await
            .unwrap_err()
            .code(),
        ErrorCode::InvalidInput
    );
    // Balanced needs no calibration, so it previews offline.
    let p = session
        .preview_profile(ProfileTarget::Preset {
            mode: HeatingMode::Balanced,
        })
        .await
        .unwrap();
    assert_eq!(p.calibration_key, None);
    assert_eq!(p.frames.len(), 1);
    assert_eq!(p.frames[0].hex, "02 a4 00");
    assert_eq!(p.frames[0].awaits_opcode, Some(0x3F));
    let err = session
        .preview_profile(ProfileTarget::Preset {
            mode: HeatingMode::Intense,
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::NotConnected);
}

#[tokio::test(start_paused = true)]
async fn preview_reads_calibration_only() {
    let h = connected().await;
    h.fake.clear_writes();
    let p = h
        .session
        .preview_profile(ProfileTarget::Preset {
            mode: HeatingMode::Delicate,
        })
        .await
        .unwrap();
    assert_eq!(p.calibration_key, Some(1200));
    assert_eq!(p.frames.len(), 38);
    assert_eq!(p.frames[37].hex, "02 a4 01");
    let writes: Vec<u8> = h.fake.writes().iter().map(|w| w[1]).collect();
    assert!(
        writes
            .iter()
            .all(|op| [0xA5, 0xE3, 0xFD, 0xA2, 0xA3, 0xF1].contains(op)),
        "{writes:x?}"
    );
}

#[tokio::test(start_paused = true)]
async fn vibration_switches_itself_off() {
    let h = connected().await;
    h.session.set_vibration(true).await.unwrap();
    wait_for(&h.session, "vibrating", |s| s.status.vibrating).await;
    assert!(h.fake.state().vibrating);
    sleep(Duration::from_secs(29)).await;
    assert!(h.fake.state().vibrating);
    sleep(Duration::from_secs(2)).await;
    assert!(!h.fake.state().vibrating);
    wait_for(&h.session, "quiet", |s| !s.status.vibrating).await;
    let evs = events(&h.session);
    assert!(evs.contains(&ActivityKind::VibrationChanged { on: true }));
    assert!(evs.contains(&ActivityKind::VibrationChanged { on: false }));
}

#[tokio::test(start_paused = true)]
async fn turning_vibration_off_cancels_the_timer() {
    let h = connected().await;
    h.session.set_vibration(true).await.unwrap();
    sleep(Duration::from_secs(5)).await;
    h.session.set_vibration(false).await.unwrap();
    h.session.set_vibration(true).await.unwrap();
    sleep(Duration::from_secs(27)).await;
    assert!(
        h.fake.state().vibrating,
        "the first timer must not switch off the second buzz"
    );
    sleep(Duration::from_secs(5)).await;
    assert!(!h.fake.state().vibrating);
}

#[tokio::test(start_paused = true)]
async fn lock_and_auto_start() {
    let h = connected().await;
    h.session.set_lock(true).await.unwrap();
    wait_for(&h.session, "locked", |s| {
        s.status.lock == Some(LockState::Locked)
    })
    .await;
    h.session.set_auto_start(true).await.unwrap();
    wait_for(&h.session, "auto-start", |s| {
        s.status.auto_start == Some(true)
    })
    .await;
    assert_eq!(h.fake.writes_with_opcode(0x9E), [vec![0x02, 0x9E, 0x01]]);
    assert_eq!(h.fake.writes_with_opcode(0xCA), [vec![0x02, 0xCA, 0x01]]);
    let evs = events(&h.session);
    assert!(evs.contains(&ActivityKind::LockChanged {
        lock: LockState::Locked
    }));
    assert!(evs.contains(&ActivityKind::AutoStartChanged { enabled: true }));
}

#[tokio::test(start_paused = true)]
async fn logs_are_read_without_deleting() {
    let h = connected().await;
    let report = h.session.read_logs().await.unwrap();
    let session = report.session.unwrap();
    assert_eq!(session.duration_s, 360);
    assert_eq!(
        session.puff_offsets_s,
        [14, 93, 144, 208, 230, 265, 292, 343]
    );
    let charge = report.charge.unwrap();
    assert_eq!((charge.start_mv, charge.end_mv), (3581, 3734));
    assert_eq!(charge.total_charge_count, Some(42));
    assert_eq!(
        report.unacquired.map(|u| (u.sessions, u.charges)),
        Some((6, 7))
    );
    assert!(
        h.fake.writes_with_opcode(0xC4).is_empty(),
        "logs are never deleted"
    );
    assert!(
        !events(&h.session).contains(&ActivityKind::SessionLogReceived),
        "a requested log is not a new session"
    );

    // A log pushed by the device after a session is.
    h.fake.emit(
        ploom_protocol::frame::decode_hex("1336b033000068010800000100010e00000000b095").unwrap(),
    );
    sleep(Duration::from_millis(10)).await;
    assert!(events(&h.session).contains(&ActivityKind::SessionLogReceived));
}

#[tokio::test(start_paused = true)]
async fn diagnostics() {
    let h = connected().await;
    h.fake.update(|s| s.usb_cc = (456, 28));
    let d = h.session.read_diagnostics().await.unwrap();
    assert_eq!(d.enclosure_temp_c, Some(26.5));
    assert_eq!(d.usb_plugged, Some(true));
    assert_eq!(d.usb_cc, Some((456, 28)));
    let names: Vec<_> = d.adc.unwrap().into_iter().map(|a| a.name).collect();
    assert_eq!(
        names,
        [
            "Vtemp",
            "USBcc1",
            "USBcc2",
            "HeaterNtc",
            "B",
            "CaseNtc",
            "HeatTemp"
        ]
    );
    let starts = d.heating_starts.unwrap();
    assert_eq!(starts.len(), 7);
    assert_eq!((starts[0].key.as_str(), starts[0].count), ("coldStart", 12));
    assert_eq!(d.charge_cycles, Some(42));
    assert_eq!(d.battery_pct, Some(80));
    assert_eq!(d.battery_health_pct, Some(95));
    assert_eq!(h.session.snapshot().status.usb_plugged, Some(true));
}

#[tokio::test(start_paused = true)]
async fn polling_survives_unanswered_requests() {
    let fake = FakeDevice::new(ID, "Ploom aura TEST");
    fake.intercept(|f| {
        if f[1] == 0xFD {
            Response::Ignore
        } else {
            Response::Default
        }
    });
    let (session, _connector, _dir) = start(&[&fake], false, tempfile::tempdir().unwrap());
    session.connect(None).await.unwrap();
    // USB detection is read at the end of the first poll.
    let s = wait_for(&session, "first poll", |s| s.status.usb_plugged.is_some()).await;
    assert_eq!(s.status.rssi_dbm, None);
    assert_eq!(s.status.battery_pct, Some(80));
    // RSSI is retried three times per poll, then the poll moves on.
    assert!(fake.writes_with_opcode(0xFD).len() >= 3);
    let status = session.refresh_status().await.unwrap();
    assert_eq!(status.battery_pct, Some(80));
}

#[tokio::test(start_paused = true)]
async fn scan_lists_devices_and_marks_the_connected_one() {
    let h = connected().await;
    let other = FakeDevice::new("zz", "Ploom aura OTHER");
    other.set_advertised_rssi(Some(-40));
    h.connector.add(other);

    let session = h.session.clone();
    let scan = tokio::spawn(async move { session.scan().await });
    wait_for(&h.session, "scanning", |s| s.scanning).await;
    assert_eq!(h.session.scan().await.unwrap_err().code(), ErrorCode::Busy);
    let found = scan.await.unwrap().unwrap();
    assert!(!h.session.snapshot().scanning);

    let summary: Vec<_> = found
        .iter()
        .map(|d| (d.id.as_str(), d.connected, d.rssi_dbm))
        .collect();
    assert_eq!(summary, [("zz", false, Some(-40)), (ID, true, Some(-60))]);
}

#[tokio::test(start_paused = true)]
async fn catalog_and_history_are_available() {
    let h = connected().await;
    let catalog = h.session.catalog();
    assert_eq!(catalog.modes.len(), 4);
    let mut rx = h.session.subscribe_activity();
    h.session.set_lock(true).await.unwrap();
    let event = timeout(Duration::from_secs(10), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        event.kind,
        ActivityKind::LockChanged {
            lock: LockState::Locked
        }
    );
    let ids: Vec<u64> = h.session.activity_history().iter().map(|e| e.id).collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "ids increase: {ids:?}");
}
