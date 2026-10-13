//! JV/XP series (Model ID `6A`): where a DT1 lands, and how a Patch
//! converts into a PCM Synth tone.
//!
//! A Patch converts from an INIT PCM Synth tone: one mapping row per JV
//! field that has an INTEGRA-7 target ([`COMMON_ROWS`], [`TONE_ROWS`]),
//! then the named [`CODE_ROWS`] for the conversions that need more than one
//! byte or a report. Where Roland's own JV → XV conversions settle a rule,
//! they win over the manuals.
//!
//! Reference: `docs/research/jv-patch-format.md` (layout and parameter map),
//! `docs/research/jv-xv-roland-pairs.md` (Roland's rules) and
//! `docs/research/foreign-rhythm-format.md` (JV/XP Rhythm Setup).

use super::frame::{self, Frame};
use super::fx_map;
use super::pcm_tone::{self, Output, PcmTone};
use super::report::{
    ConversionItem, ConversionReport, EffectFamily, ForeignEffect, Location, SourceModel as Model,
};
use super::studio_set_fx::{StudioSetEffect, jv_chorus, jv_reverb};
use super::wave_map::ForeignWaveGroup;
use super::xv::CATEGORIES;
use super::{Converted, Placed, Unit};
use crate::address::Address;
use crate::fx_params::FxUnit;
use crate::librarian::foreign::report::Skipped;
use crate::mfx::{MFX_PARAM_COUNT, MfxHeaderField, mfx_header_offset};
use crate::parse_helpers::{encode_nibble2, nibble2};

/// JV/XP Model ID.
pub(super) const MODEL: &[u8] = &[0x6A];

/// Performance-mode Temporary area: Patches of Parts 1–16, and the Rhythm
/// Setup at Part 10's place.
const TEMPORARY_PARTS: u8 = 0x02;
/// The Temporary Rhythm Setup's second address byte.
const TEMPORARY_RHYTHM: u8 = 0x09;
/// Patch-mode Temporary Patch.
const TEMPORARY_PATCH: u8 = 0x03;
/// User Performances and Rhythm Setups.
const USER_PERFORMANCES: u8 = 0x10;
/// User Patches.
const USER_PATCHES: u8 = 0x11;
/// Data-card Performances and Rhythm Setup (JV-1080).
const CARD_PERFORMANCES: u8 = 0x20;
/// Data-card Patches (JV-1080).
const CARD_PATCHES: u8 = 0x21;
/// Second address byte of the first Rhythm Setup among Performances.
const RHYTHMS: u8 = 0x40;
/// System.
const SYSTEM: u8 = 0x00;
/// Temporary Performance.
const TEMPORARY_PERFORMANCE: u8 = 0x01;

/// The unit a DT1 at `address` belongs to. Every block of a unit shares
/// the first two address bytes, and Common sits at offset `00 00`.
pub(super) fn place(address: [u8; 4]) -> Option<Placed> {
    let [a, b, c, d] = address;
    let kind = match (a, b) {
        (SYSTEM, _) => Skipped::System,
        (TEMPORARY_PERFORMANCE, _) => Skipped::Performance,
        (TEMPORARY_PARTS, TEMPORARY_RHYTHM) => Skipped::Rhythm,
        (TEMPORARY_PARTS, 0x00..=0x0F) | (TEMPORARY_PATCH | USER_PATCHES | CARD_PATCHES, _) => {
            Skipped::Patch
        }
        (USER_PERFORMANCES | CARD_PERFORMANCES, RHYTHMS..) => Skipped::Rhythm,
        (USER_PERFORMANCES | CARD_PERFORMANCES, _) => Skipped::Performance,
        _ => return None,
    };
    Some(Placed {
        unit: Unit { kind, key: [a, b] },
        common: (c, d) == (0, 0),
    })
}

// ---------------------------------------------------------------------------
// Patch layout
// ---------------------------------------------------------------------------

/// Patch Common lengths: JV-1080, XP-50/60/80, JV-2080/1010/XP-30. Each
/// adds one field to the one before.
const COMMON_JV1080: usize = 0x48;
/// XP-50/60/80 Patch Common length: adds Clock Source.
const COMMON_XP: usize = 0x49;
/// JV-2080/1010 and XP-30 Patch Common length: adds Patch Category.
const COMMON_JV2080: usize = 0x4A;
/// Patch Tone length.
const TONE_LEN: usize = 0x81;
/// Offsets (address bytes 2 and 3) of Patch Tones 1–4.
const TONE_OFFSETS: [[u8; 2]; 4] = [[0x10, 0], [0x12, 0], [0x14, 0], [0x16, 0]];

// Patch Common fields read by the code rows.
/// Common: EFX Type, the first EFX byte.
const EFX_TYPE: usize = 0x0C;
/// Common: EFX Parameter 12, the last EFX byte that sets its sound.
const EFX_LAST_PARAM: usize = 0x18;
/// Common: EFX Output Assign.
const EFX_OUTPUT_ASSIGN: usize = 0x19;
/// Common: EFX Mix Out Send Level, the XV MFX Dry Send Level.
const EFX_MIX_OUT: usize = 0x1A;
/// Common: EFX Control Source 1 and 2, each followed by its Depth.
const EFX_CONTROLS: [(usize, &str); 2] = [(0x1D, "EFX Control 1"), (0x1F, "EFX Control 2")];
/// The last EFX Control Source, AFT: the list is the Patch one's first 11.
const EFX_LAST_SOURCE: u8 = 10;
/// EFX Control Depth 0.
const NEUTRAL_DEPTH: u8 = 63;
/// Common: Chorus Level … Chorus Output.
const CHORUS: core::ops::RangeInclusive<usize> = 0x21..=0x26;
/// Common: Reverb Type … Delay Feedback.
const REVERB: core::ops::RangeInclusive<usize> = 0x27..=0x2B;
/// Common: Patch Tempo (2 nibbles).
const PATCH_TEMPO: usize = 0x2C;
/// Common: Patch Control Source 2 and 3.
const CONTROL_SOURCES: [usize; 2] = [0x3A, 0x3B];
/// Common: EFX and Control 1–3 Hold/Peak.
const HOLD_PEAK: usize = 0x3C;
/// Common: Velocity Range Switch.
const VELOCITY_RANGE_SWITCH: usize = 0x40;
/// Common: Clock Source, PATCH first (XP and JV-2080 only).
const CLOCK_SOURCE: usize = 0x48;
/// Common: Patch Category (JV-2080 only).
const PATCH_CATEGORY: usize = 0x49;

// Patch Tone fields read by the code rows.
/// Tone: Tone Switch.
const TONE_SWITCH: usize = 0x00;
/// Tone: Wave Group Type, Wave Group ID, Wave Number (2 nibbles).
const WAVE_GROUP_TYPE: usize = 0x01;
/// Tone: Wave Group ID.
const WAVE_GROUP_ID: usize = 0x02;
/// Tone: Wave Number (2 nibbles, 0 = Wave 1).
const WAVE_NUMBER: usize = 0x03;
/// Tone: Tone Delay Mode, then Tone Delay Time.
const DELAY_MODE: usize = 0x09;
/// Tone: Tone Delay Time.
const DELAY_TIME: usize = 0x0A;
/// Tone: Velocity Cross Fade, then Velocity Range Lower and Upper.
const VELOCITY_CROSS_FADE: usize = 0x0B;
/// Tone: Volume Control Switch.
const VOLUME_CONTROL: usize = 0x11;
/// Tone: Pan Control Switch.
const PAN_CONTROL: usize = 0x14;
/// Tone: Controller 1 Destination 1; then Depth 1, Destination 2 … for
/// Controllers 1–3.
const CONTROLLERS: usize = 0x15;
/// Tone: LFO1 and LFO2 Rate and External Sync.
const LFO_RATES: [(usize, usize); 2] = [(0x2F, 0x34), (0x37, 0x3C)];
/// Tone: Output Assign.
const TONE_OUTPUT_ASSIGN: usize = 0x7D;
/// Tone: Chorus and Reverb Send Level.
const TONE_SENDS: [usize; 2] = [0x7F, 0x80];

// INTEGRA-7 fields written by the code rows.
/// Common: Legato Retrigger.
const LEGATO_RETRIGGER: usize = 0x18;
/// Common: the reserve that keeps the XV-style Patch Category.
const LEGACY_CATEGORY: usize = 0x0C;
/// Common: Matrix Control 1 Source; Destination 1, Sens 1 … follow.
const MATRIX_CONTROLS: usize = 0x2B;
/// Common: bytes per Matrix Control.
const MATRIX_STRIDE: usize = 9;
/// PMT: PMT Velocity Control.
const PMT_VELOCITY_CONTROL: usize = 0x04;
/// PMT: Partial 1's entry; each is 9 bytes.
const PMT_ENTRY: usize = 0x05;
/// PMT: bytes per partial entry.
const PMT_STRIDE: usize = 9;
/// Partial: Delay Mode, then Delay Time (2 nibbles).
const PARTIAL_DELAY_MODE: usize = 0x09;
/// Partial: Delay Time.
const PARTIAL_DELAY_TIME: usize = 0x0A;
/// Partial: the reserve that keeps the XV Receive Pan Mode.
const RECEIVE_PAN_MODE: usize = 0x15;
/// Partial: Control 1 Switch 1; 16 switches, 4 per Matrix Control.
const CONTROL_SWITCHES: usize = 0x17;
/// Partial: LFO1 and LFO2 Rate (2 nibbles each).
const PARTIAL_LFO_RATES: [usize; 2] = [0x6E, 0x7C];

/// The JV INIT PATCH's Common from EFX Type (`0C`) to Patch Tempo (`2D`):
/// EFX STEREO-EQ flat, Output MIX, Chorus level 127, Reverb STAGE1, Tempo
/// 120.
///
/// No manual lists these values. They are Roland's `INIT PATCH` as JV-1080,
/// XP and JV-2080 units dump it, byte-identical in every such dump of the
/// corpus (e.g. `planet-groove` `RNDYBRWN.SYX` User 065).
pub(super) const INIT_EFFECTS: [u8; 0x2E - EFX_TYPE] = [
    0x00, 0x00, 0x0F, 0x00, 0x0F, 0x07, 0x01, 0x0F, 0x07, 0x01, 0x0F, 0x7F, 0x00, // EFX
    0x00, 0x7F, 0x00, 0x00, 0x00, 0x3F, 0x00, 0x3F, // Output, sends, controls
    0x7F, 0x03, 0x50, 0x00, 0x00, 0x00, // Chorus
    0x02, 0x7F, 0x5A, 0x0D, 0x00, // Reverb
    0x07, 0x08, // Tempo 120
];

/// The JV INIT bytes of Common `range`.
fn init(range: core::ops::RangeInclusive<usize>) -> &'static [u8] {
    &INIT_EFFECTS[range.start() - EFX_TYPE..=range.end() - EFX_TYPE]
}

// ---------------------------------------------------------------------------
// Value tables
// ---------------------------------------------------------------------------

/// `T15`: the 16-step keyfollow table, in % (P1 p. 153).
const T15: [i16; 16] = [
    -100, -70, -50, -30, -10, 0, 10, 20, 30, 40, 50, 70, 100, 120, 150, 200,
];
/// `T14`: the 15-step keyfollow and velocity time table, in %.
const T14: [i16; 15] = [
    -100, -70, -50, -40, -30, -20, -10, 0, 10, 20, 30, 40, 50, 70, 100,
];

/// A keyfollow in % as the INTEGRA-7 stores it: 64 ± steps of 10 %.
const fn keyfollow<const N: usize>(pct: [i16; N]) -> [u8; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = (64 + pct[i] / 10) as u8;
        i += 1;
    }
    out
}

/// Velocity Time Sens (`T14` %), Roland's rule: `64 + trunc(pct × 63/100)`.
const fn time_sens<const N: usize>(pct: [i16; N]) -> [u8; N] {
    let mut out = [0; N];
    let mut i = 0;
    while i < N {
        out[i] = (64 + pct[i] * 63 / 100) as u8;
        i += 1;
    }
    out
}

/// Keyfollow rows over `T15`.
const T15_KEYFOLLOW: [u8; 16] = keyfollow(T15);
/// Keyfollow and Bias Level rows over `T14`.
const T14_KEYFOLLOW: [u8; 15] = keyfollow(T14);
/// Velocity Time Sens rows.
pub(super) const TIME_SENS: [u8; 15] = time_sens(T14);

/// Highest JV Velocity Sens on Roland's linear part.
const LINEAR_SENS_MAX: usize = 105;
/// Roland's Velocity Sens for JV raw 106–125 (+112 … +150 %).
const ROLAND_SENS_CURVE: [u8; 20] = [
    100, 101, 101, 102, 103, 104, 105, 106, 108, 109, 110, 111, 113, 115, 116, 118, 120, 122, 124,
    127,
];

/// Velocity Sens (JV raw 0–125 = −100 … +150 %), Roland's rule: linear
/// `64 + trunc((v − 50) × 16/25)` up to raw 105, so ±100 % is ±32, then
/// Roland's curve up to 127. Nothing is clamped.
const fn velocity_sens() -> [u8; 126] {
    let mut out = [0; 126];
    let mut v = 0;
    while v < out.len() {
        out[v] = if v <= LINEAR_SENS_MAX {
            (64 + (v as i32 - 50) * 16 / 25) as u8
        } else {
            ROLAND_SENS_CURVE[v - LINEAR_SENS_MAX - 1]
        };
        v += 1;
    }
    out
}

/// P-Env, Resonance, F-Env and Level Velocity Sens rows.
pub(super) const VELOCITY_SENS: [u8; 126] = velocity_sens();

/// LFO Waveform: TRI, SIN, SAW, SQR, TRP, S&H, RND, CHS. SAW is SAW-UP, as
/// Roland converts it.
const LFO_WAVEFORMS: [u8; 8] = [1, 0, 2, 4, 8, 9, 5, 10];

/// Key Assign Mode POLY, SOLO → Mono/Poly POLY, MONO.
const KEY_ASSIGN: [u8; 2] = [1, 0];

/// Roland's note value for a CLOCK- or TAP-synced Delay Time or LFO Rate:
/// `(last JV value, INTEGRA-7 note value 128–149)`, from 1/64T to 2/1.
///
/// The ranges Roland's pairs show are exact. The JV values no pair shows
/// (5, 15, 28, 34–35, 43–44, 49–52, 58, 77–79, 81–82, 101, 105–106, 114)
/// are a chosen stand-in: each goes to the nearer shown neighbour, ties to
/// the shorter note.
const SYNC_NOTES: [(u8, u16); 22] = [
    (2, 128),
    (3, 129),
    (5, 130),
    (7, 131),
    (8, 132),
    (10, 133),
    (15, 134),
    (17, 135),
    (21, 136),
    (28, 137),
    (34, 138),
    (43, 139),
    (50, 140),
    (58, 141),
    (66, 142),
    (78, 143),
    (81, 144),
    (90, 145),
    (101, 146),
    (105, 147),
    (114, 148),
    (127, 149),
];

/// The note value a synced JV rate or time `v` becomes.
fn sync_note(v: u8) -> u16 {
    SYNC_NOTES
        .iter()
        .find(|(last, _)| v <= *last)
        .map_or(149, |&(_, note)| note)
}

/// A JV control source: Patch Control Source 2/3 (`OFF` … `PLAYMATE`), or
/// EFX Control Source 1/2 (its first 11 entries).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlSource {
    /// OFF: its routes do nothing on the JV either.
    Off,
    /// An INTEGRA-7 Matrix or MFX Control Source.
    To(u8),
    /// PLAYMATE, or out of range: no INTEGRA-7 source.
    Unmapped,
}

/// JV MOD: Controller 1's fixed source.
const SOURCE_MOD: u8 = 3;

/// INTEGRA-7 Control Source of each JV source, in the JV's order. Indices
/// follow CC numbers (BEND = 96); SYS-CTRL1/2 → CTRL2/3, as Roland converts
/// them (`jv-xv-roland-pairs.md`, section b).
const CONTROL_SOURCES_TO: [Option<u8>; 16] = [
    None,      // OFF
    Some(99),  // SYS-CTRL1 → CTRL2
    Some(100), // SYS-CTRL2 → CTRL3
    Some(1),   // MOD → CC01
    Some(2),   // BREATH → CC02
    Some(4),   // FOOT → CC04
    Some(7),   // VOLUME → CC07
    Some(10),  // PAN → CC10
    Some(11),  // EXPRESSION → CC11
    Some(96),  // BEND
    Some(97),  // AFT
    Some(105), // LFO1
    Some(106), // LFO2
    Some(102), // VELOCITY
    Some(103), // KEYFOLLOW
    None,      // PLAYMATE: no target
];

/// The INTEGRA-7 source of JV control source `jv`.
pub(super) fn control_source(jv: u8) -> ControlSource {
    match (jv, CONTROL_SOURCES_TO.get(usize::from(jv))) {
        (0, _) => ControlSource::Off,
        (_, Some(Some(to))) => ControlSource::To(*to),
        _ => ControlSource::Unmapped,
    }
}

/// JV Controller Destinations, which index the INTEGRA-7 Matrix Control
/// Destinations 0–18 one for one.
const DESTINATIONS: [&str; 19] = [
    "OFF", "PCH", "CUT", "RES", "LEV", "PAN", "MIX", "CHO", "REV", "PL1", "PL2", "FL1", "FL2",
    "AL1", "AL2", "pL1", "pL2", "L1R", "L2R",
];

// ---------------------------------------------------------------------------
// Mapping rows
// ---------------------------------------------------------------------------

/// How a JV value becomes an INTEGRA-7 value.
#[derive(Debug, Clone, Copy)]
pub(super) enum How {
    /// The same value.
    Copy,
    /// The value plus a constant. A value pushed past 127 (out of the JV
    /// range) is held at 127.
    Offset(u8),
    /// A lookup. A value past the table (out of the JV range) leaves the
    /// INIT value.
    Table(&'static [u8]),
}

/// Where a row writes.
#[derive(Debug, Clone, Copy)]
enum To {
    /// Common, at an offset.
    Common(usize),
    /// MFX.
    Mfx(usize),
    /// PMT, outside the partial entries.
    Pmt(usize),
    /// The tone's own PMT partial entry, at an offset within it.
    PmtEntry(usize),
    /// The tone's own Partial.
    Partial(usize),
}

/// One mapping row: `count` consecutive JV fields from `from`, written to
/// as many consecutive INTEGRA-7 fields from `to`, a [`To`] for a Patch or a
/// Partial offset for a Rhythm Setup.
#[derive(Debug, Clone, Copy)]
pub(super) struct Row<T> {
    pub(super) from: usize,
    pub(super) to: T,
    pub(super) how: How,
    pub(super) count: usize,
}

/// A row of one field.
pub(super) const fn row<T: Copy>(from: usize, to: T, how: How) -> Row<T> {
    Row {
        from,
        to,
        how,
        count: 1,
    }
}

/// A row of one field copied as it is.
pub(super) const fn copy<T: Copy>(from: usize, to: T) -> Row<T> {
    copies(from, to, 1)
}

/// A row of `count` fields copied as they are.
pub(super) const fn copies<T: Copy>(from: usize, to: T, count: usize) -> Row<T> {
    Row {
        from,
        to,
        how: How::Copy,
        count,
    }
}

impl How {
    /// The INTEGRA-7 value of JV value `v`, or `None` to leave the INIT one.
    pub(super) fn apply(self, v: u8) -> Option<u8> {
        match self {
            Self::Copy => Some(v),
            Self::Offset(n) => Some(v.saturating_add(n).min(0x7F)),
            Self::Table(table) => table.get(usize::from(v)).copied(),
        }
    }
}

use How::{Offset, Table};
use To::{Common, Mfx, Partial, Pmt, PmtEntry};

/// Patch Common → Common, MFX and PMT (`jv-patch-format.md`, Common table).
/// EFX Type, Parameters, Mix Out Send Level and Control 1/2 go through the
/// effect map (see [`mfx`]).
const COMMON_ROWS: [Row<To>; 15] = [
    copies(0x00, Common(0x00), 12),              // Patch Name 1–12
    copy(0x1B, Mfx(0x02)),                       // EFX Chorus Send Level
    copy(0x1C, Mfx(0x03)),                       // EFX Reverb Send Level
    copy(0x2E, Common(0x0E)),                    // Patch Level
    copy(0x2F, Common(0x0F)),                    // Patch Pan
    copy(0x30, Common(0x15)),                    // Analog Feel
    copy(0x31, Common(0x29)),                    // Bend Range Up
    copy(0x32, Common(0x2A)),                    // Bend Range Down
    row(0x33, Common(0x16), Table(&KEY_ASSIGN)), // Key Assign Mode
    copy(0x34, Common(0x17)),                    // Solo Legato
    copies(0x35, Common(0x19), 5),               // Portamento Switch, Mode, Type, Start, Time
    row(0x41, Common(0x13), Offset(61)),         // Octave Shift
    copy(0x42, Common(0x14)),                    // Stretch Tune Depth
    copy(0x43, Common(0x10)),                    // Voice Priority
    copies(0x44, Pmt(0x00), 4),                  // Structure Type and Booster 1&2, 3&4
];

/// Patch Tone n → Partial n and its PMT entry (`jv-patch-format.md`, Tone
/// table). Envelope start levels need no row: the INIT Pitch L0 (64) and
/// TVF L0 (0) are what Roland converts to.
const TONE_ROWS: [Row<To>; 68] = [
    copy(0x00, PmtEntry(0)),                         // Tone Switch
    copy(0x05, Partial(0x34)),                       // Wave Gain
    copy(0x06, Partial(0x35)),                       // FXM Switch
    copy(0x07, Partial(0x36)),                       // FXM Color
    row(0x08, Partial(0x37), Offset(1)),             // FXM Depth
    copies(0x0E, PmtEntry(1), 2),                    // Keyboard Range Lower, Upper
    copy(0x10, Partial(0x16)),                       // Redamper Control Switch
    copy(0x12, Partial(0x14)),                       // Hold-1 Control Switch
    copy(0x13, Partial(0x12)),                       // Pitch Bend Control Switch
    row(0x2D, Partial(0x6D), Table(&LFO_WAVEFORMS)), // LFO1 Waveform
    copy(0x2E, Partial(0x76)),                       // LFO1 Key Sync
    copy(0x30, Partial(0x70)),                       // LFO1 Offset
    copy(0x31, Partial(0x72)),                       // LFO1 Delay Time
    copy(0x32, Partial(0x74)),                       // LFO1 Fade Mode
    copy(0x33, Partial(0x75)),                       // LFO1 Fade Time
    row(0x35, Partial(0x7B), Table(&LFO_WAVEFORMS)), // LFO2 Waveform
    copy(0x36, Partial(0x84)),                       // LFO2 Key Sync
    copy(0x38, Partial(0x7E)),                       // LFO2 Offset
    copy(0x39, Partial(0x80)),                       // LFO2 Delay Time
    copy(0x3A, Partial(0x82)),                       // LFO2 Fade Mode
    copy(0x3B, Partial(0x83)),                       // LFO2 Fade Time
    row(0x3D, Partial(0x01), Offset(16)),            // Coarse Tune
    row(0x3E, Partial(0x02), Offset(14)),            // Fine Tune
    copy(0x3F, Partial(0x03)),                       // Random Pitch Depth
    row(0x40, Partial(0x39), Table(&T15_KEYFOLLOW)), // Pitch Keyfollow
    row(0x41, Partial(0x3A), Offset(52)),            // P-Env Depth
    row(0x42, Partial(0x3B), Table(&VELOCITY_SENS)), // P-Env Velocity Sens
    row(0x43, Partial(0x3C), Table(&TIME_SENS)),     // P-Env Velocity Time 1
    row(0x44, Partial(0x3D), Table(&TIME_SENS)),     // P-Env Velocity Time 4
    row(0x45, Partial(0x3E), Table(&T14_KEYFOLLOW)), // P-Env Time Keyfollow
    copies(0x46, Partial(0x3F), 4),                  // P-Env Time 1–4
    row(0x4A, Partial(0x44), Offset(1)),             // P-Env Level 1
    row(0x4B, Partial(0x45), Offset(1)),             // P-Env Level 2
    row(0x4C, Partial(0x46), Offset(1)),             // P-Env Level 3
    row(0x4D, Partial(0x47), Offset(1)),             // P-Env Level 4
    row(0x4E, Partial(0x77), Offset(1)),             // Pitch LFO1 Depth
    row(0x4F, Partial(0x85), Offset(1)),             // Pitch LFO2 Depth
    copy(0x50, Partial(0x48)),                       // Filter Type
    copy(0x51, Partial(0x49)),                       // Cutoff Frequency
    row(0x52, Partial(0x4A), Table(&T15_KEYFOLLOW)), // Cutoff Keyfollow
    copy(0x53, Partial(0x4D)),                       // Resonance
    row(0x54, Partial(0x4E), Table(&VELOCITY_SENS)), // Resonance Velocity Sens
    row(0x55, Partial(0x4F), Offset(1)),             // F-Env Depth
    row(0x56, Partial(0x50), Offset(1)),             // F-Env Velocity Curve
    row(0x57, Partial(0x51), Table(&VELOCITY_SENS)), // F-Env Velocity Sens
    row(0x58, Partial(0x52), Table(&TIME_SENS)),     // F-Env Velocity Time 1
    row(0x59, Partial(0x53), Table(&TIME_SENS)),     // F-Env Velocity Time 4
    row(0x5A, Partial(0x54), Table(&T14_KEYFOLLOW)), // F-Env Time Keyfollow
    copies(0x5B, Partial(0x55), 4),                  // F-Env Time 1–4
    copies(0x5F, Partial(0x5A), 4),                  // F-Env Level 1–4
    row(0x63, Partial(0x78), Offset(1)),             // Filter LFO1 Depth
    row(0x64, Partial(0x86), Offset(1)),             // Filter LFO2 Depth
    copy(0x65, Partial(0x00)),                       // Tone Level
    copy(0x66, Partial(0x60)),                       // Bias Direction
    copy(0x67, Partial(0x5F)),                       // Bias Position
    row(0x68, Partial(0x5E), Table(&T14_KEYFOLLOW)), // Bias Level
    row(0x69, Partial(0x61), Offset(1)),             // Level Velocity Curve
    row(0x6A, Partial(0x62), Table(&VELOCITY_SENS)), // Level Velocity Sens
    row(0x6B, Partial(0x63), Table(&TIME_SENS)),     // Level Velocity Time 1
    row(0x6C, Partial(0x64), Table(&TIME_SENS)),     // Level Velocity Time 4
    row(0x6D, Partial(0x65), Table(&T14_KEYFOLLOW)), // Level Time Keyfollow
    copies(0x6E, Partial(0x66), 7),                  // Level Env Time 1–4, Level 1–3
    row(0x75, Partial(0x79), Offset(1)),             // Amp LFO1 Depth
    row(0x76, Partial(0x87), Offset(1)),             // Amp LFO2 Depth
    copy(0x77, Partial(0x04)),                       // Tone Pan
    row(0x78, Partial(0x05), Table(&T14_KEYFOLLOW)), // Pan Keyfollow
    copies(0x79, Partial(0x06), 2),                  // Random Pan Depth, Alternate Pan Depth
    copy(0x7E, Partial(0x0C)),                       // Mix/EFX Send Level → Output Level
];

/// The named code rows, in order: what needs more than one JV byte, writes
/// more than one field, or reports.
const CODE_ROWS: [fn(&mut Patch<'_>); 14] = [
    mfx,
    studio_set,
    hold_peak,
    legato_retrigger,
    velocity_ranges,
    matrix_controls,
    delay,
    lfo_rates,
    tempo,
    volume_control,
    pan_control,
    sends,
    category,
    waves,
];

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

/// A Patch being converted.
struct Patch<'a> {
    model: Model,
    /// The Patch Common, as long as its model makes it.
    common: &'a [u8],
    tones: [&'a [u8]; 4],
    tone: PcmTone,
    /// The INTEGRA-7 Tone Category.
    category: u8,
    report: ConversionReport,
}

impl Patch<'_> {
    /// Whether JV Tone `i` (0-indexed) is switched on.
    fn active(&self, i: usize) -> bool {
        self.tones[i][TONE_SWITCH] != 0
    }

    /// The byte `k` fields past `to`, for partial `i`.
    fn target(&mut self, i: usize, to: To, k: usize) -> &mut u8 {
        match to {
            Common(at) => &mut self.tone.common()[at + k],
            Mfx(at) => &mut self.tone.mfx()[at + k],
            Pmt(at) => &mut self.tone.pmt()[at + k],
            PmtEntry(at) => &mut self.tone.pmt()[PMT_ENTRY + PMT_STRIDE * i + at + k],
            Partial(at) => &mut self.tone.partial(i)[at + k],
        }
    }

    /// Apply `rows` reading `source`, for partial `i`.
    fn apply(&mut self, rows: &[Row<To>], source: &[u8], i: usize) {
        for row in rows {
            for k in 0..row.count {
                if let Some(value) = row.how.apply(source[row.from + k]) {
                    *self.target(i, row.to, k) = value;
                }
            }
        }
    }

    fn drop_param(&mut self, partial: Option<usize>, param: &'static str) {
        self.report.push(ConversionItem::ParamDropped {
            at: partial.map(|i| Location::Partial(i as u8 + 1)),
            param,
        });
    }

    fn not_applied(&mut self, what: String, settings: Option<StudioSetEffect>) {
        self.report
            .push(ConversionItem::NotApplied { what, settings });
    }
}

/// The Patch Common length of the Patch at `key`: how far its frames run
/// past the Common start, before Tone 1.
fn common_len(key: [u8; 2], frames: &[Frame<'_>]) -> usize {
    let base = Address::new(key[0], key[1], 0, 0).value() as usize;
    let tone1 = Address::new(key[0], key[1], TONE_OFFSETS[0][0], 0).value() as usize;
    frames
        .iter()
        .map(|f| (Address(f.address).value() as usize, f.data.len()))
        .filter(|&(start, _)| start < tone1)
        .map(|(start, len)| (start + len).min(tone1) - base)
        .max()
        .unwrap_or(0)
}

/// Convert the Patch whose frames share address bytes `key`. A Patch whose
/// Common is shorter than the JV-1080's, or that misses a block, is
/// incomplete. A longer Common is read as far as the JV-2080's.
pub(super) fn convert(key: [u8; 2], frames: &[Frame<'_>]) -> Result<Converted, Skipped> {
    let (model, len) = match common_len(key, frames) {
        COMMON_JV1080 => (Model::Jv1080, COMMON_JV1080),
        COMMON_XP => (Model::Xp, COMMON_XP),
        n if n >= COMMON_JV2080 => (Model::Jv2080, COMMON_JV2080),
        _ => return Err(Skipped::Incomplete),
    };
    let block = |[c, d]: [u8; 2], size| (Address::new(key[0], key[1], c, d).value(), size);
    let layout: Vec<(u32, usize)> = [block([0, 0], len)]
        .into_iter()
        .chain(TONE_OFFSETS.map(|o| block(o, TONE_LEN)))
        .collect();
    let dt1s = frames.iter().map(|f| (Address(f.address), f.data));
    let jv = frame::assemble(&layout, dt1s).ok_or(Skipped::Incomplete)?;

    let mut patch = Patch {
        model,
        common: &jv[0],
        tones: core::array::from_fn(|i| jv[1 + i].as_slice()),
        tone: PcmTone::init(),
        category: 0,
        report: ConversionReport::new(),
    };
    patch.apply(&COMMON_ROWS, patch.common, 0);
    for i in 0..4 {
        let source = patch.tones[i];
        patch.apply(&TONE_ROWS, source, i);
    }
    for code in CODE_ROWS {
        code(&mut patch);
    }
    patch.tone.fill_common2(patch.category);
    Ok((patch.tone.into_entry(), patch.report, Some(model)))
}

/// EFX → MFX through the effect map, when a Tone goes through it; the MFX
/// stays THRU otherwise. Tones that bypassed the EFX go through the MFX on
/// the INTEGRA-7, and are reported, as is a Mix Out Send Level below 127
/// (the INTEGRA-7 keeps a reserve there). EFX Control Source and Depth 1/2 map to
/// MFX Control 1/2; their destination is fixed per JV type, so Control
/// Assign stays OFF and a control in use is reported.
// ponytail: the map has no per-type destinations yet (#236).
fn mfx(p: &mut Patch<'_>) {
    let bypassed: Vec<usize> = (0..4)
        .filter(|&i| p.active(i) && p.tones[i][TONE_OUTPUT_ASSIGN] != OUTPUT_EFX)
        .collect();
    if (0..4).filter(|&i| p.active(i)).count() == bypassed.len() {
        return;
    }
    let effect = ForeignEffect {
        family: EffectFamily::JvEfx,
        number: p.common[EFX_TYPE] + 1,
    };
    let mut source = [0; MFX_PARAM_COUNT];
    for (slot, &v) in source
        .iter_mut()
        .zip(&p.common[EFX_TYPE + 1..=EFX_LAST_PARAM])
    {
        *slot = i32::from(v);
    }
    if !fx_map::convert(effect, &source, p.tone.mfx(), &mut p.report) {
        return;
    }
    for i in bypassed {
        p.drop_param(Some(i), "EFX bypass");
    }
    if p.common[EFX_MIX_OUT] != pcm_tone::FULL_LEVEL {
        p.drop_param(None, "EFX Mix Out Send Level");
    }
    for (k, (at, name)) in (0..).zip(EFX_CONTROLS) {
        let (source, depth) = (p.common[at], p.common[at + 1]);
        let mapped = match control_source(source) {
            ControlSource::To(to) if source <= EFX_LAST_SOURCE => Some(to),
            ControlSource::Off => Some(0),
            _ => None,
        };
        let at = |field| usize::from(mfx_header_offset(field, k).expect("MFX Control 1/2"));
        let mfx = p.tone.mfx();
        mfx[at(MfxHeaderField::ControlSource)] = mapped.unwrap_or(0);
        mfx[at(MfxHeaderField::ControlSens)] = depth.saturating_add(1).min(0x7F);
        if source != 0 && depth != NEUTRAL_DEPTH {
            p.drop_param(None, name);
        }
    }
}

/// Tone Output Assign EFX: through the EFX.
pub(super) const OUTPUT_EFX: u8 = 1;
/// EFX Output Assign names.
const EFX_OUTPUTS: [&str; 3] = ["MIX", "OUTPUT 1", "OUTPUT 2"];
/// Tone and Rhythm Note Output Assign names.
pub(super) const TONE_OUTPUTS: [&str; 4] = ["MIX", "EFX", "OUTPUT 1", "OUTPUT 2"];
/// Chorus Output names.
const CHORUS_OUTPUTS: [&str; 3] = ["MIX", "REV", "MIX+REV"];
/// Reverb Type names.
const REVERB_TYPES: [&str; 8] = [
    "ROOM1", "ROOM2", "STAGE1", "STAGE2", "HALL1", "HALL2", "DELAY", "PAN-DLY",
];
/// The JV INIT Patch Tempo.
const INIT_TEMPO: u16 = 120;
/// Clock Source PATCH: the Patch Tempo is used.
const CLOCK_PATCH: u8 = 0;

/// The name of `value` in `names`, or the raw value.
pub(super) fn named(names: &[&str], value: u8) -> String {
    names
        .get(usize::from(value))
        .map_or_else(|| value.to_string(), |n| (*n).to_string())
}

/// Patch Chorus, Reverb and EFX Output Assign: they live in the Studio Set
/// on the INTEGRA-7. Reported when they differ from the JV INIT.
fn studio_set(p: &mut Patch<'_>) {
    let c = p.common;
    if c[CHORUS] != *init(CHORUS) {
        let [level, rate, depth, pre_delay, feedback, output] =
            c[CHORUS].try_into().expect("6 Chorus bytes");
        p.not_applied(
            format!(
                "Chorus level {level}, rate {rate}, depth {depth}, pre-delay {pre_delay}, \
                 feedback {feedback}, output {}",
                named(&CHORUS_OUTPUTS, output)
            ),
            Some(jv_chorus([level, rate, depth, pre_delay, feedback, output])),
        );
    }
    if c[REVERB] != *init(REVERB) {
        let [kind, level, time, damp, feedback] = c[REVERB].try_into().expect("5 Reverb bytes");
        p.not_applied(
            format!(
                "Reverb {}, level {level}, time {time}, HF damp {}, feedback {feedback}",
                named(&REVERB_TYPES, kind),
                FxUnit::Freq.format(damp.into())
            ),
            jv_reverb(kind, level, time),
        );
    }
    let output = c[EFX_OUTPUT_ASSIGN];
    if output != 0 {
        p.not_applied(
            format!("EFX Output Assign {}", named(&EFX_OUTPUTS, output)),
            None,
        );
    }
}

/// EFX and Controller 1–3 Hold/Peak: no INTEGRA-7 equivalent.
fn hold_peak(p: &mut Patch<'_>) {
    const NAMES: [&str; 4] = [
        "EFX Control Hold/Peak",
        "Control 1 Hold/Peak",
        "Control 2 Hold/Peak",
        "Control 3 Hold/Peak",
    ];
    for (k, name) in NAMES.into_iter().enumerate() {
        if p.common[HOLD_PEAK + k] != 0 {
            p.drop_param(None, name);
        }
    }
}

/// Legato Retrigger has no JV source; Roland's conversions set it ON.
fn legato_retrigger(p: &mut Patch<'_>) {
    p.tone.common()[LEGATO_RETRIGGER] = 1;
}

/// Velocity ranges: PMT Velocity Control is ON, as Roland converts it. With
/// the Velocity Range Switch ON, each Tone's range is copied and its Cross
/// Fade goes to both fade widths; OFF leaves the INIT 1–127, no fade.
fn velocity_ranges(p: &mut Patch<'_>) {
    p.tone.pmt()[PMT_VELOCITY_CONTROL] = 1;
    if p.common[VELOCITY_RANGE_SWITCH] == 0 {
        return;
    }
    for i in 0..4 {
        let [fade, lower, upper] = p.tones[i][VELOCITY_CROSS_FADE..VELOCITY_CROSS_FADE + 3]
            .try_into()
            .expect("3 bytes");
        let entry = PMT_ENTRY + PMT_STRIDE * i;
        p.tone.pmt()[entry + 5..entry + 9].copy_from_slice(&[lower, upper, fade, fade]);
    }
}

/// Matrix Control state while packing.
struct MatrixControl {
    source: u8,
    /// `(destination, sens)` of the used slots.
    slots: Vec<(u8, u8)>,
}

/// INTEGRA-7 Matrix Controls.
const MATRIX_COUNT: usize = 4;
/// Slots per Matrix Control.
const SLOTS: usize = 4;
/// Partial Control Switch ON and REVERSE.
const SWITCH_ON: u8 = 1;
/// Partial Control Switch REVERSE.
const SWITCH_REVERSE: u8 = 2;
/// A sens of 0.
const SENS_ZERO: u8 = 64;

/// Controllers 1–3 → Matrix Controls 1–4, by Roland's greedy rule
/// (`jv-xv-roland-pairs.md`, section b). Tone by tone, controller by
/// controller, slot by slot, each route `(destination, sens)` goes:
///
/// 1. to the first Matrix Control of its source with a slot holding it
///    (switch ON) or its negated sens (REVERSE), that the tone doesn't use
///    yet;
/// 2. else to the first Matrix Control of its source with a free slot;
/// 3. else to the next unused Matrix Control;
/// 4. else it is dropped and reported.
///
/// Matrix Controls are handed out in order of first use, and controllers
/// with the same source share them. A route whose source has no INTEGRA-7
/// equivalent is reported too.
fn matrix_controls(p: &mut Patch<'_>) {
    let sources = [
        control_source(SOURCE_MOD),
        control_source(p.common[CONTROL_SOURCES[0]]),
        control_source(p.common[CONTROL_SOURCES[1]]),
    ];
    let mut mcs: Vec<MatrixControl> = Vec::with_capacity(MATRIX_COUNT);
    let mut switches = [[0u8; MATRIX_COUNT * SLOTS]; 4];
    for (t, switches) in switches.iter_mut().enumerate() {
        if !p.active(t) {
            continue;
        }
        for (c, source) in sources.into_iter().enumerate() {
            for k in 0..SLOTS {
                let at = CONTROLLERS + 8 * c + 2 * k;
                let (dest, sens) = (p.tones[t][at], p.tones[t][at + 1].saturating_add(1));
                if dest == 0 || sens == SENS_ZERO {
                    continue;
                }
                let source = match source {
                    ControlSource::Off => continue,
                    ControlSource::To(source) => Some(source),
                    ControlSource::Unmapped => None,
                };
                let placed = source.and_then(|s| place_route(&mut mcs, switches, s, dest, sens));
                if placed.is_none() {
                    p.report.push(ConversionItem::RoutingDropped {
                        partial: t as u8 + 1,
                        control: c as u8 + 1,
                        destination: DESTINATIONS.get(usize::from(dest)).copied().unwrap_or("?"),
                        sens: sens as i8 - SENS_ZERO as i8,
                    });
                }
            }
        }
    }
    let common = p.tone.common();
    // Unused Matrix Controls and slots are OFF, not INIT TONE's routes.
    for m in 0..MATRIX_COUNT {
        let base = MATRIX_CONTROLS + MATRIX_STRIDE * m;
        let mc = mcs.get(m);
        common[base] = mc.map_or(0, |mc| mc.source);
        for j in 0..SLOTS {
            let slot = mc.and_then(|mc| mc.slots.get(j).copied());
            let (dest, sens) = slot.unwrap_or((0, SENS_ZERO));
            common[base + 1 + 2 * j..base + 3 + 2 * j].copy_from_slice(&[dest, sens]);
        }
    }
    for (t, switches) in switches.iter().enumerate() {
        p.tone.partial(t)[CONTROL_SWITCHES..CONTROL_SWITCHES + switches.len()]
            .copy_from_slice(switches);
    }
}

/// Place one route of `source`, setting the tone's `switches`. `None` when
/// all Matrix Controls are in use.
fn place_route(
    mcs: &mut Vec<MatrixControl>,
    switches: &mut [u8; MATRIX_COUNT * SLOTS],
    source: u8,
    dest: u8,
    sens: u8,
) -> Option<()> {
    for (m, mc) in mcs.iter().enumerate().filter(|(_, mc)| mc.source == source) {
        for (j, &(d, s)) in mc.slots.iter().enumerate() {
            let switch = &mut switches[SLOTS * m + j];
            if *switch != 0 || d != dest {
                continue;
            }
            if s == sens {
                *switch = SWITCH_ON;
                return Some(());
            }
            if s == 128 - sens {
                *switch = SWITCH_REVERSE;
                return Some(());
            }
        }
    }
    let free = mcs
        .iter()
        .position(|mc| mc.source == source && mc.slots.len() < SLOTS);
    let m = match free {
        Some(m) => m,
        None if mcs.len() < MATRIX_COUNT => {
            mcs.push(MatrixControl {
                source,
                slots: Vec::with_capacity(SLOTS),
            });
            mcs.len() - 1
        }
        None => return None,
    };
    mcs[m].slots.push((dest, sens));
    switches[SLOTS * m + mcs[m].slots.len() - 1] = SWITCH_ON;
    Some(())
}

/// Tone Delay Mode and Time, as Roland converts them: HOLD and KEY-OFF
/// modes copy; CLOCK-SYNC and TAP-SYNC become NORMAL with a note-valued
/// time; KEY-INTERVAL/PLAYMATE becomes NORMAL with no delay, reported.
/// TEMPO-SYNC (JV-2080/XP) no pair shows: it is read as CLOCK-SYNC, a
/// chosen stand-in.
fn delay(p: &mut Patch<'_>) {
    for i in 0..4 {
        let (mode, time) = (p.tones[i][DELAY_MODE], p.tones[i][DELAY_TIME]);
        let (mode, time) = match mode {
            0 => (0, u16::from(time)), // NORMAL
            1 => (1, u16::from(time)), // HOLD
            2 => {
                if p.active(i) {
                    let name = match p.model {
                        Model::Jv1080 => "Delay Mode KEY-INTERVAL",
                        Model::Xp | Model::Jv2080 => "Delay Mode PLAYMATE",
                    };
                    p.drop_param(Some(i), name);
                }
                (0, 0)
            }
            3 | 4 | 7 => (0, sync_note(time)), // CLOCK-, TAP-, TEMPO-SYNC
            5 => (2, u16::from(time)),         // KEY-OFF-NORMAL
            6 => (3, u16::from(time)),         // KEY-OFF-DECAY
            _ => (0, u16::from(time)),
        };
        let partial = p.tone.partial(i);
        partial[PARTIAL_DELAY_MODE] = mode;
        partial[PARTIAL_DELAY_TIME..PARTIAL_DELAY_TIME + 2].copy_from_slice(&encode_nibble2(time));
    }
}

/// LFO1/2 Rate: copied, or a note value when External Sync is CLOCK or TAP,
/// as Roland converts it.
fn lfo_rates(p: &mut Patch<'_>) {
    for i in 0..4 {
        for ((rate, sync), to) in LFO_RATES.into_iter().zip(PARTIAL_LFO_RATES) {
            let (rate, sync) = (p.tones[i][rate], p.tones[i][sync]);
            let rate = if sync == 0 {
                u16::from(rate)
            } else {
                sync_note(rate)
            };
            p.tone.partial(i)[to..to + 2].copy_from_slice(&encode_nibble2(rate));
        }
    }
}

/// Patch Tempo: it lives in the Studio Set on the INTEGRA-7. Reported when
/// the Patch uses it, once its synced Delay and LFO Rates are note values.
fn tempo(p: &mut Patch<'_>) {
    let tempo = nibble2(p.common, PATCH_TEMPO);
    let clock_patch = p.model == Model::Jv1080 || p.common[CLOCK_SOURCE] == CLOCK_PATCH;
    if clock_patch && tempo != INIT_TEMPO && p.tone.syncs_to_tempo() {
        p.not_applied(format!("Tempo {tempo}"), None);
    }
}

/// Volume Control Switch: not carried; Receive Expression stays ON.
fn volume_control(p: &mut Patch<'_>) {
    for i in 0..4 {
        if p.active(i) && p.tones[i][VOLUME_CONTROL] == 0 {
            p.drop_param(Some(i), "Volume Control Switch");
        }
    }
}

/// Pan Control Switch CONTINUOUS.
const PAN_CONTINUOUS: u8 = 1;
/// Pan Control Switch KEY-ON.
const PAN_KEY_ON: u8 = 2;

/// Pan Control Switch `pan` → the Receive Pan Mode reserve, and whether it
/// is dropped. KEY-ON goes to the reserve as KEY-ON (1), as Roland converts
/// it. The INTEGRA-7 reads neither KEY-ON nor OFF, so both are dropped.
pub(super) fn receive_pan_mode(pan: u8) -> (u8, bool) {
    (u8::from(pan == PAN_KEY_ON), pan != PAN_CONTINUOUS)
}

/// Pan Control Switch, by [`receive_pan_mode`].
fn pan_control(p: &mut Patch<'_>) {
    for i in 0..4 {
        let (mode, dropped) = receive_pan_mode(p.tones[i][PAN_CONTROL]);
        p.tone.partial(i)[RECEIVE_PAN_MODE] = mode;
        if p.active(i) && dropped {
            p.drop_param(Some(i), "Pan Control Switch");
        }
    }
}

/// Tone Chorus/Reverb Send Levels, in the send pair the Tone's output picks.
/// A direct output lives in the Studio Set on the INTEGRA-7.
fn sends(p: &mut Patch<'_>) {
    for i in 0..4 {
        let tone = p.tones[i];
        let output = tone[TONE_OUTPUT_ASSIGN];
        let routed = if output == OUTPUT_EFX {
            Output::Mfx
        } else {
            Output::Mix
        };
        let [chorus, reverb] = TONE_SENDS.map(|at| tone[at]);
        pcm_tone::write_sends(p.tone.partial(i), routed, chorus, reverb);
        if p.active(i) && output > OUTPUT_EFX {
            p.not_applied(
                format!(
                    "Partial {} Output Assign {}",
                    i + 1,
                    named(&TONE_OUTPUTS, output)
                ),
                None,
            );
        }
    }
}

/// Patch Category (JV-2080 family): the XV Patch Category of the same
/// number, as Roland converts it, kept in the Common reserve the XV fills.
fn category(p: &mut Patch<'_>) {
    if p.model != Model::Jv2080 {
        return;
    }
    let jv = p.common[PATCH_CATEGORY];
    if let Some(&category) = CATEGORIES.get(usize::from(jv)) {
        p.category = category;
        p.tone.common()[LEGACY_CATEGORY] = jv;
    }
}

/// Wave Group Type INT.
const WAVE_INT: u8 = 0;
/// Wave Group Type PCM (card).
const WAVE_PCM: u8 = 1;
/// INT Wave Group ID INT-B; 1 is INT-A.
const INT_B: u8 = 2;

/// The Foreign Wave of a Patch Tone or Rhythm Note, which both start with
/// the switch, then Wave Group Type, Wave Group ID and Wave Number.
pub(super) fn foreign_wave(tone: &[u8]) -> (ForeignWaveGroup, u16) {
    let id = tone[WAVE_GROUP_ID];
    let group = match tone[WAVE_GROUP_TYPE] {
        WAVE_INT if id == INT_B => ForeignWaveGroup::JvIntB,
        WAVE_INT => ForeignWaveGroup::JvIntA,
        WAVE_PCM => ForeignWaveGroup::PcmCard(id.into()),
        _ => ForeignWaveGroup::SrJv80(id.into()),
    };
    (group, nibble2(tone, WAVE_NUMBER) + 1)
}

/// Waves: INT-A/B and SR-JV80 boards (Wave Group Type EXP) go through the
/// Wave map; PCM-card Waves have none.
fn waves(p: &mut Patch<'_>) {
    for i in 0..4 {
        let (group, number) = foreign_wave(p.tones[i]);
        pcm_tone::map_waves(&mut p.tone, i, group, [number, 0], &mut p.report);
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
        for patch in [
            [0x03, 0, 0, 0],
            [0x02, 0x0F, 0, 0],
            [0x11, 0x7F, 0, 0],
            [0x21, 5, 0, 0],
        ] {
            assert_eq!(kind(patch), Some(Skipped::Patch), "{patch:02X?}");
        }
        for rhythm in [
            [0x02, 0x09, 0x23, 0],
            [0x10, 0x41, 0, 0],
            [0x20, 0x40, 0, 0],
        ] {
            assert_eq!(kind(rhythm), Some(Skipped::Rhythm), "{rhythm:02X?}");
        }
        assert_eq!(kind([0x10, 0x1F, 0, 0]), Some(Skipped::Performance));
        assert_eq!(kind([0x01, 0x00, 0x10, 0]), Some(Skipped::Performance));
        assert_eq!(kind([0x00, 0x00, 0x00, 0x05]), Some(Skipped::System));
        assert_eq!(kind([0x7F, 0, 0, 0]), None);
    }

    #[test]
    fn velocity_sens_reproduces_rolands_table() {
        // `jv-xv-roland-pairs.md`, section a: JV raw 0–125 → XV/INTEGRA-7.
        #[rustfmt::skip]
        let roland: [u8; 126] = [
            32, 33, 34, 34, 35, 36, 36, 37, 38, 38, 39, 40, 40, 41, 41, 42,
            43, 43, 44, 45, 45, 46, 47, 47, 48, 48, 49, 50, 50, 51, 52, 52,
            53, 54, 54, 55, 56, 56, 57, 57, 58, 59, 59, 60, 61, 61, 62, 63,
            63, 64, 64, 64, 65, 65, 66, 67, 67, 68, 69, 69, 70, 71, 71, 72,
            72, 73, 74, 74, 75, 76, 76, 77, 78, 78, 79, 80, 80, 81, 81, 82,
            83, 83, 84, 85, 85, 86, 87, 87, 88, 88, 89, 90, 90, 91, 92, 92,
            93, 94, 94, 95, 96, 96, 97, 97, 98, 99, 100, 101, 101, 102, 103, 104,
            105, 106, 108, 109, 110, 111, 113, 115, 116, 118, 120, 122, 124, 127,
        ];
        for v in 0..=125u8 {
            assert_eq!(
                Table(&VELOCITY_SENS).apply(v),
                Some(roland[usize::from(v)]),
                "{v}"
            );
        }
        assert_eq!(Table(&VELOCITY_SENS).apply(126), None);
    }

    #[test]
    fn time_sens_and_keyfollows_follow_rolands_rules() {
        assert_eq!(
            TIME_SENS,
            [1, 20, 33, 39, 46, 52, 58, 64, 70, 76, 82, 89, 95, 108, 127]
        );
        assert_eq!(
            T15_KEYFOLLOW,
            [
                54, 57, 59, 61, 63, 64, 65, 66, 67, 68, 69, 71, 74, 76, 79, 84
            ]
        );
        assert_eq!(
            T14_KEYFOLLOW,
            [54, 57, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 71, 74]
        );
    }

    #[test]
    fn sync_notes_hit_rolands_anchors() {
        // 1/16, 1/8, 1/4, 1/2, 1/1, 2/1.
        let anchors = [
            (12, 134),
            (24, 137),
            (48, 140),
            (72, 143),
            (96, 146),
            (120, 149),
        ];
        for (v, note) in anchors {
            assert_eq!(sync_note(v), note, "{v}");
        }
        let notes: Vec<u16> = (0..=127).map(sync_note).collect();
        assert!(notes.windows(2).all(|w| w[0] <= w[1]));
        assert_eq!((notes[0], notes[127]), (128, 149));
    }

    #[test]
    fn control_sources_follow_cc_numbers() {
        use ControlSource::{Off, To, Unmapped};
        let sources: Vec<_> = (0..17).map(control_source).collect();
        assert_eq!(
            sources,
            [
                Off,
                To(99),
                To(100),
                To(1),
                To(2),
                To(4),
                To(7),
                To(10),
                To(11),
                To(96),
                To(97),
                To(105),
                To(106),
                To(102),
                To(103),
                Unmapped,
                Unmapped,
            ]
        );
    }

    #[test]
    fn a_patch_is_one_unit() {
        let common = place([0x11, 0x05, 0, 0]).unwrap();
        let tone = place([0x11, 0x05, 0x12, 0]).unwrap();
        assert_eq!(common.unit, tone.unit);
        assert!(common.common && !tone.common);
    }
}
