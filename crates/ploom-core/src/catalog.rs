//! The static [`Catalog`]: modes, curves, custom-profile limits, dangerous opcodes.

use ploom_protocol::HeatingMode;
use ploom_protocol::heating::{CUSTOM_HOLD_RANGE, CUSTOM_TEMP_RANGE};
use ploom_protocol::opcode::DANGEROUS_OPCODES;

use crate::model::{Catalog, CustomLimits, DangerousOpcode, ModeInfo};

/// Builds the catalog from the protocol constants.
pub fn catalog() -> Catalog {
    Catalog {
        modes: HeatingMode::ALL
            .iter()
            .map(|&mode| ModeInfo {
                id: mode,
                max_sticks: mode.max_sticks(),
                minutes: mode.minutes(),
                heat: mode.heat(),
                curve: mode
                    .curve()
                    .map(|c| c.iter().copied().map(Into::into).collect()),
                hold_step_index: mode.hold_step_index().and_then(|i| u32::try_from(i).ok()),
            })
            .collect(),
        custom: CustomLimits {
            temp_c: (*CUSTOM_TEMP_RANGE.start(), *CUSTOM_TEMP_RANGE.end()),
            hold_secs: (
                u16::from(*CUSTOM_HOLD_RANGE.start()),
                u16::from(*CUSTOM_HOLD_RANGE.end()),
            ),
            bases: HeatingMode::CUSTOM_BASES.to_vec(),
        },
        dangerous_opcodes: DANGEROUS_OPCODES
            .iter()
            .map(|&(opcode, reason)| DangerousOpcode { opcode, reason })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contents() {
        let c = catalog();
        let ids: Vec<_> = c.modes.iter().map(|m| m.id).collect();
        assert_eq!(ids, HeatingMode::ALL);
        let balanced = &c.modes[1];
        assert!(balanced.curve.is_none() && balanced.hold_step_index.is_none());
        let delicate = &c.modes[0];
        assert_eq!(delicate.curve.as_ref().map(Vec::len), Some(8));
        assert_eq!(delicate.hold_step_index, Some(5));
        assert_eq!(
            (delicate.max_sticks, delicate.minutes, delicate.heat),
            (19, 6, 1)
        );
        assert_eq!(c.custom.temp_c, (240, 320));
        assert_eq!(c.custom.hold_secs, (30, 255));
        assert!(!c.custom.bases.contains(&HeatingMode::Balanced));
        assert_eq!(c.dangerous_opcodes.len(), 3);
    }

    #[test]
    fn catalog_json_shape() {
        let json = serde_json::to_value(catalog()).unwrap();
        assert_eq!(json["modes"][0]["id"], "delicate");
        assert_eq!(json["modes"][0]["maxSticks"], 19);
        assert_eq!(json["modes"][0]["holdStepIndex"], 5);
        assert_eq!(json["modes"][0]["curve"][0]["durationS"], 60);
        assert_eq!(json["modes"][0]["curve"][2]["gradient"], -0.013);
        assert_eq!(json["modes"][1]["curve"], serde_json::Value::Null);
        assert_eq!(json["custom"]["tempC"], serde_json::json!([240, 320]));
        assert_eq!(json["dangerousOpcodes"][0]["reason"], "erasesLogs");
    }
}
