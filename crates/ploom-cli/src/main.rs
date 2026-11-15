//! `ploom` — command-line controller for the Ploom Aura (GEN4L).

mod cli;
mod output;

use std::io::{BufRead as _, IsTerminal as _, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use clap::Parser as _;
use ploom_core::model::{
    ActivityKind, ConnectionState, ErrorCode, Operation, ProfileTarget, ReceivedFrame,
};
use ploom_core::{SessionConfig, SessionHandle};
use ploom_protocol::{HeatingMode, Notification};
use serde::Serialize;
use tokio::sync::broadcast::error::RecvError;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Command, FlashArgs, LimitArgs, OnOff};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    let data_dir = cli.data_dir.clone().unwrap_or_else(default_data_dir);
    let session = SessionHandle::start(&SessionConfig {
        data_dir,
        auto_connect: false,
    });
    let ctx = Ctx {
        session: &session,
        device: cli.device.clone(),
        json: cli.json,
    };
    let result = ctx.dispatch(cli.command).await;
    if session.snapshot().connection != ConnectionState::Idle {
        // Release the device: it accepts a single connection.
        let _ = session.disconnect().await;
    }
    result
}

fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn default_data_dir() -> PathBuf {
    dirs::data_dir().map_or_else(|| PathBuf::from(".ploom"), |d| d.join("ploom"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatteryJson {
    battery_pct: Option<u8>,
    battery_health_pct: Option<u8>,
    charging: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModeJson {
    heating_mode: Option<HeatingMode>,
    custom_profile: Option<ploom_core::model::CustomProfile>,
}

struct Ctx<'a> {
    session: &'a SessionHandle,
    device: Option<String>,
    json: bool,
}

impl Ctx<'_> {
    async fn dispatch(&self, command: Command) -> Result<()> {
        let s = self.session;
        match command {
            Command::Scan => {
                self.note("scanning (about 10 s)…");
                let devices = s.scan().await?;
                self.emit(&devices, || output::scan(&devices))
            }
            Command::Status => {
                self.connect().await?;
                s.refresh_status().await?;
                let snapshot = s.snapshot();
                self.emit(&snapshot, || output::status(&snapshot))
            }
            Command::Battery => {
                self.connect().await?;
                let st = s.refresh_status().await?;
                let json = BatteryJson {
                    battery_pct: st.battery_pct,
                    battery_health_pct: st.battery_health_pct,
                    charging: st.charging,
                };
                self.emit(&json, || output::battery(&st))
            }
            Command::Watch => self.watch().await,
            Command::Buzz { secs } => {
                self.connect().await?;
                s.set_vibration(true).await?;
                self.note(&format!("buzzing for {secs} s…"));
                tokio::select! {
                    () = tokio::time::sleep(Duration::from_secs_f64(secs)) => {}
                    _ = tokio::signal::ctrl_c() => {}
                }
                // Also after Ctrl-C, or it keeps buzzing.
                s.set_vibration(false).await?;
                self.done("buzzer off")
            }
            Command::Lock | Command::Unlock => {
                let locked = matches!(command, Command::Lock);
                self.connect().await?;
                s.set_lock(locked).await?;
                self.done(if locked { "locked" } else { "unlocked" })
            }
            Command::Autostart { state } => {
                let on = state == OnOff::On;
                self.connect().await?;
                s.set_auto_start(on).await?;
                self.done(if on {
                    "auto-start heating on"
                } else {
                    "auto-start heating off"
                })
            }
            Command::Mode(args) => match args.name {
                None => {
                    self.connect().await?;
                    let st = s.refresh_status().await?;
                    let json = ModeJson {
                        heating_mode: st.heating_mode,
                        custom_profile: st.custom_profile,
                    };
                    self.emit(&json, || format!("  heating mode  {}\n", output::mode(&st)))
                }
                Some(mode) => self.flash(ProfileTarget::Preset { mode }, args.flash).await,
            },
            Command::Custom(args) => {
                let target = ProfileTarget::Custom {
                    base: args.base,
                    hold_temp_c: args.temp,
                    hold_secs: args.secs,
                };
                self.flash(target, args.flash).await
            }
            Command::Limit(args) => self.limit(&args),
            Command::Logs => {
                self.connect().await?;
                let report = s.read_logs().await?;
                self.emit(&report, || output::logs(&report))
            }
            Command::Diag => {
                self.connect().await?;
                self.note("reading sensors…");
                let d = s.read_diagnostics().await?;
                self.emit(&d, || output::diagnostics(&d))
            }
            Command::Raw { hex, force } => self.raw(&hex.concat(), force).await,
        }
    }

    fn limit(&self, args: &LimitArgs) -> Result<()> {
        let current = self.session.snapshot().limit.settings;
        let wanted = if args.off {
            Some(ploom_core::model::LimitSettings {
                enabled: false,
                ..current
            })
        } else if args.sticks.is_some() || args.cooldown.is_some() || args.window.is_some() {
            Some(ploom_core::model::LimitSettings {
                enabled: true,
                sticks: args.sticks.unwrap_or(current.sticks),
                cooldown_minutes: args.cooldown.unwrap_or(current.cooldown_minutes),
                window_hours: args.window.unwrap_or(current.window_hours),
            })
        } else {
            None
        };
        if let Some(settings) = wanted {
            self.session
                .set_limit_settings(settings)
                .map_err(|e| match e.code() {
                    ErrorCode::CooldownActive => anyhow::anyhow!(
                        "a cooldown is running; the limit can only be made stricter until it ends"
                    ),
                    _ => anyhow::Error::new(e),
                })?;
        }
        let limit = self.session.snapshot().limit;
        self.emit(&limit, || output::limit(&limit, unix_ms()))
    }

    async fn raw(&self, hex: &str, force: bool) -> Result<()> {
        // Validate, and refuse dangerous frames, before connecting.
        let frame: ploom_protocol::Frame = hex.parse()?;
        if let Some(reason) = ploom_protocol::opcode::danger(frame.opcode()) {
            if !force {
                bail!(
                    "0x{:02x} is dangerous: {}. Add --force if that is really what you want",
                    frame.opcode(),
                    danger_text(reason)
                );
            }
        }
        self.connect().await?;
        let x = self.session.send_raw(hex, force).await?;
        self.emit(&x, || output::raw(&x))
    }

    fn emit<T: Serialize>(&self, value: &T, text: impl FnOnce() -> String) -> Result<()> {
        if self.json {
            println!("{}", serde_json::to_string_pretty(value)?);
        } else {
            print!("{}", text());
        }
        Ok(())
    }

    fn done(&self, message: &str) -> Result<()> {
        self.emit(&serde_json::json!({ "ok": true }), || {
            format!("  {message}\n")
        })
    }

    fn note(&self, message: &str) {
        if !self.json && std::io::stderr().is_terminal() {
            eprintln!("{message}");
        }
    }

    async fn connect(&self) -> Result<()> {
        let mut rx = self.session.subscribe_snapshot();
        let what = match &self.device {
            Some(id) => format!("connecting to {id}…"),
            None => "looking for a Ploom…".to_owned(),
        };
        self.note(&what);
        self.session.connect(self.device.clone()).await?;
        let snapshot = tokio::time::timeout(
            CONNECT_TIMEOUT,
            rx.wait_for(|s| {
                matches!(
                    s.connection,
                    ConnectionState::Connected | ConnectionState::Reconnecting
                )
            }),
        )
        .await
        .context("timed out connecting")?
        .context("session stopped")?
        .clone();
        if snapshot.connection == ConnectionState::Connected {
            return Ok(());
        }
        bail!(connect_failure(snapshot.connection_error))
    }

    async fn flash(&self, target: ProfileTarget, args: FlashArgs) -> Result<()> {
        let factory = matches!(
            target,
            ProfileTarget::Preset {
                mode: HeatingMode::Balanced
            }
        );
        if args.dry {
            if !factory {
                self.connect().await?;
            }
            let preview = self.session.preview_profile(target).await?;
            return self.emit(&preview, || output::preview(&preview));
        }

        self.connect().await?;
        if !factory && !args.yes && !confirm(&target)? {
            self.note("cancelled");
            return Ok(());
        }

        let show_progress = !self.json && std::io::stderr().is_terminal();
        let mut rx = self.session.subscribe_snapshot();
        let progress = tokio::spawn(async move {
            while rx.changed().await.is_ok() {
                let op = rx.borrow_and_update().operation.clone();
                if let (true, Some(Operation::FlashingProfile { done, total, .. })) =
                    (show_progress, op)
                {
                    eprint!("\r{}", output::progress(done, total));
                }
            }
        });
        let result = self.session.apply_profile(target).await;
        progress.abort();
        if show_progress {
            eprintln!();
        }
        result?;

        let confirmed = self
            .session
            .activity_history()
            .iter()
            .rev()
            .find_map(|e| match &e.kind {
                ActivityKind::ProfileApplied { confirmed, .. } => Some(*confirmed),
                _ => None,
            });
        let message = if confirmed == Some(false) {
            format!(
                "{} flashed, but the device did not confirm it",
                output::target(&target)
            )
        } else {
            format!("{} active", output::target(&target))
        };
        self.emit(
            &serde_json::json!({ "ok": true, "confirmed": confirmed }),
            || format!("  {message}\n"),
        )
    }

    async fn watch(&self) -> Result<()> {
        self.connect().await?;
        let mut frames = self.session.subscribe_frames();
        let mut events = self.session.subscribe_activity();
        self.note("listening; Ctrl-C to stop");
        let start = tokio::time::Instant::now();
        loop {
            tokio::select! {
                frame = frames.recv() => match frame {
                    Ok(f) => self.print_frame(start.elapsed(), &f)?,
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                },
                event = events.recv() => match event {
                    Ok(e) if self.json => println!("{}", serde_json::json!({ "event": e })),
                    Ok(e) => println!("{}", output::event(&e)),
                    Err(RecvError::Lagged(_)) => {}
                    Err(RecvError::Closed) => break,
                },
                _ = tokio::signal::ctrl_c() => break,
            }
        }
        Ok(())
    }

    fn print_frame(&self, elapsed: Duration, f: &ReceivedFrame) -> Result<()> {
        let decoded = Notification::parse(&f.bytes).ok();
        if self.json {
            println!("{}", serde_json::json!({ "frame": f, "decoded": decoded }));
        } else {
            let hex = ploom_protocol::frame::to_hex(&f.bytes);
            let text = decoded
                .map(|n| serde_json::to_string(&n))
                .transpose()?
                .unwrap_or_default();
            println!("{:>9.3}  {hex:<40}  {text}", elapsed.as_secs_f64());
        }
        Ok(())
    }
}

fn confirm(target: &ProfileTarget) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("flashing needs confirmation; add --yes in non-interactive use");
    }
    eprint!("{}  Continue? [y/N] ", output::flash_warning(target));
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().lock().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn connect_failure(code: Option<ErrorCode>) -> String {
    match code {
        Some(ErrorCode::DeviceNotFound) => "no Ploom found. It sleeps and stops advertising: wake \
            it with its button. It also accepts a single connection, so close the official app."
            .into(),
        Some(ErrorCode::BluetoothUnavailable) => "Bluetooth is unavailable: it is off, or this \
            terminal lacks Bluetooth permission (System Settings → Privacy & Security → Bluetooth)."
            .into(),
        Some(ErrorCode::Timeout) => "the device did not answer while connecting".into(),
        other => format!("could not connect ({other:?})"),
    }
}

fn danger_text(reason: ploom_protocol::DangerReason) -> &'static str {
    match reason {
        ploom_protocol::DangerReason::ErasesLogs => "it permanently erases the logs on the device",
        ploom_protocol::DangerReason::DisablesBluetooth => {
            "it turns Bluetooth off, and only the device itself can turn it back on"
        }
        ploom_protocol::DangerReason::EntersFirmwareUpdate => {
            "it puts the device into firmware-update mode"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_failures_explain_themselves() {
        assert!(connect_failure(Some(ErrorCode::DeviceNotFound)).contains("wake"));
        assert!(connect_failure(Some(ErrorCode::BluetoothUnavailable)).contains("permission"));
        assert!(connect_failure(None).contains("could not connect"));
    }

    #[test]
    fn danger_texts() {
        for (_, reason) in ploom_protocol::opcode::DANGEROUS_OPCODES {
            assert!(!danger_text(reason).is_empty());
        }
    }
}
