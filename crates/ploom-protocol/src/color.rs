//! Device body color, decoded from the DIS hardware-revision string.

/// A known device body color.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct DeviceColor {
    /// The three-character code read from the hardware string, e.g. `"L4U"`.
    pub code: String,
    /// Stable finish id the UI maps to a material: one of `black`, `silver`, `gold`, `blue`,
    /// `glacier`, `teal`, or `unknown` for an unmapped code.
    pub id: String,
    /// Catalog name in English; the UI localizes known ids itself.
    pub name: String,
    /// Characteristic body color as `#rrggbb`.
    pub hex: String,
}

type Entry = (&'static str, &'static str, &'static str, &'static str);

/// The GEN4L (Ploom Aura) color catalog.
pub const CATALOG: &[Entry] = &[
    ("L51", "black", "Black", "#0b0b0b"),
    ("L4Y", "silver", "Silver", "#848385"),
    ("L4W", "gold", "Gold", "#af857e"),
    ("L4X", "blue", "Blue", "#061832"),
    ("L4U", "glacier", "Glacier White", "#dce0df"),
    ("L6D", "teal", "Teal Electric Pop", "#699e85"),
];

const UNKNOWN_HEX: &str = "#8a8a8d";

/// Extracts the three-character color code from a hardware-revision string, reproducing the
/// official app's algorithm.
///
/// ```
/// # use ploom_protocol::color::extract_code;
/// assert_eq!(extract_code("U123456VL4U66CJ 170B").as_deref(), Some("L4U"));
/// ```
pub fn extract_code(hardware: &str) -> Option<String> {
    let mut s = hardware.trim();

    if let Some((_, rest)) = s.rsplit_once("GEN4") {
        s = rest;
    } else if s.contains("Ploom") {
        s = s.rsplit(' ').next().unwrap_or(s);
    }
    s = s.strip_prefix("ZZ").unwrap_or(s);

    // Byte indexing is sound here: color codes are ASCII. Bail out otherwise.
    if !s.is_ascii() {
        return None;
    }
    let len = s.len();
    let start = if len == 20 || len == 15 { 8 } else { 7 };
    let end = start + 3;
    if len < end {
        return None;
    }
    Some(s[start..end].to_ascii_uppercase())
}

/// Looks up a catalog entry by its exact three-character code.
pub fn lookup(code: &str) -> Option<DeviceColor> {
    CATALOG
        .iter()
        .find(|(c, ..)| *c == code)
        .map(|&(code, id, name, hex)| DeviceColor {
            code: code.to_owned(),
            id: id.to_owned(),
            name: name.to_owned(),
            hex: hex.to_owned(),
        })
}

/// Decodes the body color from a hardware-revision string.
///
/// ```
/// # use ploom_protocol::color::decode;
/// assert_eq!(decode("U123456VL4U66CJ 170B").unwrap().id, "glacier");
/// ```
pub fn decode(hardware: &str) -> Option<DeviceColor> {
    let code = extract_code(hardware)?;
    Some(lookup(&code).unwrap_or(DeviceColor {
        id: "unknown".to_owned(),
        name: code.clone(),
        hex: UNKNOWN_HEX.to_owned(),
        code,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_device_is_glacier_white() {
        let c = decode("U123456VL4U66CJ 170B").unwrap();
        assert_eq!(c.code, "L4U");
        assert_eq!(c.id, "glacier");
        assert_eq!(c.name, "Glacier White");
        assert_eq!(c.hex, "#dce0df");
    }

    #[test]
    fn every_catalog_code_round_trips() {
        for &(code, id, ..) in CATALOG {
            assert_eq!(lookup(code).unwrap().id, id);
        }
    }

    #[test]
    fn extraction_position_depends_on_length() {
        assert_eq!(extract_code("U123456VL4U66CJ 170B").as_deref(), Some("L4U"));
        assert_eq!(extract_code("U123456VL51XX66").as_deref(), Some("L51"));
        assert_eq!(extract_code("U123456L4W66CJ").as_deref(), Some("L4W"));
    }

    #[test]
    fn unknown_code_keeps_the_code() {
        let c = decode("U123456VL9Z66CJ 170B").unwrap();
        assert_eq!(c.code, "L9Z");
        assert_eq!(c.id, "unknown");
        assert_eq!(c.name, "L9Z");
    }

    #[test]
    fn lowercase_is_normalized() {
        assert_eq!(extract_code("u123456vl4u66cj 170b").as_deref(), Some("L4U"));
    }

    #[test]
    fn too_short_yields_none() {
        assert_eq!(extract_code("123456VL"), None);
        assert_eq!(decode("short"), None);
    }

    #[test]
    fn strips_leading_zz() {
        assert_eq!(
            extract_code("ZZU123456VL4U66CJ 170B").as_deref(),
            Some("L4U")
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serializes_camel_case() {
        let c = decode("U123456VL4U66CJ 170B").unwrap();
        let v: serde_json::Value = serde_json::to_value(&c).unwrap();
        assert_eq!(v["code"], "L4U");
        assert_eq!(v["id"], "glacier");
        assert_eq!(v["hex"], "#dce0df");
    }
}
