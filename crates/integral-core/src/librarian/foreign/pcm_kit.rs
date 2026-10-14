//! The PCM Drum Kit a Foreign Rhythm Set converts into: an INIT kit, its
//! per-key Partials and WMT Waves, and the fields only the INTEGRA-7 has.
//!
//! Reference: `docs/midi/07-pcm-drum-kit.md`, `docs/svd/07-pcm-drum.md`
//! (reserves with content) and `docs/research/foreign-rhythm-format.md`.

use super::fx_map;
use super::pcm_tone::{self, map_wave_pair};
use super::report::{ConversionReport, Location};
use super::wave_map::ForeignWaveGroup;
use crate::librarian::pack::PackEntry;
use crate::parse_helpers::{encode_ascii_name, encode_nibble4};
use crate::pcm_drum::{
    PCMD_FIRST_KEY, PCMD_KEY_COUNT, PCMD_WMT_COUNT, PcmDrumKitState, PcmDrumPartial, PcmDrumWmt,
    PcmdWmtField, encode_pcmd_common, encode_pcmd_common2, encode_pcmd_partial, encode_pcmd_wmt,
    pcmd_wmt_byte,
};
use crate::state::DrumCompEqState;
use crate::state::parse::encode_comp_eq_block;
use crate::svd::ChunkType;
use crate::waves::WaveGroup;

/// Index of the MFX block.
const MFX: usize = 1;
/// Index of key 21's Partial block.
const FIRST_PARTIAL: usize = 3;
/// Index of the Common 2 block.
const COMMON2: usize = FIRST_PARTIAL + PCMD_KEY_COUNT;

/// Common 2: bytes of the long name, a reserve that real kits fill.
const LONG_NAME_LEN: usize = 16;

/// The PCM Drum Kit blocks, in the order of the `pcm-drum` layout of
/// [`USER_MEMORY`](crate::device_spec::USER_MEMORY): Common, MFX, Comp/EQ,
/// the Partials of keys 21–108, Common 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PcmKit {
    blocks: Vec<Vec<u8>>,
}

impl PcmKit {
    /// An INIT kit: every block holds the state defaults (MFX THRU, Comp/EQ
    /// off, every WMT off) except TFX Switch ON, since a Foreign kit's MFX
    /// always runs, and the reserves what INTEGRA-7 kits carry.
    pub(super) fn init() -> Self {
        let mut state = PcmDrumKitState::default();
        state.common2.tfx_switch = 1;
        let partial = encode_pcmd_partial(&PcmDrumPartial::default());
        let mut blocks = vec![
            encode_pcmd_common(&state.common),
            pcm_tone::init_mfx(state.mfx),
            encode_comp_eq_block(&DrumCompEqState::default().units),
        ];
        blocks.extend(std::iter::repeat_n(partial, PCMD_KEY_COUNT));
        blocks.push(encode_pcmd_common2(&state.common2));
        Self { blocks }
    }

    /// The Common block.
    pub(super) fn common(&mut self) -> &mut [u8] {
        &mut self.blocks[0]
    }

    /// The MFX block.
    pub(super) fn mfx(&mut self) -> &mut [u8] {
        &mut self.blocks[MFX]
    }

    /// The Partial block of `key` (21–108).
    pub(super) fn key(&mut self, key: u8) -> &mut [u8] {
        &mut self.blocks[FIRST_PARTIAL + usize::from(key - PCMD_FIRST_KEY)]
    }

    /// Whether WMT `wmt` (0-indexed) of `key` is switched on.
    pub(super) fn wmt_on(&mut self, key: u8, wmt: usize) -> bool {
        self.key(key)[pcmd_wmt_byte(wmt, PcmdWmtField::Switch)] != 0
    }

    /// Reset WMT `wmt` (0-indexed) of `key` to the default, switched off.
    pub(super) fn reset_wmt(&mut self, key: u8, wmt: usize) {
        let at = pcmd_wmt_byte(wmt, PcmdWmtField::Switch);
        encode_pcmd_wmt(&PcmDrumWmt::default(), &mut self.key(key)[at..]);
    }

    /// Whether the kit follows the tempo: a switched-on WMT has Wave Tempo
    /// Sync on, or the MFX has a rate or delay at a note value.
    pub(super) fn syncs_to_tempo(&self) -> bool {
        fx_map::syncs_to_tempo(&self.blocks[MFX])
            || self.blocks[FIRST_PARTIAL..COMMON2].iter().any(|partial| {
                (0..PCMD_WMT_COUNT).any(|w| {
                    let on = |f| partial[pcmd_wmt_byte(w, f)] != 0;
                    on(PcmdWmtField::Switch) && on(PcmdWmtField::WaveTempoSync)
                })
            })
    }

    /// Write INTEGRA-7 Waves into WMT `wmt` (0-indexed) of `key`: Wave Group
    /// Type, Group ID, and Wave Numbers L and R.
    pub(super) fn write_waves(&mut self, key: u8, wmt: usize, group: WaveGroup, numbers: [u16; 2]) {
        let (group_type, group_id) = group.params().expect("a PCM Wave Group");
        let partial = self.key(key);
        partial[pcmd_wmt_byte(wmt, PcmdWmtField::WaveGroupType)] = group_type;
        let fields = [
            (PcmdWmtField::WaveGroupId, group_id.into()),
            (PcmdWmtField::WaveNumberL, numbers[0]),
            (PcmdWmtField::WaveNumberR, numbers[1]),
        ];
        for (field, value) in fields {
            let at = pcmd_wmt_byte(wmt, field);
            partial[at..at + 4].copy_from_slice(&encode_nibble4(value));
        }
    }

    /// Map the Foreign Waves L and R (0 = OFF) of `group` in WMT `wmt`
    /// (0-indexed) of `key` through the Wave map and write them. The WMT is
    /// reported only when it is switched on; when its Waves don't map, it
    /// gets no Wave and is switched off.
    pub(super) fn map_waves(
        &mut self,
        key: u8,
        wmt: usize,
        group: ForeignWaveGroup,
        numbers: [u16; 2],
        report: &mut ConversionReport,
    ) {
        let at = Location::Wmt {
            key,
            wmt: wmt as u8 + 1,
        };
        let used = self.wmt_on(key, wmt);
        match map_wave_pair(group, numbers, at, used, report) {
            Some((group, mapped)) => self.write_waves(key, wmt, group, mapped),
            None => {
                self.write_waves(key, wmt, WaveGroup::Int, [0, 0]);
                self.key(key)[pcmd_wmt_byte(wmt, PcmdWmtField::Switch)] = 0;
            }
        }
    }

    /// The Pack entry, with the kit name space-padded into the long name.
    pub(super) fn into_entry(mut self) -> PackEntry {
        let name_len = ChunkType::PcmDrumKit.name_length().expect("a named type");
        let name = String::from_utf8_lossy(&self.blocks[0][..name_len]).into_owned();
        self.blocks[COMMON2][..LONG_NAME_LEN]
            .copy_from_slice(&encode_ascii_name(&name, LONG_NAME_LEN));
        PackEntry::new(ChunkType::PcmDrumKit, None, self.blocks).expect("PCM Drum Kit layout")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_helpers::nibble4;
    use crate::pcm_drum::{parse_pcmd_common, parse_pcmd_common2, parse_pcmd_partial};

    /// Index of the Comp/EQ block.
    const COMP_EQ: usize = 2;

    #[test]
    fn init_holds_the_state_defaults() {
        let mut kit = PcmKit::init();
        kit.common()[..4].copy_from_slice(b"Init");
        let entry = kit.into_entry();
        let blocks = entry.blocks();
        assert_eq!(blocks.len(), COMMON2 + 1);
        assert_eq!(entry.name(), "Init");
        let state = PcmDrumKitState::default();
        assert_eq!(
            parse_pcmd_common(&blocks[0]).unwrap().kit_level,
            state.common.kit_level
        );
        assert_eq!(&blocks[0][0x0D..], [0, 7, 8, 0, 13]);
        assert_eq!((blocks[MFX][0x00], blocks[MFX][0x01]), (0, 127));
        // Comp and EQ Switches, at bytes 0 and 6 of each 14-byte unit.
        let switches = blocks[COMP_EQ]
            .iter()
            .enumerate()
            .filter(|(i, _)| matches!(i % 14, 0 | 6));
        assert!(switches.map(|(_, &b)| b).all(|b| b == 0));
        assert_eq!(
            parse_pcmd_partial(&blocks[FIRST_PARTIAL]).unwrap(),
            PcmDrumPartial::default()
        );
        assert_eq!(&blocks[COMMON2][..LONG_NAME_LEN], b"Init            ");
        let mut common2 = state.common2;
        common2.tfx_switch = 1;
        assert_eq!(parse_pcmd_common2(&blocks[COMMON2]).unwrap(), common2);
    }

    #[test]
    fn maps_foreign_waves_or_switches_the_wmt_off() {
        let mut kit = PcmKit::init();
        let switch = pcmd_wmt_byte(1, PcmdWmtField::Switch);
        kit.key(36)[switch] = 1;
        let mut report = ConversionReport::new();
        // SR-JV80-02 Waves 1 and 2 are SRX-06 Waves 29 and 30, both high.
        kit.map_waves(36, 1, ForeignWaveGroup::SrJv80(2), [1, 2], &mut report);
        let wmt = |kit: &mut PcmKit, field| nibble4(kit.key(36), pcmd_wmt_byte(1, field));
        assert_eq!(
            kit.key(36)[pcmd_wmt_byte(1, PcmdWmtField::WaveGroupType)],
            1
        );
        assert_eq!(wmt(&mut kit, PcmdWmtField::WaveGroupId), 6);
        assert_eq!(wmt(&mut kit, PcmdWmtField::WaveNumberL), 29);
        assert_eq!(wmt(&mut kit, PcmdWmtField::WaveNumberR), 30);
        assert!(kit.wmt_on(36, 1) && report.is_empty());

        // SR-JV80-01 Wave 1 has no match.
        kit.map_waves(36, 1, ForeignWaveGroup::SrJv80(1), [1, 0], &mut report);
        assert!(!kit.wmt_on(36, 1));
        assert_eq!(
            report.iter().map(ToString::to_string).collect::<Vec<_>>(),
            ["Key 36 WMT 2: SR-JV80-01 Wave 1 (Grand sft 1A) has no INTEGRA-7 Wave"]
        );
    }
}
