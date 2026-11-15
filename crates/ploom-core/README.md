# ploom-core

Bluetooth LE transport, device session and state for the Ploom Aura (GEN4L),
built on [`ploom-protocol`](../ploom-protocol).

- `transport` — `Transport`/`Connector` traits, a [btleplug] implementation
  (default feature `btleplug`) and an in-memory simulated device
  (`transport::fake`, feature `test-util`) for tests.
- `device` — request/response: waiters registered before writing and always
  cleaned up, reply-window exchanges, readiness probing, diagnostics reads.
- `session` — one supervisor task per application: connect, poll, derive
  status, reconnect with backoff, serialize BLE conversations, flash
  profiles. Exposed through a cheap-to-clone `SessionHandle`.
- `model` — the IPC data types, mirroring `app/ui/src/lib/api/types.ts`.

```rust,no_run
use ploom_core::{SessionConfig, SessionHandle};
use ploom_core::model::{ConnectionState, HeatingMode, ProfileTarget};

# async fn demo() -> Result<(), ploom_core::Error> {
let session = SessionHandle::start(&SessionConfig {
    data_dir: "/path/to/app-data".into(),
    auto_connect: true,
});

let mut snapshots = session.subscribe_snapshot();
snapshots.wait_for(|s| s.connection == ConnectionState::Connected).await.unwrap();

session.apply_profile(ProfileTarget::Preset { mode: HeatingMode::Intense }).await?;
println!("{:#?}", session.snapshot().status);
# Ok(()) }
```

Session tests (`tests/session.rs`) run against the simulated device on
Tokio's paused clock, so reconnect backoff, polling and timers are exercised
deterministically and quickly.

[btleplug]: https://docs.rs/btleplug
