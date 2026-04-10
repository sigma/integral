//! SuperNATURAL Acoustic Tone address map, state types, and parse functions.
//!
//! The SN-A tone occupies offset `02 00 00` within the temporary tone block.
//! It contains: Common parameters and MFX (handled by `mfx.rs`).

use crate::address::{Address, DataSize};
use crate::mfx::MfxState;
use crate::params;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// SN-A tone type offset within the temporary tone block.
const SNA_TONE_OFFSET: [u8; 4] = [0x00, 0x02, 0x00, 0x00];

/// SN-A Common block offset (relative to SN-A base).
const SNA_COMMON_OFFSET: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// SN-A Common block size.
pub const SNA_COMMON_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x46);

/// SN-A Common parse size in bytes.
pub const SNA_COMMON_SIZE: usize = 0x46;

/// Compute the absolute SN-A base address for a part.
const fn sna_base(part_index: u8) -> Address {
    params::temporary_tone_base(part_index).offset(SNA_TONE_OFFSET)
}

/// Compute the absolute SN-A Common block address for a part.
pub const fn sna_common_address(part_index: u8) -> Address {
    sna_base(part_index).offset(SNA_COMMON_OFFSET)
}

/// Compute the absolute address for a single SN-A Common parameter.
pub const fn sna_common_param_address(part_index: u8, offset: u8) -> Address {
    sna_common_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// SN Acoustic Tone Common parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnAcousticCommon {
    /// Tone name (up to 12 ASCII characters).
    pub tone_name: String,
    /// Tone Level (0–127).
    pub tone_level: u8,
    /// Mono/Poly (0=MONO, 1=POLY).
    pub mono_poly: u8,
    /// Portamento Time Offset (0–127, display −64 to +63).
    pub portamento_time_offset: u8,
    /// Cutoff Offset (0–127, display −64 to +63).
    pub cutoff_offset: u8,
    /// Resonance Offset (0–127, display −64 to +63).
    pub resonance_offset: u8,
    /// Attack Time Offset (0–127, display −64 to +63).
    pub attack_time_offset: u8,
    /// Release Time Offset (0–127, display −64 to +63).
    pub release_time_offset: u8,
    /// Vibrato Rate (0–127, display −64 to +63).
    pub vibrato_rate: u8,
    /// Vibrato Depth (0–127, display −64 to +63).
    pub vibrato_depth: u8,
    /// Vibrato Delay (0–127, display −64 to +63).
    pub vibrato_delay: u8,
    /// Octave Shift (raw 61–67, display −3 to +3).
    pub octave_shift: u8,
    /// Category (0–127).
    pub category: u8,
    /// Phrase Number (0–255, nibblized across 2 bytes).
    pub phrase_number: u16,
    /// Phrase Octave Shift (raw 61–67, display −3 to +3).
    pub phrase_octave_shift: u8,
    /// TFX Switch (0=OFF, 1=ON).
    pub tfx_switch: u8,
    /// Inst Variation (0–127).
    pub inst_variation: u8,
    /// Inst Number (0–127).
    pub inst_number: u8,
    /// Modify Parameters 1–32 (each 0–127).
    pub modify_params: [u8; 32],
}

impl Default for SnAcousticCommon {
    fn default() -> Self {
        Self {
            tone_name: String::new(),
            tone_level: 127,
            mono_poly: 1, // POLY
            portamento_time_offset: 64,
            cutoff_offset: 64,
            resonance_offset: 64,
            attack_time_offset: 64,
            release_time_offset: 64,
            vibrato_rate: 64,
            vibrato_depth: 64,
            vibrato_delay: 64,
            octave_shift: 64, // 0
            category: 0,
            phrase_number: 0,
            phrase_octave_shift: 64, // 0
            tfx_switch: 0,
            inst_variation: 0,
            inst_number: 0,
            modify_params: [0; 32],
        }
    }
}

/// Full SN Acoustic Tone state for one part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnAcousticState {
    /// Common parameters.
    pub common: SnAcousticCommon,
    /// MFX parameters.
    pub mfx: MfxState,
}

// ---------------------------------------------------------------------------
// Parse functions
// ---------------------------------------------------------------------------

/// Parse an SN Acoustic Tone Common dump (0x46 bytes).
///
/// Offsets follow the MIDI implementation doc:
/// - `00 00`–`00 0B`: Tone Name (12 ASCII chars)
/// - `00 0C`–`00 0F`: reserve
/// - `00 10`: Tone Level
/// - `00 11`: Mono/Poly
/// - `00 12`–`00 19`: Portamento Time Offset through Vibrato Delay
/// - `00 1A`: Octave Shift
/// - `00 1B`: Category
/// - `00 1C`–`00 1D`: Phrase Number (nibblized)
/// - `00 1E`: Phrase Octave Shift
/// - `00 1F`: TFX Switch
/// - `00 20`: Inst Variation
/// - `00 21`: Inst Number
/// - `00 22`–`00 41`: Modify Parameters 1–32
/// - `00 42`–`00 45`: reserve
pub fn parse_sna_common(data: &[u8]) -> SnAcousticCommon {
    let mut c = SnAcousticCommon::default();
    if data.len() < SNA_COMMON_SIZE {
        return c;
    }

    // Tone Name: bytes 0x00–0x0B
    c.tone_name = data[0x00..0x0C]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string();

    // 0x0C–0x0F: reserve
    c.tone_level = data[0x10];
    c.mono_poly = data[0x11];
    c.portamento_time_offset = data[0x12];
    c.cutoff_offset = data[0x13];
    c.resonance_offset = data[0x14];
    c.attack_time_offset = data[0x15];
    c.release_time_offset = data[0x16];
    c.vibrato_rate = data[0x17];
    c.vibrato_depth = data[0x18];
    c.vibrato_delay = data[0x19];
    c.octave_shift = data[0x1A];
    c.category = data[0x1B];

    // Phrase Number: nibblized 2 bytes at 0x1C–0x1D
    c.phrase_number = ((data[0x1C] as u16 & 0x0F) << 4) | (data[0x1D] as u16 & 0x0F);

    c.phrase_octave_shift = data[0x1E];
    c.tfx_switch = data[0x1F];
    c.inst_variation = data[0x20];
    c.inst_number = data[0x21];

    // Modify Parameters 1–32: bytes 0x22–0x41
    c.modify_params.copy_from_slice(&data[0x22..0x22 + 32]);

    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sna_common_address_part1() {
        let addr = sna_common_address(0);
        // Part 1 tone base = 19 00 00 00, + 02 00 00 = 19 02 00 00
        assert_eq!(addr, Address::new(0x19, 0x02, 0x00, 0x00));
    }

    #[test]
    fn sna_common_param_address_tone_level() {
        let addr = sna_common_param_address(0, 0x10);
        assert_eq!(addr, Address::new(0x19, 0x02, 0x00, 0x10));
    }

    #[test]
    fn sna_common_address_part16() {
        let addr = sna_common_address(15);
        // Part 16 tone base = 1C 60 00 00, + 02 00 00 = 1C 62 00 00
        assert_eq!(addr, Address::new(0x1C, 0x62, 0x00, 0x00));
    }

    #[test]
    fn parse_common_basic() {
        let mut data = [0u8; SNA_COMMON_SIZE];
        // Tone name "Grand Piano "
        let name = b"Grand Piano ";
        data[0x00..0x0C].copy_from_slice(name);
        data[0x10] = 100; // tone level
        data[0x11] = 1; // POLY
        data[0x12] = 70; // portamento time offset
        data[0x13] = 64; // cutoff offset (0)
        data[0x1A] = 65; // octave shift +1
        data[0x1B] = 1; // category
        // Phrase number = 42 → nibbles: 2, 10 → 0x2A
        data[0x1C] = 0x02;
        data[0x1D] = 0x0A;
        data[0x1F] = 1; // TFX on
        data[0x20] = 3; // inst variation
        data[0x21] = 5; // inst number
        // Modify param 1 = 100
        data[0x22] = 100;
        // Modify param 32 = 50
        data[0x41] = 50;

        let c = parse_sna_common(&data);
        assert_eq!(c.tone_name, "Grand Piano");
        assert_eq!(c.tone_level, 100);
        assert_eq!(c.mono_poly, 1);
        assert_eq!(c.portamento_time_offset, 70);
        assert_eq!(c.cutoff_offset, 64);
        assert_eq!(c.octave_shift, 65);
        assert_eq!(c.category, 1);
        assert_eq!(c.phrase_number, 42);
        assert_eq!(c.tfx_switch, 1);
        assert_eq!(c.inst_variation, 3);
        assert_eq!(c.inst_number, 5);
        assert_eq!(c.modify_params[0], 100);
        assert_eq!(c.modify_params[31], 50);
    }

    #[test]
    fn parse_common_short_data_returns_default() {
        let data = [0u8; 10];
        let c = parse_sna_common(&data);
        assert_eq!(c, SnAcousticCommon::default());
    }
}
