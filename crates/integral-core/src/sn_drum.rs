//! SuperNATURAL Drum Kit address map, state types, and parse functions.
//!
//! The SN-D tone occupies offset `03 00 00` within the temporary tone block.
//! It contains: Common parameters, MFX (handled by `mfx.rs`), Comp+EQ
//! (handled by `params.rs`), and 62 per-key Note blocks.

use crate::address::{Address, DataSize};
use crate::block_field::BlockField;
use crate::device_spec::{KeyOutOfRange, KeyRange};
use crate::mfx::MfxState;
use crate::params;
use crate::parse_helpers::parse_ascii_name;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

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

/// Highest SN-D Note Inst Number (0 is OFF).
pub const SND_INST_NUMBER_MAX: u16 = 512;

/// Keys an SN-D kit has a Note block for.
pub const SND_KEYS: KeyRange = KeyRange {
    first: SND_FIRST_KEY,
    last: SND_LAST_KEY,
};

/// Number of SN-D note slots.
pub const SND_NOTE_COUNT: usize = (SND_LAST_KEY - SND_FIRST_KEY + 1) as usize; // 62

/// Compute the absolute SN-D base address for a part.
const fn snd_base(part_index: u8) -> Address {
    params::ToneType::SnDrum.base_address(part_index)
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
/// Key 27 maps to offset `00 10 00`, key 28 to `00 11 00`, etc. Keys outside
/// [`SND_KEYS`] are rejected.
pub fn snd_note_address(part_index: u8, key: u8) -> Result<Address, KeyOutOfRange> {
    let note_offset = 0x10 + (SND_KEYS.check(key)? - SND_FIRST_KEY);
    Ok(snd_base(part_index).offset([0x00, 0x00, note_offset, 0x00]))
}

/// Compute the absolute address for a single SN-D Note parameter.
pub fn snd_note_param_address(
    part_index: u8,
    key: u8,
    offset: u8,
) -> Result<Address, KeyOutOfRange> {
    Ok(snd_note_address(part_index, key)?.offset([0x00, 0x00, 0x00, offset]))
}

/// A field of an SN Drum Kit Common block. The discriminant is the field's
/// byte position within the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SndCommonField {
    /// Kit Level.
    KitLevel = 0x10,
    /// Ambience Level.
    AmbienceLevel = 0x11,
    /// Phrase Number.
    PhraseNumber = 0x12,
    /// TFX Switch.
    TfxSwitch = 0x13,
}

impl BlockField for SndCommonField {
    fn position(self) -> usize {
        self as usize
    }
}

/// A field of an SN Drum Kit per-key Note block. The discriminant is the
/// field's first byte position within the block; [`BlockField`] gives the
/// nibblized fields' width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SndNoteField {
    /// Inst Number (nibblized, 4 bytes).
    InstNumber = 0x00,
    /// Level.
    Level = 0x04,
    /// Pan.
    Pan = 0x05,
    /// Chorus Send Level.
    ChorusSend = 0x06,
    /// Reverb Send Level.
    ReverbSend = 0x07,
    /// Tune (nibblized, 4 bytes).
    Tune = 0x08,
    /// Attack.
    Attack = 0x0C,
    /// Decay.
    Decay = 0x0D,
    /// Brilliance.
    Brilliance = 0x0E,
    /// Variation.
    Variation = 0x0F,
    /// Dynamic Range.
    DynamicRange = 0x10,
    /// Stereo Width.
    StereoWidth = 0x11,
    /// Output Assign.
    OutputAssign = 0x12,
}

impl BlockField for SndNoteField {
    fn position(self) -> usize {
        self as usize
    }

    fn width(self) -> usize {
        match self {
            Self::InstNumber | Self::Tune => 4,
            _ => 1,
        }
    }
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
    /// Inst Number (0–[`SND_INST_NUMBER_MAX`], nibblized across 4 bytes).
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

    use SndCommonField as F;
    c.kit_name = parse_ascii_name(data);
    c.kit_level = F::KitLevel.byte(data);
    c.ambience_level = F::AmbienceLevel.byte(data);
    c.phrase_number = F::PhraseNumber.byte(data);
    c.tfx_switch = F::TfxSwitch.byte(data);

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

    use SndNoteField as F;
    n.inst_number = F::InstNumber.nibbles(data);
    n.level = F::Level.byte(data);
    n.pan = F::Pan.byte(data);
    n.chorus_send = F::ChorusSend.byte(data);
    n.reverb_send = F::ReverbSend.byte(data);
    n.tune = F::Tune.nibbles(data);
    n.attack = F::Attack.byte(data);
    n.decay = F::Decay.byte(data);
    n.brilliance = F::Brilliance.byte(data);
    n.variation = F::Variation.byte(data);
    n.dynamic_range = F::DynamicRange.byte(data);
    n.stereo_width = F::StereoWidth.byte(data);
    n.output_assign = F::OutputAssign.byte(data);

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
        assert_eq!(addr, Ok(Address::new(0x19, 0x03, 0x10, 0x00)));
    }

    #[test]
    fn snd_note_address_last_key() {
        let addr = snd_note_address(0, 88);
        // key 88 → offset 0x10 + (88 - 27) = 0x10 + 61 = 0x10 + 0x3D = 0x4D
        assert_eq!(addr, Ok(Address::new(0x19, 0x03, 0x4D, 0x00)));
    }

    #[test]
    fn snd_note_address_rejects_keys_outside_the_kit() {
        for key in [SND_FIRST_KEY - 1, SND_LAST_KEY + 1] {
            assert_eq!(
                snd_note_address(0, key),
                Err(KeyOutOfRange {
                    key,
                    range: SND_KEYS
                })
            );
            assert!(snd_note_param_address(0, key, 0x04).is_err());
        }
    }

    #[test]
    fn snd_note_param_address_level() {
        let addr = snd_note_param_address(0, 27, 0x04);
        assert_eq!(addr, Ok(Address::new(0x19, 0x03, 0x10, 0x04)));
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

    #[test]
    fn parse_common_reads_every_field_at_its_position() {
        use crate::block_field::test_support::{assert_reads_at_positions, positional_block};
        use SndCommonField as F;
        let c = parse_snd_common(&positional_block(SND_COMMON_SIZE)).unwrap();
        assert_reads_at_positions!(c, {
            kit_level: F::KitLevel,
            ambience_level: F::AmbienceLevel,
            phrase_number: F::PhraseNumber,
            tfx_switch: F::TfxSwitch,
        });
    }

    #[test]
    fn parse_note_reads_every_field_at_its_position() {
        use crate::block_field::test_support::{assert_reads_at_positions, positional_block};
        use SndNoteField as F;
        let n = parse_snd_note(&positional_block(SND_NOTE_SIZE)).unwrap();
        assert_reads_at_positions!(n, {
            inst_number: F::InstNumber,
            level: F::Level,
            pan: F::Pan,
            chorus_send: F::ChorusSend,
            reverb_send: F::ReverbSend,
            tune: F::Tune,
            attack: F::Attack,
            decay: F::Decay,
            brilliance: F::Brilliance,
            variation: F::Variation,
            dynamic_range: F::DynamicRange,
            stereo_width: F::StereoWidth,
            output_assign: F::OutputAssign,
        });
    }
}
