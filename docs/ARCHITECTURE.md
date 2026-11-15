# Architecture

Ploom is a desktop controller for the Ploom Aura (GEN4L) over Bluetooth LE.
The protocol was reverse-engineered from the official web app; the original
reference client lives outside this repository and was used to produce
the golden test vectors in `crates/ploom-protocol/tests/`. The wire protocol
itself is documented in [PROTOCOL.md](PROTOCOL.md).

## Layout

```
crates/
  ploom-protocol   Pure protocol. No I/O, no async, no BLE types.
  ploom-core       BLE transport (btleplug), device session, state, commands.
  ploom-cli        `ploom` binary for scripting and debugging.
app/
  src-tauri        Desktop shell (Tauri 2). Thin: maps commands/events to ploom-core.
  ui               Svelte 5 + TypeScript + Vite. Talks only to the API in src/lib/api.
docs/
  PROTOCOL.md      Wire protocol reference.
```

Dependency direction is strictly one-way:

```
ploom-protocol  <-  ploom-core  <-  ploom-cli
                                <-  app/src-tauri  <-(IPC)->  app/ui
```

`ploom-protocol` must never depend on tokio, btleplug or serde-json; it may
optionally derive `serde` behind a feature flag. Anything that can be
unit-tested without a device belongs there.

## The wire contract

`app/ui/src/lib/api/types.ts` is the single source of truth for everything
that crosses IPC. `ploom-core::model` mirrors it field by field with
`#[serde(rename_all = "camelCase")]`. When one side changes, the other changes
in the same commit.

The backend never sends user-facing prose. Errors carry an `ErrorCode`,
activity events carry a typed payload, diagnostics carry stable keys; the UI
localizes all of it (Russian and English).

## ploom-protocol

Modules (suggested):

| module        | contents |
|---------------|----------|
| `frame`       | `[LEN][OPCODE][payload]` encoding, `Frame` type, raw-hex parsing with LEN validation |
| `opcode`      | named opcode constants for requests and notifications, `DANGEROUS_OPCODES` |
| `command`     | builders for every request (`Command::RequestStatus`, `SetLock(bool)`, …) |
| `notification`| `Notification` enum: one variant per known reply, `Unknown { opcode, payload }` |
| `heating`     | `HeatingMode`, curves, profile constants, `get_res`, `build_profile_frames`, custom profiles |
| `calibration` | `Masters` newtype with validation (len ≥ 16, masters[8] ≠ 0), `leaflet_const` |
| `diagnostics` | `0xF1/0xF2` channel: request builder, decoders for types 7, 9, 12 |
| `status`      | `StatusUpdate` (partial status fields) and the windowed merge rule |

Correctness rules that are easy to get wrong (all verified against the
official app's JavaScript):

- Step frame: `[res u16 LE][duration u8 seconds][temp i16 LE][trunc(gradient*1000) i32 LE][0xFF]`,
  with an extra leading `0x01` on the first step only. Duration is **one byte**,
  so custom hold phases are capped at 255 s.
- `get_res` rounds half away from zero (Dart `round()`), gradients are
  truncated toward zero (Dart `toInt()`).
- Frame `0xFB` payload is `1 if profile.number == 8 else 0` (it compares the
  profile's field, not the firmware version) — so it is `0` for every preset.
- `0x31` with several bits set maps to the highest set bit.
- `0x3D` is a flag (0 = factory profile active, 1 = downloaded profile active),
  not a mode number. The effective mode is `balanced` when the flag is 0,
  otherwise the mode reported by `0x3E` byte 1.
- Writes must be **with response**; CoreBluetooth silently drops
  write-without-response on this characteristic.

## ploom-core

### Transport

A `Transport` trait (`write`, `notifications() -> Stream<Vec<u8>>`,
`read_characteristic`, `disconnect`, `is_connected`) with a btleplug
implementation and an in-memory fake for tests (feature `test-util`, not
compiled into release builds). Everything above the trait
is testable without hardware.

### Device

`Device` owns a transport and provides:

- `send(frame)` — fire and forget (still a write-with-response).
- `request(frame, expect_opcode, timeout)` — registers a waiter **before**
  writing, resolves on the first notification with that opcode, always
  deregisters on timeout/error.
- `exchange(frame, window)` — collects every notification for `window`.
- `wait_ready()` — after subscribing, the device swallows writes for a
  variable time; resend `RequestStatus` until the first notification
  (6 attempts × 1.2 s).
- `read_diag(type)` — reply must echo the requested type in payload[0],
  otherwise treat as no answer (a late reply to a previous request).

### Session (state machine)

One `Session` task per app. It owns the connection lifecycle and the
derived `DeviceStatus`:

- Connect (by id, or first device whose name starts with `Ploom aura` or that
  advertises service `53654010-…`), subscribe, `wait_ready`, read DIS
  hardware/firmware strings.
- Poll every 3 s: `E3→3C`, `FD→FC`, `A2→3D`, `A3→3E`; each retried up to
  3 times (1 attempt while heating — the device ignores status requests then).
- Every other poll, when not heating: diag type 7 (USB-C CC lines → plugged if
  max(cc1, cc2) > 100) and type 9 (enclosure °C = value/100, 0 = no reading,
  keep last good value).
- Status merge: notifications from the last 15 s are folded in arrival order;
  a `0x31` state change expands into flags (smoking ⇒ inUse, …) so a stale
  `0x30` "standby" cannot override a fresher state change.
- Charging = usbPlugged if known, else last `0x34`, else battery % rose ≥ 2
  since the baseline (reset on discharge/unplug/standby). `0x30` bit 4 may only
  confirm charging, never clear it.
- `sessionStartedAtMs` is set on the first heating state and cleared on standby.
- On disconnect: emit `connectionLost`, reconnect to the same id with
  exponential backoff 2 s → 20 s. After user `disconnect`, stay idle.
- One BLE conversation at a time (an async mutex). Profile flashing holds it
  for the whole sequence and is exposed as `operation`.
- Vibration auto-stops after 30 s.
- A custom profile is indistinguishable on the device from its base preset,
  so the session persists `{deviceId → CustomProfile}` (JSON in the app data
  dir) and reports `customProfile` only while the device reports the base mode.

#### Stick limit

`session::limit` holds the decision logic as a pure state machine over a
persisted `LimitRecord` (settings, sticks used, window start, cooldown end,
whether the limit set the lock, last lifetime-counter total). It takes the
time and what the device reports (lock state, activity) and returns a step:
lock, re-lock, unlock, or nothing. It reads no clock and does no I/O.

The session drives it from one task per connection, woken by every snapshot
change and at least every 5 s:

- A stick is counted when the activity goes from standby/charging to
  preheating/heating. Once per connection, when not heating, diagnostics
  record 12 is read and the growth of its total since the last connection is
  added (first reading is only a baseline). This part is unverified on
  hardware; an empty reply is ignored.
- At the limit the lock is sent only once the device is back at rest. During
  a cooldown an `unlocked` report triggers a re-lock, `set_lock(false)` is
  refused with `cooldownActive`, and settings may only become stricter.
- Expiry sends `UNLOCK` only for a lock the limit set, then resets the
  counter. If the device is away, that happens on the next connection.
- Time comes from an injectable wall clock (`Clock`), checked on each tick
  rather than slept on, so a cooldown ends on time after the computer
  slept. A cooldown is never longer than its configured length from "now",
  which bounds the effect of the clock being set back.

The record is stored per device id in `session.json` and published as
`Snapshot.limit`; without a connection it shows the last device's.

The protocol is unauthenticated, so this is a self-control aid: another app
can unlock the device whenever this one is not connected.

The session publishes a `Snapshot` on every change through a
`tokio::sync::watch`, and `ActivityEvent`s through a `broadcast` channel
(bounded history of 100 for late subscribers).

### Logs

`C1` / `C2` always return the **oldest** unacknowledged log; the queue only
advances when logs are deleted with `C4`, which is destructive. The app
reads, never deletes. `C4`, `A1` (disables Bluetooth) and `FF` (firmware
update mode) are refused by `send_raw` unless `confirmDangerous` is set.

## app/src-tauri

Thin adapter only: `#[tauri::command]` functions call `ploom-core`, convert
`ploom_core::Error` into `CommandError { code, detail }`, and forward the
session's watch/broadcast channels to the `ploom://snapshot` and
`ploom://activity` events. No protocol logic lives here.

| file | contents |
|------|----------|
| `src/lib.rs` | builder, `setup` (starts the session inside Tauri's Tokio runtime, `auto_connect: true`, data in the app data dir), clean disconnect on exit |
| `src/commands.rs` | one command per entry of `Commands` in `types.ts` |
| `src/events.rs` | snapshot and activity forwarding; on broadcast lag the missed events are replayed from the session history (deduplicated by id) |
| `src/state.rs` | the managed `AppState` holding the one `SessionHandle` |

- **Window.** `tauri.conf.json` describes a plain dark window with a solid
  background for every platform. `tauri.macos.conf.json` replaces it on macOS
  with an overlay title bar (hidden title, traffic lights inset to the UI's
  52 px toolbar), a transparent webview and `sidebar` window vibrancy, which
  shows through the UI's translucent sidebar (`:root[data-vibrancy]` in
  `app.css`, set in `main.ts` only inside Tauri on macOS). Transparency on
  macOS requires `macOSPrivateApi` (the `macos-private-api` feature); this
  rules out the Mac App Store, which is not a distribution target.
- **Bluetooth permission.** `Info.plist` adds `NSBluetoothAlwaysUsageDescription`;
  without it macOS terminates the app on the first Bluetooth access. Tauri
  merges it into the bundle and embeds it in the binary for `tauri dev`.
- **Capabilities.** `capabilities/default.json` grants only
  `core:event:default` (listen/unlisten) and
  `core:window:allow-start-dragging` (for `data-tauri-drag-region`). App
  commands need no permission entries.
- **CSP.** Production allows only bundled scripts and styles (`'self'`, no
  inline) and IPC. The dev CSP adds `'unsafe-inline'` styles and the Vite HMR
  websocket; `dangerousDisableAssetCspModification: ["style-src"]` stops
  Tauri from adding style nonces, which would disable that `'unsafe-inline'`
  in dev and is not needed in production (the built HTML has no inline
  styles).
- **CLI location.** The Tauri CLI only looks for `src-tauri` below the
  working directory, so `pnpm tauri` in `app/ui` runs `scripts/tauri.js`,
  which sets `TAURI_APP_PATH=../src-tauri`.

## app/ui

Svelte 5 (runes) + TypeScript, strict mode. All backend access goes through
`src/lib/api/`, a typed client over Tauri IPC generated from the command
table in `types.ts`.

Until a device is connected (`snapshot.device` is null) the window shows only
the connect screen (`features/gate`). A link that drops later keeps the
device in the snapshot, so the shell stays and shows its reconnecting state.
