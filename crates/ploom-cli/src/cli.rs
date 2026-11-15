//! Command-line definition.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use ploom_protocol::HeatingMode;

/// Control a Ploom Aura (GEN4L) over Bluetooth LE.
#[derive(Debug, Parser)]
#[command(name = "ploom", version, about, propagate_version = true)]
pub(crate) struct Cli {
    /// Platform id of the device to use (see `ploom scan`); default: first Ploom found.
    #[arg(long, global = true, value_name = "ID")]
    pub(crate) device: Option<String>,

    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    pub(crate) json: bool,

    /// Where custom profiles and the last device are remembered.
    #[arg(long, global = true, value_name = "DIR", env = "PLOOM_DATA_DIR")]
    pub(crate) data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// List nearby Ploom devices with signal strength (does not connect).
    Scan,
    /// Show the full device status.
    Status,
    /// Show battery level and health.
    Battery,
    /// Stay connected and print every frame and event (Ctrl-C to stop).
    Watch,
    /// Buzz and blink to find the device.
    Buzz {
        /// Seconds to buzz (at most 30; the device is switched off after that anyway).
        #[arg(default_value_t = 3.0, value_parser = parse_secs)]
        secs: f64,
    },
    /// Lock the device.
    Lock,
    /// Unlock the device.
    Unlock,
    /// Turn Auto-Start Heating on or off.
    Autostart {
        /// New setting.
        state: OnOff,
    },
    /// Show the heating mode, or switch to another one.
    Mode(ModeArgs),
    /// Flash a custom profile: a preset with your own hold temperature and length.
    Custom(CustomArgs),
    /// Show or change the stick limit: lock the device for a cooldown after
    /// a number of sticks, then unlock it again.
    ///
    /// The limit is enforced while a session is connected (the desktop app,
    /// or `ploom watch`). It is a self-control aid: the protocol has no
    /// authentication, so another app can unlock the device when this one
    /// is not connected. Settings live in the data directory; point
    /// `--data-dir` at the desktop app's to share them.
    Limit(LimitArgs),
    /// Read the oldest session and charge logs (never deletes them).
    Logs,
    /// Read hidden sensors and lifetime statistics.
    Diag,
    /// Send a raw frame, e.g. `ploom raw 02e300`.
    Raw {
        /// Frame as hex, `LEN OPCODE payload…` (e.g. 02e300). Spaces are allowed.
        #[arg(required = true, num_args = 1..)]
        hex: Vec<String>,
        /// Allow dangerous opcodes (C4 erases logs, A1 disables Bluetooth, FF enters firmware update).
        #[arg(long)]
        force: bool,
    },
}

#[derive(Debug, Args)]
pub(crate) struct ModeArgs {
    /// delicate, balanced, intense or saver. Omit to show the current mode.
    #[arg(value_parser = parse_mode)]
    pub(crate) name: Option<HeatingMode>,
    #[command(flatten)]
    pub(crate) flash: FlashArgs,
}

#[derive(Debug, Args)]
pub(crate) struct LimitArgs {
    /// Sticks allowed before the device is locked (1–40). Turns the limit on.
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..=40))]
    pub(crate) sticks: Option<u32>,
    /// How long the device stays locked, minutes (5–1440). Turns the limit on.
    #[arg(long, value_name = "MIN", value_parser = clap::value_parser!(u32).range(5..=1440))]
    pub(crate) cooldown: Option<u32>,
    /// Hours after which the counter resets by itself (1–168). Turns the limit on.
    #[arg(long, value_name = "H", value_parser = clap::value_parser!(u32).range(1..=168))]
    pub(crate) window: Option<u32>,
    /// Turn the limit off (refused while a cooldown is running).
    #[arg(long, conflicts_with_all = ["sticks", "cooldown", "window"])]
    pub(crate) off: bool,
}

#[derive(Debug, Args)]
pub(crate) struct CustomArgs {
    /// Preset to start from: delicate, intense or saver.
    #[arg(long, value_parser = parse_mode)]
    pub(crate) base: HeatingMode,
    /// Hold-phase temperature in °C (clamped to 240–320).
    #[arg(long, allow_negative_numbers = true)]
    pub(crate) temp: i64,
    /// Hold-phase length in seconds (clamped to 30–255).
    #[arg(long, allow_negative_numbers = true)]
    pub(crate) secs: i64,
    #[command(flatten)]
    pub(crate) flash: FlashArgs,
}

#[derive(Debug, Clone, Copy, Args)]
pub(crate) struct FlashArgs {
    /// Print the frames that would be sent, without sending them.
    #[arg(long)]
    pub(crate) dry: bool,
    /// Do not ask for confirmation before flashing.
    #[arg(long, short = 'y')]
    pub(crate) yes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum OnOff {
    On,
    Off,
}

fn parse_mode(s: &str) -> Result<HeatingMode, String> {
    s.parse()
        .map_err(|e: ploom_protocol::heating::UnknownMode| e.to_string())
}

fn parse_secs(s: &str) -> Result<f64, String> {
    let secs: f64 = s
        .parse()
        .map_err(|_| format!("{s:?} is not a number of seconds"))?;
    if (0.0..=30.0).contains(&secs) {
        Ok(secs)
    } else {
        Err("seconds must be between 0 and 30".into())
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("ploom").chain(args.iter().copied()))
    }

    #[test]
    fn definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn global_flags_anywhere() {
        let cli = parse(&["status", "--json", "--device", "ABC"]).unwrap();
        assert!(cli.json);
        assert_eq!(cli.device.as_deref(), Some("ABC"));
        assert!(matches!(cli.command, Command::Status));
    }

    #[test]
    fn mode_arguments() {
        let cli = parse(&["mode", "saver", "--dry"]).unwrap();
        let Command::Mode(m) = cli.command else {
            panic!()
        };
        assert_eq!(m.name, Some(HeatingMode::BatterySaver));
        assert!(m.flash.dry && !m.flash.yes);
        let Command::Mode(m) = parse(&["mode"]).unwrap().command else {
            panic!()
        };
        assert_eq!(m.name, None);
        assert!(parse(&["mode", "turbo"]).is_err());
    }

    #[test]
    fn limit_arguments() {
        let Command::Limit(l) = parse(&["limit"]).unwrap().command else {
            panic!()
        };
        assert_eq!(
            (l.sticks, l.cooldown, l.window, l.off),
            (None, None, None, false)
        );
        let Command::Limit(l) = parse(&[
            "limit",
            "--sticks",
            "10",
            "--cooldown",
            "120",
            "--window",
            "24",
        ])
        .unwrap()
        .command
        else {
            panic!()
        };
        assert_eq!(
            (l.sticks, l.cooldown, l.window),
            (Some(10), Some(120), Some(24))
        );
        assert!(parse(&["limit", "--off"]).is_ok());
        assert!(parse(&["limit", "--off", "--sticks", "3"]).is_err());
        assert!(parse(&["limit", "--sticks", "0"]).is_err());
        assert!(parse(&["limit", "--cooldown", "4"]).is_err());
        assert!(parse(&["limit", "--window", "200"]).is_err());
    }

    #[test]
    fn custom_arguments() {
        let cli = parse(&[
            "custom", "--base", "intense", "--temp", "290", "--secs", "-5", "-y",
        ])
        .unwrap();
        let Command::Custom(c) = cli.command else {
            panic!()
        };
        assert_eq!(
            (c.base, c.temp, c.secs, c.flash.yes),
            (HeatingMode::Intense, 290, -5, true)
        );
        assert!(parse(&["custom", "--base", "intense", "--temp", "290"]).is_err());
    }

    #[test]
    fn buzz_and_raw() {
        let Command::Buzz { secs } = parse(&["buzz"]).unwrap().command else {
            panic!()
        };
        assert!((secs - 3.0).abs() < f64::EPSILON);
        assert!(parse(&["buzz", "31"]).is_err());
        assert!(parse(&["buzz", "x"]).is_err());
        let Command::Raw { hex, force } = parse(&["raw", "02", "e3", "00", "--force"])
            .unwrap()
            .command
        else {
            panic!()
        };
        assert_eq!(hex, ["02", "e3", "00"]);
        assert!(force);
        assert!(parse(&["raw"]).is_err());
    }

    #[test]
    fn autostart_values() {
        let Command::Autostart { state } = parse(&["autostart", "on"]).unwrap().command else {
            panic!()
        };
        assert_eq!(state, OnOff::On);
        assert!(parse(&["autostart", "maybe"]).is_err());
    }
}
