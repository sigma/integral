//! Type-dependent FX parameter definitions.
//!
//! Each chorus/reverb type has a different set of meaningful parameters.
//! The param index maps to nibblized parameters at fixed SysEx offsets.

use std::fmt;

/// Definition of a single FX parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxParamDef {
    /// Nibblized parameter index.
    pub index: u8,
    /// Human-readable name.
    pub name: &'static str,
    /// Minimum value (inclusive).
    pub min: i32,
    /// Maximum value (inclusive).
    pub max: i32,
    /// Default value.
    pub default_value: i32,
    /// Display unit/format hint.
    pub unit: FxUnit,
}

/// How to format a parameter value for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxUnit {
    /// Plain integer (no suffix).
    Plain,
    /// Percentage (`%`).
    Percent,
    /// Hertz (`Hz`).
    Hz,
    /// Milliseconds: value is in tenths, display as `X.Xms`.
    MillisTenths,
    /// Milliseconds: value is direct, display as `Xms`.
    Millis,
    /// Hertz with 1/100 resolution: display as `X.XXHz`.
    HzHundredths,
    /// Degrees (`°`).
    Degrees,
    /// Seconds with 1/10 resolution: display as `X.Xs`.
    SecondsTenths,
    /// HF Damp: value >8000 means "BYP", otherwise `XHz`.
    HfDamp,
    /// Filter type: OFF / LPF / HPF.
    FilterType,
}

impl FxParamDef {
    /// Format a raw parameter value for display.
    pub fn format_value(&self, v: i32) -> String {
        match self.unit {
            FxUnit::Plain => v.to_string(),
            FxUnit::Percent => format!("{v}%"),
            FxUnit::Hz => format!("{v}Hz"),
            FxUnit::MillisTenths => format!("{:.1}ms", v as f64 / 10.0),
            FxUnit::Millis => format!("{v}ms"),
            FxUnit::HzHundredths => format!("{:.2}Hz", v as f64 / 100.0),
            FxUnit::Degrees => format!("{v}°"),
            FxUnit::SecondsTenths => format!("{:.1}s", v as f64 / 10.0),
            FxUnit::HfDamp => {
                if v > 8000 {
                    "BYP".to_string()
                } else {
                    format!("{v}Hz")
                }
            }
            FxUnit::FilterType => match v {
                0 => "OFF".to_string(),
                1 => "LPF".to_string(),
                2 => "HPF".to_string(),
                _ => v.to_string(),
            },
        }
    }
}

impl fmt::Display for FxParamDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// ---------------------------------------------------------------------------
// Chorus type parameter maps
// ---------------------------------------------------------------------------

const CHORUS_CHORUS: &[FxParamDef] = &[
    FxParamDef {
        index: 0,
        name: "Filter",
        min: 0,
        max: 2,
        default_value: 0,
        unit: FxUnit::FilterType,
    },
    FxParamDef {
        index: 1,
        name: "Cutoff",
        min: 200,
        max: 8000,
        default_value: 800,
        unit: FxUnit::Hz,
    },
    FxParamDef {
        index: 2,
        name: "PreDly",
        min: 0,
        max: 100,
        default_value: 0,
        unit: FxUnit::MillisTenths,
    },
    FxParamDef {
        index: 3,
        name: "Rate",
        min: 0,
        max: 1000,
        default_value: 100,
        unit: FxUnit::HzHundredths,
    },
    FxParamDef {
        index: 4,
        name: "Depth",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 5,
        name: "Phase",
        min: 0,
        max: 180,
        default_value: 0,
        unit: FxUnit::Degrees,
    },
    FxParamDef {
        index: 6,
        name: "Fdbk",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 7,
        name: "→Rev",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
];

const CHORUS_DELAY: &[FxParamDef] = &[
    FxParamDef {
        index: 0,
        name: "Dly L",
        min: 0,
        max: 1000,
        default_value: 200,
        unit: FxUnit::Millis,
    },
    FxParamDef {
        index: 1,
        name: "Dly R",
        min: 0,
        max: 1000,
        default_value: 200,
        unit: FxUnit::Millis,
    },
    FxParamDef {
        index: 2,
        name: "Dly C",
        min: 0,
        max: 1000,
        default_value: 200,
        unit: FxUnit::Millis,
    },
    FxParamDef {
        index: 3,
        name: "C Fdbk",
        min: -98,
        max: 98,
        default_value: 0,
        unit: FxUnit::Percent,
    },
    FxParamDef {
        index: 4,
        name: "HFDamp",
        min: 200,
        max: 8001,
        default_value: 8001,
        unit: FxUnit::HfDamp,
    },
    FxParamDef {
        index: 5,
        name: "Lv L",
        min: 0,
        max: 127,
        default_value: 127,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 6,
        name: "Lv R",
        min: 0,
        max: 127,
        default_value: 127,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 7,
        name: "Lv C",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 8,
        name: "→Rev",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
];

const CHORUS_GM2: &[FxParamDef] = &[
    FxParamDef {
        index: 0,
        name: "PreLPF",
        min: 0,
        max: 7,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 1,
        name: "Level",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 2,
        name: "Fdbk",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 3,
        name: "Delay",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 4,
        name: "Rate",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 5,
        name: "Depth",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 6,
        name: "→Rev",
        min: 0,
        max: 127,
        default_value: 0,
        unit: FxUnit::Plain,
    },
];

/// Chorus parameter definitions indexed by chorus type (0=OFF, 1=Chorus,
/// 2=Delay, 3=GM2).
pub fn chorus_params(chorus_type: u8) -> &'static [FxParamDef] {
    match chorus_type {
        1 => CHORUS_CHORUS,
        2 => CHORUS_DELAY,
        3 => CHORUS_GM2,
        _ => &[],
    }
}

/// Chorus type display names.
pub static CHORUS_TYPE_NAMES: &[&str] = &["OFF", "Chorus", "Delay", "GM2 Cho"];

// ---------------------------------------------------------------------------
// Reverb type parameter maps
// ---------------------------------------------------------------------------

const REVERB_ROOM_HALL_PLATE: &[FxParamDef] = &[
    FxParamDef {
        index: 0,
        name: "PreDly",
        min: 0,
        max: 100,
        default_value: 0,
        unit: FxUnit::Millis,
    },
    FxParamDef {
        index: 1,
        name: "Time",
        min: 1,
        max: 100,
        default_value: 30,
        unit: FxUnit::SecondsTenths,
    },
    FxParamDef {
        index: 2,
        name: "Densty",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 3,
        name: "Diffus",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 4,
        name: "LFDamp",
        min: 0,
        max: 100,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 5,
        name: "HFDamp",
        min: 0,
        max: 100,
        default_value: 50,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 6,
        name: "Spread",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 7,
        name: "Tone",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
];

const REVERB_GM2: &[FxParamDef] = &[
    FxParamDef {
        index: 0,
        name: "Char",
        min: 0,
        max: 5,
        default_value: 0,
        unit: FxUnit::Plain,
    },
    FxParamDef {
        index: 1,
        name: "Time",
        min: 0,
        max: 127,
        default_value: 64,
        unit: FxUnit::Plain,
    },
];

/// Reverb parameter definitions indexed by reverb type (0=OFF, 1–5=Room/Hall/Plate,
/// 6=GM2).
pub fn reverb_params(reverb_type: u8) -> &'static [FxParamDef] {
    match reverb_type {
        1..=5 => REVERB_ROOM_HALL_PLATE,
        6 => REVERB_GM2,
        _ => &[],
    }
}

/// Reverb type display names.
pub static REVERB_TYPE_NAMES: &[&str] = &[
    "OFF", "Room 1", "Room 2", "Hall 1", "Hall 2", "Plate", "GM2 Rev",
];

/// Chorus output routing names.
pub static CHORUS_OUTPUT_NAMES: &[&str] = &["MAIN", "REV", "MAIN+REV"];

/// Reverb output routing names.
pub static REVERB_OUTPUT_NAMES: &[&str] = &["A", "B", "C", "D"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chorus_off_has_no_params() {
        assert!(chorus_params(0).is_empty());
    }

    #[test]
    fn chorus_type_1_has_8_params() {
        assert_eq!(chorus_params(1).len(), 8);
    }

    #[test]
    fn chorus_delay_has_9_params() {
        assert_eq!(chorus_params(2).len(), 9);
    }

    #[test]
    fn reverb_types_1_through_5_share_params() {
        for t in 1..=5 {
            assert_eq!(reverb_params(t).len(), 8);
        }
    }

    #[test]
    fn reverb_gm2_has_2_params() {
        assert_eq!(reverb_params(6).len(), 2);
    }

    #[test]
    fn format_filter_type() {
        let p = &chorus_params(1)[0];
        assert_eq!(p.format_value(0), "OFF");
        assert_eq!(p.format_value(1), "LPF");
        assert_eq!(p.format_value(2), "HPF");
    }

    #[test]
    fn format_hf_damp_bypass() {
        let p = &chorus_params(2)[4];
        assert_eq!(p.format_value(8001), "BYP");
        assert_eq!(p.format_value(4000), "4000Hz");
    }

    #[test]
    fn format_millis_tenths() {
        let p = &chorus_params(1)[2]; // PreDly
        assert_eq!(p.format_value(15), "1.5ms");
    }

    #[test]
    fn format_hz_hundredths() {
        let p = &chorus_params(1)[3]; // Rate
        assert_eq!(p.format_value(350), "3.50Hz");
    }

    #[test]
    fn format_seconds_tenths() {
        let p = &reverb_params(1)[1]; // Time
        assert_eq!(p.format_value(30), "3.0s");
    }
}
