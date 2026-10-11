//! MFX (Multi-Effect) address map, state types, and parse/encode functions.
//!
//! Each of the 5 tone types has its own MFX block, at offset `00 02 00`
//! within that tone type's block of the Temporary Tone (MIDI Implementation,
//! Temporary Tone Sub-Structure).  It contains: effect type, chorus/reverb
//! sends, 4 control source slots, and up to 32 nibblized parameters.

use crate::address::{Address, DataSize};
use crate::params;

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

/// Per-param offsets within the MFX block.
pub mod offset {
    /// MFX Type (0–67).
    pub const TYPE: u8 = 0x00;
    /// MFX Chorus Send Level (0–127).
    pub const CHORUS_SEND: u8 = 0x02;
    /// MFX Reverb Send Level (0–127).
    pub const REVERB_SEND: u8 = 0x03;
    /// MFX Control 1 Source.
    pub const CTRL1_SOURCE: u8 = 0x05;
    /// MFX Control 1 Sens.
    pub const CTRL1_SENS: u8 = 0x06;
    /// MFX Control 2 Source.
    pub const CTRL2_SOURCE: u8 = 0x07;
    /// MFX Control 2 Sens.
    pub const CTRL2_SENS: u8 = 0x08;
    /// MFX Control 3 Source.
    pub const CTRL3_SOURCE: u8 = 0x09;
    /// MFX Control 3 Sens.
    pub const CTRL3_SENS: u8 = 0x0A;
    /// MFX Control 4 Source.
    pub const CTRL4_SOURCE: u8 = 0x0B;
    /// MFX Control 4 Sens.
    pub const CTRL4_SENS: u8 = 0x0C;
    /// MFX Control Assign 1 (0–16: OFF, param 1–16).
    pub const CTRL_ASSIGN1: u8 = 0x0D;
    /// MFX Control Assign 2.
    pub const CTRL_ASSIGN2: u8 = 0x0E;
    /// MFX Control Assign 3.
    pub const CTRL_ASSIGN3: u8 = 0x0F;
    /// MFX Control Assign 4.
    pub const CTRL_ASSIGN4: u8 = 0x10;
    /// First nibblized parameter (4 bytes each, starting at 0x11).
    pub const PARAMS_START: u8 = 0x11;
}

/// Compute the absolute MFX block address for a part's tone.
///
/// `tone_type_offset` is a [`params::tone_type`] constant: each tone type has
/// its own MFX block at offset `00 02 00` within its tone type block
/// (MIDI Implementation, Temporary Tone).
pub const fn tone_mfx_block_address(part_index: u8, tone_type_offset: [u8; 3]) -> Address {
    params::temporary_tone_base(part_index)
        .offset([
            0x00,
            tone_type_offset[0],
            tone_type_offset[1],
            tone_type_offset[2],
        ])
        .offset([
            0x00,
            MFX_BLOCK_OFFSET[0],
            MFX_BLOCK_OFFSET[1],
            MFX_BLOCK_OFFSET[2],
        ])
}

/// Compute the absolute address for an MFX header byte (type, sends, controls)
/// of a part's tone of type `tone_type_offset` (a [`params::tone_type`] constant).
pub const fn mfx_address(part_index: u8, tone_type_offset: [u8; 3], param_offset: u8) -> Address {
    tone_mfx_block_address(part_index, tone_type_offset).offset([0x00, 0x00, 0x00, param_offset])
}

/// Compute the absolute address for a nibblized MFX parameter (0-based index)
/// of a part's tone of type `tone_type_offset` (a [`params::tone_type`] constant).
pub const fn mfx_param_address(
    part_index: u8,
    tone_type_offset: [u8; 3],
    param_index: u8,
) -> Address {
    let byte_offset = offset::PARAMS_START + param_index * 4;
    tone_mfx_block_address(part_index, tone_type_offset).offset([0x00, 0x00, 0x00, byte_offset])
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
    if data.len() < 0x11 {
        return state;
    }
    state.mfx_type = data[0x00];
    state.chorus_send = data[0x02];
    state.reverb_send = data[0x03];
    for i in 0..MFX_CONTROL_COUNT as u8 {
        let src_off = (0x05 + i * 2) as usize;
        let sens_off = (0x06 + i * 2) as usize;
        let assign_off = (0x0D + i) as usize;
        state.controls[i as usize] = MfxCtrlSlot {
            source: data[src_off],
            sens: data[sens_off],
            assign: data[assign_off],
        };
    }
    state
}

/// Parse the full MFX block (header + 32 nibblized params).
pub fn parse_mfx_block(data: &[u8]) -> MfxState {
    let mut state = parse_mfx_header(data);
    if data.len() > 0x11 {
        state.params = crate::state::parse::decode_nib_params(&data[0x11..], MFX_PARAM_COUNT);
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
            tone_mfx_block_address(0, params::tone_type::PCM_SYNTH),
            Address::new(0x19, 0x00, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(4, params::tone_type::SN_SYNTH),
            Address::new(0x1A, 0x01, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(1, params::tone_type::SN_ACOUSTIC),
            Address::new(0x19, 0x22, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(0, params::tone_type::SN_DRUM),
            Address::new(0x19, 0x03, 0x02, 0x00)
        );
        assert_eq!(
            tone_mfx_block_address(0, params::tone_type::PCM_DRUM),
            Address::new(0x19, 0x10, 0x02, 0x00)
        );
    }

    #[test]
    fn mfx_address_pcm_synth_part1() {
        assert_eq!(
            mfx_address(0, params::tone_type::PCM_SYNTH, offset::TYPE),
            Address::new(0x19, 0x00, 0x02, 0x00)
        );
        assert_eq!(
            mfx_address(0, params::tone_type::PCM_SYNTH, offset::REVERB_SEND),
            Address::new(0x19, 0x00, 0x02, 0x03)
        );
    }

    #[test]
    fn mfx_address_sn_acoustic_part1() {
        // Must not land on SN-A Common `19 02 00 00` (the tone name).
        assert_eq!(
            mfx_address(0, params::tone_type::SN_ACOUSTIC, offset::TYPE),
            Address::new(0x19, 0x02, 0x02, 0x00)
        );
    }

    #[test]
    fn mfx_address_higher_part() {
        // Part 5 base = 1A 00 00 00.
        assert_eq!(
            mfx_address(4, params::tone_type::SN_ACOUSTIC, offset::CTRL_ASSIGN4),
            Address::new(0x1A, 0x02, 0x02, 0x10)
        );
        // Part 16 base = 1C 60 00 00.
        assert_eq!(
            mfx_address(15, params::tone_type::PCM_DRUM, offset::TYPE),
            Address::new(0x1C, 0x70, 0x02, 0x00)
        );
    }

    #[test]
    fn mfx_param_address_pcm_synth_part1() {
        assert_eq!(
            mfx_param_address(0, params::tone_type::PCM_SYNTH, 0),
            Address::new(0x19, 0x00, 0x02, 0x11)
        );
        // 0x11 + 9*4 = 0x35
        assert_eq!(
            mfx_param_address(0, params::tone_type::PCM_SYNTH, 9),
            Address::new(0x19, 0x00, 0x02, 0x35)
        );
    }

    #[test]
    fn mfx_param_address_sn_acoustic() {
        assert_eq!(
            mfx_param_address(0, params::tone_type::SN_ACOUSTIC, 0),
            Address::new(0x19, 0x02, 0x02, 0x11)
        );
        // Last param: 0x11 + 31*4 = 0x8D -> carries into byte 2: 02 03 0D.
        assert_eq!(
            mfx_param_address(1, params::tone_type::SN_ACOUSTIC, 31),
            Address::new(0x19, 0x22, 0x03, 0x0D)
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
