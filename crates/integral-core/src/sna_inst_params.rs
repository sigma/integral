//! SuperNATURAL Acoustic instrument-specific parameter definitions.
//!
//! Each SN-A instrument has its own "Modify Parameters" on the INST tab, at
//! fixed slots of the 32 generic ones in
//! [`crate::sn_acoustic::SnAcousticCommon`]. The slots and names come from
//! Roland's Editor list (`docs/sn-acoustic-insts/`), the ranges from the
//! Parameter Guide; the acoustic pianos and TW Organ, which the Editor gives
//! pages of their own, keep their page here.

use crate::sn_acoustic::SnaInst;

/// Definition of a single SN-A instrument parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnaInstParamDef {
    /// Modify parameter index (1–32).
    pub index: u8,
    /// Human-readable name.
    pub name: &'static str,
    /// Minimum raw value (inclusive).
    pub min: u8,
    /// Maximum raw value (inclusive).
    pub max: u8,
    /// Default raw value.
    pub default_value: u8,
}

// ---------------------------------------------------------------------------
// The two instruments with pages of their own
// ---------------------------------------------------------------------------

// shortcut: the Ac.Piano and TW Organ slots are numbered in Parameter Guide order (1, 2, …), unlike the other instruments' Editor-given slots; check them against the Editor's page code or the device.
const AC_PIANO_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "String Resonance",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 2,
        name: "Key Off Resonance",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 3,
        name: "Hammer Noise",
        min: 0,
        max: 4,
        default_value: 2,
    },
    SnaInstParamDef {
        index: 4,
        name: "Stereo Width",
        min: 0,
        max: 63,
        default_value: 32,
    },
    SnaInstParamDef {
        index: 5,
        name: "Nuance",
        min: 0,
        max: 2,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 6,
        name: "Tone Character",
        min: 0,
        max: 10,
        default_value: 5,
    },
];

const TW_ORGAN_PARAMS: &[SnaInstParamDef] = &[
    SnaInstParamDef {
        index: 1,
        name: "Harmonic Bar 16'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 2,
        name: "Harmonic Bar 5-1/3'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 3,
        name: "Harmonic Bar 8'",
        min: 0,
        max: 8,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 4,
        name: "Harmonic Bar 4'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 5,
        name: "Harmonic Bar 2-2/3'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 6,
        name: "Harmonic Bar 2'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 7,
        name: "Harmonic Bar 1-3/5'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 8,
        name: "Harmonic Bar 1-1/3'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 9,
        name: "Harmonic Bar 1'",
        min: 0,
        max: 8,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 10,
        name: "Leakage Level",
        min: 0,
        max: 127,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 11,
        name: "Percussion Switch",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 12,
        name: "Percussion Soft",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 13,
        name: "Percussion Soft Level",
        min: 0,
        max: 15,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 14,
        name: "Percussion Normal Level",
        min: 0,
        max: 15,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 15,
        name: "Percussion Slow",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 16,
        name: "Percussion Slow Time",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 17,
        name: "Percussion Fast Time",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 18,
        name: "Percussion Harmonic",
        min: 0,
        max: 1,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 19,
        name: "Percussion Recharge Time",
        min: 0,
        max: 15,
        default_value: 8,
    },
    SnaInstParamDef {
        index: 20,
        name: "Percussion Harmonic Bar Level",
        min: 0,
        max: 127,
        default_value: 64,
    },
    SnaInstParamDef {
        index: 21,
        name: "Key On Click Level",
        min: 0,
        max: 31,
        default_value: 0,
    },
    SnaInstParamDef {
        index: 22,
        name: "Key Off Click Level",
        min: 0,
        max: 31,
        default_value: 0,
    },
];

// ---------------------------------------------------------------------------
// Every instrument
// ---------------------------------------------------------------------------

/// Which INST page an instrument gets: one of its own, or the generic one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnaInstPage {
    /// The acoustic pianos' page.
    AcPiano,
    /// TW Organ's drawbar page.
    TwOrgan,
    /// The generic page of its Modify Parameters.
    Generic,
}

/// The INST page of `inst`.
pub fn sna_inst_page(inst: &SnaInst) -> SnaInstPage {
    if !inst.modify.is_empty() {
        SnaInstPage::Generic
    } else if inst.category == "Ac.Piano" {
        SnaInstPage::AcPiano
    } else if inst.name == "TW Organ" {
        SnaInstPage::TwOrgan
    } else {
        SnaInstPage::Generic
    }
}

/// The Modify Parameters of `inst`, at their slots (1-32).
pub fn sna_inst_params(inst: &SnaInst) -> Vec<SnaInstParamDef> {
    match sna_inst_page(inst) {
        SnaInstPage::AcPiano => AC_PIANO_PARAMS.to_vec(),
        SnaInstPage::TwOrgan => TW_ORGAN_PARAMS.to_vec(),
        SnaInstPage::Generic => inst
            .modify
            .iter()
            .map(|&(index, name)| {
                let (min, max, default_value) = modify_range(name);
                SnaInstParamDef {
                    index,
                    name,
                    min,
                    max,
                    default_value,
                }
            })
            .collect(),
    }
}

/// Raw range and default of a Modify Parameter, by its name in Roland's list
/// (values per the Parameter Guide's SN-A pages).
fn modify_range(name: &str) -> (u8, u8, u8) {
    match name {
        // -64…+63
        "Noise Level" | "Mallet Hardness" | "Roll Speed" | "Resonance Level" | "Strum Speed"
        | "Tremolo Speed" | "Bend Depth" | "Crescendo Depth" | "Sub String Tune"
        | "Drone Level" | "Tambura Level" => (0, 127, 64),
        // 0…127
        "Growl Sens" => (0, 127, 0),
        // -12…+12 semitones
        // shortcut: the guide gives -12…+12; the raw 52-76 (centred on 64, as other ±12 parameters) is assumed, not documented; device check: #252.
        "Drone Pitch" | "Tambura Pitch" => (52, 76, 64),
        // OFF/ON, and Glide's Porta/Gliss
        "Strum Mode" | "Picking Harmonics" | "Glissando Mode" | "Hold Legato Mode"
        | "Buzz Key Sw" | "Glide" => (0, 1, 0),
        // Six scales (harp/koto: 7th, Major, Minor, Hrm-Mi, Dim, Whole; winds: Chroma, Major, Minor, 7th, Dim, Whole)
        "Play Scale" => (0, 5, 0),
        // C … B
        "Scale Key" => (0, 11, 0),
        // OFF, then up to 4 performance variations (Parameter Guide p. 28)
        "Variation" => (0, 4, 0),
        _ => panic!("no range for SN-A Modify Parameter {name:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sn_acoustic::{sna_inst, sna_insts};

    fn params(variation: u8, number: u8) -> Vec<(u8, &'static str)> {
        sna_inst_params(sna_inst(variation, number).unwrap())
            .iter()
            .map(|p| (p.index, p.name))
            .collect()
    }

    #[test]
    fn every_instrument_has_a_range_for_each_of_its_parameters() {
        for inst in sna_insts() {
            for p in sna_inst_params(inst) {
                assert!((1..=32).contains(&p.index), "{}: {}", inst.name, p.name);
                assert!(
                    p.min <= p.default_value && p.default_value <= p.max,
                    "{}: {}",
                    inst.name,
                    p.name
                );
            }
        }
    }

    #[test]
    fn parameters_sit_at_rolands_slots() {
        // Glockenspiel: Variation is Modify Parameter 10, not 3.
        assert_eq!(
            params(0, 9),
            [(1, "Mallet Hardness"), (2, "Roll Speed"), (10, "Variation")]
        );
        assert_eq!(params(0, 4), [(1, "Noise Level")]); // Pure Vintage EP1
    }

    #[test]
    fn pianos_and_tw_organ_get_their_own_pages() {
        assert_eq!(
            sna_inst_page(sna_inst(64, 0).unwrap()),
            SnaInstPage::AcPiano
        );
        assert_eq!(
            sna_inst_page(sna_inst(65, 0).unwrap()),
            SnaInstPage::TwOrgan
        );
        assert_eq!(params(65, 0).len(), TW_ORGAN_PARAMS.len());
        assert_eq!(sna_inst_page(sna_inst(0, 9).unwrap()), SnaInstPage::Generic);
    }
}
