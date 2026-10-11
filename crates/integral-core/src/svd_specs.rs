//! SVD parameter specification tables for bit-packing/unpacking.
//!
//! Each tone type has a set of [`SvdSection`]s that define how SysEx
//! parameters map to the SVD bitstream. The spec tables are derived from
//! the MIDI Implementation documents in `docs/midi/`.
//!
//! Reference: `docs/svd/02-encoding.md`, `docs/svd/03-sn-synth.md`

use crate::svd::ChunkType;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// How a single SysEx parameter packs into the SVD bitstream.
#[derive(Debug, Clone, Copy)]
pub struct ParamBits {
    /// Number of significant bits (from the SysEx bit-mask pattern).
    /// For nibblized params, this is 4 × the nibble count.
    pub bits: u8,
    /// Number of SysEx bytes this parameter spans.
    /// Normal params: 1. Nibblized (`#`-marked) params: the nibble count.
    /// Padding ([`ParamBits::pad`]): 0.
    pub sysex_bytes: u8,
    /// Whether this parameter uses signed/centered encoding.
    ///
    /// Signed params have SysEx values centered at 64 (e.g., range 61–67
    /// where 64 = center). The SVD re-centers these at `2^(bits-1)`.
    /// Conversion: `sysex = svd + 64 - 2^(bits-1)`.
    ///
    /// Only relevant for params with `bits < 7` and non-zero range minimums.
    pub signed: bool,
}

impl ParamBits {
    /// A normal (non-nibblized) unsigned parameter with the given bit width.
    const fn normal(bits: u8) -> Self {
        Self {
            bits,
            sysex_bytes: 1,
            signed: false,
        }
    }

    /// A normal parameter with signed/centered encoding.
    ///
    /// Use for params whose SysEx range is centered at 64 and whose bit
    /// width is less than 7 (e.g., Octave Shift, Pitch, Keyfollow).
    const fn signed(bits: u8) -> Self {
        Self {
            bits,
            sysex_bytes: 1,
            signed: true,
        }
    }

    /// A nibblized (`#`-marked) parameter spanning `count` SysEx bytes,
    /// each carrying a 4-bit nibble.
    const fn nibblized(count: u8) -> Self {
        Self {
            bits: count * 4,
            sysex_bytes: count,
            signed: false,
        }
    }

    /// `bits` zero bits in the SVD that map to no SysEx byte.
    const fn pad(bits: u8) -> Self {
        Self {
            bits,
            sysex_bytes: 0,
            signed: false,
        }
    }
}

/// A section of parameters that is independently byte-aligned in SVD.
#[derive(Debug, Clone, Copy)]
pub struct SvdSection {
    /// Parameter specifications, in SysEx address order.
    pub params: &'static [ParamBits],
    /// Exact number of bytes this section occupies in the SVD entry.
    /// This may be larger than `padded_bytes()` due to additional
    /// zero padding beyond byte alignment.
    pub svd_bytes: usize,
}

impl SvdSection {
    /// Total significant bits in this section (before padding).
    pub fn total_bits(&self) -> usize {
        self.params.iter().map(|p| p.bits as usize).sum()
    }

    /// Byte-padded size of this section in the SVD bitstream.
    pub fn padded_bytes(&self) -> usize {
        self.total_bits().div_ceil(8)
    }

    /// Total number of SysEx bytes this section maps to.
    pub fn sysex_size(&self) -> usize {
        self.params.iter().map(|p| p.sysex_bytes as usize).sum()
    }
}

/// Full tone type specification for SVD encoding.
#[derive(Debug, Clone)]
pub struct SvdToneSpec {
    /// The chunk type whose entries this spec encodes.
    pub kind: ChunkType,
    /// Sections in SVD packing order. Each section is independently
    /// byte-aligned in the SVD entry.
    pub sections: &'static [SvdSection],
    /// Total SVD entry size in bytes, including any bytes after the data.
    pub entry_size: usize,
    /// Bytes after the data sections in a fresh entry, before zero padding
    /// to `entry_size`. SN-S has a `0x0E` end marker and an INIT trailer.
    pub init_tail: &'static [u8],
}

impl SvdToneSpec {
    /// Sum of all sections' SVD byte sizes (data portion only,
    /// excluding end marker and tail padding).
    pub fn data_bytes(&self) -> usize {
        self.sections.iter().map(|s| s.svd_bytes).sum()
    }
}

// ---------------------------------------------------------------------------
// SN Synth Tone — Common (228 bits)
// ---------------------------------------------------------------------------

/// SN Synth Tone Common parameters.
///
/// Source: `docs/midi/08-supernatural-synth-tone.md` — offsets `00 00`–`00 3F`.
static SNS_COMMON_PARAMS: &[ParamBits] = &[
    // 0x00–0x0B: Tone Name (12 × 7 bits)
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    // 0x0C: Tone Level
    ParamBits::normal(7),
    // 0x0D–0x0F: reserve (nibblized, 3 bytes)
    ParamBits::nibblized(3),
    // 0x10: reserve
    ParamBits::normal(1),
    // 0x11: reserve
    ParamBits::normal(1),
    // 0x12: Portamento Switch
    ParamBits::normal(1),
    // 0x13: Portamento Time
    ParamBits::normal(7),
    // 0x14: Mono Switch
    ParamBits::normal(2),
    // 0x15: Octave Shift (range 61-67, centered at 64)
    ParamBits::signed(3),
    // 0x16: Pitch Bend Range Up
    ParamBits::normal(5),
    // 0x17: Pitch Bend Range Down
    ParamBits::normal(5),
    // 0x18: reserve
    ParamBits::normal(3),
    // 0x19: Partial1 Switch
    ParamBits::normal(1),
    // 0x1A: Partial1 Select
    ParamBits::normal(1),
    // 0x1B: Partial2 Switch
    ParamBits::normal(1),
    // 0x1C: Partial2 Select
    ParamBits::normal(1),
    // 0x1D: Partial3 Switch
    ParamBits::normal(1),
    // 0x1E: Partial3 Select
    ParamBits::normal(1),
    // 0x1F: RING Switch
    ParamBits::normal(2),
    // 0x20: TFX Switch
    ParamBits::normal(1),
    // 0x21: reserve
    ParamBits::normal(2),
    // 0x22: reserve
    ParamBits::normal(1),
    // 0x23: reserve
    ParamBits::normal(1),
    // 0x24: reserve
    ParamBits::normal(6),
    // 0x25: reserve
    ParamBits::normal(1),
    // 0x26: reserve
    ParamBits::normal(1),
    // 0x27: reserve
    ParamBits::normal(1),
    // 0x28: reserve
    ParamBits::normal(1),
    // 0x29: reserve
    ParamBits::normal(1),
    // 0x2A: reserve
    ParamBits::normal(1),
    // 0x2B: reserve
    ParamBits::normal(1),
    // 0x2C: reserve
    ParamBits::normal(1),
    // 0x2D: reserve
    ParamBits::normal(1),
    // 0x2E: Unison Switch
    ParamBits::normal(1),
    // 0x2F: reserve
    ParamBits::normal(1),
    // 0x30: reserve
    ParamBits::normal(1),
    // 0x31: Portamento Mode
    ParamBits::normal(1),
    // 0x32: Legato Switch
    ParamBits::normal(1),
    // 0x33: reserve
    ParamBits::normal(1),
    // 0x34: Analog Feel
    ParamBits::normal(7),
    // 0x35: Wave Shape
    ParamBits::normal(7),
    // 0x36: Tone Category
    ParamBits::normal(7),
    // 0x37–0x3A: Phrase Number (nibblized, 4 bytes)
    ParamBits::nibblized(4),
    // 0x3B: Phrase Octave Shift (range 61-67, centered at 64)
    ParamBits::signed(3),
    // 0x3C: Unison Size
    ParamBits::normal(2),
    // 0x3D: reserve
    ParamBits::normal(7),
    // 0x3E: reserve
    ParamBits::normal(7),
    // 0x3F: reserve
    ParamBits::normal(7),
];

// ---------------------------------------------------------------------------
// SN Synth Tone — MFX (618 bits)
// ---------------------------------------------------------------------------

/// SN Synth Tone MFX parameters.
///
/// Source: `docs/midi/08-supernatural-synth-tone.md` — offsets `00 02 00`–`01 10`.
/// The MFX block is identical across all tone types.
pub static MFX_PARAMS: &[ParamBits] = &[
    // 0x00: MFX Type
    ParamBits::normal(7),
    // 0x01: reserve
    ParamBits::normal(7),
    // 0x02: MFX Chorus Send Level
    ParamBits::normal(7),
    // 0x03: MFX Reverb Send Level
    ParamBits::normal(7),
    // 0x04: reserve
    ParamBits::normal(2),
    // 0x05: MFX Control 1 Source
    ParamBits::normal(7),
    // 0x06: MFX Control 1 Sens
    ParamBits::normal(7),
    // 0x07: MFX Control 2 Source
    ParamBits::normal(7),
    // 0x08: MFX Control 2 Sens
    ParamBits::normal(7),
    // 0x09: MFX Control 3 Source
    ParamBits::normal(7),
    // 0x0A: MFX Control 3 Sens
    ParamBits::normal(7),
    // 0x0B: MFX Control 4 Source
    ParamBits::normal(7),
    // 0x0C: MFX Control 4 Sens
    ParamBits::normal(7),
    // 0x0D: MFX Control Assign 1
    ParamBits::normal(5),
    // 0x0E: MFX Control Assign 2
    ParamBits::normal(5),
    // 0x0F: MFX Control Assign 3
    ParamBits::normal(5),
    // 0x10: MFX Control Assign 4
    ParamBits::normal(5),
    // 0x11–0x14: MFX Parameter 1 (nibblized, 4 bytes)
    ParamBits::nibblized(4),
    // MFX Parameter 2
    ParamBits::nibblized(4),
    // MFX Parameter 3
    ParamBits::nibblized(4),
    // MFX Parameter 4
    ParamBits::nibblized(4),
    // MFX Parameter 5
    ParamBits::nibblized(4),
    // MFX Parameter 6
    ParamBits::nibblized(4),
    // MFX Parameter 7
    ParamBits::nibblized(4),
    // MFX Parameter 8
    ParamBits::nibblized(4),
    // MFX Parameter 9
    ParamBits::nibblized(4),
    // MFX Parameter 10
    ParamBits::nibblized(4),
    // MFX Parameter 11
    ParamBits::nibblized(4),
    // MFX Parameter 12
    ParamBits::nibblized(4),
    // MFX Parameter 13
    ParamBits::nibblized(4),
    // MFX Parameter 14
    ParamBits::nibblized(4),
    // MFX Parameter 15
    ParamBits::nibblized(4),
    // MFX Parameter 16
    ParamBits::nibblized(4),
    // MFX Parameter 17
    ParamBits::nibblized(4),
    // MFX Parameter 18
    ParamBits::nibblized(4),
    // MFX Parameter 19
    ParamBits::nibblized(4),
    // MFX Parameter 20
    ParamBits::nibblized(4),
    // MFX Parameter 21
    ParamBits::nibblized(4),
    // MFX Parameter 22
    ParamBits::nibblized(4),
    // MFX Parameter 23
    ParamBits::nibblized(4),
    // MFX Parameter 24
    ParamBits::nibblized(4),
    // MFX Parameter 25
    ParamBits::nibblized(4),
    // MFX Parameter 26
    ParamBits::nibblized(4),
    // MFX Parameter 27
    ParamBits::nibblized(4),
    // MFX Parameter 28
    ParamBits::nibblized(4),
    // MFX Parameter 29
    ParamBits::nibblized(4),
    // MFX Parameter 30
    ParamBits::nibblized(4),
    // MFX Parameter 31
    ParamBits::nibblized(4),
    // MFX Parameter 32
    ParamBits::nibblized(4),
];

// ---------------------------------------------------------------------------
// SN Synth Tone — Partial (350 bits)
// ---------------------------------------------------------------------------

/// SN Synth Tone Partial parameters.
///
/// Source: `docs/midi/08-supernatural-synth-tone.md` — offsets `00 20 00`–`00 20 3C`
/// (identical for Partial 1, 2, 3).
static SNS_PARTIAL_PARAMS: &[ParamBits] = &[
    // 0x00: OSC Wave
    ParamBits::normal(3),
    // 0x01: OSC Wave Variation
    ParamBits::normal(6),
    // 0x02: reserve
    ParamBits::normal(2),
    // 0x03: OSC Pitch (range 40-88, centered at 64)
    ParamBits::signed(6),
    // 0x04: OSC Detune
    ParamBits::normal(7),
    // 0x05: OSC Pulse Width Mod Depth
    ParamBits::normal(7),
    // 0x06: OSC Pulse Width
    ParamBits::normal(7),
    // 0x07: OSC Pitch Env Attack Time
    ParamBits::normal(7),
    // 0x08: OSC Pitch Env Decay
    ParamBits::normal(7),
    // 0x09: OSC Pitch Env Depth
    ParamBits::normal(7),
    // 0x0A: FILTER Mode
    ParamBits::normal(3),
    // 0x0B: FILTER Slope
    ParamBits::normal(1),
    // 0x0C: FILTER Cutoff
    ParamBits::normal(7),
    // 0x0D: FILTER Cutoff Keyfollow (range 54-74, centered at 64)
    ParamBits::signed(6),
    // 0x0E: FILTER Env Velocity Sens
    ParamBits::normal(7),
    // 0x0F: FILTER Resonance
    ParamBits::normal(7),
    // 0x10: FILTER Env Attack Time
    ParamBits::normal(7),
    // 0x11: FILTER Env Decay Time
    ParamBits::normal(7),
    // 0x12: FILTER Env Sustain Level
    ParamBits::normal(7),
    // 0x13: FILTER Env Release Time
    ParamBits::normal(7),
    // 0x14: FILTER Env Depth
    ParamBits::normal(7),
    // 0x15: AMP Level
    ParamBits::normal(7),
    // 0x16: AMP Level Velocity Sens
    ParamBits::normal(7),
    // 0x17: AMP Env Attack Time
    ParamBits::normal(7),
    // 0x18: AMP Env Decay Time
    ParamBits::normal(7),
    // 0x19: AMP Env Sustain Level
    ParamBits::normal(7),
    // 0x1A: AMP Env Release Time
    ParamBits::normal(7),
    // 0x1B: AMP Pan
    ParamBits::normal(7),
    // 0x1C: LFO Shape
    ParamBits::normal(3),
    // 0x1D: LFO Rate
    ParamBits::normal(7),
    // 0x1E: LFO Tempo Sync Switch
    ParamBits::normal(1),
    // 0x1F: LFO Tempo Sync Note
    ParamBits::normal(5),
    // 0x20: LFO Fade Time
    ParamBits::normal(7),
    // 0x21: LFO Key Trigger
    ParamBits::normal(1),
    // 0x22: LFO Pitch Depth
    ParamBits::normal(7),
    // 0x23: LFO Filter Depth
    ParamBits::normal(7),
    // 0x24: LFO Amp Depth
    ParamBits::normal(7),
    // 0x25: LFO Pan Depth
    ParamBits::normal(7),
    // 0x26: Modulation LFO Shape
    ParamBits::normal(3),
    // 0x27: Modulation LFO Rate
    ParamBits::normal(7),
    // 0x28: Modulation LFO Tempo Sync Switch
    ParamBits::normal(1),
    // 0x29: Modulation LFO Tempo Sync Note
    ParamBits::normal(5),
    // 0x2A: OSC Pulse Width Shift
    ParamBits::normal(7),
    // 0x2B: reserve
    ParamBits::normal(1),
    // 0x2C: Mod LFO Pitch Depth
    ParamBits::normal(7),
    // 0x2D: Mod LFO Filter Depth
    ParamBits::normal(7),
    // 0x2E: Mod LFO Amp Depth
    ParamBits::normal(7),
    // 0x2F: Mod LFO Pan Depth
    ParamBits::normal(7),
    // 0x30: Cutoff Aftertouch Sens
    ParamBits::normal(7),
    // 0x31: Level Aftertouch Sens
    ParamBits::normal(7),
    // 0x32: reserve
    ParamBits::normal(7),
    // 0x33: reserve
    ParamBits::normal(7),
    // 0x34: Wave Gain
    ParamBits::normal(2),
    // 0x35–0x38: Wave Number (nibblized, 4 bytes)
    ParamBits::nibblized(4),
    // 0x39: HPF Cutoff
    ParamBits::normal(7),
    // 0x3A: Super Saw Detune
    ParamBits::normal(7),
    // 0x3B: Mod LFO Rate Control
    ParamBits::normal(7),
    // 0x3C: AMP Level Keyfollow (range 54-74, centered at 64)
    ParamBits::signed(5),
];

// ---------------------------------------------------------------------------
// SN Synth Tone — Assembled Sections
// ---------------------------------------------------------------------------

/// SN Synth Common section (for bit-count queries only; not used in SVD directly).
pub static SNS_COMMON_SECTION: SvdSection = SvdSection {
    params: SNS_COMMON_PARAMS,
    svd_bytes: 0, // Common is always packed with MFX
};

/// SN Synth MFX section (for bit-count queries only).
pub static MFX_SECTION: SvdSection = SvdSection {
    params: MFX_PARAMS,
    svd_bytes: 0, // MFX is always packed with Common
};

/// SN Synth Partial section (used for each of the 3 partials).
pub static SNS_PARTIAL_SECTION: SvdSection = SvdSection {
    params: SNS_PARTIAL_PARAMS,
    svd_bytes: 46,
};

/// SN Synth section layout in SVD packing order.
///
/// Validated against device: Common and MFX are SEPARATE sections, each
/// independently byte-aligned. Common = 30 bytes, MFX = 78 bytes.
static SNS_SECTIONS: [SvdSection; 5] = [
    SvdSection {
        params: SNS_COMMON_PARAMS,
        svd_bytes: 30,
    },
    SvdSection {
        params: MFX_PARAMS,
        svd_bytes: 78,
    },
    SvdSection {
        params: SNS_PARTIAL_PARAMS,
        svd_bytes: 46,
    },
    SvdSection {
        params: SNS_PARTIAL_PARAMS,
        svd_bytes: 46,
    },
    SvdSection {
        params: SNS_PARTIAL_PARAMS,
        svd_bytes: 46,
    },
];

/// Complete SN Synth Tone SVD specification.
pub static SNS_TONE_SPEC: SvdToneSpec = SvdToneSpec {
    kind: ChunkType::SnSynthTone,
    sections: &SNS_SECTIONS,
    entry_size: 280,
    // Bytes 246–251 of every INIT TONE (`docs/svd/03-sn-synth.md`).
    init_tail: &[0x0E, 0x00, 0x00, 0x00, 0x02, 0xC0],
};

// ---------------------------------------------------------------------------
// SN Acoustic Tone — Common (464 bits)
// ---------------------------------------------------------------------------

/// SN Acoustic Tone Common parameters.
///
/// Source: `docs/midi/09-supernatural-acoustic-tone.md` — offsets `00 00`–`00 45`.
static SNA_COMMON_PARAMS: &[ParamBits] = &[
    // 0x00–0x0F: Tone Name (12 chars) + 4 reserve chars (16 × 7 bits)
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    // 0x10: Tone Level
    ParamBits::normal(7),
    // 0x11: Mono/Poly
    ParamBits::normal(1),
    // 0x12–0x19: Portamento Time, Cutoff, Resonance, Attack, Release, Vibrato ×3
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    // 0x1A: Octave Shift (range 61-67, centered at 64)
    ParamBits::signed(3),
    // 0x1B: Category
    ParamBits::normal(7),
    // 0x1C–0x1D: Phrase Number (nibblized, 2 bytes)
    ParamBits::nibblized(2),
    // 0x1E: Phrase Octave Shift (range 61-67, centered at 64)
    ParamBits::signed(3),
    // 0x1F: TFX Switch
    ParamBits::normal(1),
    // 0x20–0x41: Inst Variation, Inst Number, Modify 1–32 (34 × 7 bits)
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    // 0x42–0x45: reserves (4 × 7 bits)
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
    ParamBits::normal(7),
];

// ---------------------------------------------------------------------------
// SN Acoustic Tone — Assembled Spec
// ---------------------------------------------------------------------------

/// SN Acoustic Common section (for bit-count queries).
pub static SNA_COMMON_SECTION: SvdSection = SvdSection {
    params: SNA_COMMON_PARAMS,
    svd_bytes: 0,
};

/// SN Acoustic section layout: separate Common and MFX sections, no partials.
///
/// Common (464 bits) takes 60 bytes and MFX (618 bits) 78, filling the
/// 138-byte entry exactly. Reference: `docs/svd/08-sn-acoustic.md`.
static SNA_SECTIONS: [SvdSection; 2] = [
    SvdSection {
        params: SNA_COMMON_PARAMS,
        svd_bytes: 60, // ceil(464/8) = 58, +2 padding
    },
    SvdSection {
        params: MFX_PARAMS,
        svd_bytes: 78, // same as SN-S MFX
    },
];

/// Complete SN Acoustic Tone SVD specification.
pub static SNA_TONE_SPEC: SvdToneSpec = SvdToneSpec {
    kind: ChunkType::SnAcousticTone,
    sections: &SNA_SECTIONS,
    entry_size: 138,
    init_tail: &[],
};

// ---------------------------------------------------------------------------
// Studio Set (PRFb) — 1068 bytes, no end marker
// ---------------------------------------------------------------------------

/// Studio Set Common parameters.
///
/// Source: `docs/midi/05-studio-set.md` — offsets `00 00`–`00 53`.
/// Total SysEx size: 0x54 = 84 bytes.
#[rustfmt::skip]
static SS_COMMON_PARAMS: &[ParamBits] = &[
    // 0x00–0x0F: Name (16 × 7-bit ASCII)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x10–0x17: reserves at their MIDI Implementation widths (7, 6, 6, 6,
    // 1, 1, 1, 1). Always `FE 82 08 78` (127, 16, 16, 16, 1, 1, 1, 1).
    ParamBits::normal(7), ParamBits::normal(6), ParamBits::normal(6), ParamBits::normal(6),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    // 0x18–0x27: Voice Reserve 1-16 (16 × 7-bit)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x28–0x37: reserves (16 × 7-bit)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x38: reserve
    ParamBits::normal(4),
    // 0x39–0x3C: Tone Control Sources (4 × 7-bit)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x3D–0x3E: Tempo (nibblized, 2 bytes)
    ParamBits::nibblized(2),
    // 0x3F: Solo Part
    ParamBits::normal(5),
    // 0x40–0x43: Switches (Reverb, Chorus, Master EQ, Drum Comp)
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    // 0x44: Drum Comp/EQ Part
    ParamBits::normal(4),
    // 0x45–0x4A: Drum Comp/EQ Output Assigns 1-6
    ParamBits::normal(4), ParamBits::normal(4), ParamBits::normal(4),
    ParamBits::normal(4), ParamBits::normal(4), ParamBits::normal(4),
    // 0x4B: reserve
    ParamBits::normal(1),
    // 0x4C–0x4E: Ext Part Level, Chorus Send, Reverb Send
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x4F: Ext Part Mute
    ParamBits::normal(1),
    // 0x50–0x53: reserves. The MIDI Implementation gives 4 × 7 bits, but the
    // section only has 23 bits left; the split is assumed.
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(2),
];

/// Studio Set Chorus parameters.
///
/// Source: `docs/midi/05-studio-set.md` — section 2. The SVD header is
/// `[7 zero bits][Type][Level][Output][3 zero bits]`; reserve `0x02` takes
/// no bits.
#[rustfmt::skip]
static SS_CHORUS_PARAMS: &[ParamBits] = &[
    ParamBits::pad(7),
    ParamBits::normal(4),  // 0x00: Chorus Type
    ParamBits::normal(7),  // 0x01: Chorus Level
    ParamBits::normal(0),  // 0x02: reserve (not stored)
    ParamBits::normal(2),  // 0x03: Chorus Output Select
    ParamBits::pad(3),
    // 0x04–0x53: 20 nibblized Chorus Parameters (20 × 4 nibbles)
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
];

/// Studio Set Reverb parameters.
///
/// Source: `docs/midi/05-studio-set.md` — section 3.
#[rustfmt::skip]
static SS_REVERB_PARAMS: &[ParamBits] = &[
    ParamBits::normal(4),  // 0x00: Reverb Type
    ParamBits::normal(7),  // 0x01: Reverb Level
    ParamBits::normal(2),  // 0x02: Reverb Output Assign
    // 0x03–0x62: 24 nibblized Reverb Parameters (24 × 4 nibbles)
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
    ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4), ParamBits::nibblized(4),
];

/// Studio Set Motional Surround parameters.
///
/// Source: `docs/midi/05-studio-set.md` — section 4.
#[rustfmt::skip]
static SS_MOTIONAL_SURROUND_PARAMS: &[ParamBits] = &[
    ParamBits::normal(1),  // 0x00: Switch
    ParamBits::normal(2),  // 0x01: Room Type
    ParamBits::normal(7),  // 0x02: Ambience Level
    ParamBits::normal(7),  // 0x03: Room Size
    ParamBits::normal(7),  // 0x04: Ambience Time
    ParamBits::normal(7),  // 0x05: Ambience Density
    ParamBits::normal(7),  // 0x06: Ambience HF Damp
    ParamBits::normal(7),  // 0x07: Ext Part L-R
    ParamBits::normal(7),  // 0x08: Ext Part F-B
    ParamBits::normal(6),  // 0x09: Ext Part Width
    ParamBits::normal(7),  // 0x0A: Ext Part Ambience Send Level
    ParamBits::normal(5),  // 0x0B: Ext Part Control Channel
    ParamBits::normal(7),  // 0x0C: Depth
    // 0x0D–0x0F: reserves
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
];

/// Studio Set Master EQ parameters.
///
/// Source: `docs/midi/05-studio-set.md` — section 5.
static SS_MASTER_EQ_PARAMS: &[ParamBits] = &[
    ParamBits::normal(1), // 0x00: Low Freq
    ParamBits::normal(5), // 0x01: Low Gain
    ParamBits::normal(5), // 0x02: Mid Freq
    ParamBits::normal(5), // 0x03: Mid Gain
    ParamBits::normal(3), // 0x04: Mid Q
    ParamBits::normal(2), // 0x05: High Freq
    ParamBits::normal(5), // 0x06: High Gain
];

/// Studio Set MIDI Channel (Phase Lock).
///
/// Source: `docs/midi/05-studio-set.md` — section 6.
/// One per channel, 16 channels.
static SS_MIDI_CHANNEL_PARAMS: &[ParamBits] = &[
    ParamBits::normal(1), // 0x00: Phase Lock
];

/// Studio Set Part parameters (per part).
///
/// Source: `docs/midi/05-studio-set.md` — section 7.
/// Total SysEx size: 0x4D = 77 bytes per part.
#[rustfmt::skip]
static SS_PART_PARAMS: &[ParamBits] = &[
    ParamBits::normal(4),  // 0x00: Receive Channel
    ParamBits::normal(1),  // 0x01: Receive Switch
    // 0x02–0x05: reserves (1 bit each, always 1)
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    // 0x06–0x08: Bank MSB, LSB, PC (0-based)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x09–0x0A: Level, Pan
    ParamBits::normal(7), ParamBits::normal(7),
    // 0x0B–0x0C: Coarse Tune, Fine Tune
    ParamBits::normal(7), ParamBits::normal(7),
    // 0x0D: Mono/Poly
    ParamBits::normal(2),
    // 0x0E: Legato Switch
    ParamBits::normal(2),
    // 0x0F: Pitch Bend Range
    ParamBits::normal(5),
    // 0x10: Portamento Switch
    ParamBits::normal(2),
    // 0x11–0x12: Portamento Time (nibblized, 2 bytes)
    ParamBits::nibblized(2),
    // 0x13–0x1A: Cutoff/Res/Attack/Decay/Release/VibratoRate/Depth/Delay offsets
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x1B: Octave Shift (range 61-67, signed)
    ParamBits::signed(3),
    // 0x1C–0x24: Velocity Sens, Key Range Lower/Upper, Fade Lower/Upper, Vel Range Lower/Upper, Vel Fade Lower/Upper
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7),
    // 0x25: Mute Switch
    ParamBits::normal(1),
    // 0x26: reserve
    ParamBits::normal(7),
    // 0x27–0x28: Chorus Send, Reverb Send
    ParamBits::normal(7), ParamBits::normal(7),
    // 0x29: Output Assign
    ParamBits::normal(4),
    // 0x2A: reserve (Output Assign + this take 6 bits; split assumed)
    ParamBits::normal(2),
    // 0x2B–0x2C: Scale Tune Type, Key
    ParamBits::normal(7), ParamBits::normal(7),
    // 0x2D–0x38: Scale Tune for C through B (12 notes)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    // 0x39–0x42: Receive switches (10 × 1-bit)
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1),
    // 0x43: Velocity Curve Type
    ParamBits::normal(3),
    // 0x44: Motional Surround L-R
    ParamBits::normal(7),
    // 0x45: reserve
    ParamBits::normal(7),
    // 0x46: Motional Surround F-B
    ParamBits::normal(7),
    // 0x47: reserve
    ParamBits::normal(7),
    // 0x48: Motional Surround Width
    ParamBits::normal(6),
    // 0x49: Motional Surround Ambience Send Level
    ParamBits::normal(7),
    // 0x4A–0x4C: reserves
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
];

/// Studio Set Part EQ parameters (per part).
///
/// Source: `docs/midi/05-studio-set.md` — section 8.
static SS_PART_EQ_PARAMS: &[ParamBits] = &[
    ParamBits::normal(1), // 0x00: EQ Switch
    ParamBits::normal(1), // 0x01: Low Freq
    ParamBits::normal(5), // 0x02: Low Gain
    ParamBits::normal(5), // 0x03: Mid Freq
    ParamBits::normal(5), // 0x04: Mid Gain
    ParamBits::normal(3), // 0x05: Mid Q
    ParamBits::normal(2), // 0x06: High Freq
    ParamBits::normal(5), // 0x07: High Gain
];

// ---------------------------------------------------------------------------
// Studio Set — Assembled Spec
// ---------------------------------------------------------------------------

/// Studio Set section layout.
///
/// Measured on pack data; reference `docs/svd/05-studio-set.md`.
static SS_SECTIONS: [SvdSection; 53] = {
    let common = SvdSection {
        params: SS_COMMON_PARAMS,
        svd_bytes: 61,
    };
    let chorus = SvdSection {
        params: SS_CHORUS_PARAMS,
        svd_bytes: 43,
    };
    let reverb = SvdSection {
        params: SS_REVERB_PARAMS,
        svd_bytes: 50,
    };
    let ms = SvdSection {
        params: SS_MOTIONAL_SURROUND_PARAMS,
        svd_bytes: 14,
    };
    let meq = SvdSection {
        params: SS_MASTER_EQ_PARAMS,
        svd_bytes: 4,
    };
    let midi = SvdSection {
        params: SS_MIDI_CHANNEL_PARAMS,
        svd_bytes: 1,
    };
    let part = SvdSection {
        params: SS_PART_PARAMS,
        svd_bytes: 51,
    };
    let peq = SvdSection {
        params: SS_PART_EQ_PARAMS,
        svd_bytes: 4,
    };
    [
        common, chorus, reverb, ms, meq, // 16 MIDI channels
        midi, midi, midi, midi, midi, midi, midi, midi, midi, midi, midi, midi, midi, midi, midi,
        midi, // 16 Parts
        part, part, part, part, part, part, part, part, part, part, part, part, part, part, part,
        part, // 16 Part EQs
        peq, peq, peq, peq, peq, peq, peq, peq, peq, peq, peq, peq, peq, peq, peq, peq,
    ]
};

/// Complete Studio Set SVD specification.
pub static SS_TONE_SPEC: SvdToneSpec = SvdToneSpec {
    kind: ChunkType::StudioSet,
    sections: &SS_SECTIONS,
    entry_size: 1068,
    init_tail: &[],
};

// ---------------------------------------------------------------------------
// Drum Kit Comp/EQ (shared by SN Drum and PCM Drum)
// ---------------------------------------------------------------------------

/// Drum Kit Comp/EQ parameters: 6 × (Comp n + EQ n), 330 bits.
///
/// Source: `docs/svd/07-pcm-drum.md` — Common Comp/EQ. SN Drum Kit uses the
/// same layout (`docs/svd/09-sn-drum.md`).
#[rustfmt::skip]
static COMP_EQ_PARAMS: &[ParamBits] = &[
    ParamBits::normal(1), // 00 00: Comp1 Switch (0–1)
    ParamBits::normal(5), // 00 01: Comp1 Attack Time (0–31)
    ParamBits::normal(5), // 00 02: Comp1 Release Time (0–23)
    ParamBits::normal(7), // 00 03: Comp1 Threshold (0–127)
    ParamBits::normal(5), // 00 04: Comp1 Ratio (0–19)
    ParamBits::normal(5), // 00 05: Comp1 Output Gain (0–24)
    ParamBits::normal(1), // 00 06: EQ1 Switch (0–1)
    ParamBits::normal(1), // 00 07: EQ1 Low Freq (0–1)
    ParamBits::normal(5), // 00 08: EQ1 Low Gain (0–30)
    ParamBits::normal(5), // 00 09: EQ1 Mid Freq (0–16)
    ParamBits::normal(5), // 00 0A: EQ1 Mid Gain (0–30)
    ParamBits::normal(3), // 00 0B: EQ1 Mid Q (0–4)
    ParamBits::normal(2), // 00 0C: EQ1 High Freq (0–2)
    ParamBits::normal(5), // 00 0D: EQ1 High Gain (0–30)
    ParamBits::normal(1), // 00 0E: Comp2 Switch (0–1)
    ParamBits::normal(5), // 00 0F: Comp2 Attack Time (0–31)
    ParamBits::normal(5), // 00 10: Comp2 Release Time (0–23)
    ParamBits::normal(7), // 00 11: Comp2 Threshold (0–127)
    ParamBits::normal(5), // 00 12: Comp2 Ratio (0–19)
    ParamBits::normal(5), // 00 13: Comp2 Output Gain (0–24)
    ParamBits::normal(1), // 00 14: EQ2 Switch (0–1)
    ParamBits::normal(1), // 00 15: EQ2 Low Freq (0–1)
    ParamBits::normal(5), // 00 16: EQ2 Low Gain (0–30)
    ParamBits::normal(5), // 00 17: EQ2 Mid Freq (0–16)
    ParamBits::normal(5), // 00 18: EQ2 Mid Gain (0–30)
    ParamBits::normal(3), // 00 19: EQ2 Mid Q (0–4)
    ParamBits::normal(2), // 00 1A: EQ2 High Freq (0–2)
    ParamBits::normal(5), // 00 1B: EQ2 High Gain (0–30)
    ParamBits::normal(1), // 00 1C: Comp3 Switch (0–1)
    ParamBits::normal(5), // 00 1D: Comp3 Attack Time (0–31)
    ParamBits::normal(5), // 00 1E: Comp3 Release Time (0–23)
    ParamBits::normal(7), // 00 1F: Comp3 Threshold (0–127)
    ParamBits::normal(5), // 00 20: Comp3 Ratio (0–19)
    ParamBits::normal(5), // 00 21: Comp3 Output Gain (0–24)
    ParamBits::normal(1), // 00 22: EQ3 Switch (0–1)
    ParamBits::normal(1), // 00 23: EQ3 Low Freq (0–1)
    ParamBits::normal(5), // 00 24: EQ3 Low Gain (0–30)
    ParamBits::normal(5), // 00 25: EQ3 Mid Freq (0–16)
    ParamBits::normal(5), // 00 26: EQ3 Mid Gain (0–30)
    ParamBits::normal(3), // 00 27: EQ3 Mid Q (0–4)
    ParamBits::normal(2), // 00 28: EQ3 High Freq (0–2)
    ParamBits::normal(5), // 00 29: EQ3 High Gain (0–30)
    ParamBits::normal(1), // 00 2A: Comp4 Switch (0–1)
    ParamBits::normal(5), // 00 2B: Comp4 Attack Time (0–31)
    ParamBits::normal(5), // 00 2C: Comp4 Release Time (0–23)
    ParamBits::normal(7), // 00 2D: Comp4 Threshold (0–127)
    ParamBits::normal(5), // 00 2E: Comp4 Ratio (0–19)
    ParamBits::normal(5), // 00 2F: Comp4 Output Gain (0–24)
    ParamBits::normal(1), // 00 30: EQ4 Switch (0–1)
    ParamBits::normal(1), // 00 31: EQ4 Low Freq (0–1)
    ParamBits::normal(5), // 00 32: EQ4 Low Gain (0–30)
    ParamBits::normal(5), // 00 33: EQ4 Mid Freq (0–16)
    ParamBits::normal(5), // 00 34: EQ4 Mid Gain (0–30)
    ParamBits::normal(3), // 00 35: EQ4 Mid Q (0–4)
    ParamBits::normal(2), // 00 36: EQ4 High Freq (0–2)
    ParamBits::normal(5), // 00 37: EQ4 High Gain (0–30)
    ParamBits::normal(1), // 00 38: Comp5 Switch (0–1)
    ParamBits::normal(5), // 00 39: Comp5 Attack Time (0–31)
    ParamBits::normal(5), // 00 3A: Comp5 Release Time (0–23)
    ParamBits::normal(7), // 00 3B: Comp5 Threshold (0–127)
    ParamBits::normal(5), // 00 3C: Comp5 Ratio (0–19)
    ParamBits::normal(5), // 00 3D: Comp5 Output Gain (0–24)
    ParamBits::normal(1), // 00 3E: EQ5 Switch (0–1)
    ParamBits::normal(1), // 00 3F: EQ5 Low Freq (0–1)
    ParamBits::normal(5), // 00 40: EQ5 Low Gain (0–30)
    ParamBits::normal(5), // 00 41: EQ5 Mid Freq (0–16)
    ParamBits::normal(5), // 00 42: EQ5 Mid Gain (0–30)
    ParamBits::normal(3), // 00 43: EQ5 Mid Q (0–4)
    ParamBits::normal(2), // 00 44: EQ5 High Freq (0–2)
    ParamBits::normal(5), // 00 45: EQ5 High Gain (0–30)
    ParamBits::normal(1), // 00 46: Comp6 Switch (0–1)
    ParamBits::normal(5), // 00 47: Comp6 Attack Time (0–31)
    ParamBits::normal(5), // 00 48: Comp6 Release Time (0–23)
    ParamBits::normal(7), // 00 49: Comp6 Threshold (0–127)
    ParamBits::normal(5), // 00 4A: Comp6 Ratio (0–19)
    ParamBits::normal(5), // 00 4B: Comp6 Output Gain (0–24)
    ParamBits::normal(1), // 00 4C: EQ6 Switch (0–1)
    ParamBits::normal(1), // 00 4D: EQ6 Low Freq (0–1)
    ParamBits::normal(5), // 00 4E: EQ6 Low Gain (0–30)
    ParamBits::normal(5), // 00 4F: EQ6 Mid Freq (0–16)
    ParamBits::normal(5), // 00 50: EQ6 Mid Gain (0–30)
    ParamBits::normal(3), // 00 51: EQ6 Mid Q (0–4)
    ParamBits::normal(2), // 00 52: EQ6 High Freq (0–2)
    ParamBits::normal(5), // 00 53: EQ6 High Gain (0–30)
];

// ---------------------------------------------------------------------------
// SN Drum Kit (SDKa) — 1006 bytes, no end marker
// ---------------------------------------------------------------------------

/// SN Drum Kit Common parameters (134 bits).
///
/// Source: `docs/svd/09-sn-drum.md` — Common.
#[rustfmt::skip]
static SND_COMMON_PARAMS: &[ParamBits] = &[
    ParamBits::normal(7), // 00 00: Kit Name 1 (32–127)
    ParamBits::normal(7), // 00 01: Kit Name 2 (32–127)
    ParamBits::normal(7), // 00 02: Kit Name 3 (32–127)
    ParamBits::normal(7), // 00 03: Kit Name 4 (32–127)
    ParamBits::normal(7), // 00 04: Kit Name 5 (32–127)
    ParamBits::normal(7), // 00 05: Kit Name 6 (32–127)
    ParamBits::normal(7), // 00 06: Kit Name 7 (32–127)
    ParamBits::normal(7), // 00 07: Kit Name 8 (32–127)
    ParamBits::normal(7), // 00 08: Kit Name 9 (32–127)
    ParamBits::normal(7), // 00 09: Kit Name 10 (32–127)
    ParamBits::normal(7), // 00 0A: Kit Name 11 (32–127)
    ParamBits::normal(7), // 00 0B: Kit Name 12 (32–127)
    ParamBits::normal(7), // 00 0C: (reserve name 13) (32–127)
    ParamBits::normal(7), // 00 0D: (reserve name 14) (32–127)
    ParamBits::normal(7), // 00 0E: (reserve name 15) (32–127)
    ParamBits::normal(7), // 00 0F: (reserve name 16) (32–127)
    ParamBits::normal(7), // 00 10: Kit Level (0–127)
    ParamBits::normal(7), // 00 11: Ambience Level (0–127)
    ParamBits::normal(7), // 00 12: Phrase Number (0–127)
    ParamBits::normal(1), // 00 13: TFX Switch (0–1)
];

/// SN Drum Kit Note parameters (102 bits), one section per key 27–88.
///
/// Source: `docs/svd/09-sn-drum.md` — Note.
#[rustfmt::skip]
static SND_NOTE_PARAMS: &[ParamBits] = &[
    ParamBits::nibblized(4), // 00 00: Inst Number (0–512)
    ParamBits::normal(7), // 00 04: Level (0–127)
    ParamBits::normal(7), // 00 05: Pan (0–127)
    ParamBits::normal(7), // 00 06: Chorus Send Level (0–127)
    ParamBits::normal(7), // 00 07: Reverb Send Level (0–127)
    ParamBits::nibblized(4), // 00 08: Tune (8–248)
    ParamBits::normal(7), // 00 0C: Attack (0–100)
    ParamBits::normal(7), // 00 0D: Decay (1–64)
    ParamBits::signed(5), // 00 0E: Brilliance (49–76)
    ParamBits::normal(7), // 00 0F: Variation (0–7)
    ParamBits::normal(6), // 00 10: Dynamic Range (0–63)
    ParamBits::normal(7), // 00 11: Stereo Width (0–127)
    ParamBits::normal(3), // 00 12: Output Assign (0–6)
];

/// SN Drum Kit section layout: Common 18 B, MFX 78 B, Comp/EQ 42 B, then
/// 62 Notes of 14 B, filling the 1006-byte entry exactly.
static SND_SECTIONS: [SvdSection; 65] = {
    let note = SvdSection {
        params: SND_NOTE_PARAMS,
        svd_bytes: 14,
    };
    let mut sections = [note; 65];
    sections[0] = SvdSection {
        params: SND_COMMON_PARAMS,
        svd_bytes: 18,
    };
    sections[1] = SvdSection {
        params: MFX_PARAMS,
        svd_bytes: 78,
    };
    sections[2] = SvdSection {
        params: COMP_EQ_PARAMS,
        svd_bytes: 42,
    };
    sections
};

/// Complete SN Drum Kit SVD specification.
pub static SND_TONE_SPEC: SvdToneSpec = SvdToneSpec {
    kind: ChunkType::SnDrumKit,
    sections: &SND_SECTIONS,
    entry_size: 1006,
    init_tail: &[],
};

// ---------------------------------------------------------------------------
// PCM Synth Tone (RFPa) — 590 bytes, no end marker
// ---------------------------------------------------------------------------

/// PCM Synth Tone Common parameters (446 bits), reserves included.
///
/// Source: `docs/svd/06-pcm-synth.md` — Common.
#[rustfmt::skip]
static PCMS_COMMON_PARAMS: &[ParamBits] = &[
    ParamBits::normal(7), // 00 00: PCM Synth Tone Name 1 (32–127)
    ParamBits::normal(7), // 00 01: PCM Synth Tone Name 2 (32–127)
    ParamBits::normal(7), // 00 02: PCM Synth Tone Name 3 (32–127)
    ParamBits::normal(7), // 00 03: PCM Synth Tone Name 4 (32–127)
    ParamBits::normal(7), // 00 04: PCM Synth Tone Name 5 (32–127)
    ParamBits::normal(7), // 00 05: PCM Synth Tone Name 6 (32–127)
    ParamBits::normal(7), // 00 06: PCM Synth Tone Name 7 (32–127)
    ParamBits::normal(7), // 00 07: PCM Synth Tone Name 8 (32–127)
    ParamBits::normal(7), // 00 08: PCM Synth Tone Name 9 (32–127)
    ParamBits::normal(7), // 00 09: PCM Synth Tone Name 10 (32–127)
    ParamBits::normal(7), // 00 0A: PCM Synth Tone Name 11 (32–127)
    ParamBits::normal(7), // 00 0B: PCM Synth Tone Name 12 (32–127)
    ParamBits::normal(7), // 00 0C: (reserve)
    ParamBits::normal(1), // 00 0D: (reserve)
    ParamBits::normal(7), // 00 0E: PCM Synth Tone Level (0–127)
    ParamBits::normal(7), // 00 0F: PCM Synth Tone Pan (0–127)
    ParamBits::normal(1), // 00 10: PCM Synth Tone Priority (0–1)
    ParamBits::normal(7), // 00 11: PCM Synth Tone Coarse Tune (16–112)
    ParamBits::normal(7), // 00 12: PCM Synth Tone Fine Tune (14–114)
    ParamBits::signed(3), // 00 13: Octave Shift (61–67)
    ParamBits::normal(2), // 00 14: Stretch Tune Depth (0–3)
    ParamBits::normal(7), // 00 15: Analog Feel (0–127)
    ParamBits::normal(1), // 00 16: Mono/Poly (0–1)
    ParamBits::normal(1), // 00 17: Legato Switch (0–1)
    ParamBits::normal(1), // 00 18: Legato Retrigger (0–1)
    ParamBits::normal(1), // 00 19: Portamento Switch (0–1)
    ParamBits::normal(1), // 00 1A: Portamento Mode (0–1)
    ParamBits::normal(1), // 00 1B: Portamento Type (0–1)
    ParamBits::normal(1), // 00 1C: Portamento Start (0–1)
    ParamBits::normal(7), // 00 1D: Portamento Time (0–127)
    ParamBits::normal(1), // 00 1E: (reserve)
    ParamBits::nibblized(2), // 00 1F: (reserve)
    ParamBits::normal(1), // 00 21: (reserve)
    ParamBits::normal(7), // 00 22: Cutoff Offset (1–127)
    ParamBits::normal(7), // 00 23: Resonance Offset (1–127)
    ParamBits::normal(7), // 00 24: Attack Time Offset (1–127)
    ParamBits::normal(7), // 00 25: Release Time Offset (1–127)
    ParamBits::normal(7), // 00 26: Velocity Sens Offset (1–127)
    ParamBits::normal(4), // 00 27: (reserve)
    ParamBits::normal(1), // 00 28: PMT Control Switch (0–1)
    ParamBits::normal(6), // 00 29: Pitch Bend Range Up (0–48)
    ParamBits::normal(6), // 00 2A: Pitch Bend Range Down (0–48)
    ParamBits::normal(7), // 00 2B: Matrix Control 1 Source (0–109)
    ParamBits::normal(6), // 00 2C: Matrix Control 1 Destination 1 (0–33)
    ParamBits::normal(7), // 00 2D: Matrix Control 1 Sens 1 (1–127)
    ParamBits::normal(6), // 00 2E: Matrix Control 1 Destination 2 (0–33)
    ParamBits::normal(7), // 00 2F: Matrix Control 1 Sens 2 (1–127)
    ParamBits::normal(6), // 00 30: Matrix Control 1 Destination 3 (0–33)
    ParamBits::normal(7), // 00 31: Matrix Control 1 Sens 3 (1–127)
    ParamBits::normal(6), // 00 32: Matrix Control 1 Destination 4 (0–33)
    ParamBits::normal(7), // 00 33: Matrix Control 1 Sens 4 (1–127)
    ParamBits::normal(7), // 00 34: Matrix Control 2 Source (0–109)
    ParamBits::normal(6), // 00 35: Matrix Control 2 Destination 1 (0–33)
    ParamBits::normal(7), // 00 36: Matrix Control 2 Sens 1 (1–127)
    ParamBits::normal(6), // 00 37: Matrix Control 2 Destination 2 (0–33)
    ParamBits::normal(7), // 00 38: Matrix Control 2 Sens 2 (1–127)
    ParamBits::normal(6), // 00 39: Matrix Control 2 Destination 3 (0–33)
    ParamBits::normal(7), // 00 3A: Matrix Control 2 Sens 3 (1–127)
    ParamBits::normal(6), // 00 3B: Matrix Control 2 Destination 4 (0–33)
    ParamBits::normal(7), // 00 3C: Matrix Control 2 Sens 4 (1–127)
    ParamBits::normal(7), // 00 3D: Matrix Control 3 Source (0–109)
    ParamBits::normal(6), // 00 3E: Matrix Control 3 Destination 1 (0–33)
    ParamBits::normal(7), // 00 3F: Matrix Control 3 Sens 1 (1–127)
    ParamBits::normal(6), // 00 40: Matrix Control 3 Destination 2 (0–33)
    ParamBits::normal(7), // 00 41: Matrix Control 3 Sens 2 (1–127)
    ParamBits::normal(6), // 00 42: Matrix Control 3 Destination 3 (0–33)
    ParamBits::normal(7), // 00 43: Matrix Control 3 Sens 3 (1–127)
    ParamBits::normal(6), // 00 44: Matrix Control 3 Destination 4 (0–33)
    ParamBits::normal(7), // 00 45: Matrix Control 3 Sens 4 (1–127)
    ParamBits::normal(7), // 00 46: Matrix Control 4 Source (0–109)
    ParamBits::normal(6), // 00 47: Matrix Control 4 Destination 1 (0–33)
    ParamBits::normal(7), // 00 48: Matrix Control 4 Sens 1 (1–127)
    ParamBits::normal(6), // 00 49: Matrix Control 4 Destination 2 (0–33)
    ParamBits::normal(7), // 00 4A: Matrix Control 4 Sens 2 (1–127)
    ParamBits::normal(6), // 00 4B: Matrix Control 4 Destination 3 (0–33)
    ParamBits::normal(7), // 00 4C: Matrix Control 4 Sens 3 (1–127)
    ParamBits::normal(6), // 00 4D: Matrix Control 4 Destination 4 (0–33)
    ParamBits::normal(7), // 00 4E: Matrix Control 4 Sens 4 (1–127)
    ParamBits::normal(1), // 00 4F: (reserve)
];

/// PCM Synth Tone Partial Mix Table parameters (242 bits).
///
/// Source: `docs/svd/06-pcm-synth.md` — PMT.
#[rustfmt::skip]
static PCMS_PMT_PARAMS: &[ParamBits] = &[
    ParamBits::normal(4), // 00 00: Structure Type 1 & 2 (0–9)
    ParamBits::normal(2), // 00 01: Booster 1 & 2 (0–3)
    ParamBits::normal(4), // 00 02: Structure Type 3 & 4 (0–9)
    ParamBits::normal(2), // 00 03: Booster 3 & 4 (0–3)
    ParamBits::normal(2), // 00 04: PMT Velocity Control (0–3)
    ParamBits::normal(1), // 00 05: PMT1 Partial Switch (0–1)
    ParamBits::normal(7), // 00 06: PMT1 Keyboard Range Lower (0–127)
    ParamBits::normal(7), // 00 07: PMT1 Keyboard Range Upper (0–127)
    ParamBits::normal(7), // 00 08: PMT1 Keyboard Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 09: PMT1 Keyboard Fade Width Upper (0–127)
    ParamBits::normal(7), // 00 0A: PMT1 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 00 0B: PMT1 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 00 0C: PMT1 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 0D: PMT1 Velocity Fade Width Upper (0–127)
    ParamBits::normal(1), // 00 0E: PMT2 Partial Switch (0–1)
    ParamBits::normal(7), // 00 0F: PMT2 Keyboard Range Lower (0–127)
    ParamBits::normal(7), // 00 10: PMT2 Keyboard Range Upper (0–127)
    ParamBits::normal(7), // 00 11: PMT2 Keyboard Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 12: PMT2 Keyboard Fade Width Upper (0–127)
    ParamBits::normal(7), // 00 13: PMT2 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 00 14: PMT2 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 00 15: PMT2 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 16: PMT2 Velocity Fade Width Upper (0–127)
    ParamBits::normal(1), // 00 17: PMT3 Partial Switch (0–1)
    ParamBits::normal(7), // 00 18: PMT3 Keyboard Range Lower (0–127)
    ParamBits::normal(7), // 00 19: PMT3 Keyboard Range Upper (0–127)
    ParamBits::normal(7), // 00 1A: PMT3 Keyboard Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 1B: PMT3 Keyboard Fade Width Upper (0–127)
    ParamBits::normal(7), // 00 1C: PMT3 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 00 1D: PMT3 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 00 1E: PMT3 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 1F: PMT3 Velocity Fade Width Upper (0–127)
    ParamBits::normal(1), // 00 20: PMT4 Partial Switch (0–1)
    ParamBits::normal(7), // 00 21: PMT4 Keyboard Range Lower (0–127)
    ParamBits::normal(7), // 00 22: PMT4 Keyboard Range Upper (0–127)
    ParamBits::normal(7), // 00 23: PMT4 Keyboard Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 24: PMT4 Keyboard Fade Width Upper (0–127)
    ParamBits::normal(7), // 00 25: PMT4 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 00 26: PMT4 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 00 27: PMT4 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 28: PMT4 Velocity Fade Width Upper (0–127)
];

/// PCM Synth Tone Partial parameters (790 bits), one section per partial.
///
/// Source: `docs/svd/06-pcm-synth.md` — Partial.
#[rustfmt::skip]
static PCMS_PARTIAL_PARAMS: &[ParamBits] = &[
    ParamBits::normal(7), // 00 00: Partial Level (0–127)
    ParamBits::normal(7), // 00 01: Partial Coarse Tune (16–112)
    ParamBits::normal(7), // 00 02: Partial Fine Tune (14–114)
    ParamBits::normal(5), // 00 03: Partial Random Pitch Depth (0–30)
    ParamBits::normal(7), // 00 04: Partial Pan (0–127)
    ParamBits::signed(5), // 00 05: Partial Pan Keyfollow (54–74)
    ParamBits::normal(6), // 00 06: Partial Random Pan Depth (0–63)
    ParamBits::normal(7), // 00 07: Partial Alternate Pan Depth (1–127)
    ParamBits::normal(1), // 00 08: Partial Env Mode (0–1)
    ParamBits::normal(2), // 00 09: Partial Delay Mode (0–3)
    ParamBits::nibblized(2), // 00 0A: Partial Delay Time (0–149)
    ParamBits::normal(7), // 00 0C: Partial Output Level (0–127)
    ParamBits::normal(7), // 00 0D: (reserve)
    ParamBits::normal(7), // 00 0E: (reserve)
    ParamBits::normal(7), // 00 0F: Partial Chorus Send Level (0–127)
    ParamBits::normal(7), // 00 10: Partial Reverb Send Level (0–127)
    ParamBits::normal(4), // 00 11: (reserve)
    ParamBits::normal(1), // 00 12: Partial Receive Bender (0–1)
    ParamBits::normal(1), // 00 13: Partial Receive Expression (0–1)
    ParamBits::normal(1), // 00 14: Partial Receive Hold-1 (0–1)
    ParamBits::normal(1), // 00 15: (reserve)
    ParamBits::normal(1), // 00 16: Partial Redamper Switch (0–1)
    ParamBits::normal(2), // 00 17: Partial Control 1 Switch 1 (0–2)
    ParamBits::normal(2), // 00 18: Partial Control 1 Switch 2 (0–2)
    ParamBits::normal(2), // 00 19: Partial Control 1 Switch 3 (0–2)
    ParamBits::normal(2), // 00 1A: Partial Control 1 Switch 4 (0–2)
    ParamBits::normal(2), // 00 1B: Partial Control 2 Switch 1 (0–2)
    ParamBits::normal(2), // 00 1C: Partial Control 2 Switch 2 (0–2)
    ParamBits::normal(2), // 00 1D: Partial Control 2 Switch 3 (0–2)
    ParamBits::normal(2), // 00 1E: Partial Control 2 Switch 4 (0–2)
    ParamBits::normal(2), // 00 1F: Partial Control 3 Switch 1 (0–2)
    ParamBits::normal(2), // 00 20: Partial Control 3 Switch 2 (0–2)
    ParamBits::normal(2), // 00 21: Partial Control 3 Switch 3 (0–2)
    ParamBits::normal(2), // 00 22: Partial Control 3 Switch 4 (0–2)
    ParamBits::normal(2), // 00 23: Partial Control 4 Switch 1 (0–2)
    ParamBits::normal(2), // 00 24: Partial Control 4 Switch 2 (0–2)
    ParamBits::normal(2), // 00 25: Partial Control 4 Switch 3 (0–2)
    ParamBits::normal(2), // 00 26: Partial Control 4 Switch 4 (0–2)
    ParamBits::normal(2), // 00 27: Wave Group Type (0–3)
    ParamBits::nibblized(4), // 00 28: Wave Group ID (0–16384)
    ParamBits::nibblized(4), // 00 2C: Wave Number L (Mono) (0–16384)
    ParamBits::nibblized(4), // 00 30: Wave Number R (0–16384)
    ParamBits::normal(2), // 00 34: Wave Gain (0–3)
    ParamBits::normal(1), // 00 35: Wave FXM Switch (0–1)
    ParamBits::normal(2), // 00 36: Wave FXM Color (0–3)
    ParamBits::normal(5), // 00 37: Wave FXM Depth (0–16)
    ParamBits::normal(1), // 00 38: Wave Tempo Sync (0–1)
    ParamBits::signed(6), // 00 39: Wave Pitch Keyfollow (44–84)
    ParamBits::signed(5), // 00 3A: Pitch Env Depth (52–76)
    ParamBits::normal(7), // 00 3B: Pitch Env Velocity Sens (1–127)
    ParamBits::normal(7), // 00 3C: Pitch Env Time 1 Velocity Sens (1–127)
    ParamBits::normal(7), // 00 3D: Pitch Env Time 4 Velocity Sens (1–127)
    ParamBits::signed(5), // 00 3E: Pitch Env Time Keyfollow (54–74)
    ParamBits::normal(7), // 00 3F: Pitch Env Time 1 (0–127)
    ParamBits::normal(7), // 00 40: Pitch Env Time 2 (0–127)
    ParamBits::normal(7), // 00 41: Pitch Env Time 3 (0–127)
    ParamBits::normal(7), // 00 42: Pitch Env Time 4 (0–127)
    ParamBits::normal(7), // 00 43: Pitch Env Level 0 (1–127)
    ParamBits::normal(7), // 00 44: Pitch Env Level 1 (1–127)
    ParamBits::normal(7), // 00 45: Pitch Env Level 2 (1–127)
    ParamBits::normal(7), // 00 46: Pitch Env Level 3 (1–127)
    ParamBits::normal(7), // 00 47: Pitch Env Level 4 (1–127)
    ParamBits::normal(3), // 00 48: TVF Filter Type (0–6)
    ParamBits::normal(7), // 00 49: TVF Cutoff Frequency (0–127)
    ParamBits::signed(6), // 00 4A: TVF Cutoff Keyfollow (44–84)
    ParamBits::normal(3), // 00 4B: TVF Cutoff Velocity Curve (0–7)
    ParamBits::normal(7), // 00 4C: TVF Cutoff Velocity Sens (1–127)
    ParamBits::normal(7), // 00 4D: TVF Resonance (0–127)
    ParamBits::normal(7), // 00 4E: TVF Resonance Velocity Sens (1–127)
    ParamBits::normal(7), // 00 4F: TVF Env Depth (1–127)
    ParamBits::normal(3), // 00 50: TVF Env Velocity Curve (0–7)
    ParamBits::normal(7), // 00 51: TVF Env Velocity Sens (1–127)
    ParamBits::normal(7), // 00 52: TVF Env Time 1 Velocity Sens (1–127)
    ParamBits::normal(7), // 00 53: TVF Env Time 4 Velocity Sens (1–127)
    ParamBits::signed(5), // 00 54: TVF Env Time Keyfollow (54–74)
    ParamBits::normal(7), // 00 55: TVF Env Time 1 (0–127)
    ParamBits::normal(7), // 00 56: TVF Env Time 2 (0–127)
    ParamBits::normal(7), // 00 57: TVF Env Time 3 (0–127)
    ParamBits::normal(7), // 00 58: TVF Env Time 4 (0–127)
    ParamBits::normal(7), // 00 59: TVF Env Level 0 (0–127)
    ParamBits::normal(7), // 00 5A: TVF Env Level 1 (0–127)
    ParamBits::normal(7), // 00 5B: TVF Env Level 2 (0–127)
    ParamBits::normal(7), // 00 5C: TVF Env Level 3 (0–127)
    ParamBits::normal(7), // 00 5D: TVF Env Level 4 (0–127)
    ParamBits::signed(5), // 00 5E: Bias Level (54–74)
    ParamBits::normal(7), // 00 5F: Bias Position (0–127)
    ParamBits::normal(2), // 00 60: Bias Direction (0–3)
    ParamBits::normal(3), // 00 61: TVA Level Velocity Curve (0–7)
    ParamBits::normal(7), // 00 62: TVA Level Velocity Sens (1–127)
    ParamBits::normal(7), // 00 63: TVA Env Time 1 Velocity Sens (1–127)
    ParamBits::normal(7), // 00 64: TVA Env Time 4 Velocity Sens (1–127)
    ParamBits::signed(5), // 00 65: TVA Env Time Keyfollow (54–74)
    ParamBits::normal(7), // 00 66: TVA Env Time 1 (0–127)
    ParamBits::normal(7), // 00 67: TVA Env Time 2 (0–127)
    ParamBits::normal(7), // 00 68: TVA Env Time 3 (0–127)
    ParamBits::normal(7), // 00 69: TVA Env Time 4 (0–127)
    ParamBits::normal(7), // 00 6A: TVA Env Level 1 (0–127)
    ParamBits::normal(7), // 00 6B: TVA Env Level 2 (0–127)
    ParamBits::normal(7), // 00 6C: TVA Env Level 3 (0–127)
    ParamBits::normal(4), // 00 6D: LFO1 Waveform (0–12)
    ParamBits::nibblized(2), // 00 6E: LFO1 Rate (0–149)
    ParamBits::normal(3), // 00 70: LFO1 Offset (0–4)
    ParamBits::normal(7), // 00 71: LFO1 Rate Detune (0–127)
    ParamBits::normal(7), // 00 72: LFO1 Delay Time (0–127)
    ParamBits::signed(5), // 00 73: LFO1 Delay Time Keyfollow (54–74)
    ParamBits::normal(2), // 00 74: LFO1 Fade Mode (0–3)
    ParamBits::normal(7), // 00 75: LFO1 Fade Time (0–127)
    ParamBits::normal(1), // 00 76: LFO1 Key Trigger (0–1)
    ParamBits::normal(7), // 00 77: LFO1 Pitch Depth (1–127)
    ParamBits::normal(7), // 00 78: LFO1 TVF Depth (1–127)
    ParamBits::normal(7), // 00 79: LFO1 TVA Depth (1–127)
    ParamBits::normal(7), // 00 7A: LFO1 Pan Depth (1–127)
    ParamBits::normal(4), // 00 7B: LFO2 Waveform (0–12)
    ParamBits::nibblized(2), // 00 7C: LFO2 Rate (0–149)
    ParamBits::normal(3), // 00 7E: LFO2 Offset (0–4)
    ParamBits::normal(7), // 00 7F: LFO2 Rate Detune (0–127)
    ParamBits::normal(7), // 01 00: LFO2 Delay Time (0–127)
    ParamBits::signed(5), // 01 01: LFO2 Delay Time Keyfollow (54–74)
    ParamBits::normal(2), // 01 02: LFO2 Fade Mode (0–3)
    ParamBits::normal(7), // 01 03: LFO2 Fade Time (0–127)
    ParamBits::normal(1), // 01 04: LFO2 Key Trigger (0–1)
    ParamBits::normal(7), // 01 05: LFO2 Pitch Depth (1–127)
    ParamBits::normal(7), // 01 06: LFO2 TVF Depth (1–127)
    ParamBits::normal(7), // 01 07: LFO2 TVA Depth (1–127)
    ParamBits::normal(7), // 01 08: LFO2 Pan Depth (1–127)
    ParamBits::normal(4), // 01 09: LFO Step Type (0–1)
    ParamBits::normal(7), // 01 0A: LFO Step1 (28–100)
    ParamBits::normal(7), // 01 0B: LFO Step2 (28–100)
    ParamBits::normal(7), // 01 0C: LFO Step3 (28–100)
    ParamBits::normal(7), // 01 0D: LFO Step4 (28–100)
    ParamBits::normal(7), // 01 0E: LFO Step5 (28–100)
    ParamBits::normal(7), // 01 0F: LFO Step6 (28–100)
    ParamBits::normal(7), // 01 10: LFO Step7 (28–100)
    ParamBits::normal(7), // 01 11: LFO Step8 (28–100)
    ParamBits::normal(7), // 01 12: LFO Step9 (28–100)
    ParamBits::normal(7), // 01 13: LFO Step10 (28–100)
    ParamBits::normal(7), // 01 14: LFO Step11 (28–100)
    ParamBits::normal(7), // 01 15: LFO Step12 (28–100)
    ParamBits::normal(7), // 01 16: LFO Step13 (28–100)
    ParamBits::normal(7), // 01 17: LFO Step14 (28–100)
    ParamBits::normal(7), // 01 18: LFO Step15 (28–100)
    ParamBits::normal(7), // 01 19: LFO Step16 (28–100)
];

/// PCM Synth Tone Common 2 parameters (206 bits).
///
/// Source: `docs/svd/06-pcm-synth.md` — Common 2.
#[rustfmt::skip]
static PCMS_COMMON2_PARAMS: &[ParamBits] = &[
    // 00 00–00 0F: (reserve) holding the 16-character long name (7-bit ASCII)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), // 00 10: Tone Category (0–127)
    ParamBits::nibblized(2), // 00 11: (unnamed) (0–255)
    ParamBits::signed(3), // 00 13: Phrase Octave Shift (61–67)
    ParamBits::normal(1), // 00 14: (reserve)
    // 00 15–00 31: (reserve) × 29
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1),
    ParamBits::normal(1), // 00 32: (reserve)
    ParamBits::normal(1), // 00 33: TFX Switch (0–1)
    ParamBits::normal(7), // 00 34: (reserve)
    ParamBits::normal(7), // 00 35: (reserve)
    ParamBits::normal(7), // 00 36: (reserve)
    ParamBits::normal(7), // 00 37: (reserve)
    ParamBits::nibblized(4), // 00 38: Phrase Number (0–65535)
];

/// PCM Synth Tone section layout: Common 56 B, MFX 78 B, PMT 32 B (one byte
/// more than its bits need), 4 Partials of 99 B and Common 2 26 B, then 2
/// zero bytes.
static PCMS_SECTIONS: [SvdSection; 8] = {
    let partial = SvdSection {
        params: PCMS_PARTIAL_PARAMS,
        svd_bytes: 99,
    };
    [
        SvdSection {
            params: PCMS_COMMON_PARAMS,
            svd_bytes: 56,
        },
        SvdSection {
            params: MFX_PARAMS,
            svd_bytes: 78,
        },
        SvdSection {
            params: PCMS_PMT_PARAMS,
            svd_bytes: 32,
        },
        partial,
        partial,
        partial,
        partial,
        SvdSection {
            params: PCMS_COMMON2_PARAMS,
            svd_bytes: 26,
        },
    ]
};

/// Complete PCM Synth Tone SVD specification.
pub static PCMS_TONE_SPEC: SvdToneSpec = SvdToneSpec {
    kind: ChunkType::PcmSynthTone,
    sections: &PCMS_SECTIONS,
    entry_size: 590,
    init_tail: &[],
};

// ---------------------------------------------------------------------------
// PCM Drum Kit (RFRa) — 10890 bytes, no end marker
// ---------------------------------------------------------------------------

/// PCM Drum Kit Common parameters (105 bits), reserves included.
///
/// Source: `docs/svd/07-pcm-drum.md` — Common.
#[rustfmt::skip]
static PCMD_COMMON_PARAMS: &[ParamBits] = &[
    ParamBits::normal(7), // 00 00: Kit Name 1 (32–127)
    ParamBits::normal(7), // 00 01: Kit Name 2 (32–127)
    ParamBits::normal(7), // 00 02: Kit Name 3 (32–127)
    ParamBits::normal(7), // 00 03: Kit Name 4 (32–127)
    ParamBits::normal(7), // 00 04: Kit Name 5 (32–127)
    ParamBits::normal(7), // 00 05: Kit Name 6 (32–127)
    ParamBits::normal(7), // 00 06: Kit Name 7 (32–127)
    ParamBits::normal(7), // 00 07: Kit Name 8 (32–127)
    ParamBits::normal(7), // 00 08: Kit Name 9 (32–127)
    ParamBits::normal(7), // 00 09: Kit Name 10 (32–127)
    ParamBits::normal(7), // 00 0A: Kit Name 11 (32–127)
    ParamBits::normal(7), // 00 0B: Kit Name 12 (32–127)
    ParamBits::normal(7), // 00 0C: Kit Level (0–127)
    ParamBits::normal(1), // 00 0D: (unnamed)
    ParamBits::nibblized(2), // 00 0E: (unnamed)
    ParamBits::normal(1), // 00 10: (unnamed)
    ParamBits::normal(4), // 00 11: (reserve)
];

/// PCM Drum Kit Partial parameters (969 bits), one section per key 21–108.
///
/// Source: `docs/svd/07-pcm-drum.md` — Partial.
#[rustfmt::skip]
static PCMD_PARTIAL_PARAMS: &[ParamBits] = &[
    ParamBits::normal(7), // 00 00: Partial Name 1 (32–127)
    ParamBits::normal(7), // 00 01: Partial Name 2 (32–127)
    ParamBits::normal(7), // 00 02: Partial Name 3 (32–127)
    ParamBits::normal(7), // 00 03: Partial Name 4 (32–127)
    ParamBits::normal(7), // 00 04: Partial Name 5 (32–127)
    ParamBits::normal(7), // 00 05: Partial Name 6 (32–127)
    ParamBits::normal(7), // 00 06: Partial Name 7 (32–127)
    ParamBits::normal(7), // 00 07: Partial Name 8 (32–127)
    ParamBits::normal(7), // 00 08: Partial Name 9 (32–127)
    ParamBits::normal(7), // 00 09: Partial Name 10 (32–127)
    ParamBits::normal(7), // 00 0A: Partial Name 11 (32–127)
    ParamBits::normal(7), // 00 0B: Partial Name 12 (32–127)
    ParamBits::normal(1), // 00 0C: Assign Type (0–1)
    ParamBits::normal(5), // 00 0D: Mute Group (0–31)
    ParamBits::normal(7), // 00 0E: Partial Level (0–127)
    ParamBits::normal(7), // 00 0F: Partial Coarse Tune (0–127)
    ParamBits::normal(7), // 00 10: Partial Fine Tune (14–114)
    ParamBits::normal(5), // 00 11: Partial Random Pitch Depth (0–30)
    ParamBits::normal(7), // 00 12: Partial Pan (0–127)
    ParamBits::normal(6), // 00 13: Partial Random Pan Depth (0–63)
    ParamBits::normal(7), // 00 14: Partial Alternate Pan Depth (1–127)
    ParamBits::normal(1), // 00 15: Partial Env Mode (0–1)
    ParamBits::normal(7), // 00 16: Partial Output Level (0–127)
    ParamBits::normal(7), // 00 17: (reserve)
    ParamBits::normal(7), // 00 18: (reserve)
    ParamBits::normal(7), // 00 19: Partial Chorus Send Level (0–127)
    ParamBits::normal(7), // 00 1A: Partial Reverb Send Level (0–127)
    ParamBits::normal(4), // 00 1B: Partial Output Assign (0–6)
    ParamBits::normal(6), // 00 1C: Partial Pitch Bend Range (0–48)
    ParamBits::normal(1), // 00 1D: Partial Receive Expression (0–1)
    ParamBits::normal(1), // 00 1E: Partial Receive Hold-1 (0–1)
    ParamBits::normal(1), // 00 1F: (reserve)
    ParamBits::normal(2), // 00 20: WMT Velocity Control (0–2)
    ParamBits::normal(1), // 00 21: WMT1 Wave Switch (0–1)
    ParamBits::normal(2), // 00 22: WMT1 Wave Group Type (0–3)
    ParamBits::nibblized(4), // 00 23: WMT1 Wave Group ID (0–16384)
    ParamBits::nibblized(4), // 00 27: WMT1 Wave Number L (Mono) (0–16384)
    ParamBits::nibblized(4), // 00 2B: WMT1 Wave Number R (0–16384)
    ParamBits::normal(2), // 00 2F: WMT1 Wave Gain (0–3)
    ParamBits::normal(1), // 00 30: WMT1 Wave FXM Switch (0–1)
    ParamBits::normal(2), // 00 31: WMT1 Wave FXM Color (0–3)
    ParamBits::normal(5), // 00 32: WMT1 Wave FXM Depth (0–16)
    ParamBits::normal(1), // 00 33: WMT1 Wave Tempo Sync (0–1)
    ParamBits::normal(7), // 00 34: WMT1 Wave Coarse Tune (16–112)
    ParamBits::normal(7), // 00 35: WMT1 Wave Fine Tune (14–114)
    ParamBits::normal(7), // 00 36: WMT1 Wave Pan (0–127)
    ParamBits::normal(1), // 00 37: WMT1 Wave Random Pan Switch (0–1)
    ParamBits::normal(2), // 00 38: WMT1 Wave Alternate Pan Switch (0–2)
    ParamBits::normal(7), // 00 39: WMT1 Wave Level (0–127)
    ParamBits::normal(7), // 00 3A: WMT1 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 00 3B: WMT1 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 00 3C: WMT1 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 3D: WMT1 Velocity Fade Width Upper (0–127)
    ParamBits::normal(1), // 00 3E: WMT2 Wave Switch (0–1)
    ParamBits::normal(2), // 00 3F: WMT2 Wave Group Type (0–3)
    ParamBits::nibblized(4), // 00 40: WMT2 Wave Group ID (0–16384)
    ParamBits::nibblized(4), // 00 44: WMT2 Wave Number L (Mono) (0–16384)
    ParamBits::nibblized(4), // 00 48: WMT2 Wave Number R (0–16384)
    ParamBits::normal(2), // 00 4C: WMT2 Wave Gain (0–3)
    ParamBits::normal(1), // 00 4D: WMT2 Wave FXM Switch (0–1)
    ParamBits::normal(2), // 00 4E: WMT2 Wave FXM Color (0–3)
    ParamBits::normal(5), // 00 4F: WMT2 Wave FXM Depth (0–16)
    ParamBits::normal(1), // 00 50: WMT2 Wave Tempo Sync (0–1)
    ParamBits::normal(7), // 00 51: WMT2 Wave Coarse Tune (16–112)
    ParamBits::normal(7), // 00 52: WMT2 Wave Fine Tune (14–114)
    ParamBits::normal(7), // 00 53: WMT2 Wave Pan (0–127)
    ParamBits::normal(1), // 00 54: WMT2 Wave Random Pan Switch (0–1)
    ParamBits::normal(2), // 00 55: WMT2 Wave Alternate Pan Switch (0–2)
    ParamBits::normal(7), // 00 56: WMT2 Wave Level (0–127)
    ParamBits::normal(7), // 00 57: WMT2 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 00 58: WMT2 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 00 59: WMT2 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 5A: WMT2 Velocity Fade Width Upper (0–127)
    ParamBits::normal(1), // 00 5B: WMT3 Wave Switch (0–1)
    ParamBits::normal(2), // 00 5C: WMT3 Wave Group Type (0–3)
    ParamBits::nibblized(4), // 00 5D: WMT3 Wave Group ID (0–16384)
    ParamBits::nibblized(4), // 00 61: WMT3 Wave Number L (Mono) (0–16384)
    ParamBits::nibblized(4), // 00 65: WMT3 Wave Number R (0–16384)
    ParamBits::normal(2), // 00 69: WMT3 Wave Gain (0–3)
    ParamBits::normal(1), // 00 6A: WMT3 Wave FXM Switch (0–1)
    ParamBits::normal(2), // 00 6B: WMT3 Wave FXM Color (0–3)
    ParamBits::normal(5), // 00 6C: WMT3 Wave FXM Depth (0–16)
    ParamBits::normal(1), // 00 6D: WMT3 Wave Tempo Sync (0–1)
    ParamBits::normal(7), // 00 6E: WMT3 Wave Coarse Tune (16–112)
    ParamBits::normal(7), // 00 6F: WMT3 Wave Fine Tune (14–114)
    ParamBits::normal(7), // 00 70: WMT3 Wave Pan (0–127)
    ParamBits::normal(1), // 00 71: WMT3 Wave Random Pan Switch (0–1)
    ParamBits::normal(2), // 00 72: WMT3 Wave Alternate Pan Switch (0–2)
    ParamBits::normal(7), // 00 73: WMT3 Wave Level (0–127)
    ParamBits::normal(7), // 00 74: WMT3 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 00 75: WMT3 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 00 76: WMT3 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 00 77: WMT3 Velocity Fade Width Upper (0–127)
    ParamBits::normal(1), // 00 78: WMT4 Wave Switch (0–1)
    ParamBits::normal(2), // 00 79: WMT4 Wave Group Type (0–3)
    ParamBits::nibblized(4), // 00 7A: WMT4 Wave Group ID (0–16384)
    ParamBits::nibblized(4), // 00 7E: WMT4 Wave Number L (Mono) (0–16384)
    ParamBits::nibblized(4), // 01 02: WMT4 Wave Number R (0–16384)
    ParamBits::normal(2), // 01 06: WMT4 Wave Gain (0–3)
    ParamBits::normal(1), // 01 07: WMT4 Wave FXM Switch (0–1)
    ParamBits::normal(2), // 01 08: WMT4 Wave FXM Color (0–3)
    ParamBits::normal(5), // 01 09: WMT4 Wave FXM Depth (0–16)
    ParamBits::normal(1), // 01 0A: WMT4 Wave Tempo Sync (0–1)
    ParamBits::normal(7), // 01 0B: WMT4 Wave Coarse Tune (16–112)
    ParamBits::normal(7), // 01 0C: WMT4 Wave Fine Tune (14–114)
    ParamBits::normal(7), // 01 0D: WMT4 Wave Pan (0–127)
    ParamBits::normal(1), // 01 0E: WMT4 Wave Random Pan Switch (0–1)
    ParamBits::normal(2), // 01 0F: WMT4 Wave Alternate Pan Switch (0–2)
    ParamBits::normal(7), // 01 10: WMT4 Wave Level (0–127)
    ParamBits::normal(7), // 01 11: WMT4 Velocity Range Lower (1–127)
    ParamBits::normal(7), // 01 12: WMT4 Velocity Range Upper (1–127)
    ParamBits::normal(7), // 01 13: WMT4 Velocity Fade Width Lower (0–127)
    ParamBits::normal(7), // 01 14: WMT4 Velocity Fade Width Upper (0–127)
    ParamBits::signed(5), // 01 15: Pitch Env Depth (52–76)
    ParamBits::normal(7), // 01 16: Pitch Env Velocity Sens (1–127)
    ParamBits::normal(7), // 01 17: Pitch Env Time 1 Velocity Sens (1–127)
    ParamBits::normal(7), // 01 18: Pitch Env Time 4 Velocity Sens (1–127)
    ParamBits::normal(7), // 01 19: Pitch Env Time 1 (0–127)
    ParamBits::normal(7), // 01 1A: Pitch Env Time 2 (0–127)
    ParamBits::normal(7), // 01 1B: Pitch Env Time 3 (0–127)
    ParamBits::normal(7), // 01 1C: Pitch Env Time 4 (0–127)
    ParamBits::normal(7), // 01 1D: Pitch Env Level 0 (1–127)
    ParamBits::normal(7), // 01 1E: Pitch Env Level 1 (1–127)
    ParamBits::normal(7), // 01 1F: Pitch Env Level 2 (1–127)
    ParamBits::normal(7), // 01 20: Pitch Env Level 3 (1–127)
    ParamBits::normal(7), // 01 21: Pitch Env Level 4 (1–127)
    ParamBits::normal(3), // 01 22: TVF Filter Type (0–6)
    ParamBits::normal(7), // 01 23: TVF Cutoff Frequency (0–127)
    ParamBits::normal(3), // 01 24: TVF Cutoff Velocity Curve (0–7)
    ParamBits::normal(7), // 01 25: TVF Cutoff Velocity Sens (1–127)
    ParamBits::normal(7), // 01 26: TVF Resonance (0–127)
    ParamBits::normal(7), // 01 27: TVF Resonance Velocity Sens (1–127)
    ParamBits::normal(7), // 01 28: TVF Env Depth (1–127)
    ParamBits::normal(3), // 01 29: TVF Env Velocity Curve Type (0–7)
    ParamBits::normal(7), // 01 2A: TVF Env Velocity Sens (1–127)
    ParamBits::normal(7), // 01 2B: TVF Env Time 1 Velocity Sens (1–127)
    ParamBits::normal(7), // 01 2C: TVF Env Time 4 Velocity Sens (1–127)
    ParamBits::normal(7), // 01 2D: TVF Env Time 1 (0–127)
    ParamBits::normal(7), // 01 2E: TVF Env Time 2 (0–127)
    ParamBits::normal(7), // 01 2F: TVF Env Time 3 (0–127)
    ParamBits::normal(7), // 01 30: TVF Env Time 4 (0–127)
    ParamBits::normal(7), // 01 31: TVF Env Level 0 (0–127)
    ParamBits::normal(7), // 01 32: TVF Env Level 1 (0–127)
    ParamBits::normal(7), // 01 33: TVF Env Level 2 (0–127)
    ParamBits::normal(7), // 01 34: TVF Env Level 3 (0–127)
    ParamBits::normal(7), // 01 35: TVF Env Level 4 (0–127)
    ParamBits::normal(3), // 01 36: TVA Level Velocity Curve (0–7)
    ParamBits::normal(7), // 01 37: TVA Level Velocity Sens (1–127)
    ParamBits::normal(7), // 01 38: TVA Env Time 1 Velocity Sens (1–127)
    ParamBits::normal(7), // 01 39: TVA Env Time 4 Velocity Sens (1–127)
    ParamBits::normal(7), // 01 3A: TVA Env Time 1 (0–127)
    ParamBits::normal(7), // 01 3B: TVA Env Time 2 (0–127)
    ParamBits::normal(7), // 01 3C: TVA Env Time 3 (0–127)
    ParamBits::normal(7), // 01 3D: TVA Env Time 4 (0–127)
    ParamBits::normal(7), // 01 3E: TVA Env Level 1 (0–127)
    ParamBits::normal(7), // 01 3F: TVA Env Level 2 (0–127)
    ParamBits::normal(7), // 01 40: TVA Env Level 3 (0–127)
    ParamBits::normal(1), // 01 41: One Shot Mode (0–1)
    ParamBits::normal(7), // 01 42: (reserve)
];

/// PCM Drum Kit Common 2 parameters (152 bits).
///
/// Source: `docs/svd/07-pcm-drum.md` — Common 2.
#[rustfmt::skip]
static PCMD_COMMON2_PARAMS: &[ParamBits] = &[
    // 00 00–00 0F: (reserve) holding the 16-character long name (7-bit ASCII)
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7), ParamBits::normal(7),
    ParamBits::nibblized(2), // 00 10: Phrase Number (0–255)
    ParamBits::normal(1), // 00 12: (reserve)
    // 00 13–00 2F: (reserve) × 29
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1), ParamBits::normal(1),
    ParamBits::normal(1),
    ParamBits::normal(1), // 00 30: (reserve)
    ParamBits::normal(1), // 00 31: TFX Switch (0–1)
];

/// PCM Drum Kit section layout: Common 14 B, MFX 78 B, Comp/EQ 42 B, 88
/// Partials of 122 B and Common 2 19 B, then 1 zero byte.
static PCMD_SECTIONS: [SvdSection; 92] = {
    let partial = SvdSection {
        params: PCMD_PARTIAL_PARAMS,
        svd_bytes: 122,
    };
    let mut sections = [partial; 92];
    sections[0] = SvdSection {
        params: PCMD_COMMON_PARAMS,
        svd_bytes: 14,
    };
    sections[1] = SvdSection {
        params: MFX_PARAMS,
        svd_bytes: 78,
    };
    sections[2] = SvdSection {
        params: COMP_EQ_PARAMS,
        svd_bytes: 42,
    };
    sections[91] = SvdSection {
        params: PCMD_COMMON2_PARAMS,
        svd_bytes: 19,
    };
    sections
};

/// Complete PCM Drum Kit SVD specification.
pub static PCMD_TONE_SPEC: SvdToneSpec = SvdToneSpec {
    kind: ChunkType::PcmDrumKit,
    sections: &PCMD_SECTIONS,
    entry_size: 10890,
    init_tail: &[],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sns_common_bit_count() {
        assert_eq!(SNS_COMMON_SECTION.total_bits(), 228);
    }

    #[test]
    fn sns_common_sysex_size() {
        // 0x00–0x3F = 64 SysEx bytes total
        assert_eq!(SNS_COMMON_SECTION.sysex_size(), 64);
    }

    #[test]
    fn mfx_bit_count() {
        assert_eq!(MFX_SECTION.total_bits(), 618);
    }

    #[test]
    fn mfx_sysex_size() {
        // 17 non-nibblized (1 byte each) + 32 nibblized (4 bytes each) = 145
        assert_eq!(MFX_SECTION.sysex_size(), 17 + 32 * 4);
    }

    #[test]
    fn sns_partial_bit_count() {
        assert_eq!(SNS_PARTIAL_SECTION.total_bits(), 350);
    }

    #[test]
    fn sns_partial_sysex_size() {
        // 0x00–0x3C = 61 SysEx bytes, but 0x35–0x38 is nibblized (4 bytes)
        // So: 57 normal (1 byte each) + 1 nibblized (4 bytes) = 61
        assert_eq!(SNS_PARTIAL_SECTION.sysex_size(), 61);
    }

    #[test]
    fn sns_common_section_bytes() {
        // 228 bits → 30 bytes in SVD (validated against device)
        assert_eq!(SNS_SECTIONS[0].svd_bytes, 30);
    }

    #[test]
    fn sns_mfx_section_bytes() {
        // 618 bits → 78 bytes in SVD (validated against device)
        assert_eq!(SNS_SECTIONS[1].svd_bytes, 78);
    }

    #[test]
    fn sns_tone_spec_sections() {
        assert_eq!(SNS_TONE_SPEC.sections.len(), 5);
        assert_eq!(SNS_TONE_SPEC.entry_size, 280);
        // Total data bytes: 30 + 78 + 46 + 46 + 46 = 246
        assert_eq!(SNS_TONE_SPEC.data_bytes(), 246);
    }

    #[test]
    fn sns_partial_padded_bytes() {
        // 350 bits → ceil(350/8) = 44 bytes
        // Actual SVD uses 46 bytes per partial section.
        assert_eq!(SNS_PARTIAL_SECTION.padded_bytes(), 44);
    }

    #[test]
    fn sna_common_bit_count() {
        assert_eq!(SNA_COMMON_SECTION.total_bits(), 464);
    }

    #[test]
    fn sna_common_sysex_size() {
        // 0x00–0x45 = 70 SysEx bytes (68 normal + 1 nibblized(2) = 70)
        assert_eq!(SNA_COMMON_SECTION.sysex_size(), 70);
    }

    #[test]
    fn sna_section_count() {
        assert_eq!(SNA_TONE_SPEC.sections.len(), 2);
        assert_eq!(SNA_TONE_SPEC.entry_size, 138);
        assert_eq!(SNA_TONE_SPEC.data_bytes(), 138);
        assert!(SNA_TONE_SPEC.init_tail.is_empty());
    }

    #[test]
    fn ss_section_count() {
        assert_eq!(SS_TONE_SPEC.sections.len(), 53);
        assert_eq!(SS_TONE_SPEC.entry_size, 1068);
        assert_eq!(SS_TONE_SPEC.data_bytes(), 1068);
        for s in SS_TONE_SPEC.sections {
            assert!(s.padded_bytes() <= s.svd_bytes, "{s:?}");
        }
        assert!(SS_TONE_SPEC.init_tail.is_empty());
    }

    #[test]
    fn specs_fill_their_entries() {
        for spec in [
            &SNS_TONE_SPEC,
            &SNA_TONE_SPEC,
            &SS_TONE_SPEC,
            &SND_TONE_SPEC,
            &PCMS_TONE_SPEC,
            &PCMD_TONE_SPEC,
        ] {
            assert!(spec.data_bytes() + spec.init_tail.len() <= spec.entry_size);
            for s in spec.sections {
                assert!(s.padded_bytes() <= s.svd_bytes, "{s:?}");
            }
        }
    }

    #[test]
    fn snd_section_sizes() {
        use crate::{mfx, params, sn_drum};
        let sysex = |i: usize| SND_TONE_SPEC.sections[i].sysex_size() as u32;
        assert_eq!(SND_TONE_SPEC.data_bytes(), 1006);
        assert_eq!(sysex(0), sn_drum::SND_COMMON_BLOCK_SIZE.value());
        assert_eq!(sysex(1), mfx::MFX_BLOCK_SIZE.value());
        assert_eq!(sysex(2), params::COMP_EQ_BLOCK_SIZE.value());
        assert_eq!(sysex(64), sn_drum::SND_NOTE_BLOCK_SIZE.value());
    }

    #[test]
    fn pcms_section_sizes() {
        use crate::{mfx, pcm_synth};
        let sysex = |i: usize| PCMS_TONE_SPEC.sections[i].sysex_size() as u32;
        assert_eq!(PCMS_TONE_SPEC.data_bytes(), 588);
        assert_eq!(sysex(0), pcm_synth::PCMS_COMMON_BLOCK_SIZE.value());
        assert_eq!(sysex(1), mfx::MFX_BLOCK_SIZE.value());
        assert_eq!(sysex(2), pcm_synth::PCMS_PMT_BLOCK_SIZE.value());
        assert_eq!(sysex(3), pcm_synth::PCMS_PARTIAL_BLOCK_SIZE.value());
        assert_eq!(sysex(7), pcm_synth::PCMS_COMMON2_BLOCK_SIZE.value());
    }

    #[test]
    fn pcmd_section_sizes() {
        use crate::{mfx, params, pcm_drum};
        let sysex = |i: usize| PCMD_TONE_SPEC.sections[i].sysex_size() as u32;
        assert_eq!(PCMD_TONE_SPEC.data_bytes(), 10889);
        assert_eq!(sysex(0), pcm_drum::PCMD_COMMON_BLOCK_SIZE.value());
        assert_eq!(sysex(1), mfx::MFX_BLOCK_SIZE.value());
        assert_eq!(sysex(2), params::COMP_EQ_BLOCK_SIZE.value());
        assert_eq!(sysex(3), pcm_drum::PCMD_PARTIAL_BLOCK_SIZE.value());
        assert_eq!(sysex(91), pcm_drum::PCMD_COMMON2_BLOCK_SIZE.value());
    }

    #[test]
    fn ss_common_sysex_size() {
        assert_eq!(
            SvdSection {
                params: SS_COMMON_PARAMS,
                svd_bytes: 0
            }
            .sysex_size(),
            84
        );
    }

    #[test]
    fn ss_part_sysex_size() {
        assert_eq!(
            SvdSection {
                params: SS_PART_PARAMS,
                svd_bytes: 0
            }
            .sysex_size(),
            77
        );
    }

    #[test]
    fn ss_part_eq_sysex_size() {
        assert_eq!(
            SvdSection {
                params: SS_PART_EQ_PARAMS,
                svd_bytes: 0
            }
            .sysex_size(),
            8
        );
    }
}
