//! Heating modes, curves and the profile upload.

use std::fmt;
use std::ops::RangeInclusive;
use std::str::FromStr;

use crate::calibration::{CalibrationError, Masters};
use crate::command::Command;
use crate::frame::Frame;
use crate::opcode::{reply, request};

/// Number of curve-step frames in every upload (`A7` … `BA`); shorter curves are padded with
/// all-zero steps.
pub const STEP_COUNT: usize = 20;

/// Number of frames in a full profile upload.
pub const UPLOAD_FRAME_COUNT: usize = STEP_COUNT + 10 + 7 + 1;

/// Gradient value meaning "no slope" (a flat step rather than a ramp).
pub const NO_SLOPE: f64 = 1000.0;

/// Hold temperatures a custom profile may use, °C. The bounds are what the presets themselves
/// already use.
pub const CUSTOM_TEMP_RANGE: RangeInclusive<u16> = 240..=320;

/// Hold durations a custom profile may use, seconds. 255 is the format limit: one byte.
pub const CUSTOM_HOLD_RANGE: RangeInclusive<u8> = 30..=255;

/// Delay between profile frames used by the official app (`FirmwareVersionEight._delayBle`), in
/// milliseconds.
pub const FRAME_DELAY_MS: u64 = 10;

/// The four heating modes, in the official app's display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum HeatingMode {
    /// Gentle, long session: 19 sticks per charge, 6 minutes.
    Delicate,
    /// The factory profile: 20 sticks, 5 minutes.
    Balanced,
    /// Hot, short session: 25 sticks, 3 minutes.
    Intense,
    /// Battery Saver: 27 sticks, 3 minutes.
    BatterySaver,
}

impl HeatingMode {
    /// All modes in display order.
    pub const ALL: [Self; 4] = [
        Self::Delicate,
        Self::Balanced,
        Self::Intense,
        Self::BatterySaver,
    ];

    /// Modes a custom profile can be based on (every mode with a known curve).
    pub const CUSTOM_BASES: [Self; 3] = [Self::Delicate, Self::Intense, Self::BatterySaver];

    /// Profile number on the wire (`BB` byte 0, `0x3E` byte 1).
    pub fn profile_number(self) -> u8 {
        match self {
            Self::Balanced => 0,
            Self::Delicate => 1,
            Self::Intense => 2,
            Self::BatterySaver => 3,
        }
    }

    /// `ResponseHeatingModeDTOMapper.toDomain`: 1, 2, 3 map to presets and every other value means
    /// Balanced.
    pub fn from_profile_number(number: u8) -> Self {
        match number {
            1 => Self::Delicate,
            2 => Self::Intense,
            3 => Self::BatterySaver,
            _ => Self::Balanced,
        }
    }

    /// Stable identifier, identical to the serde representation.
    pub fn id(self) -> &'static str {
        match self {
            Self::Delicate => "delicate",
            Self::Balanced => "balanced",
            Self::Intense => "intense",
            Self::BatterySaver => "batterySaver",
        }
    }

    /// English display name.
    pub fn title(self) -> &'static str {
        match self {
            Self::Delicate => "Delicate",
            Self::Balanced => "Balanced",
            Self::Intense => "Intense",
            Self::BatterySaver => "Battery Saver",
        }
    }

    /// Sticks per full charge (`HeatingMode.maxSticks`).
    pub fn max_sticks(self) -> u8 {
        match self {
            Self::Delicate => 19,
            Self::Balanced => 20,
            Self::Intense => 25,
            Self::BatterySaver => 27,
        }
    }

    /// Session length in minutes (`HeatingMode.heatingTime`).
    pub fn minutes(self) -> u8 {
        match self {
            Self::Delicate => 6,
            Self::Balanced => 5,
            Self::Intense | Self::BatterySaver => 3,
        }
    }

    /// Intensity dots shown by the official app, 1..=3 (`HeatingMode.heat`).
    pub fn heat(self) -> u8 {
        match self {
            Self::Delicate => 1,
            Self::Balanced | Self::BatterySaver => 2,
            Self::Intense => 3,
        }
    }

    /// The heating curve without zero padding; `None` for Balanced.
    pub fn curve(self) -> Option<&'static [CurveStep]> {
        match self {
            Self::Delicate => Some(&DELICATE_CURVE),
            Self::Balanced => None,
            Self::Intense => Some(&INTENSE_CURVE),
            Self::BatterySaver => Some(&BATTERY_SAVER_CURVE),
        }
    }

    /// Index of the hold phase (the longest step; the first one on ties), which is what a custom
    /// profile modifies.
    pub fn hold_step_index(self) -> Option<usize> {
        let curve = self.curve()?;
        let longest = curve.iter().map(|s| s.duration_s).max()?;
        curve.iter().position(|s| s.duration_s == longest)
    }

    fn constants(self) -> Option<&'static ProfileConstants> {
        match self {
            Self::Delicate => Some(&DELICATE_CONSTANTS),
            Self::Balanced => None,
            Self::Intense => Some(&INTENSE_CONSTANTS),
            Self::BatterySaver => Some(&BATTERY_SAVER_CONSTANTS),
        }
    }
}

impl fmt::Display for HeatingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

/// Error returned when a string names no heating mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownMode(pub String);

impl fmt::Display for UnknownMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown heating mode {:?}; expected delicate, balanced, intense or saver",
            self.0
        )
    }
}

impl std::error::Error for UnknownMode {}

impl FromStr for HeatingMode {
    type Err = UnknownMode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "delicate" => Ok(Self::Delicate),
            "balanced" => Ok(Self::Balanced),
            "intense" => Ok(Self::Intense),
            "saver" | "batterysaver" | "battery-saver" | "battery_saver" => Ok(Self::BatterySaver),
            _ => Err(UnknownMode(s.to_owned())),
        }
    }
}

/// One step of a heating curve.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct CurveStep {
    /// Step length in seconds; a single byte on the wire.
    pub duration_s: u8,
    /// Target temperature in °C. Negative values are device flags (the sign is preserved through
    /// `getRes`); `0` turns the heater off.
    pub temperature_c: i16,
    /// [`NO_SLOPE`] for a flat step, otherwise a ramp coefficient. Sent as `trunc(gradient * 1000)`.
    pub gradient: f64,
}

const fn step(duration_s: u8, temperature_c: i16, gradient: f64) -> CurveStep {
    CurveStep {
        duration_s,
        temperature_c,
        gradient,
    }
}

const ZERO_STEP: CurveStep = step(0, 0, 0.0);

const DELICATE_CURVE: [CurveStep; 8] = [
    step(60, 300, NO_SLOPE),
    step(19, 300, NO_SLOPE),
    step(15, 300, -0.013),
    step(15, -220, NO_SLOPE),
    step(9, 221, NO_SLOPE),
    step(231, 240, 0.009),
    step(75, 250, 0.009),
    step(15, 0, 0.025),
];

const INTENSE_CURVE: [CurveStep; 6] = [
    step(60, 320, NO_SLOPE),
    step(23, 320, NO_SLOPE),
    step(20, 320, -0.01),
    step(11, -255, NO_SLOPE),
    step(6, 256, NO_SLOPE),
    step(143, 275, 0.009),
];

const BATTERY_SAVER_CURVE: [CurveStep; 8] = [
    step(60, 320, NO_SLOPE),
    step(18, 320, NO_SLOPE),
    step(20, 320, -0.012),
    step(14, -245, NO_SLOPE),
    step(7, 245, NO_SLOPE),
    step(4, 245, 0.01),
    step(110, 240, 0.009),
    step(25, 255, 0.009),
];

#[derive(Debug, PartialEq, Eq)]
struct ProfileConstants {
    /// Profile number (`b`).
    number: u8,
    /// Number of curve steps (`c`).
    steps: u8,
    /// Unnamed per-profile byte (`d`).
    d: u8,
    /// Voltage table sent in `C7` (`CW cx cy db dx dy`).
    volts: [u16; 6],
    /// Thresholds sent twice in `C9` (`k2 k3 k4` / `ok p1 p2`).
    thresholds: [u16; 3],
    /// Byte repeated ten times in `D4`.
    d4_byte: u8,
    /// Word repeated ten times across `D5` and `D6`.
    d5_word: u16,
}

const DELICATE_CONSTANTS: ProfileConstants = ProfileConstants {
    number: 1,
    steps: 8,
    d: 19,
    volts: [3390, 3435, 3520, 4775, 4730, 4593],
    thresholds: [3340, 3315, 3300],
    d4_byte: 24,
    d5_word: 20_000,
};

const INTENSE_CONSTANTS: ProfileConstants = ProfileConstants {
    number: 2,
    steps: 6,
    d: 23,
    volts: [3425, 3470, 3555, 4810, 4765, 4628],
    thresholds: [3375, 3350, 3335],
    d4_byte: 29,
    d5_word: 23_100,
};

const BATTERY_SAVER_CONSTANTS: ProfileConstants = ProfileConstants {
    number: 3,
    steps: 8,
    d: 18,
    volts: [3390, 3435, 3520, 4775, 4730, 4593],
    thresholds: [3340, 3315, 3300],
    d4_byte: 24,
    d5_word: 22_100,
};

/// Errors from building a heating profile.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProfileError {
    /// Balanced is the factory profile; it has no curve to modify or upload.
    NoCurve,
    /// The device's calibration table cannot be used.
    Calibration(CalibrationError),
}

impl fmt::Display for ProfileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCurve => f.write_str("balanced is the factory profile and has no curve"),
            Self::Calibration(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for ProfileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoCurve => None,
            Self::Calibration(e) => Some(e),
        }
    }
}

impl From<CalibrationError> for ProfileError {
    fn from(e: CalibrationError) -> Self {
        Self::Calibration(e)
    }
}

/// A user-defined profile: a preset's curve and calibration constants with a different hold phase
/// (temperature and length of the longest step).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase", try_from = "CustomProfileRepr")
)]
pub struct CustomProfile {
    base: HeatingMode,
    hold_temp_c: u16,
    hold_secs: u8,
}

impl CustomProfile {
    /// Builds a custom profile, clamping the hold phase into the safe ranges.
    pub fn new(base: HeatingMode, hold_temp_c: i64, hold_secs: i64) -> Result<Self, ProfileError> {
        if base.curve().is_none() {
            return Err(ProfileError::NoCurve);
        }
        Ok(Self {
            base,
            hold_temp_c: clamp_into(hold_temp_c, &CUSTOM_TEMP_RANGE),
            hold_secs: clamp_into(hold_secs, &CUSTOM_HOLD_RANGE),
        })
    }

    /// The preset this profile is derived from (never Balanced).
    pub fn base(&self) -> HeatingMode {
        self.base
    }

    /// Hold-phase temperature, °C.
    pub fn hold_temp_c(&self) -> u16 {
        self.hold_temp_c
    }

    /// Hold-phase length, seconds.
    pub fn hold_secs(&self) -> u8 {
        self.hold_secs
    }
}

fn clamp_into<T>(value: i64, range: &RangeInclusive<T>) -> T
where
    T: Copy + Into<i64> + TryFrom<i64>,
{
    let clamped = value.clamp((*range.start()).into(), (*range.end()).into());
    // The clamped value lies inside a range of `T`, so the conversion succeeds.
    T::try_from(clamped).unwrap_or(*range.start())
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomProfileRepr {
    base: HeatingMode,
    hold_temp_c: i64,
    hold_secs: i64,
}

#[cfg(feature = "serde")]
impl TryFrom<CustomProfileRepr> for CustomProfile {
    type Error = ProfileError;

    fn try_from(r: CustomProfileRepr) -> Result<Self, Self::Error> {
        Self::new(r.base, r.hold_temp_c, r.hold_secs)
    }
}

/// A profile that can be uploaded: a preset or a custom variation of one.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatingProfile {
    base: HeatingMode,
    steps: Vec<CurveStep>,
    constants: &'static ProfileConstants,
}

impl HeatingProfile {
    /// The profile of a preset; `None` for Balanced.
    pub fn preset(mode: HeatingMode) -> Option<Self> {
        Some(Self {
            base: mode,
            steps: mode.curve()?.to_vec(),
            constants: mode.constants()?,
        })
    }

    /// The profile of a custom variation: its base preset with the hold step's duration and
    /// temperature replaced (the gradient is kept).
    pub fn custom(custom: &CustomProfile) -> Self {
        let base = custom.base;
        let (Some(curve), Some(constants), Some(hold)) =
            (base.curve(), base.constants(), base.hold_step_index())
        else {
            // `CustomProfile::new` rejects bases without a curve.
            unreachable!("custom profile with a curve-less base")
        };
        let mut steps = curve.to_vec();
        steps[hold].duration_s = custom.hold_secs;
        // 240..=320 always fits in i16.
        steps[hold].temperature_c = custom.hold_temp_c as i16;
        Self {
            base,
            steps,
            constants,
        }
    }

    /// The preset the profile is (based on).
    pub fn base(&self) -> HeatingMode {
        self.base
    }

    /// Profile number written in `BB` and reported back by `0x3E`.
    pub fn profile_number(&self) -> u8 {
        self.constants.number
    }

    /// The curve, without zero padding.
    pub fn steps(&self) -> &[CurveStep] {
        &self.steps
    }
}

/// One frame of a profile upload, with the reply it must wait for (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileFrame {
    /// The frame to write.
    pub frame: Frame,
    /// Opcode of the acknowledgement to wait for; its first payload byte is a status, `0` meaning
    /// success.
    pub awaits: Option<u8>,
}

impl ProfileFrame {
    fn send(frame: Frame) -> Self {
        Self {
            frame,
            awaits: None,
        }
    }

    fn wait(frame: Frame, awaits: u8) -> Self {
        Self {
            frame,
            awaits: Some(awaits),
        }
    }
}

/// The single frame that selects Balanced (the factory profile): `02 a4 00 → 3F`.
pub fn factory_profile_frames() -> Vec<ProfileFrame> {
    vec![ProfileFrame::wait(
        Command::ActivateProfile { downloaded: false }.frame(),
        reply::PROFILE_ACTIVATED,
    )]
}

/// `BTUtils.getRes`: converts a curve temperature into heater resistance using the unit's
/// calibration table.
pub fn get_res(temp: i32, masters: &Masters) -> i64 {
    if temp == 0 || temp == 1000 {
        return i64::from(temp);
    }
    let negative = temp < 0;
    let r = i64::from(temp).abs();
    let fixed = match r {
        245 => Some(3),
        250 => Some(2),
        275 => Some(1),
        320 => Some(14),
        _ => None,
    };
    let magnitude = if let Some(index) = fixed {
        masters.int(index)
    } else {
        // Same operation order as the reference: integer products divided as
        // floats, so every intermediate rounds identically.
        let p = ((masters.int(11) * (320 - r)) as f64 / 75.0
            + (masters.int(15) * (r - 245)) as f64 / 75.0)
            / 1000.0;
        let base = if r == 300 { 306.0 } else { r as f64 };
        let ratio = masters.at(7) / masters.at(8);
        dart_round(masters.at(3) + p * ratio * (base - masters.at(4) / 10.0))
    };
    if negative { -magnitude } else { magnitude }
}

// Dart `round()`: halves away from zero.
fn dart_round(x: f64) -> i64 {
    (x.abs() + 0.5).floor().copysign(x) as i64
}

/// Builds the full 38-frame upload for `profile`.
/// Step: `[res u16][duration u8][temp i16][trunc(gradient*1000) i32][FF]`, first one prefixed `01`.
pub fn build_profile_frames(
    profile: &HeatingProfile,
    masters: &Masters,
) -> Result<Vec<ProfileFrame>, CalibrationError> {
    let leaflet = masters
        .leaflet_const()
        .ok_or(CalibrationError::OutOfRange {
            key: masters.calibration_key(),
        })?;
    let c = profile.constants;
    let mut out = Vec::with_capacity(UPLOAD_FRAME_COUNT);

    let padded = profile
        .steps
        .iter()
        .copied()
        .chain(std::iter::repeat(ZERO_STEP));
    for (i, s) in padded.take(STEP_COUNT).enumerate() {
        let mut body = Vec::with_capacity(11);
        if i == 0 {
            body.push(0x01);
        }
        let res = get_res(i32::from(s.temperature_c), masters);
        body.extend_from_slice(&(res as u16).to_le_bytes());
        body.push(s.duration_s);
        body.extend_from_slice(&s.temperature_c.to_le_bytes());
        // Dart `(gradient * 1000).toInt()` truncates toward zero.
        let gradient = (s.gradient * 1000.0) as i64;
        body.extend_from_slice(&(gradient as u32).to_le_bytes());
        body.push(0xFF);
        out.push(ProfileFrame::send(Frame::build(
            request::CURVE_STEP_FIRST + i as u8,
            &body,
        )));
    }

    let mut bb = vec![c.number, 0, 15, c.steps, c.d, 0];
    bb.extend(words(&[2215, 2176]));
    out.push(ProfileFrame::send(Frame::build(0xBB, &bb)));
    out.push(ProfileFrame::send(Frame::build(
        0xBC,
        &dwords(&[100, 10, 10]),
    )));
    out.push(ProfileFrame::send(Frame::build(0xBD, &dwords(&[100, 10]))));
    out.push(ProfileFrame::send(Frame::build(
        0xBE,
        &words(&[0, 0, 0, 150, 140, 117]),
    )));
    out.push(ProfileFrame::send(Frame::build(0xC7, &words(&c.volts))));
    out.push(ProfileFrame::send(Frame::build(
        0xC8,
        &words(&[64, 65, 68, 64, 65, 68]),
    )));
    let [k0, k1, k2] = c.thresholds;
    out.push(ProfileFrame::send(Frame::build(
        0xC9,
        &words(&[k0, k1, k2, k0, k1, k2, 70, 75, 85]),
    )));
    out.push(ProfileFrame::send(Frame::build(0xD4, &[c.d4_byte; 10])));
    out.push(ProfileFrame::send(Frame::build(
        0xD5,
        &words(&[c.d5_word; 5]),
    )));
    out.push(ProfileFrame::send(Frame::build(
        0xD6,
        &words(&[c.d5_word; 5]),
    )));

    out.push(ProfileFrame::send(Frame::build(0xD7, &[0; 10])));
    out.push(ProfileFrame::send(Frame::build(0xD8, &[0; 10])));
    // The app compares the *profile's* number, not the firmware version.
    out.push(ProfileFrame::send(Frame::build(
        0xFB,
        &[u8::from(c.number == 8)],
    )));
    out.push(ProfileFrame::send(Frame::build(
        0xCB,
        &[
            1, 92, 3, 80, 55, 8, 8, 70, leaflet, 165, 13, 219, 26, 60, 100, 0, 10,
        ],
    )));
    out.push(ProfileFrame::wait(
        Frame::build(
            0xCC,
            &[
                1, 20, 5, 100, 0, 0, 10, 238, 2, 144, 31, 238, 2, 144, 31, 0, 0,
            ],
        ),
        reply::AUTO_START_ACK_1,
    ));
    out.push(ProfileFrame::send(Frame::build(
        0xCD,
        &[1, 232, 3, 0, 0, 45, 7, 232, 3, 25, 1, 223, 0, 222, 3],
    )));
    out.push(ProfileFrame::wait(
        Frame::build(0xCE, &[1, 160, 15, 208, 7, 62, 168, 11]),
        reply::AUTO_START_ACK_2,
    ));

    out.push(ProfileFrame::wait(
        Command::ActivateProfile { downloaded: true }.frame(),
        reply::PROFILE_ACTIVATED,
    ));
    debug_assert_eq!(out.len(), UPLOAD_FRAME_COUNT);
    Ok(out)
}

fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn dwords(values: &[u32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn masters(key: u16) -> Masters {
        let mut v: Vec<u16> = (0..16).map(|i| 3000 + i * 7).collect();
        v[4] = 2500;
        v[7] = key;
        v[8] = 1200;
        Masters::new(v).unwrap()
    }

    #[test]
    fn profile_numbers_round_trip() {
        for mode in HeatingMode::ALL {
            assert_eq!(
                HeatingMode::from_profile_number(mode.profile_number()),
                mode
            );
        }
        assert_eq!(HeatingMode::from_profile_number(14), HeatingMode::Balanced);
    }

    #[test]
    fn mode_aliases() {
        assert_eq!("saver".parse(), Ok(HeatingMode::BatterySaver));
        assert_eq!("Battery-Saver".parse(), Ok(HeatingMode::BatterySaver));
        assert_eq!("batterySaver".parse(), Ok(HeatingMode::BatterySaver));
        assert_eq!("INTENSE".parse(), Ok(HeatingMode::Intense));
        assert!("turbo".parse::<HeatingMode>().is_err());
        for mode in HeatingMode::ALL {
            assert_eq!(mode.id().parse(), Ok(mode));
        }
    }

    #[test]
    fn hold_steps_are_the_longest() {
        assert_eq!(HeatingMode::Delicate.hold_step_index(), Some(5));
        assert_eq!(HeatingMode::Intense.hold_step_index(), Some(5));
        assert_eq!(HeatingMode::BatterySaver.hold_step_index(), Some(6));
        assert_eq!(HeatingMode::Balanced.hold_step_index(), None);
    }

    #[test]
    fn custom_profile_clamps() {
        let c = CustomProfile::new(HeatingMode::Intense, 100, 1000).unwrap();
        assert_eq!((c.hold_temp_c(), c.hold_secs()), (240, 255));
        let c = CustomProfile::new(HeatingMode::Intense, 999, -5).unwrap();
        assert_eq!((c.hold_temp_c(), c.hold_secs()), (320, 30));
        assert_eq!(
            CustomProfile::new(HeatingMode::Balanced, 280, 60),
            Err(ProfileError::NoCurve)
        );
    }

    #[test]
    fn custom_profile_replaces_hold_step_only() {
        let custom = CustomProfile::new(HeatingMode::Delicate, 260, 200).unwrap();
        let p = HeatingProfile::custom(&custom);
        let preset = HeatingProfile::preset(HeatingMode::Delicate).unwrap();
        assert_eq!(p.profile_number(), 1);
        for (i, (a, b)) in p.steps().iter().zip(preset.steps()).enumerate() {
            if i == 5 {
                assert_eq!(
                    (a.duration_s, a.temperature_c, a.gradient),
                    (200, 260, 0.009)
                );
            } else {
                assert_eq!(a, b);
            }
        }
    }

    #[test]
    fn upload_layout() {
        let p = HeatingProfile::preset(HeatingMode::Intense).unwrap();
        let frames = build_profile_frames(&p, &masters(1200)).unwrap();
        assert_eq!(frames.len(), 38);
        assert_eq!(
            frames[0].frame.as_bytes().len(),
            13,
            "first step has a leading 01"
        );
        assert_eq!(frames[0].frame.payload()[0], 1);
        assert_eq!(frames[1].frame.as_bytes().len(), 12);
        let waits: Vec<_> = frames.iter().filter_map(|f| f.awaits).collect();
        assert_eq!(waits, [0x49, 0x4B, 0x3F]);
        assert_eq!(frames[37].frame.to_hex(), "02 a4 01");
        let fb = frames.iter().find(|f| f.frame.opcode() == 0xFB).unwrap();
        assert_eq!(fb.frame.payload(), [0]);
    }

    #[test]
    fn upload_refuses_out_of_range_calibration() {
        let p = HeatingProfile::preset(HeatingMode::Delicate).unwrap();
        assert_eq!(
            build_profile_frames(&p, &masters(1000)),
            Err(CalibrationError::OutOfRange { key: 1000 })
        );
    }

    #[test]
    fn get_res_passthrough_and_fixed_points() {
        let m = masters(1200);
        assert_eq!(get_res(0, &m), 0);
        assert_eq!(get_res(1000, &m), 1000);
        assert_eq!(get_res(245, &m), i64::from(m.values()[3]));
        assert_eq!(get_res(-245, &m), -i64::from(m.values()[3]));
        assert_eq!(get_res(320, &m), i64::from(m.values()[14]));
    }

    #[test]
    fn dart_round_is_half_away_from_zero() {
        assert_eq!(dart_round(2.5), 3);
        assert_eq!(dart_round(-2.5), -3);
        assert_eq!(dart_round(2.4999), 2);
        assert_eq!(dart_round(3.5), 4);
    }

    #[test]
    fn factory_profile_is_one_frame() {
        let f = factory_profile_frames();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].frame.to_hex(), "02 a4 00");
        assert_eq!(f[0].awaits, Some(0x3F));
    }
}
