# Ploom Aura (GEN4L) Bluetooth LE Protocol

> Unofficial, reverse-engineered documentation. This project is not affiliated with,
> endorsed by or supported by JTI.

This document describes the wire protocol spoken between the official Ploom app and the
Ploom Aura heated-tobacco device (internal type `gen4l`; the Ploom cube is `gen4c`) over
Bluetooth Low Energy. It was reconstructed from live traffic captures and from the
official web app's published source map, which preserves the original Dart class and
method names. Those names are quoted in `code` throughout so that the behaviour can be
cross-checked against the app.

Writing to the device can change its settings, erase its logs, turn its radio off or
reprogram its heater. Read [Dangerous operations](#11-dangerous-operations) before
sending anything other than the read-only requests.

## Contents

1. [Overview](#1-overview)
2. [GATT layout](#2-gatt-layout)
3. [Framing](#3-framing)
4. [Opcode reference](#4-opcode-reference)
5. [Status and telemetry](#5-status-and-telemetry)
6. [Settings: lock, auto-start, vibration](#6-settings-lock-auto-start-vibration)
7. [Heating profiles](#7-heating-profiles)
8. [Logs](#8-logs)
9. [Diagnostics channel (`0xF1` / `0xF2`)](#9-diagnostics-channel-0xf1--0xf2)
10. [Charging detection](#10-charging-detection)
11. [Dangerous operations](#11-dangerous-operations)
12. [Quirks and timing](#12-quirks-and-timing)
13. [Official app behaviour](#13-official-app-behaviour)
14. [Where this is implemented](#14-where-this-is-implemented)

---

## 1. Overview

- **No pairing, no encryption, no authentication.** After an ordinary BLE connection a
  client can write commands immediately. There is no handshake, challenge-response or
  session key, neither in the traffic nor in the app's code. Any BLE stack (desktop,
  mobile, microcontroller) can speak the protocol.
- **One connection at a time.** The device accepts a single central. The official app,
  a browser tab and a custom client compete for it.
- **Request/indication model.** Requests are written to a TX characteristic; replies
  and spontaneous telemetry arrive as *indications* on an RX characteristic. Most
  requests have exactly one reply opcode that proves they were received.
- **Advertising name.** The device advertises as `Ploom aura <serial>`, with a
  lowercase "a" in "aura". Filter on the `Ploom aura` prefix or on the primary service
  UUID below.

## 2. GATT layout

### Ploom service

| UUID | Properties | Role |
|---|---|---|
| `53654010-a391-4a65-83fa-bc58084aca28` | service | Primary Ploom service |
| `53654011-a391-4a65-83fa-bc58084aca28` | write | TX: commands to the device |
| `53654012-a391-4a65-83fa-bc58084aca28` | indicate | RX: replies and telemetry |

- **TX must be written with response.** The characteristic declares only `write`, not
  `write-without-response`. CoreBluetooth (macOS/iOS) silently drops a
  write-without-response on it: no error and no reply. The typical symptom is that
  some commands seem to work (those whose reply the device would send anyway) while
  others are ignored.
- **RX uses indications, not notifications.** Some stacks treat both the same; on
  others (for example NimBLE) the subscription must explicitly enable indications.

### Device Information service (`0x180A`)

All characteristics are read-only strings.

| UUID | Content |
|---|---|
| `0x2A24` | Model Number |
| `0x2A25` | Serial Number |
| `0x2A27` | Hardware Revision |
| `0x2A28` | Software (firmware) Revision, e.g. `M1.0014_G4` |
| `0x2A29` | Manufacturer Name |

### SUOTA service (`0xFEF5`)

The Dialog/Renesas SUOTA (software update over the air) service, with ten
characteristics; its presence indicates a Dialog/Renesas DA14xxx SoC. **Do not use it.**
It is the firmware flasher, and a mistake here can brick the device.

The only thing the official app does through it in normal operation is a reboot
(`rebootDevice`): writing `00 00 00 fd` to characteristic
`8082caa8-41a6-4021-91c6-56f9b954cc34`. This is the standard SUOTA reboot command.

## 3. Framing

Every frame, in both directions, has the same layout
(`FlutterWebBluetoothClientImpl._buildCommandPayload`):

```
[LEN] [OPCODE] [payload ...]

LEN = 1 + len(payload)    number of bytes after LEN; LEN does not count itself
```

Most requests are three bytes, `02 <opcode> <arg>`, where `<arg>` is `00` for a plain
request or `00`/`01` for an on/off setting. Multi-byte integers are **little-endian**
everywhere.

In the field descriptions below, `s[i]` is the payload byte at offset `i`, that is,
frame byte `2 + i` (after `LEN` and `OPCODE`). `s[a..b]` is the half-open byte range
`a` to `b - 1`.

The app uses two primitives:

| Primitive | Behaviour |
|---|---|
| `sendCommand(op, payload, len)` | Write the frame; any reply is handled by the shared indication stream |
| `sendCommandWithResponse(op, payload, len, expectedOp)` | Write the frame and wait for an indication with a specific opcode |

The app's receiver (`BtReadCommand.assertDataIntegrity`) rejects a frame unless
`data[0]` equals the expected `LEN` and `data[1]` the expected opcode. Do **not** copy
this check: on firmware `M1.0014_G4` two replies are longer than the app expects
(`0x3B` arrives with `LEN` 19 instead of 17, `0x3E` with 8 instead of 5). Read fields
by offset and ignore surplus bytes.

## 4. Opcode reference

### 4.1 Requests (app to device)

Names are the identifiers from `BluetoothDeviceDataSource` and `bt_write_*.dart`.

| Frame | App name | Reply | Purpose |
|---|---|---|---|
| `02 e3 00` | `requestDeviceStatusNotificationRetransmission` | `0x30`, `0x33`, `0x9F`, `0x3C` | Resend the full status |
| `02 fd 00` | `requestRssi` | `0xFC` | RSSI as measured by the device |
| `02 a2 00` | `ReqSettingProfileNumber` | `0x3D` | Is the downloaded profile active? |
| `02 a3 00` | `ReqDownloadProfileNumber` | `0x3E` | Which profile is downloaded |
| `02 a4 00` / `02 a4 01` | `ReqSendProfileNumber` | `0x3F` | Activate factory / downloaded profile |
| `02 a5 00` | `_getMasters` | `0x44`, `0x45`, `0x46` | Read calibration tables (one part per request) |
| `<LEN> a7 ...` … `<LEN> ba ...` | `_sendSteps` | none | Heating-curve steps (profile upload) |
| `0xBB`–`0xBE`, `0xC7`–`0xC9`, `0xD4`–`0xD8`, `0xFB`, `0xCB`–`0xCE` | `_sendOthers`, `_sendLeafletsAutoStart` | `0x49`, `0x4B` | Profile constants (profile upload) |
| `02 c1 00` | `requestSmokingLogs` | `0x35`, `0x36`…`0x3A` | Oldest unacknowledged session log |
| `02 c2 00` | `requestChargingLogs` | `0x35`, `0x3B`, `0x40` | Oldest unacknowledged charge log |
| `02 c4 00` | `requestLogsDelete` | | **Destructive:** acknowledge (erase) logs |
| `02 f3 00` | `requestLatestErrorLogs` | `0xF4` | Latest error log |
| `04 f1 <type> ff ff` | `requestAdditionalErrorLogs` | `0xF2` | Diagnostics record |
| `02 79 01` / `02 79 00` | `toggleDeviceVibration` | `0x7A` | Buzz and blink on / off (Find My Device) |
| `02 9e 01` / `02 9e 00` | `toggleDeviceLock` | | Lock / unlock the device |
| `0c a0 <p> 01 02 03 01 02 03 00 00 00 00` | `toggleDeviceLockFeature` | | Enable / disable the lock feature |
| `02 ca 01` / `02 ca 00` | `toggleDeviceAutoStartHeating` | | Auto-Start Heating on / off |
| `02 a1 00` | `requestDisableBluetooth` | | **Dangerous:** turn the device's radio off |
| `02 ff 00` | `_startFirmwareUpdate` | `0xFE` | **Dangerous:** enter firmware-update mode |

### 4.2 Replies and notifications (device to app)

`BtReadCommand.values` lists 21 opcodes; the profile-upload acknowledgements,
calibration tables and diagnostics replies are handled elsewhere in the app. `LEN` is
the value the app expects.

| Opcode | LEN | App name | Content |
|---|---|---|---|
| `0x18` | 12 | `DeviceConfigurationInformationDTO` | Configuration ([5.3](#53-0x18-configuration)) |
| `0x30` | 16 | `DeviceStatusConfigurationDTO` | Main status frame ([5.1](#51-0x30-main-status)) |
| `0x31` | 4 | `GetDeviceStatusChangeCommand` | State change, spontaneous ([5.2](#52-0x31-state-change)) |
| `0x33` | 2 | `GetBatteryDegradationCommand` | Battery health, percent = `s[0]` |
| `0x34` | 6 | `GetChargingStatusCommand` | Charger mode = `s[1]`, spontaneous ([10](#10-charging-detection)) |
| `0x35` | 5 | `UnacquiredLogsNumberNotification` | Unacknowledged log counts ([8](#8-logs)) |
| `0x36` | 19 | `SmokingLogTransmission1` | Session log, part 1 |
| `0x37` | 19 | `SmokingLogTransmission2` | Puff timings 1–9 |
| `0x38` | 19 | `SmokingLogTransmission3` | Puff timings 10–18 |
| `0x39` | 15 | `SmokingLogTransmission4` | Puff timings 19–25 |
| `0x3A` | 19 | `SmokingLogTransmission5` | Session energy |
| `0x3B` | 17 | `ChargeLogTransmission1` | Charge log, part 1 (may arrive with `LEN` 19) |
| `0x3C` | 3 | `GetBatteryLevelCommand` | Battery level, percent = `s[0]` |
| `0x3D` | 2 | `GetSettingProfile` | Downloaded-profile-active flag = `s[0]` ([7.1](#71-modes-and-profile-numbers)) |
| `0x3E` | 5 | (downloaded profile) | Downloaded profile number = `s[1]` (may arrive with `LEN` 8) |
| `0x3F` | | | Ack of `0xA4`; `s[0]` = status, `0` = success |
| `0x40` | 15 | `ChargeLogTransmission2` | Charge log, part 2 |
| `0x44` `0x45` `0x46` | | | Calibration table, parts 1–3 ([7.3](#73-calibration-tables-masters)) |
| `0x49` | | | Ack of `0xCC`; `s[0]` = status, `0` = success |
| `0x4B` | | | Ack of `0xCE`; `s[0]` = status, `0` = success |
| `0x50` | 10 | `ErrorLogTransmission` | Error log record (undecoded) |
| `0x7A` | 2 | `GetVibrationStatus` | Vibration, **inverted**: `s[0] == 0` means buzzing |
| `0x9F` | 12 | `GetLockPatternNotification` | Lock status = `s[0]` ([6.1](#61-lock)) |
| `0xF2` | | | Diagnostics record ([9](#9-diagnostics-channel-0xf1--0xf2)) |
| `0xF4` | 10 | `SendLatestErrorLog` | Latest error log record (undecoded) |
| `0xFC` | 2 | `GetDeviceRSSI` | RSSI in dBm, **signed** `i8` = `s[0]` |
| `0xFE` | | | Firmware-update mode entered (reply to `0xFF`) |

---

## 5. Status and telemetry

`02 e3 00` asks the device to resend its status. It answers with a burst of `0x30`,
`0x33`, `0x9F` and `0x3C`; treat `0x3C` (battery level) as the reply that proves the
request arrived. `0x31` and `0x34` are sent spontaneously and never need polling.

Other single-value readings:

| Request | Reply | Decoding |
|---|---|---|
| `02 e3 00` | `0x3C` | Battery level, percent = `s[0]` |
| `02 e3 00` | `0x33` | Battery health (degradation), percent = `s[0]` |
| `02 fd 00` | `0xFC` | RSSI, dBm = `s[0]` as signed `i8` |

### 5.1 `0x30` main status

```
s[0..2]  device counter, u16 LE; increases by one per second
         (not decoded by the app)
s[4]     flags:
           bit 3 (0x08)  temperature error
           bit 4 (0x10)  charging (see note)
           bit 5 (0x20)  in use (session in progress)
           bit 6 (0x40)  preheating
s[5]     lock status: 0x00 locked, 0x01 unlocked, 0x11 lock feature off
s[6]     shake/tap-to-wake: 0 off, 1 shakeOn, 2 tapOn, 3 shakeOrTapOn, 4 disabled
s[11]    Auto-Start Heating: 0 off, 1 on
s[12]    end session by puff count: 0 off, 1 on
s[13]    profile source: 0 factory, 1 downloaded, 2 p2p
s[14]    last stick: 0 allowed, 1 not allowed (battery too low for another stick)
```

The firmware keeps the charging bit (`s[4]` bit 4) **clear while charging**. Use it only
to confirm charging, never to conclude that the device is not charging. See
[Charging detection](#10-charging-detection).

Example (idle, locked, auto-start on, downloaded profile active):

```
10 30 43 14 00 00 00 00 00 00 ff 02 08 01 00 01 00

s[0..2] = 43 14   counter 0x1443
s[4]    = 00      no flags set (idle)
s[5]    = 00      locked
s[6]    = 00      shake/tap off
s[11]   = 01      Auto-Start Heating on
s[12]   = 00      end-by-puff off
s[13]   = 01      downloaded profile
s[14]   = 00      last stick allowed
```

### 5.2 `0x31` state change

Sent spontaneously by the device whenever its state changes; this is the event to hook
for notifications. `s[0]` is a bitmask. The app takes `log2(s[0])`, so when several bits
are set the **highest set bit** wins.

| `s[0]` | State |
|---|---|
| `0x00` | standby |
| `0x01` | automaticHeating |
| `0x02` | preheating |
| `0x04` | smoking (session in progress) |
| `0x08` | charging |
| `0x10` | temperatureError |
| `0x20` | alarmError |
| `0x40` | standby |

Example: `04 31 01 00 00` followed about 3 s later by `04 31 00 01 00` is
automaticHeating then standby: Auto-Start began to trigger and then backed off.

A `0x31` is often fresher than the last `0x30`. When folding several indications into
one status, expand a state change into the equivalent `0x30` flags (smoking implies in
use and not preheating; standby clears preheating, in use and charging; and so on) and
apply indications in arrival order, newest wins. Otherwise a stale "idle" `0x30` can
override a newer "smoking" `0x31`.

### 5.3 `0x18` configuration

The same settings as `0x30`, at different offsets:

```
s[0]   bit 3 (0x08): temperature error
s[1]   lock status (same values as 0x30 s[5])
s[2]   shake/tap-to-wake (same values as 0x30 s[6])
s[7]   Auto-Start Heating
s[8]   end session by puff count
s[9]   profile source
s[10]  last stick: 0 allowed, 1 not allowed
```

### 5.4 Derived device state

What the app ultimately shows (`DeviceManager.updateBtDevice`) and where each field
comes from:

| Field | Source |
|---|---|
| `autoStartHeating` | `0x30` `s[11]`, `0x18` `s[7]` |
| `batteryDegradation` | `0x33` |
| `batteryLevel` | `0x3C` |
| `chargeMode` | `0x34` |
| `currentHeatingMode` | `0x3D` + `0x3E` (see [7.1](#71-modes-and-profile-numbers)) |
| `isCharging` | `0x30` `s[4]` bit 4, `0x31`, `0x34` |
| `isConnected` | GATT connection state |
| `isInUse` | `0x30` `s[4]` bit 5, `0x31` |
| `isLockFeatureOn` | `0x30` `s[5] != 0x11`, `0x9F` |
| `isLocked` | `0x30` `s[5] == 0x00`, `0x9F` |
| `isPreheating` | `0x30` `s[4]` bit 6, `0x31` |
| `isTemperatureError` | `0x30` `s[4]` bit 3, `0x31` |
| `isVibrating` | `0x7A` |
| `rssi` | `0xFC` |

---

## 6. Settings: lock, auto-start, vibration

These requests change persistent device settings (except vibration). The app sends them
with `sendCommand` and does not wait for a specific reply; read the new value back from
the next `0x30` / `0x9F`.

### 6.1 Lock

| Frame | Effect |
|---|---|
| `02 9e 01` | Lock the device |
| `02 9e 00` | Unlock the device |
| `0c a0 <p> 01 02 03 01 02 03 00 00 00 00` | Enable (`<p>` = `01`) or disable (`<p>` = `00`) the lock feature itself |

The ten trailing bytes of the `0xA0` frame are the constant `ReqDeviceLockFeature`
(the unlock pattern), sent unchanged by the app.

Lock status, in `0x9F` `s[0]`, `0x30` `s[5]` and `0x18` `s[1]`:

| Value | Meaning |
|---|---|
| `0x00` | Locked |
| `0x01` | Unlocked |
| `0x11` | Lock feature turned off |

### 6.2 Auto-Start Heating

`02 ca 01` turns Auto-Start Heating on, `02 ca 00` turns it off. The current value is
`0x30` `s[11]` / `0x18` `s[7]`.

### 6.3 Vibration (Find My Device)

`02 79 01` starts buzzing and blinking, `02 79 00` stops. The device answers with
`0x7A`, whose value is **inverted**: `s[0] == 0` means the device is vibrating.

---

## 7. Heating profiles

### 7.1 Modes and profile numbers

The device holds two profile slots: the built-in **factory** profile and one
**downloaded** profile. Three requests describe and switch them:

| Request | Reply | Meaning |
|---|---|---|
| `02 a2 00` | `0x3D` | `s[0]` is a **flag**: `0` = factory profile active, `1` = downloaded profile active |
| `02 a3 00` | `0x3E` | `s[1]` = number of the downloaded profile |
| `02 a4 00` / `02 a4 01` | `0x3F` | Activate the factory / downloaded profile |

`0x3D` is not a mode number. The effective heating mode is:

```
if 0x3D s[0] == 0:   Balanced (factory profile)
else:                mode_from_number(0x3E s[1])
```

If `0x3D` has not been received yet, the profile source in `0x30` `s[13]` can stand in
for it (`1` downloaded or `2` p2p means the downloaded slot is active).

Profile number to mode (`ResponseHeatingModeDTOMapper.toDomain`); every number other
than 1, 2 and 3 maps to Balanced:

| Number | Mode | Sticks per charge | Session | Heat level |
|---|---|---|---|---|
| `0` (and any other) | Balanced | 20 | 5 min | 2 |
| `1` | Delicate | 19 | 6 min | 1 |
| `2` | Intense | 25 | 3 min | 3 |
| `3` | Battery Saver | 27 | 3 min | 2 |

Activating a slot does not erase the other. `02 a4 00` switches to the factory profile
while the downloaded profile stays stored (and `0x3E` keeps reporting it);
`02 a4 01` switches back to it.

### 7.2 Selecting a mode

- **Balanced** is the factory profile. Selecting it is a single frame:
  `02 a4 00`, wait for `0x3F`. (In the app, Balanced is the constant `B.bb4`: profile
  number 0, all fields zero and no curve steps, which takes this short path.)
- **Delicate, Intense, Battery Saver** are not stored on the device. Selecting one
  uploads a complete profile (38 frames, [7.5](#75-upload-sequence)) into the downloaded
  slot and activates it.

The curves and constants come from the app bundle, not from a server:
`HeatingModeProfileMapper.toDTO` is a plain `switch` returning one of the constants
`B.bb1` … `B.bb4`. The curve temperatures, however, must be converted to heater
resistance using calibration values read from the individual device, so the upload
frames cannot be precomputed; they have to be built on the spot.

### 7.3 Calibration tables (masters)

Send `02 a5 00` three times. The device answers with `0x44`, `0x45` and `0x46`, one
per request. Each payload is a run of little-endian `u16` values (a trailing odd byte
is ignored); concatenated in that order they form the `masters` table.

- At least 16 values are required (`getRes` reads up to `masters[15]`).
- `masters[8]` must be non-zero (`getRes` divides by it).
- `masters[7]` (the "calibration key") selects the `0xCB` leaflet constant
  ([7.7](#77-leaflet-and-auto-start-block)); outside the supported range the app
  refuses to upload a profile.

#### `getRes`: temperature to resistance

`BTUtils.getRes(t, masters)` converts a curve temperature `t` (in °C) to a heater
resistance value. With `m[i] = masters[i]` and `r = |t|`:

```
if t == 0 or t == 1000:  return t                      (passed through unchanged)

if r == 245:  res = m[3]
if r == 250:  res = m[2]
if r == 275:  res = m[1]
if r == 320:  res = m[14]
otherwise:
    p    = ((m[11] * (320 - r)) / 75 + (m[15] * (r - 245)) / 75) / 1000
    base = 306 if r == 300 else r
    res  = round(m[3] + p * (m[7] / m[8]) * (base - m[4] / 10))

return -res if t < 0 else res
```

- The products `m[11] * (320 - r)` and `m[15] * (r - 245)` are integers; the divisions
  are floating point.
- `round` is Dart's `round()`: halves round **away from zero** (2.5 to 3, -2.5 to -3),
  not to even.
- A negative temperature is a device flag; its sign is carried over to the result.

### 7.4 Curve step frames (`0xA7` … `0xBA`)

A profile always has exactly 20 step frames, opcodes `0xA7` + *i* for *i* = 0…19
(`0xA7` … `0xBA`). Curves shorter than 20 steps are padded with all-zero steps
(duration 0, temperature 0, gradient 0).

Step payload:

```
[0x01]        first step (i == 0) only
[res]         u16 LE   getRes(temp, masters), truncated to 16 bits
[duration]    u8       step length in seconds (one byte: max 255 s)
[temp]        i16 LE   target temperature, °C
[gradient]    i32 LE   trunc(gradient * 1000)
[0xFF]        terminator
```

The first step frame is 13 bytes long (`LEN` = `0x0c`), every other step frame 12 bytes
(`LEN` = `0x0b`).

- **Duration is a single byte**, so no step can be longer than 255 s. This also caps
  the hold phase of any custom profile at 255 s.
- **Gradient** `1000` means "no slope" (a flat step); it is sent as
  `1000 * 1000 = 1000000` = `40 42 0f 00`. Other gradients are multiplied by 1000 and
  **truncated toward zero** (Dart `toInt()`), then sent as a two's-complement `i32`.
- **Temperature** `0` turns the heater off. A **negative** temperature is a device flag;
  both `temp` and `res` keep the sign (two's complement).

Example: the first Intense step (60 s, 320 °C, no slope), with `R` = `masters[14]`:

```
0c a7 01 RR RR 3c 40 01 40 42 0f 00 ff
      |  |---| |  |---| |---------| |
      |  res   |  temp  gradient    terminator
      first    duration = 60
```

#### Preset curves

Each step is `(duration s, temperature °C, gradient)`:

| Mode | Steps |
|---|---|
| Delicate | (60, 300, 1000) (19, 300, 1000) (15, 300, −0.013) (15, −220, 1000) (9, 221, 1000) (231, 240, 0.009) (75, 250, 0.009) (15, 0, 0.025) |
| Intense | (60, 320, 1000) (23, 320, 1000) (20, 320, −0.01) (11, −255, 1000) (6, 256, 1000) (143, 275, 0.009) |
| Battery Saver | (60, 320, 1000) (18, 320, 1000) (20, 320, −0.012) (14, −245, 1000) (7, 245, 1000) (4, 245, 0.01) (110, 240, 0.009) (25, 255, 0.009) |

Balanced has no curve: it is the factory profile and its curve is not published.

### 7.5 Upload sequence

The app's sequence (`FirmwareVersionEight.setHeatingProfile`). Everything after the
calibration read is computed up front, so an unusable calibration table aborts before
the device is touched. The app waits 10 ms between profile frames
(`FirmwareVersionEight._delayBle`).

```
0. Calibration     02 a5 00 -> 0x44,  02 a5 00 -> 0x45,  02 a5 00 -> 0x46

1. Curve steps     A7 A8 ... BA                       20 frames, no reply
2. Constants       BB BC BD BE C7 C8 C9 D4 D5 D6      10 frames, no reply
3. Leaflet/auto    D7 D8 FB CB                        no reply
                   CC -> wait 0x49
                   CD
                   CE -> wait 0x4B
4. Activate        02 a4 01 -> wait 0x3F

5. Verify          02 a2 00 -> 0x3D   (s[0] should be 1)
                   02 a3 00 -> 0x3E   (s[1] should equal the profile number)
```

Steps 1–4 are 38 frames. The acknowledgements `0x49`, `0x4B` and `0x3F` carry a status
in `s[0]`; `0` means success.

### 7.6 Profile constants (`0xBB` … `0xD6`)

All multi-byte values are little-endian. Where the original field names are unknown,
the letters in parentheses are the field names in the minified app.

| Frame | Payload |
|---|---|
| `0xBB` | `[number] 00 0f [steps] [d] 00`, then `u16` 2215 (`e`), `u16` 2176 (`f`) |
| `0xBC` | `u32` 100 (`r`), `u32` 10 (`w`), `u32` 10 (`x`) |
| `0xBD` | `u32` 100 (`y`), `u32` 10 (`z`) |
| `0xBE` | `u16` 0, 0, 0, 150 (`ax`), 140 (`ay`), 117 (`ch`) |
| `0xC7` | 6 × `u16`: voltage table |
| `0xC8` | 6 × `u16`: 64, 65, 68, 64, 65, 68 |
| `0xC9` | 9 × `u16`: thresholds `k0 k1 k2 k0 k1 k2`, then 70, 75, 85 |
| `0xD4` | 10 × `u8`: the same byte repeated |
| `0xD5` | 5 × `u16`: the same word repeated |
| `0xD6` | 5 × `u16`: the same word repeated (as `0xD5`) |

Per-profile values:

| Field | Delicate | Intense | Battery Saver |
|---|---|---|---|
| Profile number (`0xBB` byte 0) | 1 | 2 | 3 |
| Step count (`0xBB` byte 3) | 8 | 6 | 8 |
| `d` (`0xBB` byte 4) | 19 | 23 | 18 |
| Voltages (`0xC7`) | 3390 3435 3520 4775 4730 4593 | 3425 3470 3555 4810 4765 4628 | 3390 3435 3520 4775 4730 4593 |
| Thresholds `k0 k1 k2` (`0xC9`) | 3340 3315 3300 | 3375 3350 3335 | 3340 3315 3300 |
| `0xD4` byte | 24 | 29 | 24 |
| `0xD5` / `0xD6` word | 20000 | 23100 | 22100 |

### 7.7 Leaflet and auto-start block

Fixed payloads from the app (decimal byte values):

| Frame | App name | Payload |
|---|---|---|
| `0xD7` | `ReqLeaflet1` | `[0] × 10` |
| `0xD8` | `ReqLeaflet2` | `[0] × 10` |
| `0xFB` | | `[1]` if `profile.number == 8`, else `[0]` |
| `0xCB` | `ReqAutoStartCommand1` | `[1, 92, 3, 80, 55, 8, 8, 70, <x>, 165, 13, 219, 26, 60, 100, 0, 10]` |
| `0xCC` | `ReqAutoStartCommand2` | `[1, 20, 5, 100, 0, 0, 10, 238, 2, 144, 31, 238, 2, 144, 31, 0, 0]` |
| `0xCD` | `ReqAutoStartCommand3` | `[1, 232, 3, 0, 0, 45, 7, 232, 3, 25, 1, 223, 0, 222, 3]` |
| `0xCE` | `ReqAutoStartCommand4` | `[1, 160, 15, 208, 7, 62, 168, 11]` |

`0xFB` compares the **profile's** number field, not the firmware version. No preset has
number 8, so the frame is `02 fb 00` for every preset.

The `<x>` byte of `0xCB` is chosen from `masters[7]` (`_sendLeafletsAutoStart`). The
range boundaries are uneven, exactly as in the app:

| `masters[7]` | `<x>` |
|---|---|
| 1294 – 1330 | 60 |
| 1269 – 1293 | 62 |
| 1245 – 1268 | 64 |
| 1195 – 1244 | 65 |
| 1171 – 1194 | 66 |
| 1146 – 1170 | 68 |
| 1122 – 1145 | 70 |
| anything else | upload refused |

The app's default for `<x>` is 70.

---

## 8. Logs

The device keeps a queue of session ("smoking") logs and charge logs that have not yet
been acknowledged.

| Request | Replies |
|---|---|
| `02 c1 00` | `0x35`, then `0x36`, `0x37`, `0x38`, `0x39`, `0x3A` (one session) |
| `02 c2 00` | `0x35`, then `0x3B`, `0x40` (one charge) |
| `02 c4 00` | none; **erases** the acknowledged logs |

**`0xC1` and `0xC2` always return the oldest unacknowledged log.** Repeating the request
returns the same record again; the queue only advances when logs are deleted with
`0xC4`. Reading the full history therefore requires `0xC4`, which is destructive: save
every record before sending it, and send it only deliberately.

### 8.1 `0x35` unacknowledged log counts

```
s[0..2]  u16 LE  number of unacknowledged session logs
s[2..4]  u16 LE  number of unacknowledged charge logs
```

### 8.2 `0x36` session log, part 1

```
s[0..4]    u32 LE  smokingStartTimeByte: device counter at session start
                   (not wall-clock time)
s[4..6]    u16 LE  sessionTime: session length, seconds
s[6]       u8      totalTimesSmoked
s[7]       u8      puffStopEnabled
s[8]       u8      heatingStartCondition
s[9]       u8      isAutoOnEnabled
s[10]      u8      isLockFunctionEnabled
s[11]      u8      unlockMethod
s[12..16]  u32 LE  downloadedProfileNumber
s[16..18]  u16 LE  totalTime
```

### 8.3 `0x37` / `0x38` / `0x39` puff timings

One `u16 LE` per puff: seconds from the start of the session. Zero slots are empty.

| Opcode | Puffs |
|---|---|
| `0x37` | 1–9 |
| `0x38` | 10–18 |
| `0x39` | 19–25 |

### 8.4 `0x3A` session energy

Five `u16 LE` values, raw device units:

```
s[0..2]   amountOfEnergyIn50msInterval
s[2..4]   stpe0SegmentEnergyAmount
s[4..6]   voltageOfAvg0AtTheStartOfSmoking  (mV)
s[6..8]   voltageOfAvg2AtTheStartOfSmoking  (mV)
s[8..10]  amountOfEnergyFor200msInterval
```

#### Example session

```
0x36  13 36 b0 33 00 00 68 01 00 00 00 01 00 01 0e 00 00 00 b0 95
      start counter 13232, length 360 s, auto-start on,
      downloaded-profile field 14, total time 38320

0x37  13 37 0e 00 5d 00 90 00 d0 00 e6 00 09 01 24 01 57 01 00 00
      puffs at 14, 93, 144, 208, 230, 265, 292, 343 s (8 puffs)

0x38, 0x39  all zero: no further puffs

0x3A  13 3a 1b 01 ad 02 a4 01 62 0d 9c 0b 00 ...
      283, 685, 420, 3426, 2972
```

The 360 s length matches the 6-minute Delicate session.

### 8.5 `0x3B` charge log, part 1

```
s[0..4]    u32 LE  device counter when charging started
s[4..8]    u32 LE  device counter when charging ended
s[8..10]   u16 LE  battery voltage at start, mV
s[10..12]  u16 LE  battery voltage at end, mV
s[12..14]  u16 LE  battery temperature at start, raw
s[14..16]  u16 LE  battery temperature at end, raw
```

The frame may arrive with `LEN` 19 instead of 17; ignore the surplus bytes.

Example payload `03 00 00 00 2f 01 00 00 fd 0d 96 0e d0 0b e6 0b`: counter 3 to 303
(a 300-unit charge), 3581 mV to 3734 mV, temperature 3024 to 3046 (raw).

### 8.6 `0x40` charge log, part 2

```
s[0..2]  u16 LE  totalChargeCount: lifetime charge count
s[2..6]  u32 LE  chargingMode, raw
s[6]     u8      soh0To100AtStartOfCharge: battery health at charge start, percent
```

### 8.7 Decoding pitfalls in the official app

The app misreads several fields; decode them as described above instead.

- It reads the 32-bit fields (`smokingStartTimeByte` and `downloadedProfileNumber` in
  `0x36`, the counters in `0x3B`, `chargingMode` in `0x40`) through
  `BTUtils.listOfBitsToByte`, a helper that packs a list of *bits* into a byte
  (`r |= a[i] << (7 - i)`), applied to a list of bytes. The result is garbage. Plain
  little-endian `u32` reads give consistent values.
- It fills `totalChargeCount` with the literal `256` instead of reading `0x40` `s[0..2]`.

### 8.8 Error logs

`02 f3 00` returns the latest error log as `0xF4`; `0x50` (`ErrorLogTransmission`)
carries error log records. Neither layout is decoded here; keep the raw payload.

---

## 9. Diagnostics channel (`0xF1` / `0xF2`)

The app reads a set of sensor records in the background (`requestAdditionalErrorLogs`)
without ever showing them. The values are live readings.

```
request:  04 f1 <type> ff ff
reply:    <LEN> f2 <type> <seq> <data ...>
```

`data` is a run of `u16 LE` fields. (The app's dispatcher,
`AdditionalInfoMapper.mapToErrorDto`, reads some fields big-endian; that is a bug in
the app.)

| Type | Record | Decoding |
|---|---|---|
| `4` | Battery fuel-gauge register | Not decoded |
| `6` | Lifetime puff counters | Not decoded |
| `7` | Raw ADC readings | 7 × `u16`: `Vtemp`, `USBcc1`, `USBcc2`, `HeaterNtc`, `B`, `CaseNtc`, `HeatTemp` |
| `9` | Enclosure temperature | field 0 / 100 = °C; `0` means the sensor is asleep (no reading) |
| `12` | Lifetime heating-start statistics | 7 × `u16`, see below |
| `14`, `15` | Heater resistance, panel temperature | Not decoded |

Type 12 fields, in order: cold start, hot start, button press within the auto-start
window, button press after timeout, button press after unavailable, button press after
timeout or unavailable, automatic start after unavailable.

Practical notes:

- **All types reply with the same opcode.** A late reply to a previous request can
  arrive while waiting for the next one, so check that `s[0]` echoes the requested
  type and treat a mismatch as no answer.
- **The first reply after a quiet period is often all zeros.** Retry until some data
  byte is non-zero.
- **USB-C plug detection.** In the ADC record, `USBcc1` and `USBcc2` are the USB-C CC
  lines. With a charger attached one of them is pulled up to roughly 450; with no
  charger both read about 28. `max(USBcc1, USBcc2) > 100` is an instantaneous and
  reliable "charger connected" signal.

---

## 10. Charging detection

There is no direct "is charging now" request.

- **`0x34` cannot be requested.** The app's `DeviceRepositoryImpl._getChargeStatus` is
  an empty stub. The device sends `0x34` on its own while charging, roughly at every
  1 % step, and when the cable is removed. A client connecting mid-charge may wait one
  to two minutes for the first one. Charger mode is `s[1]`:

  | `s[1]` | Mode |
  |---|---|
  | `0` … `5` | `mode0` … `mode5` |
  | `6` | other |
  | `7` | noConnect |

  The app's rule is `isCharging = (mode != noConnect)`.
- **`0x30` keeps the charging bit clear while charging.** Use `s[4]` bit 4 only to
  confirm charging, never to clear it.
- **`0x31` with `0x08`** reports the transition into charging.
- **No live battery voltage.** Millivolt values exist only in charge logs (`0x3B`),
  which describe a past charge.

A robust strategy, in order of preference:

```
charging = usb_plugged                    (diagnostics type 7, instantaneous)
        or last 0x34 mode != noConnect    (arrives within ~1-2 min)
        or battery % rose by >= 2 since a baseline
```

A rise in battery percentage only happens while charging. The `+2` threshold filters
out the voltage rebound after a heavy session, which can raise the reading by 1 %.

---

## 11. Dangerous operations

Clients should refuse these unless the user explicitly confirms them.

| Frame | App name | Effect |
|---|---|---|
| `02 c4 00` | `requestLogsDelete` | Acknowledges and **permanently erases** logs on the device |
| `02 a1 00` | `requestDisableBluetooth` | Turns the device's Bluetooth off; it stops advertising and can only be re-enabled on the device itself |
| `02 ff 00` | `_startFirmwareUpdate` | Puts the device into firmware-update mode (replies `0xFE`) |

Also avoid the SUOTA service (`0xFEF5`, [section 2](#suota-service-0xfef5)): it is the
firmware flasher.

Uploading a heating profile ([7.5](#75-upload-sequence)) reprograms the heater. Only
send the frames as documented, built from the device's own calibration table.

---

## 12. Quirks and timing

- **Writes must be with response.** See [section 2](#2-gatt-layout).
- **The device swallows writes after subscribing.** For a variable time after enabling
  indications (from a fraction of a second to several seconds), frames are accepted
  but nothing is answered. A fixed delay is unreliable; resend `02 e3 00` until the
  first indication arrives (for example up to 6 attempts, 1.2 s apart).
- **Writes are occasionally swallowed mid-session too.** Poll as request/reply with
  retry rather than "send a batch and listen". Each poll request has its own reply
  opcode; resend on silence:

  | Request | Reply |
  |---|---|
  | `02 e3 00` | `0x3C` |
  | `02 fd 00` | `0xFC` |
  | `02 a2 00` | `0x3D` |
  | `02 a3 00` | `0x3E` |

- **Two `0xE3` requests in a row produce an incomplete burst.** Assemble status from a
  time window of recent indications (for example the last 15 s, folded in arrival
  order) rather than from the reply to a single request.
- **Status requests are ignored while heating.** During preheating and sessions the
  device does not answer polls; do not retry aggressively then.
- **Over-long replies.** `0x3B` and `0x3E` may carry more bytes than the app expects;
  do not validate `LEN` on received frames.
- **Profile upload pacing.** 10 ms between profile frames; wait for `0x49`, `0x4B` and
  `0x3F` where the sequence requires it.
- **Link drops.** Connections have been seen to drop when RSSI falls to around
  −62 dBm. The official app keeps writing into a dead connection without noticing;
  a client should watch for disconnection and reconnect.

---

## 13. Official app behaviour

- Every 60 s the app sends `0xE3`, `0xF3`, `0xC2`, `0xC4`, `0xA2`, `0xA3`: refresh status, fetch the
  latest error, fetch the oldest charge log, **delete logs**, then read both profile
  values. The `0xC4` is what advances the log queue.
- On the Device Radar screen it also sends `02 fd 00` once per second.

A third-party client does not need to delete logs, and needs little polling: `0x31`
and `0x34` arrive on their own, and `0xE3` every few seconds to minutes is enough to keep
the status fresh.

---

## 14. Where this is implemented

The `crates/ploom-protocol` crate implements this document without any I/O:

| Module | Covers |
|---|---|
| `frame` | `[LEN][OPCODE][payload]` encoding and validation ([3](#3-framing)) |
| `opcode` | Named request and reply opcodes, `DANGEROUS_OPCODES` ([4](#4-opcode-reference), [11](#11-dangerous-operations)) |
| `command` | Builders for every request, the poll table ([4.1](#41-requests-app-to-device), [12](#12-quirks-and-timing)) |
| `notification` | Decoders for every reply ([4.2](#42-replies-and-notifications-device-to-app), [5](#5-status-and-telemetry), [8](#8-logs)) |
| `status` | Partial status updates, windowed merge, effective mode ([5](#5-status-and-telemetry), [7.1](#71-modes-and-profile-numbers)) |
| `heating` | Modes, curves, constants, `get_res`, the 38-frame upload ([7](#7-heating-profiles)) |
| `calibration` | Calibration tables and the `0xCB` leaflet constant ([7.3](#73-calibration-tables-masters), [7.7](#77-leaflet-and-auto-start-block)) |
| `diagnostics` | The `0xF1` / `0xF2` channel ([9](#9-diagnostics-channel-0xf1--0xf2)) |
