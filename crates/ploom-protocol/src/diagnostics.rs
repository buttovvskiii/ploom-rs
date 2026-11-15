//! The hidden diagnostics channel: `04 f1 <type> ff ff` → `0xF2`.

/// Field names of the ADC record (type 7), in wire order.
pub const ADC_FIELDS: [&str; 7] = [
    "Vtemp",
    "USBcc1",
    "USBcc2",
    "HeaterNtc",
    "B",
    "CaseNtc",
    "HeatTemp",
];

/// Stable keys of the lifetime heating-start statistics (type 12), in wire order.
pub const HEATING_START_KEYS: [&str; 7] = [
    "coldStart",
    "hotStart",
    "buttonInAutoWindow",
    "buttonAfterTimeout",
    "buttonAfterUnavailable",
    "buttonAfterTimeoutOrUnavailable",
    "autoAfterUnavailable",
];

/// A CC line above this raw value means a USB-C charger is attached (one line is pulled up to ~450;
/// both read ~28 when unplugged).
pub const USB_CC_PLUGGED_THRESHOLD: u16 = 100;

/// Diagnostic record types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub enum DiagKind {
    /// Type 4: battery fuel-gauge register.
    FuelGauge,
    /// Type 6: lifetime puff counters.
    LifetimeCounters,
    /// Type 7: raw ADC readings, see [`ADC_FIELDS`].
    Adc,
    /// Type 9: enclosure temperature in centi-degrees Celsius.
    EnclosureTemp,
    /// Type 12: lifetime heating-start statistics, see [`HEATING_START_KEYS`].
    HeatingStarts,
    /// Any other type code.
    Other(u8),
}

impl DiagKind {
    /// The type code sent in the request.
    pub fn code(self) -> u8 {
        match self {
            Self::FuelGauge => 4,
            Self::LifetimeCounters => 6,
            Self::Adc => 7,
            Self::EnclosureTemp => 9,
            Self::HeatingStarts => 12,
            Self::Other(code) => code,
        }
    }

    /// Maps a type code back to a kind.
    pub fn from_code(code: u8) -> Self {
        match code {
            4 => Self::FuelGauge,
            6 => Self::LifetimeCounters,
            7 => Self::Adc,
            9 => Self::EnclosureTemp,
            12 => Self::HeatingStarts,
            other => Self::Other(other),
        }
    }
}

/// A decoded `0xF2` payload: `[type][seq][data…]`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DiagReply {
    /// Record type, echoed from the request.
    pub kind: u8,
    /// Sequence number.
    pub seq: u8,
    /// Record data, little-endian `u16` fields.
    pub data: Vec<u8>,
}

impl DiagReply {
    /// Decodes an `0xF2` payload (the bytes after `[LEN][OPCODE]`).
    pub fn parse(payload: &[u8]) -> Option<Self> {
        match payload {
            [kind, seq, data @ ..] => Some(Self {
                kind: *kind,
                seq: *seq,
                data: data.to_vec(),
            }),
            _ => None,
        }
    }

    /// The decoded record type.
    pub fn diag_kind(&self) -> DiagKind {
        DiagKind::from_code(self.kind)
    }

    /// The `index`-th little-endian `u16` field, if present.
    pub fn field(&self, index: usize) -> Option<u16> {
        let j = index.checked_mul(2)?;
        let bytes = self.data.get(j..j + 2)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Whether any data byte is non-zero.
    pub fn has_data(&self) -> bool {
        self.data.iter().any(|&b| b != 0)
    }

    /// Enclosure temperature in °C (type 9): field 0 / 100.
    pub fn enclosure_temp_c(&self) -> Option<f64> {
        self.field(0)
            .filter(|&v| v != 0)
            .map(|v| f64::from(v) / 100.0)
    }

    /// The two USB-C CC line readings (type 7, fields 1 and 2).
    pub fn usb_cc(&self) -> Option<(u16, u16)> {
        Some((self.field(1)?, self.field(2)?))
    }

    /// Whether a USB-C charger is attached (type 7): either CC line above
    /// [`USB_CC_PLUGGED_THRESHOLD`].
    pub fn usb_plugged(&self) -> Option<bool> {
        self.usb_cc()
            .map(|(cc1, cc2)| cc1.max(cc2) > USB_CC_PLUGGED_THRESHOLD)
    }

    /// Named ADC readings (type 7); fields missing from the record are skipped.
    pub fn adc(&self) -> Vec<(&'static str, u16)> {
        ADC_FIELDS
            .iter()
            .enumerate()
            .filter_map(|(i, &name)| Some((name, self.field(i)?)))
            .collect()
    }

    /// Lifetime heating-start counters (type 12); missing fields count as 0.
    pub fn heating_starts(&self) -> Vec<(&'static str, u16)> {
        HEATING_START_KEYS
            .iter()
            .enumerate()
            .map(|(i, &key)| (key, self.field(i).unwrap_or(0)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_codes_round_trip() {
        for code in 0..=u8::MAX {
            assert_eq!(DiagKind::from_code(code).code(), code);
        }
    }

    #[test]
    fn enclosure_temperature() {
        let r = DiagReply::parse(&[9, 0, 0x1C, 0x0C]).unwrap();
        assert_eq!(r.enclosure_temp_c(), Some(31.0));
        let asleep = DiagReply::parse(&[9, 0, 0, 0]).unwrap();
        assert_eq!(asleep.enclosure_temp_c(), None);
        assert!(!asleep.has_data());
        assert_eq!(DiagReply::parse(&[9, 0]).unwrap().enclosure_temp_c(), None);
        assert_eq!(DiagReply::parse(&[9]), None);
    }

    #[test]
    fn usb_detection() {
        let plugged = DiagReply::parse(&[7, 0, 1, 0, 0xC8, 0x01, 28, 0]).unwrap();
        assert_eq!(plugged.usb_cc(), Some((456, 28)));
        assert_eq!(plugged.usb_plugged(), Some(true));
        let unplugged = DiagReply::parse(&[7, 0, 1, 0, 28, 0, 28, 0]).unwrap();
        assert_eq!(unplugged.usb_plugged(), Some(false));
        let short = DiagReply::parse(&[7, 0, 1, 0, 28, 0]).unwrap();
        assert_eq!(short.usb_plugged(), None);
        assert_eq!(short.adc(), vec![("Vtemp", 1), ("USBcc1", 28)]);
    }

    #[test]
    fn heating_starts_default_to_zero() {
        let r = DiagReply::parse(&[12, 0, 5, 0, 7, 0]).unwrap();
        let starts = r.heating_starts();
        assert_eq!(starts.len(), 7);
        assert_eq!(starts[0], ("coldStart", 5));
        assert_eq!(starts[1], ("hotStart", 7));
        assert_eq!(starts[6], ("autoAfterUnavailable", 0));
    }
}
