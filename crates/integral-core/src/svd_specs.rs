//! SVD parameter specification tables for bit-packing/unpacking.
//!
//! Each tone type has a set of [`SvdSection`]s that define how SysEx
//! parameters map to the SVD bitstream. The spec tables are derived from
//! the MIDI Implementation documents in `docs/midi/`.
//!
//! Reference: `docs/svd/02-encoding.md`, `docs/svd/03-sn-synth.md`

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
}

/// A section of parameters that is independently byte-aligned in SVD.
#[derive(Debug, Clone)]
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
    /// Sections in SVD packing order. Each section is independently
    /// byte-aligned in the SVD entry.
    pub sections: &'static [SvdSection],
    /// Total SVD entry size in bytes (including end marker + padding).
    pub entry_size: usize,
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
    sections: &SNS_SECTIONS,
    entry_size: 280,
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
/// NOTE: `svd_bytes` are predicted but not yet validated against a real SVD.
/// Common (464 bits) and MFX (618 bits) likely follow the same separate-section
/// pattern as SN-S. Entry size 138 = Common + MFX + marker + padding.
static SNA_SECTIONS: [SvdSection; 2] = [
    SvdSection {
        params: SNA_COMMON_PARAMS,
        svd_bytes: 59, // predicted: ceil(464/8) = 58, +1 padding
    },
    SvdSection {
        params: MFX_PARAMS,
        svd_bytes: 78, // same as SN-S MFX
    },
];

/// Complete SN Acoustic Tone SVD specification.
pub static SNA_TONE_SPEC: SvdToneSpec = SvdToneSpec {
    sections: &SNA_SECTIONS,
    entry_size: 138,
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
    }
}
