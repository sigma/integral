//! MFX (Multi-Effect) address map, state types, and parse/encode functions.
//!
//! Each of the 5 tone types has its own MFX block, at offset `00 02 00`
//! within that tone type's block of the Temporary Tone (MIDI Implementation,
//! Temporary Tone Sub-Structure).  It contains: effect type, chorus/reverb
//! sends, 4 control source slots, and up to 32 nibblized parameters.

use crate::address::{Address, DataSize};
use crate::params::ToneType;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// MFX block offset within a tone type's block (same for every tone type).
pub const MFX_BLOCK_OFFSET: [u8; 3] = [0x00, 0x02, 0x00];

/// MFX block total size.
pub const MFX_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x01, 0x11);

/// MFX header size (type + sends + controls, before nibblized params).
pub const MFX_HEADER_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x11);

/// Number of MFX control slots (source, sens, assign).
pub const MFX_CONTROL_COUNT: usize = 4;

/// Number of nibblized MFX parameters.
pub const MFX_PARAM_COUNT: usize = 32;

/// Offset of the first nibblized parameter (4 bytes each) in the MFX block.
pub const MFX_PARAMS_START: u8 = 0x11;

/// A header field of the MFX block. The per-control fields repeat for each
/// of the [`MFX_CONTROL_COUNT`] control slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MfxHeaderField {
    /// MFX Type (0–67).
    Type,
    /// MFX Chorus Send Level (0–127).
    ChorusSend,
    /// MFX Reverb Send Level (0–127).
    ReverbSend,
    /// MFX Control Source of a control slot.
    ControlSource,
    /// MFX Control Sens of a control slot.
    ControlSens,
    /// MFX Control Assign of a control slot (0–16: OFF, param 1–16).
    ControlAssign,
}

impl MfxHeaderField {
    /// Number of instances of the field: [`MFX_CONTROL_COUNT`] for the
    /// per-control fields, 1 otherwise.
    pub const fn count(self) -> usize {
        match self {
            Self::Type | Self::ChorusSend | Self::ReverbSend => 1,
            Self::ControlSource | Self::ControlSens | Self::ControlAssign => MFX_CONTROL_COUNT,
        }
    }

    /// Offset of instance `index` in the MFX block, unchecked.
    const fn raw_offset(self, index: u8) -> u8 {
        match self {
            Self::Type => 0x00,
            Self::ChorusSend => 0x02,
            Self::ReverbSend => 0x03,
            Self::ControlSource => 0x05 + index * 2,
            Self::ControlSens => 0x06 + index * 2,
            Self::ControlAssign => 0x0D + index,
        }
    }
}

/// Error returned for an MFX header field instance that doesn't exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("MFX {field:?} has no instance {index}")]
pub struct MfxHeaderIndexError {
    /// The field addressed.
    pub field: MfxHeaderField,
    /// The out-of-range instance (control slot) index.
    pub index: u8,
}

/// Offset in the MFX block of instance `index` (the 0-based control slot for
/// the per-control fields, 0 otherwise) of `field`.
pub const fn mfx_header_offset(
    field: MfxHeaderField,
    index: u8,
) -> Result<u8, MfxHeaderIndexError> {
    if index as usize >= field.count() {
        return Err(MfxHeaderIndexError { field, index });
    }
    Ok(field.raw_offset(index))
}

/// Compute the absolute MFX block address for a part's tone.
///
/// Each tone type has its own MFX block at offset `00 02 00` within its
/// tone type block (MIDI Implementation, Temporary Tone).
pub const fn tone_mfx_block_address(part_index: u8, tone_type: ToneType) -> Address {
    tone_type.base_address(part_index).offset([
        0x00,
        MFX_BLOCK_OFFSET[0],
        MFX_BLOCK_OFFSET[1],
        MFX_BLOCK_OFFSET[2],
    ])
}

/// Compute the absolute address for an MFX header byte (type, sends, controls)
/// of a part's tone of type `tone_type`.
pub const fn mfx_address(part_index: u8, tone_type: ToneType, param_offset: u8) -> Address {
    tone_mfx_block_address(part_index, tone_type).offset([0x00, 0x00, 0x00, param_offset])
}

/// Compute the absolute address for a nibblized MFX parameter (0-based index)
/// of a part's tone of type `tone_type`.
pub const fn mfx_param_address(part_index: u8, tone_type: ToneType, param_index: u8) -> Address {
    let byte_offset = MFX_PARAMS_START + param_index * 4;
    tone_mfx_block_address(part_index, tone_type).offset([0x00, 0x00, 0x00, byte_offset])
}

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// MFX control source slot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MfxCtrlSlot {
    /// Source (0=OFF, 1–95=CC, 96=BEND, 97=AFT, 98–101=SYS1–SYS4).
    pub source: u8,
    /// Sensitivity (raw 1–127, display −63 to +63).
    pub sens: u8,
    /// Assign target parameter (0=OFF, 1–16=param index).
    pub assign: u8,
}

/// MFX state for a single part's tone.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MfxState {
    /// Effect type (0–67, where 0=Thru).
    pub mfx_type: u8,
    /// Chorus send level (0–127).
    pub chorus_send: u8,
    /// Reverb send level (0–127).
    pub reverb_send: u8,
    /// Control source slots.
    pub controls: [MfxCtrlSlot; MFX_CONTROL_COUNT],
    /// Nibblized parameters (up to 32, decoded to signed i32).
    pub params: Vec<i32>,
}

// ---------------------------------------------------------------------------
// Parse / Encode
// ---------------------------------------------------------------------------

/// Parse the MFX header (17 bytes at offset 0x00–0x10).
pub fn parse_mfx_header(data: &[u8]) -> MfxState {
    let mut state = MfxState::default();
    if data.len() < MFX_PARAMS_START as usize {
        return state;
    }
    use MfxHeaderField as F;
    let byte = |f: F, i: u8| data[f.raw_offset(i) as usize];
    state.mfx_type = byte(F::Type, 0);
    state.chorus_send = byte(F::ChorusSend, 0);
    state.reverb_send = byte(F::ReverbSend, 0);
    for i in 0..MFX_CONTROL_COUNT as u8 {
        state.controls[i as usize] = MfxCtrlSlot {
            source: byte(F::ControlSource, i),
            sens: byte(F::ControlSens, i),
            assign: byte(F::ControlAssign, i),
        };
    }
    state
}

/// Parse the full MFX block (header + 32 nibblized params).
pub fn parse_mfx_block(data: &[u8]) -> MfxState {
    let mut state = parse_mfx_header(data);
    let start = MFX_PARAMS_START as usize;
    if data.len() > start {
        state.params = crate::state::parse::decode_nib_params(&data[start..], MFX_PARAM_COUNT);
    }
    state
}

/// Encode a single MFX nibblized parameter value to 4 bytes.
pub fn encode_mfx_param(value: i32) -> [u8; 4] {
    crate::state::parse::encode_nib_param(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_mfx_block_address_follows_tone_type() {
        assert_eq!(
            tone_mfx_block_address(0, ToneType::PcmSynth),
            Address::new(0x19, 0x00, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(4, ToneType::SnSynth),
            Address::new(0x1A, 0x01, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(1, ToneType::SnAcoustic),
            Address::new(0x19, 0x22, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(0, ToneType::SnDrum),
            Address::new(0x19, 0x03, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(0, ToneType::PcmDrum),
            Address::new(0x19, 0x10, 0x02, 0x00)
        );
    }

    #[test]
    fn mfx_address_pcm_synth_part1() {
        assert_eq!(
            mfx_address(0, ToneType::PcmSynth, 0x00),
            Address::new(0x19, 0x00, 0x02, 0x00)
        );
        assert_eq!(
            mfx_address(0, ToneType::PcmSynth, 0x03),
            Address::new(0x19, 0x00, 0x02, 0x03)
        );
    }

    #[test]
    fn mfx_address_sn_acoustic_part1() {
        // Must not land on SN-A Common `19 02 00 00` (the tone name).
        assert_eq!(
            mfx_address(0, ToneType::SnAcoustic, 0x00),
            Address::new(0x19, 0x02, 0x02, 0x00)
        );
    }

    #[test]
    fn mfx_address_higher_part() {
        // Part 5 base = 1A 00 00 00.
        assert_eq!(
            mfx_address(4, ToneType::SnAcoustic, 0x10),
            Address::new(0x1A, 0x02, 0x02, 0x10)
        );
        // Part 16 base = 1C 60 00 00.
        assert_eq!(
            mfx_address(15, ToneType::PcmDrum, 0x00),
            Address::new(0x1C, 0x70, 0x02, 0x00)
        );
    }

    #[test]
    fn mfx_param_address_pcm_synth_part1() {
        assert_eq!(
            mfx_param_address(0, ToneType::PcmSynth, 0),
            Address::new(0x19, 0x00, 0x02, 0x11)
        );
        // 0x11 + 9*4 = 0x35
        assert_eq!(
            mfx_param_address(0, ToneType::PcmSynth, 9),
            Address::new(0x19, 0x00, 0x02, 0x35)
        );
    }

    #[test]
    fn mfx_param_address_sn_acoustic() {
        assert_eq!(
            mfx_param_address(0, ToneType::SnAcoustic, 0),
            Address::new(0x19, 0x02, 0x02, 0x11)
        );
        // Last param: 0x11 + 31*4 = 0x8D -> carries into byte 2: 02 03 0D.
        assert_eq!(
            mfx_param_address(1, ToneType::SnAcoustic, 31),
            Address::new(0x19, 0x22, 0x03, 0x0D)
        );
    }

    #[test]
    fn mfx_header_offsets_follow_the_midi_implementation() {
        // Chorus Send `00 02`, Control 2 Sens `00 08`, Control Assign 4 `00 10`.
        assert_eq!(mfx_header_offset(MfxHeaderField::ChorusSend, 0), Ok(0x02));
        assert_eq!(mfx_header_offset(MfxHeaderField::ControlSens, 1), Ok(0x08));
        assert_eq!(
            mfx_header_offset(MfxHeaderField::ControlAssign, 3),
            Ok(0x10)
        );
        assert_eq!(
            mfx_header_offset(MfxHeaderField::ControlAssign, 4),
            Err(MfxHeaderIndexError {
                field: MfxHeaderField::ControlAssign,
                index: 4
            })
        );
        assert_eq!(
            mfx_header_offset(MfxHeaderField::Type, 1),
            Err(MfxHeaderIndexError {
                field: MfxHeaderField::Type,
                index: 1
            })
        );
    }

    #[test]
    fn parse_mfx_header_basic() {
        let mut data = [0u8; 0x11];
        data[0x00] = 5; // type = Enhancer
        data[0x02] = 64; // chorus send
        data[0x03] = 80; // reverb send
        data[0x05] = 1; // ctrl 1 source = CC01
        data[0x06] = 100; // ctrl 1 sens
        data[0x0D] = 3; // ctrl assign 1 = param 3

        let state = parse_mfx_header(&data);
        assert_eq!(state.mfx_type, 5);
        assert_eq!(state.chorus_send, 64);
        assert_eq!(state.reverb_send, 80);
        assert_eq!(state.controls[0].source, 1);
        assert_eq!(state.controls[0].sens, 100);
        assert_eq!(state.controls[0].assign, 3);
    }
}
