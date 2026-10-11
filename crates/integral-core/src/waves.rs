//! Catalog of the INTEGRA-7's Waves, generated at build time from
//! `docs/waves/*.json`.
//!
//! A PCM Synth partial or a PCM Drum WMT picks a Wave by Wave Group (Type +
//! ID) and Wave Number. An SN Synth OSC in PCM mode picks one from the
//! separate SN Synth list. Wave Number 0 is OFF.
//!
//! Names are Roland's INTEGRA-7 Editor spelling. See `docs/waves/README.md`
//! and `docs/research/wave-names.md`.

use crate::expansions::{self, Expansion};

/// One named Wave of a Wave Group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Wave {
    /// Wave Number (1…N; 0 is OFF and has no entry).
    pub number: u16,
    /// Wave name (up to 12 characters).
    pub name: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/wave_catalog_data.rs"));

/// A list of Waves selectable together.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum WaveGroup {
    /// The internal Waves (Group Type 0).
    Int,
    /// The Waves of SRX board `n` (Group Type 1, Group ID `n`, 1..=12).
    Srx(u8),
    /// The Waves an SN Synth OSC picks in PCM mode.
    SnSynth,
}

impl WaveGroup {
    /// Every Wave Group: INT, SRX-01…SRX-12, then SN Synth.
    pub const ALL: [WaveGroup; 14] = [
        Self::Int,
        Self::Srx(1),
        Self::Srx(2),
        Self::Srx(3),
        Self::Srx(4),
        Self::Srx(5),
        Self::Srx(6),
        Self::Srx(7),
        Self::Srx(8),
        Self::Srx(9),
        Self::Srx(10),
        Self::Srx(11),
        Self::Srx(12),
        Self::SnSynth,
    ];

    /// The group selected by a partial's or WMT's Wave Group Type and ID.
    ///
    /// Type 0 is INT, type 1 is SRX board `id`. Reserved types and SRX ids
    /// outside 1..=12 give `None`.
    pub fn from_params(group_type: u8, group_id: u8) -> Option<Self> {
        match (group_type, group_id) {
            (0, _) => Some(Self::Int),
            (1, 1..=12) => Some(Self::Srx(group_id)),
            _ => None,
        }
    }

    /// The Wave Group Type and ID to write for this group, or `None` for SN
    /// Synth, which has neither.
    pub fn params(self) -> Option<(u8, u8)> {
        match self {
            Self::Int => Some((0, 0)),
            Self::Srx(n) => Some((1, n)),
            Self::SnSynth => None,
        }
    }

    /// Identifier as `expansion load` spells it: `INT`, `SRX-01`…`SRX-12`,
    /// `SN-Synth`.
    pub fn id(self) -> &'static str {
        match self {
            Self::Int => "INT",
            Self::Srx(_) => self.expansion().map_or("SRX-??", |e| e.name),
            Self::SnSynth => "SN-Synth",
        }
    }

    /// The group named by `id` (case-insensitive), the inverse of [`id`](Self::id).
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|g| g.id().eq_ignore_ascii_case(id))
    }

    /// The SRX Expansion providing this group's Waves, or `None` for INT and
    /// SN Synth.
    pub fn expansion(self) -> Option<&'static Expansion> {
        match self {
            Self::Srx(n) => expansions::EXPANSIONS
                .iter()
                .find(|e| e.wave_group == Some(n)),
            Self::Int | Self::SnSynth => None,
        }
    }
}

/// Every Wave of `group`, in Wave Number order (OFF excluded).
pub fn waves(group: WaveGroup) -> &'static [Wave] {
    match group {
        WaveGroup::Int => WAVES_INT,
        WaveGroup::Srx(1) => WAVES_SRX_01,
        WaveGroup::Srx(2) => WAVES_SRX_02,
        WaveGroup::Srx(3) => WAVES_SRX_03,
        WaveGroup::Srx(4) => WAVES_SRX_04,
        WaveGroup::Srx(5) => WAVES_SRX_05,
        WaveGroup::Srx(6) => WAVES_SRX_06,
        WaveGroup::Srx(7) => WAVES_SRX_07,
        WaveGroup::Srx(8) => WAVES_SRX_08,
        WaveGroup::Srx(9) => WAVES_SRX_09,
        WaveGroup::Srx(10) => WAVES_SRX_10,
        WaveGroup::Srx(11) => WAVES_SRX_11,
        WaveGroup::Srx(12) => WAVES_SRX_12,
        WaveGroup::Srx(_) => &[],
        WaveGroup::SnSynth => WAVES_SN_SYNTH,
    }
}

/// The name of Wave `number` in `group`: `"OFF"` for 0, `None` past the end.
pub fn name(group: WaveGroup, number: u16) -> Option<&'static str> {
    match number {
        0 => Some("OFF"),
        n => waves(group).get(usize::from(n) - 1).map(|w| w.name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_params_int_srx_and_reserved() {
        assert_eq!(WaveGroup::from_params(0, 0), Some(WaveGroup::Int));
        assert_eq!(WaveGroup::from_params(1, 7), Some(WaveGroup::Srx(7)));
        assert_eq!(WaveGroup::from_params(2, 0), None);
        assert_eq!(WaveGroup::from_params(1, 0), None);
        assert_eq!(WaveGroup::from_params(1, 13), None);
    }

    #[test]
    fn params_round_trip() {
        for group in WaveGroup::ALL {
            if let Some((t, id)) = group.params() {
                assert_eq!(WaveGroup::from_params(t, id), Some(group));
            }
        }
        assert_eq!(WaveGroup::SnSynth.params(), None);
    }

    #[test]
    fn id_round_trip() {
        for group in WaveGroup::ALL {
            assert_eq!(WaveGroup::from_id(group.id()), Some(group));
        }
        assert_eq!(WaveGroup::Srx(7).id(), "SRX-07");
        assert_eq!(WaveGroup::from_id("srx-07"), Some(WaveGroup::Srx(7)));
        assert_eq!(WaveGroup::from_id("sn-synth"), Some(WaveGroup::SnSynth));
        assert_eq!(WaveGroup::from_id("SRX-13"), None);
    }

    #[test]
    fn names() {
        assert_eq!(name(WaveGroup::Int, 1), Some("StGrand pA L"));
        assert_eq!(name(WaveGroup::SnSynth, 1), Some("JP-8 Saw"));
        // Parameter Guide example.
        assert_eq!(name(WaveGroup::Srx(5), 3), Some("080:BladeBtL"));
        assert_eq!(name(WaveGroup::Int, 0), Some("OFF"));
        assert_eq!(name(WaveGroup::Int, 1084), None);
    }

    #[test]
    fn expansion_link() {
        let srx07 = expansions::by_name("SRX-07").unwrap();
        assert_eq!(WaveGroup::Srx(7).expansion(), Some(srx07));
        assert_eq!(srx07.waves().len(), 860);
        assert_eq!(srx07.waves(), waves(WaveGroup::Srx(7)));
        assert!(expansions::by_name("ExSN1").unwrap().waves().is_empty());
        assert_eq!(WaveGroup::Int.expansion(), None);
    }
}
