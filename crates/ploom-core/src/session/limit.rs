//! The stick limit: count heating sessions, lock the device for a cooldown once the limit is
//! reached, and release it afterwards.

use serde::{Deserialize, Serialize};

use crate::model::{Activity, LimitSettings, LimitState, LockState};

/// Allowed range of [`LimitSettings::sticks`].
pub const STICKS_RANGE: std::ops::RangeInclusive<u32> = 1..=40;
/// Allowed range of [`LimitSettings::cooldown_minutes`].
pub const COOLDOWN_MINUTES_RANGE: std::ops::RangeInclusive<u32> = 5..=1440;
/// Allowed range of [`LimitSettings::window_hours`].
pub const WINDOW_HOURS_RANGE: std::ops::RangeInclusive<u32> = 1..=168;

const MINUTE_MS: u64 = 60_000;
const HOUR_MS: u64 = 60 * MINUTE_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct LimitRecord {
    pub(crate) settings: LimitSettings,
    pub(crate) sticks_used: u32,
    pub(crate) window_started_at_ms: Option<u64>,
    pub(crate) locked_until_ms: Option<u64>,
    pub(crate) locked_by_limit: bool,
    pub(crate) lifetime_starts: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Observation {
    pub(crate) lock: Option<LockState>,
    pub(crate) activity: Option<Activity>,
}

impl Observation {
    // The lock is only ever sent at rest, never mid-session.
    fn at_rest(self) -> bool {
        matches!(self.activity, Some(Activity::Standby | Activity::Charging))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    /// Nothing to do.
    Nothing,
    /// The limit is reached: send `LOCK`, then call [`LimitRecord::begin_cooldown`].
    Lock,
    /// The limit is reached and the device is already locked: no frame is needed, call
    /// [`LimitRecord::begin_cooldown`] with `by_limit = false`.
    AdoptLock,
    /// Unlocked from elsewhere during a cooldown: send `LOCK` again.
    Relock,
    /// The cooldown is over: send `UNLOCK`, then call [`LimitRecord::end_cooldown`].
    Unlock,
    /// The cooldown is over and no frame is needed: call [`LimitRecord::end_cooldown`].
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// A value is out of range.
    Invalid(String),
    /// The change would relax the limit while a cooldown is running.
    CooldownActive { until_ms: u64 },
}

pub(crate) fn validate(s: &LimitSettings) -> Result<(), String> {
    if !STICKS_RANGE.contains(&s.sticks) {
        return Err(format!(
            "sticks must be {}..={}, got {}",
            STICKS_RANGE.start(),
            STICKS_RANGE.end(),
            s.sticks
        ));
    }
    if !COOLDOWN_MINUTES_RANGE.contains(&s.cooldown_minutes) {
        return Err(format!(
            "cooldownMinutes must be {}..={}, got {}",
            COOLDOWN_MINUTES_RANGE.start(),
            COOLDOWN_MINUTES_RANGE.end(),
            s.cooldown_minutes
        ));
    }
    if !WINDOW_HOURS_RANGE.contains(&s.window_hours) {
        return Err(format!(
            "windowHours must be {}..={}, got {}",
            WINDOW_HOURS_RANGE.start(),
            WINDOW_HOURS_RANGE.end(),
            s.window_hours
        ));
    }
    Ok(())
}

fn more_permissive(old: &LimitSettings, new: &LimitSettings) -> bool {
    (old.enabled && !new.enabled)
        || new.sticks > old.sticks
        || new.cooldown_minutes < old.cooldown_minutes
        || new.window_hours < old.window_hours
}

impl LimitRecord {
    fn cooldown_ms(&self) -> u64 {
        u64::from(self.settings.cooldown_minutes) * MINUTE_MS
    }

    fn window_ms(&self) -> u64 {
        u64::from(self.settings.window_hours) * HOUR_MS
    }

    pub(crate) fn state(&self) -> LimitState {
        LimitState {
            settings: self.settings,
            sticks_used: self.sticks_used,
            window_started_at_ms: self.window_started_at_ms,
            locked_until_ms: self.locked_until_ms,
        }
    }

    pub(crate) fn cooldown_until(&self, now: u64) -> Option<u64> {
        self.locked_until_ms.filter(|&until| now < until)
    }

    fn clamp_to_clock(&mut self, now: u64) {
        let latest = now.saturating_add(self.cooldown_ms());
        if let Some(until) = &mut self.locked_until_ms {
            *until = (*until).min(latest);
        }
        if let Some(start) = &mut self.window_started_at_ms {
            *start = (*start).min(now);
        }
    }

    fn roll_window(&mut self, now: u64) {
        if self.locked_until_ms.is_some() {
            return;
        }
        if let Some(start) = self.window_started_at_ms
            && now.saturating_sub(start) >= self.window_ms()
        {
            self.sticks_used = 0;
            self.window_started_at_ms = None;
        }
    }

    pub(crate) fn count(&mut self, n: u32, now: u64) {
        if !self.settings.enabled || n == 0 {
            return;
        }
        self.clamp_to_clock(now);
        self.roll_window(now);
        if self.window_started_at_ms.is_none() {
            self.window_started_at_ms = Some(now);
        }
        self.sticks_used = self.sticks_used.saturating_add(n);
    }

    pub(crate) fn session_started(&mut self, now: u64) {
        self.count(1, now);
        // Keep the lifetime baseline in step, so this session is not
        // counted a second time from the device's counters on reconnect.
        if let Some(total) = &mut self.lifetime_starts {
            *total = total.saturating_add(1);
        }
    }

    pub(crate) fn lifetime_total_read(&mut self, total: u32, now: u64) -> u32 {
        let away = match self.lifetime_starts {
            // The first reading only establishes the baseline.
            None => 0,
            Some(prev) => total.saturating_sub(prev),
        };
        self.lifetime_starts = Some(total);
        self.count(away, now);
        if self.settings.enabled { away } else { 0 }
    }

    pub(crate) fn change_settings(&mut self, new: LimitSettings, now: u64) -> Result<(), Refusal> {
        validate(&new).map_err(Refusal::Invalid)?;
        self.clamp_to_clock(now);
        if let Some(until_ms) = self.cooldown_until(now)
            && more_permissive(&self.settings, &new)
        {
            return Err(Refusal::CooldownActive { until_ms });
        }
        let switched_on = new.enabled && !self.settings.enabled;
        self.settings = new;
        if switched_on {
            // Start clean; in particular forget the lifetime baseline, which
            // went stale while the limit was off.
            self.sticks_used = 0;
            self.window_started_at_ms = None;
            self.lifetime_starts = None;
        }
        Ok(())
    }

    pub(crate) fn decide(&mut self, now: u64, seen: Observation) -> Step {
        self.clamp_to_clock(now);
        if let Some(until) = self.locked_until_ms {
            if now >= until {
                return match seen.lock {
                    // Only lift a lock the limit itself set.
                    Some(LockState::Locked) if self.locked_by_limit => Step::Unlock,
                    Some(_) => Step::End,
                    // Unknown yet (just connected): wait for the first status.
                    None => Step::Nothing,
                };
            }
            if seen.lock == Some(LockState::Unlocked) && seen.at_rest() {
                return Step::Relock;
            }
            return Step::Nothing;
        }
        self.roll_window(now);
        if self.settings.enabled && self.sticks_used >= self.settings.sticks && seen.at_rest() {
            return match seen.lock {
                Some(LockState::Locked) => Step::AdoptLock,
                Some(_) => Step::Lock,
                None => Step::Nothing,
            };
        }
        Step::Nothing
    }

    pub(crate) fn begin_cooldown(&mut self, now: u64, by_limit: bool) -> u64 {
        let until = now.saturating_add(self.cooldown_ms());
        self.locked_until_ms = Some(until);
        self.locked_by_limit = by_limit;
        until
    }

    pub(crate) fn relocked(&mut self) {
        self.locked_by_limit = true;
    }

    pub(crate) fn end_cooldown(&mut self) {
        self.locked_until_ms = None;
        self.locked_by_limit = false;
        self.sticks_used = 0;
        self.window_started_at_ms = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_800_000_000_000;

    fn on(sticks: u32, cooldown_minutes: u32) -> LimitRecord {
        LimitRecord {
            settings: LimitSettings {
                enabled: true,
                sticks,
                cooldown_minutes,
                window_hours: 24,
            },
            ..LimitRecord::default()
        }
    }

    fn seen(lock: LockState, activity: Activity) -> Observation {
        Observation {
            lock: Some(lock),
            activity: Some(activity),
        }
    }

    const IDLE: Observation = Observation {
        lock: Some(LockState::Unlocked),
        activity: Some(Activity::Standby),
    };

    #[test]
    fn counts_only_while_enabled() {
        let mut r = LimitRecord::default();
        r.session_started(T0);
        assert_eq!(r.sticks_used, 0);
        assert_eq!(r.window_started_at_ms, None);

        let mut r = on(3, 30);
        r.session_started(T0);
        r.session_started(T0 + 1000);
        assert_eq!(r.sticks_used, 2);
        assert_eq!(
            r.window_started_at_ms,
            Some(T0),
            "window starts at the first stick"
        );
        assert_eq!(r.decide(T0 + 2000, IDLE), Step::Nothing);
    }

    #[test]
    fn window_resets_the_counter() {
        let mut r = on(10, 30);
        r.session_started(T0);
        r.session_started(T0 + HOUR_MS);
        assert_eq!(r.decide(T0 + 24 * HOUR_MS - 1, IDLE), Step::Nothing);
        assert_eq!(r.sticks_used, 2);
        assert_eq!(r.decide(T0 + 24 * HOUR_MS, IDLE), Step::Nothing);
        assert_eq!((r.sticks_used, r.window_started_at_ms), (0, None));
        // The next stick opens a new window.
        r.session_started(T0 + 25 * HOUR_MS);
        assert_eq!(r.window_started_at_ms, Some(T0 + 25 * HOUR_MS));
        assert_eq!(r.sticks_used, 1);
    }

    #[test]
    fn locks_only_after_the_session_ends() {
        let mut r = on(2, 30);
        r.session_started(T0);
        r.session_started(T0 + 1);
        for activity in [Activity::Preheating, Activity::Heating] {
            assert_eq!(
                r.decide(T0 + 2, seen(LockState::Unlocked, activity)),
                Step::Nothing,
                "never lock mid-session"
            );
        }
        assert_eq!(
            r.decide(T0 + 2, Observation::default()),
            Step::Nothing,
            "status unknown"
        );
        assert_eq!(r.decide(T0 + 3, IDLE), Step::Lock);
        assert_eq!(
            r.decide(T0 + 3, seen(LockState::Unlocked, Activity::Charging)),
            Step::Lock
        );
        let until = r.begin_cooldown(T0 + 3, true);
        assert_eq!(until, T0 + 3 + 30 * MINUTE_MS);
        assert_eq!(r.cooldown_until(T0 + 4), Some(until));
        assert_eq!(r.state().locked_until_ms, Some(until));
    }

    #[test]
    fn relocks_when_unlocked_elsewhere() {
        let mut r = on(1, 30);
        r.session_started(T0);
        assert_eq!(r.decide(T0, IDLE), Step::Lock);
        r.begin_cooldown(T0, true);
        let locked = seen(LockState::Locked, Activity::Standby);
        assert_eq!(r.decide(T0 + 1000, locked), Step::Nothing);
        assert_eq!(r.decide(T0 + 2000, IDLE), Step::Relock);
        // Somebody unlocked and started a session: wait for it to end.
        assert_eq!(
            r.decide(T0 + 3000, seen(LockState::Unlocked, Activity::Heating)),
            Step::Nothing
        );
        // With the lock feature off there is nothing to re-lock.
        assert_eq!(
            r.decide(T0 + 3000, seen(LockState::FeatureOff, Activity::Standby)),
            Step::Nothing
        );
    }

    #[test]
    fn expiry_unlocks_and_starts_over() {
        let mut r = on(1, 30);
        r.session_started(T0);
        r.begin_cooldown(T0, true);
        let end = T0 + 30 * MINUTE_MS;
        let locked = seen(LockState::Locked, Activity::Standby);
        assert_eq!(r.decide(end - 1, locked), Step::Nothing);
        assert_eq!(
            r.decide(end, Observation::default()),
            Step::Nothing,
            "wait for status"
        );
        assert_eq!(r.decide(end, locked), Step::Unlock);
        r.end_cooldown();
        assert_eq!(
            r,
            LimitRecord {
                settings: r.settings,
                ..LimitRecord::default()
            }
        );
        assert_eq!(r.decide(end + 1, IDLE), Step::Nothing);
    }

    #[test]
    fn expiry_without_frame_when_already_unlocked() {
        let mut r = on(1, 30);
        r.session_started(T0);
        r.begin_cooldown(T0, true);
        assert_eq!(r.decide(T0 + 31 * MINUTE_MS, IDLE), Step::End);
    }

    #[test]
    fn manual_lock_is_never_lifted() {
        let mut r = on(1, 30);
        r.session_started(T0);
        let locked = seen(LockState::Locked, Activity::Standby);
        // Already locked by the user when the limit is reached.
        assert_eq!(r.decide(T0, locked), Step::AdoptLock);
        r.begin_cooldown(T0, false);
        assert_eq!(r.decide(T0 + 31 * MINUTE_MS, locked), Step::End);
        // …unless the limit had to lock it again during the cooldown.
        let mut r = on(1, 30);
        r.session_started(T0);
        r.begin_cooldown(T0, false);
        assert_eq!(r.decide(T0 + 1, IDLE), Step::Relock);
        r.relocked();
        assert_eq!(r.decide(T0 + 31 * MINUTE_MS, locked), Step::Unlock);
    }

    #[test]
    fn clock_set_back_never_extends_the_cooldown() {
        let mut r = on(1, 30);
        r.session_started(T0);
        let until = r.begin_cooldown(T0, true);
        // The clock jumps a day back: the cooldown is at most 30 min from now.
        let earlier = T0 - 24 * HOUR_MS;
        let locked = seen(LockState::Locked, Activity::Standby);
        assert_eq!(r.decide(earlier, locked), Step::Nothing);
        assert_eq!(r.locked_until_ms, Some(earlier + 30 * MINUTE_MS));
        assert!(r.locked_until_ms.unwrap() < until);
        assert_eq!(r.decide(earlier + 30 * MINUTE_MS, locked), Step::Unlock);
        // A window never starts in the future either.
        let mut r = on(5, 30);
        r.session_started(T0);
        r.session_started(earlier);
        assert_eq!(r.window_started_at_ms, Some(earlier));
    }

    #[test]
    fn clock_set_forward_just_expires() {
        let mut r = on(1, 30);
        r.session_started(T0);
        r.begin_cooldown(T0, true);
        assert_eq!(
            r.decide(
                T0 + 48 * HOUR_MS,
                seen(LockState::Locked, Activity::Standby)
            ),
            Step::Unlock
        );
    }

    #[test]
    fn permissive_changes_are_refused_during_cooldown() {
        let mut r = on(5, 60);
        for _ in 0..5 {
            r.session_started(T0);
        }
        let until = r.begin_cooldown(T0, true);
        let base = r.settings;
        let refused = Err(Refusal::CooldownActive { until_ms: until });
        let now = T0 + 1000;
        for relaxed in [
            LimitSettings {
                enabled: false,
                ..base
            },
            LimitSettings { sticks: 6, ..base },
            LimitSettings {
                cooldown_minutes: 59,
                ..base
            },
            LimitSettings {
                window_hours: 23,
                ..base
            },
        ] {
            assert_eq!(r.change_settings(relaxed, now), refused, "{relaxed:?}");
            assert_eq!(r.settings, base);
        }
        // Stricter or equal settings are fine.
        let stricter = LimitSettings {
            sticks: 4,
            cooldown_minutes: 90,
            window_hours: 48,
            ..base
        };
        assert_eq!(r.change_settings(stricter, now), Ok(()));
        assert_eq!(r.settings, stricter);
        // After the cooldown anything goes.
        assert_eq!(
            r.change_settings(
                LimitSettings {
                    enabled: false,
                    ..base
                },
                until
            ),
            Ok(())
        );
    }

    #[test]
    fn settings_are_range_checked() {
        let mut r = LimitRecord::default();
        let base = LimitSettings::default();
        for bad in [
            LimitSettings { sticks: 0, ..base },
            LimitSettings { sticks: 41, ..base },
            LimitSettings {
                cooldown_minutes: 4,
                ..base
            },
            LimitSettings {
                cooldown_minutes: 1441,
                ..base
            },
            LimitSettings {
                window_hours: 0,
                ..base
            },
            LimitSettings {
                window_hours: 169,
                ..base
            },
        ] {
            assert!(
                matches!(r.change_settings(bad, T0), Err(Refusal::Invalid(_))),
                "{bad:?}"
            );
        }
        assert_eq!(r.settings, base);
    }

    #[test]
    fn switching_on_starts_clean() {
        let mut r = LimitRecord {
            sticks_used: 7,
            window_started_at_ms: Some(T0),
            lifetime_starts: Some(100),
            ..LimitRecord::default()
        };
        let enabled = LimitSettings {
            enabled: true,
            ..LimitSettings::default()
        };
        r.change_settings(enabled, T0).unwrap();
        assert_eq!(
            (r.sticks_used, r.window_started_at_ms, r.lifetime_starts),
            (0, None, None)
        );
    }

    #[test]
    fn sticks_smoked_while_away_are_counted_once() {
        let mut r = on(10, 30);
        assert_eq!(
            r.lifetime_total_read(500, T0),
            0,
            "first reading is the baseline"
        );
        assert_eq!(r.sticks_used, 0);
        // A session seen live moves the baseline along with the counter.
        r.session_started(T0 + 1);
        assert_eq!((r.sticks_used, r.lifetime_starts), (1, Some(501)));
        // Reconnect: the device counted that one plus two more.
        assert_eq!(r.lifetime_total_read(503, T0 + 2), 2);
        assert_eq!(r.sticks_used, 3);
        // A counter that went backwards (or does not count what we assumed)
        // only resets the baseline.
        assert_eq!(r.lifetime_total_read(400, T0 + 3), 0);
        assert_eq!((r.sticks_used, r.lifetime_starts), (3, Some(400)));
    }

    #[test]
    fn away_sticks_are_not_counted_while_off() {
        let mut r = LimitRecord::default();
        assert_eq!(r.lifetime_total_read(10, T0), 0);
        assert_eq!(r.lifetime_total_read(15, T0), 0);
        assert_eq!(r.sticks_used, 0);
    }

    #[test]
    fn record_round_trips_and_tolerates_missing_fields() {
        let mut r = on(3, 45);
        r.session_started(T0);
        r.begin_cooldown(T0, true);
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("\"lockedByLimit\":true"), "{json}");
        assert_eq!(serde_json::from_str::<LimitRecord>(&json).unwrap(), r);
        assert_eq!(
            serde_json::from_str::<LimitRecord>("{}").unwrap(),
            LimitRecord::default()
        );
    }
}
