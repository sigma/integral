//! SuperNATURAL Drum Kit address map, state types, and parse functions.
//!
//! The SN-D tone occupies offset `03 00 00` within the temporary tone block.
//! It contains: Common parameters, MFX (handled by `mfx.rs`), Comp+EQ
//! (handled by `params.rs`), and 62 per-key Note blocks.

use crate::address::{Address, DataSize};
use crate::mfx::MfxState;
use crate::params;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// SN-D tone type offset within the temporary tone block.
const SND_TONE_OFFSET: [u8; 4] = [0x00, 0x03, 0x00, 0x00];

/// SN-D Common block offset (relative to SN-D base).
const SND_COMMON_OFFSET: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// SN-D Common block size (20 bytes).
pub const SND_COMMON_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x14);

/// SN-D Common parse size in bytes.
pub const SND_COMMON_SIZE: usize = 0x14;

/// SN-D Note block size (19 bytes).
pub const SND_NOTE_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x13);

/// SN-D Note parse size in bytes.
pub const SND_NOTE_SIZE: usize = 0x13;

/// First key number in the SN-D note range.
pub const SND_FIRST_KEY: u8 = 27;

/// Last key number in the SN-D note range.
pub const SND_LAST_KEY: u8 = 88;

/// Number of SN-D note slots.
pub const SND_NOTE_COUNT: usize = (SND_LAST_KEY - SND_FIRST_KEY + 1) as usize; // 62

/// Compute the absolute SN-D base address for a part.
const fn snd_base(part_index: u8) -> Address {
    params::temporary_tone_base(part_index).offset(SND_TONE_OFFSET)
}

/// Compute the absolute SN-D Common block address for a part.
pub const fn snd_common_address(part_index: u8) -> Address {
    snd_base(part_index).offset(SND_COMMON_OFFSET)
}

/// Compute the absolute address for a single SN-D Common parameter.
pub const fn snd_common_param_address(part_index: u8, offset: u8) -> Address {
    snd_common_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

/// Compute the absolute SN-D Note block address for a part and key number.
///
/// Key 27 maps to offset `00 10 00`, key 28 to `00 11 00`, etc.
pub const fn snd_note_address(part_index: u8, key: u8) -> Address {
    let note_offset = 0x10 + (key - SND_FIRST_KEY);
    snd_base(part_index).offset([0x00, 0x00, note_offset, 0x00])
}

/// Compute the absolute address for a single SN-D Note parameter.
pub const fn snd_note_param_address(part_index: u8, key: u8, offset: u8) -> Address {
    snd_note_address(part_index, key).offset([0x00, 0x00, 0x00, offset])
}

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// SN Drum Kit Common parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnDrumCommon {
    /// Kit name (up to 12 ASCII characters).
    pub kit_name: String,
    /// Kit Level (0–127).
    pub kit_level: u8,
    /// Ambience Level (0–127).
    pub ambience_level: u8,
    /// Phrase Number (0–127).
    pub phrase_number: u8,
    /// TFX Switch (0=OFF, 1=ON).
    pub tfx_switch: u8,
}

impl Default for SnDrumCommon {
    fn default() -> Self {
        Self {
            kit_name: String::new(),
            kit_level: 127,
            ambience_level: 64,
            phrase_number: 0,
            tfx_switch: 0,
        }
    }
}

/// SN Drum Kit per-key Note parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnDrumNote {
    /// Inst Number (0–512, nibblized across 4 bytes).
    pub inst_number: u16,
    /// Level (0–127).
    pub level: u8,
    /// Pan (0–127, display L64–63R).
    pub pan: u8,
    /// Chorus Send Level (0–127).
    pub chorus_send: u8,
    /// Reverb Send Level (0–127).
    pub reverb_send: u8,
    /// Tune (8–248, nibblized across 4 bytes, display −1200–+1200).
    pub tune: u16,
    /// Attack (0–100%).
    pub attack: u8,
    /// Decay (1–64, display −63–0).
    pub decay: u8,
    /// Brilliance (49–76, display −15–+12).
    pub brilliance: u8,
    /// Variation (0–7: OFF, FLAM1–3, BUZZ1–3, ROLL).
    pub variation: u8,
    /// Dynamic Range (0–63).
    pub dynamic_range: u8,
    /// Stereo Width (0–127).
    pub stereo_width: u8,
    /// Output Assign (0–6: PART, COMP+EQ 1–6).
    pub output_assign: u8,
}

impl Default for SnDrumNote {
    fn default() -> Self {
        Self {
            inst_number: 0,
            level: 127,
            pan: 64,
            chorus_send: 0,
            reverb_send: 0,
            tune: 128, // center (0)
            attack: 100,
            decay: 64,      // 0
            brilliance: 64, // 0 (center of 49–76 range)
            variation: 0,
            dynamic_range: 63,
            stereo_width: 64,
            output_assign: 0,
        }
    }
}

/// Full SN Drum Kit state for one part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnDrumKitState {
    /// Common parameters.
    pub common: SnDrumCommon,
    /// Per-key note parameters (62 notes, keys 27–88).
    pub notes: Vec<SnDrumNote>,
    /// MFX parameters.
    pub mfx: MfxState,
}

// ---------------------------------------------------------------------------
// Parse functions
// ---------------------------------------------------------------------------

/// Parse an SN Drum Kit Common dump (0x14 = 20 bytes).
///
/// Offsets:
/// - `00 00`–`00 0B`: Kit Name (12 ASCII chars)
/// - `00 0C`–`00 0F`: (reserve)
/// - `00 10`: Kit Level
/// - `00 11`: Ambience Level
/// - `00 12`: Phrase Number
/// - `00 13`: TFX Switch
#[allow(clippy::field_reassign_with_default)]
pub fn parse_snd_common(data: &[u8]) -> Result<SnDrumCommon, crate::ToneParseError> {
    if data.len() < SND_COMMON_SIZE {
        return Err(crate::ToneParseError {
            expected: SND_COMMON_SIZE,
            got: data.len(),
        });
    }
    let mut c = SnDrumCommon::default();

    // Kit Name: bytes 0x00–0x0B
    c.kit_name = data[0x00..0x0C]
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

    c.kit_level = data[0x10];
    c.ambience_level = data[0x11];
    c.phrase_number = data[0x12];
    c.tfx_switch = data[0x13];

    Ok(c)
}

/// Parse an SN Drum Kit Note dump (0x13 = 19 bytes).
///
/// Offsets:
/// - `00 00`–`00 03`: Inst Number (nibblized, 4 bytes)
/// - `00 04`: Level
/// - `00 05`: Pan
/// - `00 06`: Chorus Send Level
/// - `00 07`: Reverb Send Level
/// - `00 08`–`00 0B`: Tune (nibblized, 4 bytes)
/// - `00 0C`: Attack
/// - `00 0D`: Decay
/// - `00 0E`: Brilliance
/// - `00 0F`: Variation
/// - `00 10`: Dynamic Range
/// - `00 11`: Stereo Width
/// - `00 12`: Output Assign
#[allow(clippy::field_reassign_with_default)]
pub fn parse_snd_note(data: &[u8]) -> Result<SnDrumNote, crate::ToneParseError> {
    if data.len() < SND_NOTE_SIZE {
        return Err(crate::ToneParseError {
            expected: SND_NOTE_SIZE,
            got: data.len(),
        });
    }
    let mut n = SnDrumNote::default();

    // Inst Number: nibblized 4 bytes at 0x00–0x03
    n.inst_number = ((data[0x00] as u16 & 0x0F) << 12)
        | ((data[0x01] as u16 & 0x0F) << 8)
        | ((data[0x02] as u16 & 0x0F) << 4)
        | (data[0x03] as u16 & 0x0F);

    n.level = data[0x04];
    n.pan = data[0x05];
    n.chorus_send = data[0x06];
    n.reverb_send = data[0x07];

    // Tune: nibblized 4 bytes at 0x08–0x0B
    n.tune = ((data[0x08] as u16 & 0x0F) << 12)
        | ((data[0x09] as u16 & 0x0F) << 8)
        | ((data[0x0A] as u16 & 0x0F) << 4)
        | (data[0x0B] as u16 & 0x0F);

    n.attack = data[0x0C];
    n.decay = data[0x0D];
    n.brilliance = data[0x0E];
    n.variation = data[0x0F];
    n.dynamic_range = data[0x10];
    n.stereo_width = data[0x11];
    n.output_assign = data[0x12];

    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snd_common_address_part1() {
        let addr = snd_common_address(0);
        // Part 1 tone base = 19 00 00 00, + 03 00 00 = 19 03 00 00
        assert_eq!(addr, Address::new(0x19, 0x03, 0x00, 0x00));
    }

    #[test]
    fn snd_common_param_address_kit_level() {
        let addr = snd_common_param_address(0, 0x10);
        assert_eq!(addr, Address::new(0x19, 0x03, 0x00, 0x10));
    }

    #[test]
    fn snd_note_address_first_key() {
        let addr = snd_note_address(0, 27);
        // key 27 → offset 0x10 in byte 2
        assert_eq!(addr, Address::new(0x19, 0x03, 0x10, 0x00));
    }

    #[test]
    fn snd_note_address_last_key() {
        let addr = snd_note_address(0, 88);
        // key 88 → offset 0x10 + (88 - 27) = 0x10 + 61 = 0x10 + 0x3D = 0x4D
        assert_eq!(addr, Address::new(0x19, 0x03, 0x4D, 0x00));
    }

    #[test]
    fn snd_note_param_address_level() {
        let addr = snd_note_param_address(0, 27, 0x04);
        assert_eq!(addr, Address::new(0x19, 0x03, 0x10, 0x04));
    }

    #[test]
    fn snd_common_address_part16() {
        let addr = snd_common_address(15);
        // Part 16 tone base = 1C 60 00 00, + 03 00 00 = 1C 63 00 00
        assert_eq!(addr, Address::new(0x1C, 0x63, 0x00, 0x00));
    }

    #[test]
    fn parse_common_basic() {
        let mut data = [0u8; SND_COMMON_SIZE];
        let name = b"Jazz Kit    ";
        data[0x00..0x0C].copy_from_slice(name);
        data[0x10] = 100; // kit level
        data[0x11] = 80; // ambience level
        data[0x12] = 5; // phrase number
        data[0x13] = 1; // TFX on

        let c = parse_snd_common(&data).unwrap();
        assert_eq!(c.kit_name, "Jazz Kit");
        assert_eq!(c.kit_level, 100);
        assert_eq!(c.ambience_level, 80);
        assert_eq!(c.phrase_number, 5);
        assert_eq!(c.tfx_switch, 1);
    }

    #[test]
    fn parse_common_short_data_returns_err() {
        assert!(parse_snd_common(&[0u8; 10]).is_err());
    }

    #[test]
    fn parse_note_basic() {
        let mut data = [0u8; SND_NOTE_SIZE];
        // Inst Number = 42 → nibbles: 0, 0, 2, 10 = 0x002A
        data[0x00] = 0x00;
        data[0x01] = 0x00;
        data[0x02] = 0x02;
        data[0x03] = 0x0A;
        data[0x04] = 100; // level
        data[0x05] = 64; // pan center
        data[0x06] = 30; // chorus send
        data[0x07] = 50; // reverb send
        // Tune = 128 (center) → nibbles: 0, 0, 8, 0
        data[0x08] = 0x00;
        data[0x09] = 0x00;
        data[0x0A] = 0x08;
        data[0x0B] = 0x00;
        data[0x0C] = 80; // attack
        data[0x0D] = 64; // decay (0)
        data[0x0E] = 64; // brilliance (center)
        data[0x0F] = 1; // variation = FLAM1
        data[0x10] = 63; // dynamic range
        data[0x11] = 100; // stereo width
        data[0x12] = 2; // output assign = COMP+EQ2

        let n = parse_snd_note(&data).unwrap();
        assert_eq!(n.inst_number, 42);
        assert_eq!(n.level, 100);
        assert_eq!(n.pan, 64);
        assert_eq!(n.chorus_send, 30);
        assert_eq!(n.reverb_send, 50);
        assert_eq!(n.tune, 128);
        assert_eq!(n.attack, 80);
        assert_eq!(n.decay, 64);
        assert_eq!(n.brilliance, 64);
        assert_eq!(n.variation, 1);
        assert_eq!(n.dynamic_range, 63);
        assert_eq!(n.stereo_width, 100);
        assert_eq!(n.output_assign, 2);
    }

    #[test]
    fn parse_note_short_data_returns_err() {
        assert!(parse_snd_note(&[0u8; 5]).is_err());
    }
}
