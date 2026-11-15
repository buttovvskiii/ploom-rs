# Contributing

Thanks for your interest. Bug reports, protocol findings and pull requests
are welcome. For anything larger than a small fix, please open an issue
first so we can agree on the approach.

Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) before changing code: it
explains where things belong and the rules that are easy to get wrong.

## Setup

- Rust stable (the toolchain is pinned to `stable` with rustfmt and clippy in
  `rust-toolchain.toml`)
- Node.js 22 and pnpm 10
- Linux: `sudo apt install libdbus-1-dev pkg-config` plus the
  [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#linux)

```sh
cd app/ui
pnpm install
pnpm tauri dev    # the desktop app with hot reload; connects to a real device
```

The UI needs the desktop shell and a device. The Rust side does not:
`ploom-core` is tested against an in-memory simulated device
(`ploom_core::transport::fake`, behind the `test-util` feature).

## Checks

CI runs all of these; please run them before opening a pull request.

Rust, from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

UI, from `app/ui`:

```sh
pnpm check && pnpm lint && pnpm build
```

`pnpm format` fixes formatting. To check that the desktop shell still
compiles and bundles: `pnpm tauri build --debug --bundles app`.

## Rules

- **The IPC contract changes in one commit.** `app/ui/src/lib/api/types.ts`
  is the source of truth for everything that crosses IPC, and
  `crates/ploom-core/src/model.rs` mirrors it field by field. If you change
  one, change the other in the same commit.
- **No protocol logic outside `ploom-protocol`.** `ploom-core` does I/O and
  state; `app/src-tauri` only maps commands and events.
- **No user-facing prose from the backend.** Send codes and enums; the UI
  localizes them (English and Russian).
- **Never delete device data by default.** Destructive opcodes stay behind an
  explicit confirmation.
- **Test what you can without a device.** Protocol changes need unit tests,
  and session behaviour needs a test against the fake transport.

## Golden fixtures

`crates/ploom-protocol/tests/golden.rs` compares the crate byte for byte with
vectors produced by the original reference client. The vectors are
committed in `crates/ploom-protocol/tests/fixtures/*.json`, so the tests never
need Python.

To regenerate them you need the reference client (not part of this
repository) and a Python interpreter with `bleak` installed:

```sh
PLOOM_REFERENCE=/path/to/reference \
  /path/to/reference/.venv/bin/python crates/ploom-protocol/tests/fixtures/generate.py
```

Only regenerate when the reference itself changed, and review the JSON diff:
a changed byte in a profile frame is a behaviour change.

## Protocol findings

New findings are welcome in [docs/PROTOCOL.md](docs/PROTOCOL.md). Please say
how you verified them (traffic capture, app code, experiment) and never
include identifiers of your own device: serial numbers, Bluetooth addresses
or peripheral UUIDs.

## Commits and pull requests

- Keep commits focused, with a short imperative subject line.
- Add a line to [CHANGELOG.md](CHANGELOG.md) under *Unreleased* for
  user-visible changes.
- Releases are built by `.github/workflows/release.yml` when a `v*` tag is
  pushed; it attaches the installers to a draft release.
- By contributing you agree that your work is dual licensed under MIT and
  Apache-2.0, like the rest of the project.
