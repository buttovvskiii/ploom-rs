//! `[LEN][OPCODE][payload]` frame encoding.

use std::fmt;
use std::str::FromStr;

/// Largest payload that fits in a frame (`LEN` is a single byte).
pub const MAX_PAYLOAD: usize = u8::MAX as usize - 1;

/// Errors produced when building or parsing a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameError {
    /// The input is not a sequence of hex byte pairs.
    InvalidHex,
    /// A frame needs at least `[LEN][OPCODE]`.
    TooShort {
        /// Number of bytes received.
        len: usize,
    },
    /// The first byte does not match the number of bytes that follow it.
    LengthMismatch {
        /// `LEN` the frame should carry.
        expected: u8,
        /// `LEN` the frame actually carries.
        actual: u8,
    },
    /// The payload does not fit in a single-byte `LEN`.
    PayloadTooLong {
        /// Payload length that was requested.
        len: usize,
    },
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHex => f.write_str("not a hex string: expected byte pairs like 02e300"),
            Self::TooShort { len } => {
                write!(
                    f,
                    "frame is {len} byte(s); at least [LEN][OPCODE] is required"
                )
            }
            Self::LengthMismatch { expected, actual } => write!(
                f,
                "first byte is the length: expected {expected:02x} for this frame, got {actual:02x}"
            ),
            Self::PayloadTooLong { len } => {
                write!(
                    f,
                    "payload of {len} bytes exceeds the {MAX_PAYLOAD}-byte maximum"
                )
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// A well-formed request frame: at least two bytes, `bytes[0] == bytes.len() - 1`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Frame {
    bytes: Vec<u8>,
}

impl Frame {
    /// Builds `[1 + payload.len()][opcode][payload]`.
    pub fn new(opcode: u8, payload: &[u8]) -> Result<Self, FrameError> {
        let len = u8::try_from(payload.len() + 1)
            .map_err(|_| FrameError::PayloadTooLong { len: payload.len() })?;
        let mut bytes = Vec::with_capacity(payload.len() + 2);
        bytes.push(len);
        bytes.push(opcode);
        bytes.extend_from_slice(payload);
        Ok(Self { bytes })
    }

    pub(crate) fn build(opcode: u8, payload: &[u8]) -> Self {
        debug_assert!(payload.len() <= MAX_PAYLOAD, "payload too long for a frame");
        let mut bytes = Vec::with_capacity(payload.len() + 2);
        bytes.push((payload.len() + 1) as u8);
        bytes.push(opcode);
        bytes.extend_from_slice(payload);
        Self { bytes }
    }

    /// Validates raw bytes as a frame (`LEN` must match).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < 2 {
            return Err(FrameError::TooShort { len: bytes.len() });
        }
        let expected = u8::try_from(bytes.len() - 1).map_err(|_| FrameError::PayloadTooLong {
            len: bytes.len() - 2,
        })?;
        if bytes[0] != expected {
            return Err(FrameError::LengthMismatch {
                expected,
                actual: bytes[0],
            });
        }
        Ok(Self {
            bytes: bytes.to_vec(),
        })
    }

    /// Parses a hex string such as `"02e300"` or `"02 e3 00"`.
    pub fn parse_hex(text: &str) -> Result<Self, FrameError> {
        Self::from_bytes(&decode_hex(text)?)
    }

    /// The request opcode (`bytes[1]`).
    pub fn opcode(&self) -> u8 {
        self.bytes[1]
    }

    /// Everything after `[LEN][OPCODE]`.
    pub fn payload(&self) -> &[u8] {
        &self.bytes[2..]
    }

    /// The complete frame, ready to be written to TX.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the frame, returning its bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Lowercase hex, bytes separated by single spaces (`"02 e3 00"`).
    pub fn to_hex(&self) -> String {
        to_hex(&self.bytes)
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Frame({})", self.to_hex())
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl FromStr for Frame {
    type Err = FrameError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_hex(s)
    }
}

impl AsRef<[u8]> for Frame {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl From<Frame> for Vec<u8> {
    fn from(frame: Frame) -> Self {
        frame.bytes
    }
}

/// Formats bytes as lowercase hex separated by single spaces (`"02 e3 00"`).
pub fn to_hex(bytes: &[u8]) -> String {
    use fmt::Write as _;
    let mut out = String::with_capacity(bytes.len() * 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        // Writing to a String never fails.
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Decodes a hex string, ignoring ASCII whitespace.
pub fn decode_hex(text: &str) -> Result<Vec<u8>, FrameError> {
    let digits: Vec<u8> = text.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if digits.len() % 2 != 0 {
        return Err(FrameError::InvalidHex);
    }
    digits
        .chunks_exact(2)
        .map(|pair| Ok((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

fn nibble(c: u8) -> Result<u8, FrameError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(FrameError::InvalidHex),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_prefixes_length() {
        let f = Frame::new(0xE3, &[0]).unwrap();
        assert_eq!(f.as_bytes(), [2, 0xE3, 0]);
        assert_eq!(f.opcode(), 0xE3);
        assert_eq!(f.payload(), [0]);
    }

    #[test]
    fn new_rejects_oversized_payload() {
        assert!(Frame::new(1, &[0; MAX_PAYLOAD]).is_ok());
        assert_eq!(
            Frame::new(1, &[0; MAX_PAYLOAD + 1]),
            Err(FrameError::PayloadTooLong {
                len: MAX_PAYLOAD + 1
            })
        );
    }

    #[test]
    fn parse_hex_accepts_spaces_and_case() {
        let f: Frame = " 02 E3 00 ".parse().unwrap();
        assert_eq!(f.as_bytes(), [2, 0xE3, 0]);
        assert_eq!(f.to_hex(), "02 e3 00");
        assert_eq!(f.to_string(), "02 e3 00");
    }

    #[test]
    fn parse_hex_validates() {
        assert_eq!(Frame::parse_hex("zz"), Err(FrameError::InvalidHex));
        assert_eq!(Frame::parse_hex("02e"), Err(FrameError::InvalidHex));
        assert_eq!(Frame::parse_hex("02"), Err(FrameError::TooShort { len: 1 }));
        assert_eq!(Frame::parse_hex(""), Err(FrameError::TooShort { len: 0 }));
        assert_eq!(
            Frame::parse_hex("03e300"),
            Err(FrameError::LengthMismatch {
                expected: 2,
                actual: 3
            })
        );
    }

    #[test]
    fn errors_have_messages() {
        let msg = FrameError::LengthMismatch {
            expected: 2,
            actual: 3,
        }
        .to_string();
        assert!(msg.contains("02") && msg.contains("03"), "{msg}");
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(to_hex(&[]), "");
        assert_eq!(decode_hex("00ff7A").unwrap(), [0, 0xFF, 0x7A]);
    }
}
