//! Per-device calibration tables ("masters").

use std::fmt;

use crate::opcode::reply;

/// The replies that make up the table, in order; send `A5` once per entry.
pub const MASTERS_REPLIES: [u8; 3] = [reply::MASTERS_1, reply::MASTERS_2, reply::MASTERS_3];

/// Minimum number of values: `getRes` indexes up to `masters[15]`.
pub const MIN_LEN: usize = 16;

/// Index of the value that selects the `CB` leaflet constant; also shown to users as the
/// "calibration key".
pub const KEY_INDEX: usize = 7;

/// Problems with a calibration table.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CalibrationError {
    /// Fewer than [`MIN_LEN`] values were received.
    TooShort {
        /// Number of values received.
        len: usize,
    },
    /// `masters[8]` is zero; `getRes` divides by it.
    ZeroDivisor,
    /// `masters[7]` is outside every leaflet range.
    OutOfRange {
        /// The offending `masters[7]`.
        key: u16,
    },
}

impl fmt::Display for CalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { len } => {
                write!(
                    f,
                    "calibration table has {len} values, at least {MIN_LEN} required"
                )
            }
            Self::ZeroDivisor => f.write_str("calibration table is corrupt: masters[8] = 0"),
            Self::OutOfRange { key } => {
                write!(
                    f,
                    "masters[7] = {key} is outside every calibration range (1122..=1330)"
                )
            }
        }
    }
}

impl std::error::Error for CalibrationError {}

/// A validated calibration table: at least [`MIN_LEN`] values and a non-zero `masters[8]`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize), serde(transparent))]
pub struct Masters(Vec<u16>);

impl Masters {
    /// Validates a table.
    pub fn new(values: Vec<u16>) -> Result<Self, CalibrationError> {
        if values.len() < MIN_LEN {
            return Err(CalibrationError::TooShort { len: values.len() });
        }
        if values[8] == 0 {
            return Err(CalibrationError::ZeroDivisor);
        }
        Ok(Self(values))
    }

    /// Builds the table from the payloads of the `0x44`, `0x45` and `0x46` replies, in that order.
    pub fn from_replies<P: AsRef<[u8]>>(payloads: &[P]) -> Result<Self, CalibrationError> {
        let values = payloads
            .iter()
            .flat_map(|p| decode_values(p.as_ref()))
            .collect();
        Self::new(values)
    }

    /// All values.
    pub fn values(&self) -> &[u16] {
        &self.0
    }

    pub(crate) fn at(&self, i: usize) -> f64 {
        f64::from(self.0[i])
    }

    pub(crate) fn int(&self, i: usize) -> i64 {
        i64::from(self.0[i])
    }

    /// `masters[7]`, the value that selects the leaflet constant.
    pub fn calibration_key(&self) -> u16 {
        self.0[KEY_INDEX]
    }

    /// The `CB` frame constant chosen by `masters[7]` (`_sendLeafletsAutoStart`), or `None` outside
    /// every range.
    pub fn leaflet_const(&self) -> Option<u8> {
        leaflet_const(self.calibration_key())
    }
}

/// Decodes a `0x44`/`0x45`/`0x46` payload into its little-endian values.
pub fn decode_values(payload: &[u8]) -> impl Iterator<Item = u16> + '_ {
    payload
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
}

/// Maps `masters[7]` to the `CB` constant, exactly as the official app does.
pub fn leaflet_const(key: u16) -> Option<u8> {
    match key {
        1294..=1330 => Some(60),
        1269..=1293 => Some(62),
        1245..=1268 => Some(64),
        1195..=1244 => Some(65),
        1171..=1194 => Some(66),
        1146..=1170 => Some(68),
        1122..=1145 => Some(70),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation() {
        assert_eq!(
            Masters::new(vec![1; 15]),
            Err(CalibrationError::TooShort { len: 15 })
        );
        let mut v = vec![1; 16];
        v[8] = 0;
        assert_eq!(Masters::new(v), Err(CalibrationError::ZeroDivisor));
        assert!(Masters::new(vec![1; 16]).is_ok());
    }

    #[test]
    fn from_replies_concatenates_le_pairs() {
        let m = Masters::from_replies(&[
            vec![1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 0xFF],
            vec![7, 0, 0xB0, 0x04, 9, 0, 10, 0, 11, 0, 12, 0],
            vec![13, 0, 14, 0, 15, 0, 16, 0],
        ])
        .unwrap();
        assert_eq!(m.values().len(), 16);
        assert_eq!(m.calibration_key(), 0x04B0);
        assert_eq!(m.values()[6], 7);
    }

    #[test]
    fn leaflet_boundaries() {
        let cases = [
            (1121, None),
            (1122, Some(70)),
            (1145, Some(70)),
            (1146, Some(68)),
            (1170, Some(68)),
            (1171, Some(66)),
            (1194, Some(66)),
            (1195, Some(65)),
            (1244, Some(65)),
            (1245, Some(64)),
            (1268, Some(64)),
            (1269, Some(62)),
            (1293, Some(62)),
            (1294, Some(60)),
            (1330, Some(60)),
            (1331, None),
        ];
        for (key, expected) in cases {
            assert_eq!(leaflet_const(key), expected, "masters[7] = {key}");
        }
    }
}
