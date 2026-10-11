//! The Foreign Wave map: where a JV/XP internal Wave or an SR-JV80 board
//! Wave lands on the INTEGRA-7, generated at build time from
//! `docs/research/jv-wave-map.json`.
//!
//! Shared by every Foreign patch converter. Reference:
//! `docs/research/jv-wave-map.md` and `docs/research/map-check-roland-pairs.md`.

use core::fmt;

use crate::waves::WaveGroup;

/// A Wave Group of a Foreign synth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", content = "id"))]
pub enum ForeignWaveGroup {
    /// JV/XP internal ROM, INT-A.
    JvIntA,
    /// JV/XP internal ROM, INT-B.
    JvIntB,
    /// JV/XP PCM card `n` (SO-PCM1 series), which no Wave map covers.
    PcmCard(u16),
    /// SR-JV80 expansion board `n` (1–19).
    SrJv80(u16),
    /// XV SRX board `n`, outside the INTEGRA-7's SRX-01…SRX-12.
    Srx(u16),
    /// XV-5080 user samples (Wave Group SAMPLE).
    Sample,
}

impl fmt::Display for ForeignWaveGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JvIntA => f.write_str("INT-A"),
            Self::JvIntB => f.write_str("INT-B"),
            Self::PcmCard(n) => write!(f, "PCM card {n}"),
            Self::SrJv80(n) => write!(f, "SR-JV80-{n:02}"),
            Self::Srx(n) => write!(f, "SRX-{n:02}"),
            Self::Sample => f.write_str("SAMPLE"),
        }
    }
}

/// One Wave of a Foreign synth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ForeignWave {
    /// Its Wave Group.
    pub group: ForeignWaveGroup,
    /// Its Wave Number, from 1.
    pub number: u16,
}

impl ForeignWave {
    /// The Wave's name, when the map lists it.
    pub fn name(self) -> Option<&'static str> {
        self.entry().map(|w| w.name)
    }

    /// Where the Wave lands on the INTEGRA-7, or `None` when it has no
    /// equivalent.
    pub fn target(self) -> Option<WaveTarget> {
        self.entry().and_then(|w| w.target)
    }

    fn entry(self) -> Option<&'static SourceWave> {
        source_waves(self.group).get(usize::from(self.number).checked_sub(1)?)
    }
}

impl fmt::Display for ForeignWave {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Wave {}", self.group, self.number)?;
        match self.name() {
            Some(name) => write!(f, " ({name})"),
            None => Ok(()),
        }
    }
}

/// How sure the map is of a target, graded in `jv-wave-map.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Confidence {
    /// Name, order or a single source alone.
    Low,
    /// Two pieces of evidence.
    Medium,
    /// Name, order and Roland's word, or Roland's own conversion.
    High,
}

/// An INTEGRA-7 Wave a Foreign Wave maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WaveTarget {
    /// Its Wave Group: INT or an SRX board.
    pub group: WaveGroup,
    /// Its Wave Number.
    pub number: u16,
    /// How sure the map is.
    pub confidence: Confidence,
}

/// One entry of the generated map.
struct SourceWave {
    name: &'static str,
    target: Option<WaveTarget>,
}

include!(concat!(env!("OUT_DIR"), "/foreign_wave_map_data.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waves;

    fn wave(group: ForeignWaveGroup, number: u16) -> ForeignWave {
        ForeignWave { group, number }
    }

    #[test]
    fn jv_internal_waves_land_on_int() {
        // INT-A 1 `Ac Piano1 A` is INT 19 (jv-wave-map.md, TL;DR).
        let piano = wave(ForeignWaveGroup::JvIntA, 1);
        assert_eq!(piano.name(), Some("Ac Piano1 A"));
        let target = piano.target().unwrap();
        assert_eq!((target.group, target.number), (WaveGroup::Int, 19));
    }

    #[test]
    fn sr_jv80_waves_land_on_srx_boards() {
        // SR-JV80-02 Wave 1 `Vl Sect A` is SRX-06 Wave 29.
        let violins = wave(ForeignWaveGroup::SrJv80(2), 1);
        assert_eq!(
            violins.target(),
            Some(WaveTarget {
                group: WaveGroup::Srx(6),
                number: 29,
                confidence: Confidence::High,
            })
        );
        assert_eq!(waves::name(WaveGroup::Srx(6), 29), violins.name());
    }

    #[test]
    fn unlisted_and_unmapped_waves_have_no_target() {
        // SR-JV80-01 Wave 1 has no candidate; SR-JV80-06 has no Wave list.
        assert_eq!(wave(ForeignWaveGroup::SrJv80(1), 1).target(), None);
        assert!(wave(ForeignWaveGroup::SrJv80(1), 1).name().is_some());
        assert_eq!(wave(ForeignWaveGroup::SrJv80(6), 1).name(), None);
        assert_eq!(wave(ForeignWaveGroup::SrJv80(2), 0).target(), None);
        assert_eq!(wave(ForeignWaveGroup::SrJv80(2), 175).target(), None);
        assert_eq!(wave(ForeignWaveGroup::Sample, 1).target(), None);
    }

    #[test]
    fn names_the_wave_in_reports() {
        assert_eq!(
            wave(ForeignWaveGroup::SrJv80(2), 1).to_string(),
            "SR-JV80-02 Wave 1 (Vl Sect A)"
        );
        assert_eq!(
            wave(ForeignWaveGroup::Sample, 3).to_string(),
            "SAMPLE Wave 3"
        );
    }
}
