# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project
uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - Unreleased

First public version.

### Added

- `ploom-protocol`: I/O-free implementation of the Ploom Aura (GEN4L)
  Bluetooth LE protocol: framing, typed decoding of every known indication,
  the windowed status merge, the diagnostics channel, and the 38-frame
  heating-profile upload, and body-colour decoding from the hardware
  string. Golden-tested byte for byte against the reference client.
- `ploom-core`: btleplug transport, device session with polling, automatic
  reconnection, serialized commands, profile flashing with progress, custom
  profile persistence, and a simulated device for tests (`test-util`
  feature).
- `ploom` CLI: scan, status, battery, watch, buzz, lock and unlock,
  auto-start, heating modes, custom profiles, logs, diagnostics and raw
  frames, with JSON output.
- Desktop app (Tauri 2, Svelte 5): live status, heating modes with curves,
  custom profile editor with frame preview, device controls, logs,
  diagnostics, device list, raw console, activity feed, English and Russian
  UI, smooth animated heating curves, a battery dial, native macOS window
  with sidebar vibrancy.
- Interactive 3D model of the device in its detected body colour, reacting
  to heating, charging and connection state.
- Stick limit: after a chosen number of sticks the device is locked for a
  chosen cooldown and unlocked again afterwards. Counts sessions live and
  from the device's lifetime counters, re-locks if unlocked elsewhere,
  survives restarts. In the app (Limit section), `ploom limit`, and
  `ploom-core` (`session::limit`).
- Connect screen: the app shows only a connect screen until a device is
  connected.
- Release builds: macOS universal `.dmg`, Windows NSIS installer and the
  standalone CLI, produced by GitHub Actions on `v*` tags.
- Documentation: protocol reference (`docs/PROTOCOL.md`) and architecture
  notes (`docs/ARCHITECTURE.md`).

[0.1.0]: https://github.com/buttovvskiii/ploom-rs/commits/main
