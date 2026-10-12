//! The PCM Synth tone a Foreign patch converts into: the pieces every
//! converter shares. Its Waves, its send pair, and the fields only the
//! INTEGRA-7 has.
//!
//! Reference: `docs/midi/06-pcm-synth-tone.md`, `docs/svd/06-pcm-synth.md`
//! (reserves with content) and `docs/research/xv-patch-format.md` (section 3,
//! Integra-only parameters).

use super::fx_map;
use super::report::{ConversionItem, ConversionReport, Location};
use super::wave_map::{Confidence, ForeignWave, ForeignWaveGroup};
use crate::librarian::pack::{PackEntry, user_memory};
use crate::mfx::{MfxState, encode_mfx_block};
#[cfg(test)]
use crate::parse_helpers::nibble4;
use crate::parse_helpers::{encode_ascii_name, encode_nibble4, nibble2};
use crate::pcm_synth::{
    PCMS_COMMON2_SIZE, PCMS_PARTIAL_COUNT, PcmSynthCommon2, PcmSynthPartial, PcmSynthState,
    PcmsPartialField as F, encode_pcms_common, encode_pcms_partial, encode_pcms_pmt,
};
use crate::svd::ChunkType;
use crate::waves::WaveGroup;

/// PMT: Partial n Switch sits at `00 05` + 9 × (n − 1).
const PMT_SWITCH: usize = 0x05;
/// PMT: bytes per partial entry.
const PMT_STRIDE: usize = 9;

/// Partial: Chorus Send Level, read by the INTEGRA-7.
const CHORUS_SEND: usize = 0x0F;
/// Partial: Reverb Send Level, read by the INTEGRA-7.
const REVERB_SEND: usize = 0x10;
/// Partial: reserves that keep the unused send pair (the XV "(MFX)" sends).
const UNUSED_SENDS: [usize; 2] = [0x0D, 0x0E];
/// Partial: reserve that keeps whether the partial went through the MFX.
const OUTPUT: usize = 0x11;
/// Partial: Wave Group Type.
const WAVE_GROUP_TYPE: usize = 0x27;
/// Partial: Wave Group ID (4 nibbles).
const WAVE_GROUP_ID: usize = 0x28;
/// Partial: Wave Number L and R (4 nibbles each).
const WAVE_NUMBERS: [usize; 2] = [0x2C, 0x30];
/// Partial: LFO Step Type, then LFO Steps 1–16.
const LFO_STEP_TYPE: usize = 0x89;

/// Common 2: bytes of the long name, a reserve that real tones fill.
const LONG_NAME_LEN: usize = 16;
/// Common 2: Tone Category.
const TONE_CATEGORY: usize = 0x10;
/// Common 2: Phrase Octave Shift.
const PHRASE_OCTAVE_SHIFT: usize = 0x13;
/// Common 2: TFX Switch.
const TFX_SWITCH: usize = 0x33;
/// Common 2: Tone Number and Phrase Number (nibblized).
const NIBBLIZED: [(usize, usize); 2] = [(0x11, 2), (0x38, 4)];

/// Value of an unused send pair, as Roland's own Patches store it.
const UNUSED_SEND_LEVEL: u8 = 127;

/// Common: reserve that keeps the XV Patch Output Assign.
const COMMON_OUTPUT_ASSIGN: usize = 0x27;
/// The XV Patch Output Assign TONE, which every INTEGRA-7 tone carries in
/// its Common reserve.
pub(super) const OUTPUT_TONE: u8 = 13;
/// MFX: reserve that keeps the XV MFX Dry Send Level.
const MFX_DRY_SEND: usize = 0x01;
/// The level every INTEGRA-7 tone carries in its MFX Dry Send reserve.
pub(super) const FULL_LEVEL: u8 = 127;

/// The PCM Synth tone blocks, in the order of the `pcm-synth` layout of
/// [`USER_MEMORY`](crate::device_spec::USER_MEMORY): Common, MFX, PMT,
/// Partials 1–4, Common 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PcmTone {
    blocks: Vec<Vec<u8>>,
}

/// The first note value of an LFO Rate or Delay Time (128–149).
const FIRST_NOTE: u16 = 128;

/// Index of the MFX block.
const MFX: usize = 1;
/// Index of the PMT block.
const PMT: usize = 2;
/// Index of Partial 1's block.
const PARTIAL_1: usize = 3;
/// Index of the Common 2 block.
const COMMON2: usize = PARTIAL_1 + PCMS_PARTIAL_COUNT;

impl PcmTone {
    /// A tone whose blocks start with `common`, `mfx`, `pmt` and `partials`,
    /// each zero-padded to its INTEGRA-7 size. Common 2 holds the long name
    /// and `category`, and every Integra-only field its INIT value.
    pub(super) fn new(
        common: &[u8],
        mfx: &[u8],
        pmt: &[u8],
        partials: [&[u8]; PCMS_PARTIAL_COUNT],
        category: u8,
    ) -> Self {
        let memory = user_memory(ChunkType::PcmSynthTone).expect("PCM Synth layout");
        let sources = [common, mfx, pmt]
            .into_iter()
            .chain(partials)
            .chain([&[][..]]);
        let mut blocks: Vec<Vec<u8>> = memory
            .blocks(memory.temporary_base(0))
            .zip(sources)
            .map(|((_, size), source)| {
                let mut block = vec![0; size.value() as usize];
                block[..source.len()].copy_from_slice(source);
                block
            })
            .collect();
        for partial in &mut blocks[PARTIAL_1..COMMON2] {
            init_lfo_steps(partial);
        }
        blocks[COMMON2] = common2(&blocks[0], category);
        Self { blocks }
    }

    /// An INIT tone: every block holds the [`PcmSynthState`] defaults (MFX
    /// THRU), and the reserves hold what INTEGRA-7 tones carry. Its Common 2
    /// is filled by [`Self::fill_common2`] once the name is set.
    pub(super) fn init() -> Self {
        let init = PcmSynthState::default();
        let mfx = init_mfx(init.mfx);
        let mut common = encode_pcms_common(&init.common);
        common[COMMON_OUTPUT_ASSIGN] = OUTPUT_TONE;
        let partials = init.partials.each_ref().map(encode_pcms_partial);
        Self::new(
            &common,
            &mfx,
            &encode_pcms_pmt(&init.pmt),
            partials.each_ref().map(Vec::as_slice),
            init.common2.tone_category,
        )
    }

    /// Rebuild Common 2 from the Common name, with `category`.
    pub(super) fn fill_common2(&mut self, category: u8) {
        self.blocks[COMMON2] = common2(&self.blocks[0], category);
    }

    /// The Common block.
    pub(super) fn common(&mut self) -> &mut [u8] {
        &mut self.blocks[0]
    }

    /// The MFX block.
    pub(super) fn mfx(&mut self) -> &mut [u8] {
        &mut self.blocks[MFX]
    }

    /// The PMT block.
    pub(super) fn pmt(&mut self) -> &mut [u8] {
        &mut self.blocks[PMT]
    }

    /// Partial `index`'s block (0-indexed).
    pub(super) fn partial(&mut self, index: usize) -> &mut [u8] {
        &mut self.blocks[PARTIAL_1 + index]
    }

    /// Whether partial `index` (0-indexed) is switched on in the PMT.
    pub(super) fn is_on(&self, index: usize) -> bool {
        self.blocks[PMT][PMT_SWITCH + PMT_STRIDE * index] != 0
    }

    /// Whether the tone follows the tempo: a switched-on partial has Wave
    /// Tempo Sync on, or an LFO Rate or the Delay Time at a note value, or
    /// the MFX has a rate or delay at a note value.
    pub(super) fn syncs_to_tempo(&self) -> bool {
        fx_map::syncs_to_tempo(&self.blocks[MFX])
            || (0..PCMS_PARTIAL_COUNT)
                .filter(|&i| self.is_on(i))
                .map(|i| &self.blocks[PARTIAL_1 + i])
                .any(|partial| {
                    partial[F::WaveTempoSync as usize] != 0
                        || [F::Lfo1Rate, F::Lfo2Rate, F::DelayTime]
                            .into_iter()
                            .any(|f| nibble2(partial, f as usize) >= FIRST_NOTE)
                })
    }

    /// Switch partial `index` (0-indexed) off in the PMT.
    pub(super) fn switch_off(&mut self, index: usize) {
        self.blocks[PMT][PMT_SWITCH + PMT_STRIDE * index] = 0;
    }

    /// The Pack entry.
    pub(super) fn into_entry(self) -> PackEntry {
        PackEntry::new(ChunkType::PcmSynthTone, None, self.blocks).expect("PCM Synth layout")
    }
}

/// An INIT MFX block from `mfx` (THRU), with the MFX Dry Send reserve
/// INTEGRA-7 tones carry.
pub(super) fn init_mfx(mfx: MfxState) -> Vec<u8> {
    let mut block = encode_mfx_block(&mfx);
    block[MFX_DRY_SEND] = FULL_LEVEL;
    block
}

/// Where a Foreign partial was routed: through the tone's effect, or past
/// it to an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Output {
    /// Through the MFX: an EFX partial.
    Mfx,
    /// Past the MFX: a MIX or direct-out partial.
    Mix,
}

/// Write the send levels a partial used on its source into the Partial
/// Chorus and Reverb Send Levels. The reserves that held the other pair are
/// set to 127, the value Roland stores for an unused pair, and the output
/// reserve keeps the routing (0 = MFX, 1 = MIX).
pub(super) fn write_sends(partial: &mut [u8], output: Output, chorus: u8, reverb: u8) {
    partial[CHORUS_SEND] = chorus;
    partial[REVERB_SEND] = reverb;
    for at in UNUSED_SENDS {
        partial[at] = UNUSED_SEND_LEVEL;
    }
    partial[OUTPUT] = match output {
        Output::Mfx => 0,
        Output::Mix => 1,
    };
}

/// The Wave Group Type, Group ID and Wave Numbers L and R of a partial.
#[cfg(test)]
pub(super) fn waves(partial: &[u8]) -> (u8, u16, [u16; 2]) {
    (
        partial[WAVE_GROUP_TYPE],
        nibble4(partial, WAVE_GROUP_ID),
        WAVE_NUMBERS.map(|at| nibble4(partial, at)),
    )
}

/// Write INTEGRA-7 Waves into a partial: Wave Group Type, Group ID, and
/// Wave Numbers L and R.
pub(super) fn write_waves(partial: &mut [u8], group: WaveGroup, numbers: [u16; 2]) {
    let (group_type, group_id) = group.params().expect("a PCM Wave Group");
    partial[WAVE_GROUP_TYPE] = group_type;
    partial[WAVE_GROUP_ID..WAVE_GROUP_ID + 4].copy_from_slice(&encode_nibble4(group_id.into()));
    for (at, number) in WAVE_NUMBERS.into_iter().zip(numbers) {
        partial[at..at + 4].copy_from_slice(&encode_nibble4(number));
    }
}

/// Map a partial's Foreign Waves L and R (0 = OFF) of `group` through the
/// Wave map and write them. Partial `index` (0-indexed) is reported only
/// when it is switched on. A partial whose Waves don't map gets no Wave and
/// is switched off.
pub(super) fn map_waves(
    tone: &mut PcmTone,
    index: usize,
    group: ForeignWaveGroup,
    numbers: [u16; 2],
    report: &mut ConversionReport,
) {
    let at = Location::Partial(index as u8 + 1);
    let used = tone.is_on(index);
    match map_wave_pair(group, numbers, at, used, report) {
        Some((group, mapped)) => write_waves(tone.partial(index), group, mapped),
        None => {
            write_waves(tone.partial(index), WaveGroup::Int, [0, 0]);
            tone.switch_off(index);
        }
    }
}

/// Map Foreign Waves L and R (0 = OFF) of `group` through the Wave map:
/// their INTEGRA-7 Wave Group and Wave Numbers, or `None` when they don't
/// map. Items are reported at `at`, and only when it is `used`.
///
/// Low-confidence matches are reported as approximate. A Wave with no match,
/// or one that lands in another Wave Group than its partner, is reported as
/// unmapped.
pub(super) fn map_wave_pair(
    group: ForeignWaveGroup,
    numbers: [u16; 2],
    at: Location,
    used: bool,
    report: &mut ConversionReport,
) -> Option<(WaveGroup, [u16; 2])> {
    let mut target_group = None;
    let mut mapped = [0; 2];
    let mut unmapped = false;
    for (slot, number) in numbers.into_iter().enumerate() {
        if number == 0 {
            continue;
        }
        let source = ForeignWave { group, number };
        let target = source
            .target()
            .filter(|t| *target_group.get_or_insert(t.group) == t.group);
        match target {
            Some(target) => {
                mapped[slot] = target.number;
                if used && target.confidence == Confidence::Low {
                    report.push(ConversionItem::WaveApproximate { at, source, target });
                }
            }
            None => {
                unmapped = true;
                if used {
                    report.push(ConversionItem::WaveUnmapped { at, source });
                }
            }
        }
    }
    (!unmapped).then(|| (target_group.unwrap_or(WaveGroup::Int), mapped))
}

/// Give a partial the INIT LFO step sequencer, which no Foreign synth has.
fn init_lfo_steps(partial: &mut [u8]) {
    let init = PcmSynthPartial::default();
    partial[LFO_STEP_TYPE] = init.lfo_step_type;
    partial[LFO_STEP_TYPE + 1..][..init.lfo_step_values.len()]
        .copy_from_slice(&init.lfo_step_values);
}

/// Common 2: the 12-character name space-padded into the 16-character long
/// name, `category`, and INIT values elsewhere.
fn common2(common: &[u8], category: u8) -> Vec<u8> {
    let init = PcmSynthCommon2::default();
    let name_len = ChunkType::PcmSynthTone.name_length().expect("a named type");
    let name = String::from_utf8_lossy(&common[..name_len]).into_owned();
    let mut block = vec![0; PCMS_COMMON2_SIZE];
    block[..LONG_NAME_LEN].copy_from_slice(&encode_ascii_name(&name, LONG_NAME_LEN));
    block[TONE_CATEGORY] = category;
    block[PHRASE_OCTAVE_SHIFT] = init.phrase_octave_shift;
    block[TFX_SWITCH] = init.tfx_switch;
    let numbers = [init.tone_number, init.phrase_number];
    for ((at, len), value) in NIBBLIZED.into_iter().zip(numbers) {
        block[at..at + len].copy_from_slice(&encode_nibble4(value)[4 - len..]);
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone() -> PcmTone {
        let partial = [0u8; 137];
        let mut pmt = [0u8; 41];
        for i in 0..PCMS_PARTIAL_COUNT {
            pmt[PMT_SWITCH + PMT_STRIDE * i] = 1;
        }
        PcmTone::new(&[0x41; 79], &[0; 145], &pmt, [&partial; 4], 5)
    }

    #[test]
    fn fills_what_only_the_integra_has() {
        let mut tone = tone();
        assert_eq!(
            &tone.partial(0)[LFO_STEP_TYPE..],
            [&[0][..], &[64; 16]].concat()
        );
        let common2 = &tone.blocks[COMMON2];
        assert_eq!(&common2[..LONG_NAME_LEN], b"AAAAAAAAAAAA    ");
        assert_eq!(common2[TONE_CATEGORY], 5);
        assert_eq!(common2[PHRASE_OCTAVE_SHIFT], 64);
        assert_eq!(
            common2.iter().filter(|&&b| b != 0).count(),
            LONG_NAME_LEN + 2
        );
        assert_eq!(tone.into_entry().kind(), ChunkType::PcmSynthTone);
    }

    #[test]
    fn init_holds_the_state_defaults() {
        let mut tone = PcmTone::init();
        tone.common()[..4].copy_from_slice(b"Init");
        tone.fill_common2(3);
        let entry = tone.into_entry();
        let blocks = entry.blocks();
        let state = PcmSynthState::default();
        let common = crate::pcm_synth::parse_pcms_common(&blocks[0]).unwrap();
        assert_eq!(common.tone_name, "Init");
        assert_eq!(common.matrix_controls, state.common.matrix_controls);
        assert_eq!(
            (blocks[0][0x27], blocks[1][0x01], blocks[1][0x06]),
            (13, 127, 64)
        );
        assert_eq!(
            crate::pcm_synth::parse_pcms_partial(&blocks[3]).unwrap(),
            state.partials[0]
        );
        assert_eq!(&blocks[COMMON2][..LONG_NAME_LEN], b"Init            ");
        assert_eq!(blocks[COMMON2][TONE_CATEGORY], 3);
    }

    #[test]
    fn sends_keep_the_used_pair() {
        let mut partial = [0u8; 154];
        write_sends(&mut partial, Output::Mfx, 10, 20);
        assert_eq!(partial[0x0D..=0x11], [127, 127, 10, 20, 0]);
        write_sends(&mut partial, Output::Mix, 30, 40);
        assert_eq!(partial[0x0D..=0x11], [127, 127, 30, 40, 1]);
    }

    #[test]
    fn waves_round_trip() {
        let mut partial = [0u8; 154];
        write_waves(&mut partial, WaveGroup::Srx(6), [29, 300]);
        assert_eq!(waves(&partial), (1, 6, [29, 300]));
    }

    #[test]
    fn maps_foreign_waves_or_switches_the_partial_off() {
        let group = ForeignWaveGroup::SrJv80(2);
        let mut tone = tone();
        let mut report = ConversionReport::new();
        // SR-JV80-02 Waves 1 and 2 are SRX-06 Waves 29 and 30, both high.
        map_waves(&mut tone, 0, group, [1, 2], &mut report);
        assert_eq!(waves(tone.partial(0)), (1, 6, [29, 30]));
        assert!(tone.is_on(0) && report.is_empty());

        // SR-JV80-01 Wave 1 has no match.
        map_waves(
            &mut tone,
            1,
            ForeignWaveGroup::SrJv80(1),
            [1, 0],
            &mut report,
        );
        assert_eq!(waves(tone.partial(1)), (0, 1, [0, 0]));
        assert!(!tone.is_on(1));
        assert_eq!(report.len(), 1);

        // A switched-off partial is not reported.
        map_waves(&mut tone, 1, ForeignWaveGroup::Sample, [1, 0], &mut report);
        assert_eq!(report.len(), 1);
    }
}
