# ploom-cli

`ploom`, a command-line controller for the Ploom Aura (GEN4L) over Bluetooth LE.

```console
$ ploom scan                      # nearby devices, no connection
$ ploom status                    # full status
$ ploom battery
$ ploom watch                     # every frame and event until Ctrl-C
$ ploom buzz 5                    # find the device
$ ploom lock | unlock
$ ploom autostart on|off
$ ploom mode                      # current heating mode
$ ploom mode intense --dry        # print the 38 frames, send nothing
$ ploom mode saver --yes          # flash a preset without prompting
$ ploom custom --base intense --temp 290 --secs 150
$ ploom logs                      # oldest logs, never deleted
$ ploom diag                      # hidden sensors and lifetime stats
$ ploom raw 02e300                # dangerous opcodes need --force
```

Global options: `--device <ID>` (platform id from `ploom scan`; default: first
Ploom found), `--json` for machine-readable output, `--data-dir <DIR>` (or
`PLOOM_DATA_DIR`) for where custom profiles are remembered. Logging goes to
stderr and is controlled with `RUST_LOG` (e.g. `RUST_LOG=ploom_core=debug`).

On macOS the terminal needs Bluetooth permission. The device accepts a single
connection, so close the official app first.
