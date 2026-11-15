//! User commands and the status poll.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use ploom_protocol::command::POLL;
use ploom_protocol::diagnostics::{DiagKind, DiagReply};
use ploom_protocol::heating::{
    self, FRAME_DELAY_MS, HeatingProfile, UPLOAD_FRAME_COUNT, factory_profile_frames,
};
use ploom_protocol::opcode::{self, reply};
use ploom_protocol::{Command, Frame, HeatingMode, Notification};
use tokio::time::sleep;
use tracing::{debug, info, trace};

use super::limit::{Observation, Refusal, Step};
use super::{Shared, VIBRATION_AUTO_OFF, lock, now_ms};
use crate::device::{DEFAULT_TIMEOUT, Device};
use crate::error::{Error, Result};
use crate::model::{
    ActivityKind, AdcReading, ChargeLog, CustomProfile, DeviceStatus, Diagnostics,
    DiscoveredDevice, FramePreview, HeatingStartCount, LimitSettings, LogCounts, LogsReport,
    Operation, PreviewFrame, ProfileTarget, RawExchange, RawReply, SessionLog,
};
use crate::transport::SCAN_PASSES;

const POLL_TIMEOUT: Duration = Duration::from_millis(1500);
const POLL_ATTEMPTS: u32 = 3;
const POLL_DIAG_TIMEOUT: Duration = Duration::from_millis(800);
const DIAG_ATTEMPTS: u32 = 3;
const DIAG_SPACING: Duration = Duration::from_millis(300);
const DIAG_TIMEOUT: Duration = Duration::from_secs(1);
const LOG_WINDOW: Duration = Duration::from_secs(3);
const CHARGE_SUMMARY_WINDOW: Duration = Duration::from_secs(2);
const RAW_WINDOW: Duration = Duration::from_millis(2500);
const SCAN_WINDOW: Duration = Duration::from_secs(5);

const HEATING: &str = "the device is heating and ignores most requests until the session ends";

enum Plan {
    /// Balanced: `A4 00`.
    Factory,
    /// A 38-frame upload.
    Upload {
        profile: HeatingProfile,
        custom: Option<CustomProfile>,
    },
}

fn resolve(target: ProfileTarget) -> Result<(ProfileTarget, Plan)> {
    match target {
        ProfileTarget::Preset {
            mode: HeatingMode::Balanced,
        } => Ok((target, Plan::Factory)),
        ProfileTarget::Preset { mode } => {
            let profile = HeatingProfile::preset(mode)
                .ok_or_else(|| Error::Internal(format!("{mode} has no curve")))?;
            Ok((
                target,
                Plan::Upload {
                    profile,
                    custom: None,
                },
            ))
        }
        ProfileTarget::Custom {
            base,
            hold_temp_c,
            hold_secs,
        } => {
            let custom = ploom_protocol::CustomProfile::new(base, hold_temp_c, hold_secs)?;
            let normalized = ProfileTarget::Custom {
                base,
                hold_temp_c: i64::from(custom.hold_temp_c()),
                hold_secs: i64::from(custom.hold_secs()),
            };
            let profile = HeatingProfile::custom(&custom);
            Ok((
                normalized,
                Plan::Upload {
                    profile,
                    custom: Some(custom.into()),
                },
            ))
        }
    }
}

struct OperationGuard<'a>(&'a Shared);

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        self.0.update(|st| st.operation = None);
    }
}

struct FlagGuard<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for FlagGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

struct ScanGuard<'a>(&'a Shared);

impl Drop for ScanGuard<'_> {
    fn drop(&mut self) {
        self.0.update(|st| st.scanning = false);
    }
}

impl Shared {
    pub(super) async fn poll_once(&self, device: &Device) -> Result<()> {
        if self.operation_active() {
            return Ok(());
        }
        let attempts = if self.is_heating() { 1 } else { POLL_ATTEMPTS };
        for (command, expect) in POLL {
            for attempt in 1..=attempts {
                let result = {
                    let _conversation = self.conversation.lock().await;
                    device.request(&command.frame(), expect, POLL_TIMEOUT).await
                };
                match result {
                    Ok(_) => break,
                    Err(Error::Timeout { .. }) => {
                        trace!(?command, attempt, "poll request unanswered");
                    }
                    Err(e) => return Err(e),
                }
            }
        }

        let n = self.poll_count.fetch_add(1, Ordering::Relaxed) + 1;
        if !self.is_heating() && n % 2 == 1 {
            let adc = {
                let _conversation = self.conversation.lock().await;
                device.read_diag(DiagKind::Adc, POLL_DIAG_TIMEOUT).await?
            };
            let temp = {
                let _conversation = self.conversation.lock().await;
                device
                    .read_diag(DiagKind::EnclosureTemp, POLL_DIAG_TIMEOUT)
                    .await?
            };
            self.apply_diagnostics(adc.as_ref(), temp.as_ref());
        }
        let wall = now_ms();
        self.update(|st| st.tracker.after_poll(wall));
        Ok(())
    }

    fn apply_diagnostics(&self, adc: Option<&DiagReply>, temp: Option<&DiagReply>) {
        let plugged = adc.and_then(DiagReply::usb_plugged);
        let celsius = temp.and_then(DiagReply::enclosure_temp_c);
        if plugged.is_some() || celsius.is_some() {
            self.update(|st| {
                if let Some(p) = plugged {
                    st.tracker.set_usb_plugged(p);
                }
                if let Some(c) = celsius {
                    st.tracker.set_enclosure_temp_c(c);
                }
            });
        }
    }

    pub(super) async fn refresh_status(&self) -> Result<DeviceStatus> {
        let device = self.device()?;
        if self.operation_active() {
            return Err(Error::Busy("a heating profile is being flashed"));
        }
        self.poll_once(&device).await?;
        Ok(lock(&self.state).tracker.status())
    }

    async fn send_command(&self, command: Command) -> Result<Arc<Device>> {
        let device = self.device()?;
        let _conversation = self.converse().await?;
        device.send(&command.frame()).await?;
        Ok(device)
    }

    async fn refresh_after_change(&self, device: &Device) {
        let _conversation = self.conversation.lock().await;
        let frame = Command::RequestStatus.frame();
        if let Err(e) = device
            .request(&frame, reply::BATTERY_LEVEL, POLL_TIMEOUT)
            .await
        {
            debug!(error = %e, "status refresh after change failed");
        }
    }

    pub(super) async fn set_vibration(self: &Arc<Self>, on: bool) -> Result<()> {
        self.send_command(Command::SetVibration(on)).await?;
        let generation = self.vibration_generation.fetch_add(1, Ordering::SeqCst) + 1;
        if on {
            let shared = Arc::clone(self);
            tokio::spawn(async move {
                sleep(VIBRATION_AUTO_OFF).await;
                // A later on/off call supersedes this timer.
                if shared.vibration_generation.load(Ordering::SeqCst) == generation {
                    debug!("vibration auto-off");
                    if let Err(e) = shared.send_command(Command::SetVibration(false)).await {
                        debug!(error = %e, "vibration auto-off failed");
                    }
                }
            });
        }
        Ok(())
    }

    pub(super) async fn set_lock(&self, locked: bool) -> Result<()> {
        if !locked {
            let now = (self.clock)();
            if let Some(until_ms) = lock(&self.state).limit.cooldown_until(now) {
                return Err(Error::CooldownActive { until_ms });
            }
        }
        let device = self.send_command(Command::SetLock(locked)).await?;
        self.refresh_after_change(&device).await;
        Ok(())
    }

    pub(super) fn set_limit_settings(&self, settings: LimitSettings) -> Result<()> {
        let now = (self.clock)();
        let switched_on = self.update(|st| {
            let was_on = st.limit.settings.enabled;
            st.limit
                .change_settings(settings, now)
                .map(|()| settings.enabled && !was_on)
        });
        match switched_on {
            Ok(switched_on) => {
                if switched_on {
                    // The lifetime baseline was dropped: read it again.
                    self.limit_synced.store(false, Ordering::SeqCst);
                }
                self.limit_wake.notify_one();
                Ok(())
            }
            Err(Refusal::Invalid(detail)) => Err(Error::InvalidInput(detail)),
            Err(Refusal::CooldownActive { until_ms }) => Err(Error::CooldownActive { until_ms }),
        }
    }

    pub(super) async fn sync_lifetime_starts(&self, device: &Device) {
        if self.limit_synced.load(Ordering::SeqCst)
            || !lock(&self.state).limit.settings.enabled
            || self.is_heating()
            || self.operation_active()
        {
            return;
        }
        let reply = {
            let _conversation = self.conversation.lock().await;
            self.read_diag_retrying(device, DiagKind::HeatingStarts)
                .await
        };
        // One attempt per connection, whatever came back.
        self.limit_synced.store(true, Ordering::SeqCst);
        let total = match reply {
            Ok(Some(r)) if r.has_data() => r
                .heating_starts()
                .iter()
                .map(|&(_, n)| u32::from(n))
                .sum::<u32>(),
            Ok(_) => return,
            Err(e) => {
                debug!(error = %e, "lifetime counters unavailable");
                return;
            }
        };
        let now = (self.clock)();
        let away = self.update(|st| st.limit.lifetime_total_read(total, now));
        if away > 0 {
            info!(away, "counted sticks used while disconnected");
        }
    }

    pub(super) async fn limit_step(&self, device: &Device) {
        if self.operation_active() {
            return;
        }
        let now = (self.clock)();
        let step = self.update(|st| {
            let status = st.tracker.status();
            st.limit.decide(
                now,
                Observation {
                    lock: status.lock,
                    activity: status.activity,
                },
            )
        });
        let result = match step {
            Step::Nothing => return,
            Step::AdoptLock => {
                self.cooldown_started(now, false);
                return;
            }
            Step::End => {
                self.update(|st| st.limit.end_cooldown());
                self.emit(ActivityKind::CooldownEnded);
                return;
            }
            Step::Lock => self.limit_command(device, true).await.map(|()| {
                self.cooldown_started(now, true);
            }),
            Step::Relock => self.limit_command(device, true).await.map(|()| {
                self.update(|st| st.limit.relocked());
                info!("device was unlocked during a cooldown; locked again");
                self.emit(ActivityKind::Relocked);
            }),
            Step::Unlock => self.limit_command(device, false).await.map(|()| {
                self.update(|st| st.limit.end_cooldown());
                info!("cooldown over; device unlocked");
                self.emit(ActivityKind::CooldownEnded);
            }),
        };
        match result {
            // Make the new lock state visible before deciding again.
            Ok(()) => self.refresh_after_change(device).await,
            Err(e) => debug!(error = %e, ?step, "limit command failed; will retry"),
        }
    }

    async fn limit_command(&self, device: &Device, locked: bool) -> Result<()> {
        let _conversation = self.conversation.lock().await;
        device.send(&Command::SetLock(locked).frame()).await
    }

    fn cooldown_started(&self, now: u64, by_limit: bool) {
        let (sticks, locked_until_ms) = self.update(|st| {
            let until = st.limit.begin_cooldown(now, by_limit);
            (st.limit.settings.sticks, until)
        });
        info!(
            sticks,
            locked_until_ms, "stick limit reached; device locked"
        );
        self.emit(ActivityKind::LimitReached {
            sticks,
            locked_until_ms,
        });
    }

    pub(super) async fn set_auto_start(&self, enabled: bool) -> Result<()> {
        let device = self.send_command(Command::SetAutoStart(enabled)).await?;
        self.refresh_after_change(&device).await;
        Ok(())
    }

    fn start_operation(&self, target: ProfileTarget, total: usize) -> Result<OperationGuard<'_>> {
        let total = u32::try_from(total).unwrap_or(u32::MAX);
        self.update(|st| {
            if st.operation.is_some() {
                return Err(Error::Busy("a heating profile is being flashed"));
            }
            if st.tracker.is_heating() {
                return Err(Error::Busy(HEATING));
            }
            st.operation = Some(Operation::FlashingProfile {
                target,
                done: 0,
                total,
            });
            Ok(())
        })?;
        Ok(OperationGuard(self))
    }

    fn set_progress(&self, done: usize) {
        let done = u32::try_from(done).unwrap_or(u32::MAX);
        self.update(|st| {
            if let Some(Operation::FlashingProfile { done: d, .. }) = &mut st.operation {
                *d = done;
            }
        });
    }

    pub(super) async fn apply_profile(&self, target: ProfileTarget) -> Result<()> {
        let (target, plan) = resolve(target)?;
        let device = self.device()?;
        let device_id = self.device_id().ok_or(Error::NotConnected)?;
        let total = match plan {
            Plan::Factory => 1,
            Plan::Upload { .. } => UPLOAD_FRAME_COUNT,
        };
        let _operation = self.start_operation(target, total)?;
        let _conversation = self.conversation.lock().await;
        info!(?target, "applying heating profile");

        let (confirmed, custom) = match plan {
            Plan::Factory => (self.activate_factory(&device).await?, None),
            Plan::Upload { profile, custom } => (self.upload(&device, &profile).await?, custom),
        };

        self.persist(|s| s.set_custom_profile(&device_id, custom));
        self.update(|st| st.tracker.set_custom_profile(custom));
        self.emit(ActivityKind::ProfileApplied { target, confirmed });
        info!(confirmed, "heating profile applied");
        Ok(())
    }

    async fn activate_factory(&self, device: &Device) -> Result<bool> {
        let frames = factory_profile_frames();
        let mut confirmed = false;
        for (i, pf) in frames.iter().enumerate() {
            let expect = pf.awaits.unwrap_or(reply::PROFILE_ACTIVATED);
            let status = device.request(&pf.frame, expect, DEFAULT_TIMEOUT).await?;
            check_status(&pf.frame, &status)?;
            confirmed = !status.is_empty();
            self.set_progress(i + 1);
        }
        self.refresh_mode(device).await;
        Ok(confirmed)
    }

    async fn upload(&self, device: &Device, profile: &HeatingProfile) -> Result<bool> {
        let masters = device.read_masters().await?;
        // Everything is computed before the first write.
        let frames = heating::build_profile_frames(profile, &masters)?;
        debug!(
            key = masters.calibration_key(),
            frames = frames.len(),
            "uploading profile"
        );
        for (i, pf) in frames.iter().enumerate() {
            sleep(Duration::from_millis(FRAME_DELAY_MS)).await;
            match pf.awaits {
                Some(expect) => {
                    let status = device.request(&pf.frame, expect, DEFAULT_TIMEOUT).await?;
                    check_status(&pf.frame, &status)?;
                }
                None => device.send(&pf.frame).await?,
            }
            self.set_progress(i + 1);
        }
        self.refresh_mode(device).await;

        let number = profile.profile_number();
        let verify = Command::RequestDownloadedProfile.frame();
        match device
            .request(&verify, reply::DOWNLOADED_PROFILE, DEFAULT_TIMEOUT)
            .await
        {
            Ok(payload) if payload.get(1) == Some(&number) => Ok(true),
            Ok(payload) => Err(Error::DeviceRejected {
                opcode: verify.opcode(),
                detail: format!(
                    "device reports profile {:?} after upload, expected {number}",
                    payload.get(1)
                ),
            }),
            Err(Error::Timeout { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn refresh_mode(&self, device: &Device) {
        let frame = Command::RequestProfileActive.frame();
        match device
            .request(&frame, reply::PROFILE_ACTIVE, DEFAULT_TIMEOUT)
            .await
        {
            Ok(p) if p.first().is_some_and(|&b| b != 0) => {
                if let Err(e) = device
                    .send(&Command::RequestDownloadedProfile.frame())
                    .await
                {
                    debug!(error = %e, "mode refresh failed");
                }
            }
            Ok(_) => {}
            Err(e) => debug!(error = %e, "mode refresh failed"),
        }
    }

    pub(super) async fn preview_profile(&self, target: ProfileTarget) -> Result<FramePreview> {
        let (_, plan) = resolve(target)?;
        let (calibration_key, frames) = match plan {
            Plan::Factory => (None, factory_profile_frames()),
            Plan::Upload { profile, .. } => {
                let device = self.device()?;
                let masters = {
                    let _conversation = self.converse().await?;
                    device.read_masters().await?
                };
                (
                    Some(masters.calibration_key()),
                    heating::build_profile_frames(&profile, &masters)?,
                )
            }
        };
        Ok(FramePreview {
            calibration_key,
            frames: frames
                .into_iter()
                .map(|f| PreviewFrame {
                    hex: f.frame.to_hex(),
                    awaits_opcode: f.awaits,
                })
                .collect(),
        })
    }

    pub(super) async fn read_logs(&self) -> Result<LogsReport> {
        let device = self.device()?;
        let _conversation = self.converse().await?;
        self.quiet_logs.store(true, Ordering::SeqCst);
        let _quiet = FlagGuard(&self.quiet_logs);
        let mut frames = device
            .exchange(&Command::RequestSmokingLogs.frame(), LOG_WINDOW)
            .await?;
        frames.extend(
            device
                .exchange(&Command::RequestChargingLogs.frame(), LOG_WINDOW)
                .await?,
        );
        Ok(logs_report(
            frames.iter().filter_map(|b| Notification::parse(b).ok()),
        ))
    }

    pub(super) async fn read_diag_retrying(
        &self,
        device: &Device,
        kind: DiagKind,
    ) -> Result<Option<DiagReply>> {
        // The channel is flaky under rapid requests: space them out and retry
        // until a record with real data arrives.
        let mut last = None;
        for _ in 0..DIAG_ATTEMPTS {
            sleep(DIAG_SPACING).await;
            last = device.read_diag(kind, DIAG_TIMEOUT).await?;
            if last.as_ref().is_some_and(DiagReply::has_data) {
                break;
            }
        }
        Ok(last)
    }

    pub(super) async fn read_diagnostics(&self) -> Result<Diagnostics> {
        let device = self.device()?;
        if self.is_heating() {
            return Err(Error::Busy(HEATING));
        }
        let _conversation = self.converse().await?;
        let temp = self
            .read_diag_retrying(&device, DiagKind::EnclosureTemp)
            .await?;
        let adc = self.read_diag_retrying(&device, DiagKind::Adc).await?;
        let starts = self
            .read_diag_retrying(&device, DiagKind::HeatingStarts)
            .await?;
        let charge = device
            .exchange(&Command::RequestChargingLogs.frame(), CHARGE_SUMMARY_WINDOW)
            .await?;
        self.apply_diagnostics(adc.as_ref(), temp.as_ref());

        let charge_cycles = charge.iter().find_map(|b| match Notification::parse(b) {
            Ok(Notification::ChargeLogSummary(s)) => Some(s.total_charge_count),
            _ => None,
        });
        let (battery_pct, battery_health_pct) = lock(&self.state).tracker.battery();
        Ok(Diagnostics {
            enclosure_temp_c: temp.as_ref().and_then(DiagReply::enclosure_temp_c),
            usb_plugged: adc.as_ref().and_then(DiagReply::usb_plugged),
            usb_cc: adc.as_ref().and_then(DiagReply::usb_cc),
            adc: adc.as_ref().map(|r| {
                r.adc()
                    .into_iter()
                    .map(|(name, value)| AdcReading {
                        name: name.to_owned(),
                        value,
                    })
                    .collect()
            }),
            heating_starts: starts.as_ref().map(|r| {
                r.heating_starts()
                    .into_iter()
                    .map(|(key, count)| HeatingStartCount {
                        key: key.to_owned(),
                        count,
                    })
                    .collect()
            }),
            charge_cycles,
            battery_health_pct,
            battery_pct,
        })
    }

    pub(super) async fn send_raw(&self, hex: &str, confirm_dangerous: bool) -> Result<RawExchange> {
        let frame = Frame::parse_hex(hex)?;
        if let Some(reason) = opcode::danger(frame.opcode()) {
            if !confirm_dangerous {
                return Err(Error::DangerousFrame {
                    opcode: frame.opcode(),
                    reason,
                });
            }
        }
        let device = self.device()?;
        let replies = {
            let _conversation = self.converse().await?;
            info!(frame = %frame, "sending raw frame");
            device.exchange(&frame, RAW_WINDOW).await?
        };
        let replies: Vec<RawReply> = replies.iter().map(|b| raw_reply(b)).collect();
        self.emit(ActivityKind::RawFrameSent {
            hex: frame.to_hex(),
            replies: u32::try_from(replies.len()).unwrap_or(u32::MAX),
        });
        Ok(RawExchange {
            sent: frame.to_hex(),
            replies,
        })
    }

    pub(super) async fn scan(&self) -> Result<Vec<DiscoveredDevice>> {
        self.update(|st| {
            if st.scanning {
                return Err(Error::Busy("a scan is already running"));
            }
            st.scanning = true;
            Ok(())
        })?;
        let _scanning = ScanGuard(self);

        let mut seen = BTreeMap::new();
        for _ in 0..SCAN_PASSES {
            for ad in self.connector.scan(SCAN_WINDOW).await? {
                seen.insert(ad.id.clone(), ad);
            }
        }
        let (current, current_rssi) = {
            let st = lock(&self.state);
            (st.device.clone(), st.tracker.status().rssi_dbm)
        };
        let connected_id = (self.device().is_ok()).then(|| current.as_ref().map(|d| d.id.clone()));
        let connected_id = connected_id.flatten();

        let mut out: Vec<DiscoveredDevice> = seen
            .into_values()
            .map(|ad| DiscoveredDevice {
                connected: connected_id.as_deref() == Some(ad.id.as_str()),
                name: ad
                    .name
                    .unwrap_or_else(|| crate::transport::NAME_PREFIX.to_owned()),
                rssi_dbm: ad.rssi_dbm,
                id: ad.id,
            })
            .collect();
        // A connected peripheral usually stops advertising; list it anyway.
        if let (Some(id), Some(dev)) = (&connected_id, &current) {
            if !out.iter().any(|d| &d.id == id) {
                out.push(DiscoveredDevice {
                    id: id.clone(),
                    name: dev
                        .name
                        .clone()
                        .unwrap_or_else(|| crate::transport::NAME_PREFIX.into()),
                    rssi_dbm: current_rssi,
                    connected: true,
                });
            }
        }
        out.sort_by_key(|d| std::cmp::Reverse(d.rssi_dbm.unwrap_or(i16::MIN)));
        Ok(out)
    }
}

fn check_status(frame: &Frame, payload: &[u8]) -> Result<()> {
    match payload.first() {
        Some(&status) if status != 0 => Err(Error::DeviceRejected {
            opcode: frame.opcode(),
            detail: format!("status 0x{status:02x}"),
        }),
        _ => Ok(()),
    }
}

fn raw_reply(bytes: &[u8]) -> RawReply {
    let decoded = Notification::parse(bytes)
        .ok()
        .and_then(|n| serde_json::to_value(n).ok())
        .and_then(|v| match v {
            serde_json::Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default();
    RawReply {
        opcode: bytes.get(1).copied().unwrap_or(0),
        hex: ploom_protocol::frame::to_hex(bytes),
        decoded,
    }
}

fn logs_report(frames: impl IntoIterator<Item = Notification>) -> LogsReport {
    let mut session = None;
    let mut puffs = BTreeMap::new();
    let mut charge = None;
    let mut summary = None;
    let mut unacquired = None;
    for n in frames {
        match n {
            Notification::SmokingLog(l) => session = Some(l),
            Notification::PuffTimings(p) => puffs.extend(p.puffs),
            Notification::ChargeLog(c) => charge = Some(c),
            Notification::ChargeLogSummary(s) => summary = Some(s),
            Notification::UnacquiredLogs { sessions, charges } => {
                unacquired = Some(LogCounts { sessions, charges });
            }
            _ => {}
        }
    }
    LogsReport {
        session: session.map(|l| SessionLog {
            start_counter: l.start_counter,
            duration_s: l.duration_s,
            puff_count: l.puff_count,
            puff_offsets_s: puffs.into_values().collect(),
            puff_stop_enabled: l.puff_stop_enabled,
            auto_start_enabled: l.auto_start_enabled,
            lock_enabled: l.lock_enabled,
        }),
        charge: charge.map(|c| ChargeLog {
            start_counter: c.start_counter,
            end_counter: c.end_counter,
            start_mv: c.start_mv,
            end_mv: c.end_mv,
            total_charge_count: summary.map(|s| s.total_charge_count),
            state_of_health_pct: summary.map(|s| s.state_of_health_pct),
        }),
        unacquired,
    }
}

#[cfg(test)]
mod tests {
    use ploom_protocol::frame::decode_hex;

    use super::*;

    #[test]
    fn resolve_clamps_and_rejects_balanced_base() {
        let (t, _) = resolve(ProfileTarget::Custom {
            base: HeatingMode::Intense,
            hold_temp_c: 500,
            hold_secs: 1,
        })
        .unwrap();
        assert_eq!(
            t,
            ProfileTarget::Custom {
                base: HeatingMode::Intense,
                hold_temp_c: 320,
                hold_secs: 30
            }
        );
        let err = resolve(ProfileTarget::Custom {
            base: HeatingMode::Balanced,
            hold_temp_c: 280,
            hold_secs: 60,
        });
        assert!(matches!(err, Err(Error::InvalidInput(_))));
        assert!(matches!(
            resolve(ProfileTarget::Preset {
                mode: HeatingMode::Balanced
            }),
            Ok((_, Plan::Factory))
        ));
    }

    #[test]
    fn logs_from_live_frames() {
        let frames = [
            "053506000700",
            "1336b033000068010800000100010e00000000b095",
            "13370e005d009000d000e6000901240157010000",
            "1338000000000000000000000000000000000000",
            "133b030000002f010000fd0d960ed00be60b0000",
            "0f400200010000005f00000000000000",
        ]
        .map(|h| Notification::parse(&decode_hex(h).unwrap()).unwrap());
        let r = logs_report(frames);
        let s = r.session.unwrap();
        assert_eq!(
            (s.start_counter, s.duration_s, s.puff_count),
            (13_232, 360, 8)
        );
        assert_eq!(s.puff_offsets_s, [14, 93, 144, 208, 230, 265, 292, 343]);
        assert!(s.auto_start_enabled && !s.lock_enabled && !s.puff_stop_enabled);
        let c = r.charge.unwrap();
        assert_eq!(
            (c.start_counter, c.end_counter, c.start_mv, c.end_mv),
            (3, 303, 3581, 3734)
        );
        assert_eq!(
            (c.total_charge_count, c.state_of_health_pct),
            (Some(2), Some(95))
        );
        assert_eq!(
            r.unacquired,
            Some(LogCounts {
                sessions: 6,
                charges: 7
            })
        );
    }

    #[test]
    fn empty_logs() {
        assert_eq!(logs_report([]), LogsReport::default());
    }

    #[test]
    fn raw_reply_decodes() {
        let r = raw_reply(&[0x03, 0x3C, 0x55, 0x00]);
        assert_eq!(r.opcode, 0x3C);
        assert_eq!(r.hex, "03 3c 55 00");
        assert_eq!(r.decoded["type"], "batteryLevel");
        assert_eq!(r.decoded["pct"], 85);
        let unknown = raw_reply(&[0x02, 0x99, 0x01]);
        assert_eq!(unknown.decoded["type"], "unknown");
        assert_eq!(unknown.decoded["opcode"], 0x99);
        assert!(raw_reply(&[0x01]).decoded.is_empty());
    }

    #[test]
    fn rejection_status() {
        let f = Command::RequestStatus.frame();
        assert!(check_status(&f, &[0]).is_ok());
        assert!(check_status(&f, &[]).is_ok());
        assert!(matches!(
            check_status(&f, &[2]),
            Err(Error::DeviceRejected { opcode: 0xE3, .. })
        ));
    }
}
