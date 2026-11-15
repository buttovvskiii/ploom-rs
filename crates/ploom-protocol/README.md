# ploom-protocol

Pure, I/O-free implementation of the Ploom Aura (GEN4L) Bluetooth LE protocol:
frame encoding, typed decoding of every known indication, the windowed status
merge, the hidden diagnostics channel, and the 38-frame heating-profile upload
computed from the device's calibration tables.

No async, no Bluetooth stack, no `serde_json`. `serde` derives are available
behind the optional `serde` feature.

```rust
use ploom_protocol::{Command, Masters, Notification};
use ploom_protocol::heating::{build_profile_frames, CustomProfile, HeatingMode, HeatingProfile};

// Requests
assert_eq!(Command::RequestStatus.frame().to_hex(), "02 e3 00");

// Indications
let n = Notification::parse(&[0x03, 0x3C, 0x55, 0x00]).unwrap();
assert_eq!(n, Notification::BatteryLevel { pct: 85 });

// A custom profile: Intense with a 290 °C × 150 s hold phase
let masters = Masters::new(vec![3000, 3010, 3020, 3030, 2500, 3050, 3060, 1200,
                                1190, 3090, 3100, 3110, 3120, 3130, 3140, 3150]).unwrap();
let custom = CustomProfile::new(HeatingMode::Intense, 290, 150).unwrap();
let frames = build_profile_frames(&HeatingProfile::custom(&custom), &masters).unwrap();
assert_eq!(frames.len(), 38);
```

## Golden tests

`tests/golden.rs` compares this crate byte for byte against vectors produced by
the original reference client, which was developed against a real
device. The vectors are committed in `tests/fixtures/`; to regenerate them,
see the docstring of `tests/fixtures/generate.py`
(`PLOOM_REFERENCE=/path/to/reference python generate.py`).
