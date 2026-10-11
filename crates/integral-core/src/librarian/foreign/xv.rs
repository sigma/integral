//! XV series (Model ID `00 10`): where a DT1 lands, and how a Patch
//! converts into a PCM Synth tone.
//!
//! Every XV Patch is read as an XV-5080's, which also covers the original
//! Fantom. The PCM Synth tone is the XV Patch with a few fields turned into
//! reserves, so a Patch converts by copying its blocks byte for byte, then
//! applying one fix row per field that is not identical.
//!
//! Reference: `docs/research/xv-patch-format.md` and its per-field
//! `xv-patch-format.csv`, `docs/research/xv-svd-format.md` (`.svd`
//! entries), and `docs/research/foreign-rhythm-format.md` (XV
//! Rhythm Set).

use super::frame::{self, Frame};
use super::pcm_tone::{self, FULL_LEVEL, OUTPUT_TONE, Output, PcmTone};
use super::report::{ConversionItem, ConversionReport, EffectFamily, ForeignEffect, Location};
use super::wave_map::ForeignWaveGroup;
use super::{Placed, Unit};
use crate::address::Address;
use crate::librarian::foreign::report::Skipped;
use crate::librarian::pack::PackEntry;
use crate::parse_helpers::{nibble2, nibble4};
use crate::svd_convert::unpack_sections;
use crate::svd_specs::XV_PATCH_SECTIONS;
use crate::waves::WaveGroup;

/// XV Model ID.
pub(super) const MODEL: &[u8] = &[0x00, 0x10];

/// Highest first address byte of System and Setup (XV-5050/2020 put
/// System at `02`).
const LAST_SYSTEM: u8 = 0x02;
/// Temporary Performance.
const TEMPORARY_PERFORMANCE: u8 = 0x10;
/// First and last address bytes of the Performance parts' Temporary Patches
/// and Rhythms.
const PARTS: core::ops::RangeInclusive<u8> = 0x11..=0x18;
/// Patch-mode Temporary Patch and Rhythm.
const TEMPORARY: u8 = 0x1F;
/// User Performances.
const USER_PERFORMANCES: u8 = 0x20;
/// User Patches.
const USER_PATCHES: u8 = 0x30;
/// User Rhythms.
const USER_RHYTHMS: u8 = 0x40;
/// Bit of the second address byte that moves a Temporary Patch to its
/// Temporary Rhythm.
const RHYTHM_BIT: u8 = 0x10;
/// Bits of the second address byte that name a Rhythm's slot. The others
/// carry the Rhythm Tone offset.
const RHYTHM_SLOT: u8 = 0x70;

/// The unit a DT1 at `address` belongs to. A Patch's blocks share the
/// first two address bytes; a Rhythm's share the first and the slot bits
/// of the second.
pub(super) fn place(address: [u8; 4]) -> Option<Placed> {
    let [a, b, c, d] = address;
    let kind = match a {
        ..=LAST_SYSTEM => Skipped::System,
        TEMPORARY_PERFORMANCE | USER_PERFORMANCES => Skipped::Performance,
        USER_PATCHES => Skipped::Patch,
        USER_RHYTHMS => Skipped::Rhythm,
        _ if a == TEMPORARY || PARTS.contains(&a) => {
            if b & RHYTHM_BIT == 0 {
                Skipped::Patch
            } else {
                Skipped::Rhythm
            }
        }
        _ => return None,
    };
    let (key, block) = if kind == Skipped::Rhythm {
        (b & RHYTHM_SLOT, b & !RHYTHM_SLOT)
    } else {
        (b, 0)
    };
    Some(Placed {
        unit: Unit {
            kind,
            key: [a, key],
        },
        common: (block, c, d) == (0, 0, 0),
    })
}

/// A Patch block: its offset (address bytes 2 and 3) and size in bytes.
type Block = ([u8; 2], usize);

/// The 9 blocks of a Patch dump: Common, MFX, Chorus, Reverb, TMT and Tones
/// 1–4. Split Key blocks (Multi-Partial Patches only) are left out.
const BLOCKS: [Block; 9] = [
    ([0x00, 0x00], 79),  // Patch Common, `00 00 00 4F`
    ([0x02, 0x00], 145), // Patch Common MFX, `00 00 01 11`
    ([0x04, 0x00], 52),  // Patch Common Chorus, `00 00 00 34`
    ([0x06, 0x00], 83),  // Patch Common Reverb, `00 00 00 53`
    ([0x10, 0x00], 41),  // Patch TMT, `00 00 00 29`
    ([0x20, 0x00], 137), // Patch Tone 1, `00 00 01 09`
    ([0x22, 0x00], 137), // Patch Tone 2
    ([0x24, 0x00], 137), // Patch Tone 3
    ([0x26, 0x00], 137), // Patch Tone 4
];
/// Index in [`BLOCKS`] of Patch Common.
const COMMON: usize = 0;
/// Index of Patch Common MFX.
const MFX: usize = 1;
/// Index of Patch Common Chorus.
const CHORUS: usize = 2;
/// Index of Patch Common Reverb.
const REVERB: usize = 3;
/// Index of Patch TMT.
const TMT: usize = 4;
/// Index of Patch Tone 1.
const TONE_1: usize = 5;

/// Common: Patch Category.
const PATCH_CATEGORY: usize = 0x0C;
/// Common: Tone Type, 4TONES or MULTI-PARTIAL.
const TONE_TYPE: usize = 0x0D;
/// Tone Type MULTI-PARTIAL.
const MULTI_PARTIAL: u8 = 1;
/// Common: Patch Clock Source, PATCH or SYSTEM.
const CLOCK_SOURCE: usize = 0x1E;
/// Clock Source PATCH: the Patch Tempo is used.
const CLOCK_PATCH: u8 = 0;
/// Common: Patch Tempo (2 nibbles).
const PATCH_TEMPO: usize = 0x1F;
/// Common: One Shot Mode.
const ONE_SHOT_MODE: usize = 0x21;
/// Common: Patch Output Assign.
const PATCH_OUTPUT_ASSIGN: usize = 0x27;
/// MFX: MFX Type.
const MFX_TYPE: usize = 0x00;
/// MFX: MFX Dry Send Level.
const MFX_DRY_SEND: usize = 0x01;
/// MFX: MFX Output Assign.
const MFX_OUTPUT_ASSIGN: usize = 0x04;
/// Chorus and Reverb: Type, then Level.
const EFFECT_TYPE: usize = 0x00;
/// Chorus and Reverb: Level.
const EFFECT_LEVEL: usize = 0x01;
/// Tone: Chorus and Reverb Send Level (MFX).
const MFX_SENDS: [usize; 2] = [0x0D, 0x0E];
/// Tone: Chorus and Reverb Send Level (non MFX).
const MIX_SENDS: [usize; 2] = [0x0F, 0x10];
/// Tone: Tone Output Assign.
const TONE_OUTPUT_ASSIGN: usize = 0x11;
/// Tone: Tone Receive Pan Mode, CONTINUOUS or KEY-ON.
const RECEIVE_PAN_MODE: usize = 0x15;
/// Tone: Wave Group Type, ID and Wave Numbers L and R (nibblized).
const WAVE_GROUP_TYPE: usize = 0x27;
/// Tone: Wave Group ID (4 nibbles).
const WAVE_GROUP_ID: usize = 0x28;
/// Tone: Wave Number L and R (4 nibbles each).
const WAVE_NUMBERS: [usize; 2] = [0x2C, 0x30];

/// Wave Group Types: INT-WAVE, SRJV80-EXP, SRX-EXP, SAMPLE.
const WAVE_INT: u8 = 0;
/// Wave Group Type SRJV80-EXP.
const WAVE_SR_JV80: u8 = 1;
/// Wave Group Type SRX-EXP.
const WAVE_SRX: u8 = 2;

/// Output Assign MFX; Patch Output Assign also has [`OUTPUT_TONE`].
pub(super) const OUTPUT_MFX: u8 = 0;
/// Output Assign values, as the XV-5080 names them.
pub(super) const OUTPUTS: [&str; 14] = [
    "MFX", "A", "B", "C", "D", "1", "2", "3", "4", "5", "6", "7", "8", "TONE",
];
/// Chorus Types (XV-5050 adds GM2 CHORUS).
const CHORUS_TYPES: [&str; 4] = ["OFF", "CHORUS", "DELAY", "GM2 CHORUS"];
/// Reverb Types (XV-5050 adds GM2 REVERB).
const REVERB_TYPES: [&str; 6] = [
    "OFF",
    "REVERB",
    "SRV ROOM",
    "SRV HALL",
    "SRV PLATE",
    "GM2 REVERB",
];

/// The tempo a Patch or Rhythm that doesn't use one carries.
pub(super) const DEFAULT_TEMPO: u16 = 120;

/// INTEGRA-7 Tone Category of each XV Patch Category, in the XV-5080's list
/// order (0 = NO ASSIGN … 38 = CMB). From `xv-patch-format.md`, section 6.
pub(super) const CATEGORIES: [u8; 39] = [
    0, 1, 5, 10, 14, 14, 6, 12, 12, 16, // —, PNO, EP, KEY, BEL, MLT, ORG, ACD, HRM, AGT
    17, 18, 20, 21, 24, 24, 44, 28, 29, 26, // EGT, DGT, BS, SBS, STR, ORC, HIT, WND, FLT, BRS
    35, 30, 34, 34, 40, 42, 39, 38, 36, 36, // SBR, SAX, HLD, SLD, TEK, PLS, FX, SYN, BPD, SPD
    32, 22, 22, 22, 47, 45, 43, 46, 48, // VOX, PLK, ETH, FRT, PRC, SFX, BTS, DRM, CMB
];

/// Why a Patch doesn't convert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rejected {
    /// A Multi-Partial Patch: it plays user samples.
    MultiPartial,
    /// A block is missing, or an `.svd` entry is too short to hold them
    /// all.
    Incomplete,
}

impl From<Rejected> for Skipped {
    fn from(rejected: Rejected) -> Self {
        match rejected {
            Rejected::MultiPartial => Skipped::MultiPartial,
            Rejected::Incomplete => Skipped::Incomplete,
        }
    }
}

/// A Patch being converted.
struct Patch<'a> {
    /// The XV blocks, in [`BLOCKS`] order.
    xv: &'a [Vec<u8>],
    tone: PcmTone,
    report: ConversionReport,
}

/// The fix rows, one per XV field that does not copy byte for byte, in
/// `xv-patch-format.csv` order. Patch Category and Tone Type are read
/// before them, and the Integra-only fields are filled by [`PcmTone`].
const FIXES: [fn(&mut Patch<'_>); 8] = [
    patch_tempo,
    one_shot_mode,
    patch_output_assign,
    mfx,
    chorus_and_reverb,
    tone_sends,
    receive_pan_mode,
    wave_group,
];

/// Convert the Patch whose frames share address bytes `key`.
pub(super) fn convert(
    key: [u8; 2],
    frames: &[Frame<'_>],
) -> Result<(PackEntry, ConversionReport), Rejected> {
    let layout = BLOCKS.map(|([c, d], size)| (Address::new(key[0], key[1], c, d).value(), size));
    let dt1s = || frames.iter().map(|f| (Address(f.address), f.data));
    // A Multi-Partial Patch is rejected as such even with blocks missing.
    let common = frame::assemble(&layout[..1], dt1s()).ok_or(Rejected::Incomplete)?;
    if common[0][TONE_TYPE] == MULTI_PARTIAL {
        return Err(Rejected::MultiPartial);
    }
    let xv = frame::assemble(&layout, dt1s()).ok_or(Rejected::Incomplete)?;
    convert_blocks(&xv)
}

/// Convert the Patch decoded from an XV `.svd` entry: its leading
/// [`XV_PATCH_SECTIONS`] bytes hold the 9 blocks. Split Key data after them
/// is ignored, since only Multi-Partial Patches use it.
pub(super) fn convert_svd(entry: &[u8]) -> Result<(PackEntry, ConversionReport), Rejected> {
    let xv = unpack_sections(entry, &XV_PATCH_SECTIONS).map_err(|_| Rejected::Incomplete)?;
    convert_blocks(&xv)
}

/// Convert a Patch from its 9 blocks, in [`BLOCKS`] order.
fn convert_blocks(xv: &[Vec<u8>]) -> Result<(PackEntry, ConversionReport), Rejected> {
    debug_assert!(xv.iter().map(Vec::len).eq(BLOCKS.map(|(_, size)| size)));
    if xv[COMMON][TONE_TYPE] == MULTI_PARTIAL {
        return Err(Rejected::MultiPartial);
    }

    let category = CATEGORIES
        .get(usize::from(xv[COMMON][PATCH_CATEGORY]))
        .copied()
        .unwrap_or_default();
    let tones = core::array::from_fn(|i| xv[TONE_1 + i].as_slice());
    let mut patch = Patch {
        tone: PcmTone::new(&xv[COMMON], &xv[MFX], &xv[TMT], tones, category),
        xv,
        report: ConversionReport::new(),
    };
    for fix in FIXES {
        fix(&mut patch);
    }
    Ok((patch.tone.into_entry(), patch.report))
}

/// The name of `value` in `names`, or the raw value.
pub(super) fn named(names: &[&str], value: u8) -> String {
    names
        .get(usize::from(value))
        .map_or_else(|| value.to_string(), |n| (*n).to_string())
}

/// Patch Tempo: the INTEGRA-7 takes the tempo from the Studio Set. The byte
/// stays in its reserve.
fn patch_tempo(p: &mut Patch<'_>) {
    let common = &p.xv[COMMON];
    tempo(
        common[CLOCK_SOURCE],
        nibble2(common, PATCH_TEMPO),
        p.tone.syncs_to_tempo(),
        &mut p.report,
    );
}

/// A Patch or Rhythm Tempo, read when its Clock Source is the Patch or
/// Rhythm (0) and the tone is `synced` to it: it lives in the Studio Set on
/// the INTEGRA-7.
pub(super) fn tempo(clock_source: u8, tempo: u16, synced: bool, report: &mut ConversionReport) {
    if synced && clock_source == CLOCK_PATCH && tempo != DEFAULT_TEMPO {
        report.push(ConversionItem::NotApplied {
            what: format!("Tempo {tempo}"),
        });
    }
}

/// One Shot Mode: no INTEGRA-7 equivalent. The byte stays in its reserve.
fn one_shot_mode(p: &mut Patch<'_>) {
    if p.xv[COMMON][ONE_SHOT_MODE] != 0 {
        p.report.push(ConversionItem::ParamDropped {
            at: None,
            param: "One Shot Mode",
        });
    }
}

/// Patch Output Assign, by [`output_assign`].
fn patch_output_assign(p: &mut Patch<'_>) {
    let output = p.xv[COMMON][PATCH_OUTPUT_ASSIGN];
    p.tone.common()[PATCH_OUTPUT_ASSIGN] = output_assign(output, &mut p.report);
}

/// A Patch or Rhythm Output Assign: set in the Studio Set Part on the
/// INTEGRA-7. Returns what its reserve gets: TONE, which INTEGRA-7 tones
/// always carry.
pub(super) fn output_assign(output: u8, report: &mut ConversionReport) -> u8 {
    if output != OUTPUT_TONE {
        report.push(ConversionItem::NotApplied {
            what: format!("Output Assign {}", named(&OUTPUTS, output)),
        });
    }
    OUTPUT_TONE
}

/// MFX, by [`mfx_thru`].
fn mfx(p: &mut Patch<'_>) {
    mfx_thru(&p.xv[MFX], p.tone.mfx(), &mut p.report);
}

/// MFX Type of a Patch or Rhythm MFX block `xv`, copied into `mfx`: THRU
/// until the effect types are mapped. MFX Output Assign names physical
/// outputs and is dropped. MFX Dry Send Level has no INTEGRA-7 equivalent;
/// its reserve gets 127, as INTEGRA-7 tones carry.
pub(super) fn mfx_thru(xv: &[u8], mfx: &mut [u8], report: &mut ConversionReport) {
    // ponytail: THRU until the Foreign MFX conversion lands (#213); it then
    // maps the type and its parameters through the effects rows.
    let number = xv[MFX_TYPE];
    if number != 0 {
        report.push(ConversionItem::EffectDropped {
            source: ForeignEffect {
                family: EffectFamily::XvMfx,
                number,
            },
        });
    }
    mfx[MFX_TYPE] = 0;
    mfx[MFX_OUTPUT_ASSIGN] = 0;
    if xv[MFX_DRY_SEND] != FULL_LEVEL {
        report.push(ConversionItem::ParamDropped {
            at: None,
            param: "MFX Dry Send Level",
        });
    }
    mfx[MFX_DRY_SEND] = FULL_LEVEL;
}

/// A Patch or Rhythm effect an INTEGRA-7 tone has no place for: reported
/// when it sounds.
fn studio_set_effect(block: &[u8], label: &str, types: &[&str], report: &mut ConversionReport) {
    let (kind, level) = (block[EFFECT_TYPE], block[EFFECT_LEVEL]);
    if kind != 0 && level != 0 {
        report.push(ConversionItem::NotApplied {
            what: format!("{label} {}, level {level}", named(types, kind)),
        });
    }
}

/// Patch Chorus and Reverb, by [`chorus_reverb`].
fn chorus_and_reverb(p: &mut Patch<'_>) {
    chorus_reverb(&p.xv[CHORUS], &p.xv[REVERB], &mut p.report);
}

/// A Patch or Rhythm Chorus and Reverb block: they live in the Studio Set
/// on the INTEGRA-7.
pub(super) fn chorus_reverb(chorus: &[u8], reverb: &[u8], report: &mut ConversionReport) {
    studio_set_effect(chorus, "Chorus", &CHORUS_TYPES, report);
    studio_set_effect(reverb, "Reverb", &REVERB_TYPES, report);
}

/// Tone Chorus/Reverb Send Levels: an XV Tone holds a pair for when it goes
/// through the MFX and a pair for when it doesn't, and its Output Assign
/// picks one. The INTEGRA-7 reads one pair, so it gets the picked one.
fn tone_sends(p: &mut Patch<'_>) {
    for i in 0..4 {
        let tone = &p.xv[TONE_1 + i];
        let (output, [chorus, reverb]) = if tone[TONE_OUTPUT_ASSIGN] == OUTPUT_MFX {
            (Output::Mfx, MFX_SENDS)
        } else {
            (Output::Mix, MIX_SENDS)
        };
        let (chorus, reverb) = (tone[chorus], tone[reverb]);
        pcm_tone::write_sends(p.tone.partial(i), output, chorus, reverb);
    }
}

/// Tone Receive Pan Mode: no INTEGRA-7 equivalent. The byte stays in its
/// reserve.
fn receive_pan_mode(p: &mut Patch<'_>) {
    for i in 0..4 {
        if p.tone.is_on(i) && p.xv[TONE_1 + i][RECEIVE_PAN_MODE] != 0 {
            p.report.push(ConversionItem::ParamDropped {
                at: Some(Location::Partial(i as u8 + 1)),
                param: "Receive Pan Mode",
            });
        }
    }
}

/// Where the Waves of an XV Wave Group Type and ID land.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WaveSource {
    /// INT: the bytes copy as they are.
    Int,
    /// SRX board n: INTEGRA-7 SRX n, with the same Wave Numbers.
    Srx(WaveGroup),
    /// Through the Wave map: SR-JV80 boards, unknown SRX boards, and SAMPLE
    /// Waves, which are user samples and don't map.
    Foreign(ForeignWaveGroup),
}

/// Where the Waves of Wave Group Type `group_type` and ID `id` land.
pub(super) fn wave_source(group_type: u8, id: u16) -> WaveSource {
    let srx = u8::try_from(id)
        .ok()
        .map(WaveGroup::Srx)
        .filter(|g| WaveGroup::ALL.contains(g));
    match (group_type, srx) {
        (WAVE_INT, _) => WaveSource::Int,
        (WAVE_SRX, Some(group)) => WaveSource::Srx(group),
        (WAVE_SRX, None) => WaveSource::Foreign(ForeignWaveGroup::Srx(id)),
        (WAVE_SR_JV80, _) => WaveSource::Foreign(ForeignWaveGroup::SrJv80(id)),
        _ => WaveSource::Foreign(ForeignWaveGroup::Sample),
    }
}

/// Wave Group Type and ID, by [`wave_source`].
fn wave_group(p: &mut Patch<'_>) {
    for i in 0..4 {
        let tone = &p.xv[TONE_1 + i];
        let numbers = WAVE_NUMBERS.map(|at| nibble4(tone, at));
        match wave_source(tone[WAVE_GROUP_TYPE], nibble4(tone, WAVE_GROUP_ID)) {
            WaveSource::Int => {}
            WaveSource::Srx(group) => pcm_tone::write_waves(p.tone.partial(i), group, numbers),
            WaveSource::Foreign(group) => {
                pcm_tone::map_waves(&mut p.tone, i, group, numbers, &mut p.report);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(address: [u8; 4]) -> Option<Skipped> {
        place(address).map(|p| p.unit.kind)
    }

    #[test]
    fn places_patches_rhythms_and_the_rest() {
        // Patch mode, part 1's Tone 1, part 32 and User Patch 128's MFX.
        for patch in [
            [0x1F, 0, 0, 0],
            [0x11, 0, 0x20, 0],
            [0x18, 0x60, 0, 0],
            [0x30, 0x7F, 2, 0],
        ] {
            assert_eq!(kind(patch), Some(Skipped::Patch), "{patch:02X?}");
        }
        // Patch mode, part 10 and User Rhythm 4.
        for rhythm in [
            [0x1F, 0x10, 0, 0],
            [0x13, 0x30, 0x10, 0],
            [0x40, 0x30, 0, 0],
        ] {
            assert_eq!(kind(rhythm), Some(Skipped::Rhythm), "{rhythm:02X?}");
        }
        assert_eq!(kind([0x20, 0x3F, 0, 0]), Some(Skipped::Performance));
        assert_eq!(kind([0x02, 0, 0, 0]), Some(Skipped::System));
        assert_eq!(kind([0x50, 0, 0, 0]), None);
    }

    #[test]
    fn a_rhythm_is_one_unit_across_its_tones() {
        let common = place([0x40, 0x10, 0, 0]).unwrap();
        // Rhythm Tone for key 108: `00 10 00` + 2 × 87, with a 7-bit carry.
        let last_tone = place([0x40, 0x11, 0x3E, 0]).unwrap();
        assert_eq!(common.unit, last_tone.unit);
        assert!(common.common && !last_tone.common);
        assert_ne!(common.unit, place([0x40, 0x20, 0, 0]).unwrap().unit);
    }
}
