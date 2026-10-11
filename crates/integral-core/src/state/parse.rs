//! Parsing functions for SysEx dump data into state structs.

use super::{EqState, PartState};

/// Part mixer dump size (0x2A = 42 bytes, through Output Assign at 0x29).
pub const PART_DUMP_SIZE: usize = 0x2A;

/// Part EQ dump size (8 bytes: switch + 7 band params).
pub const PART_EQ_DUMP_SIZE: usize = 8;

/// Master EQ dump size (7 bytes: no switch, just 7 band params).
pub const MASTER_EQ_DUMP_SIZE: usize = 7;

/// Parse a 0x29-byte part mixer dump into a [`PartState`].
///
/// The dump layout matches the Studio Set Part block starting at offset
/// 0x00 (receive channel) through 0x28 (reverb send).
pub fn parse_part_dump(data: &[u8]) -> PartState {
    let mut part = PartState::default();
    if data.len() < PART_DUMP_SIZE {
        return part;
    }
    part.receive_channel = data[0x00];
    part.tone_bank_msb = data[0x06];
    part.tone_bank_lsb = data[0x07];
    part.tone_pc = data[0x08];
    part.level = data[0x09];
    part.pan = data[0x0a];
    part.muted = data[0x25] == 1;
    part.chorus_send = data[0x27];
    part.reverb_send = data[0x28];
    part.output_assign = data[0x29];
    part
}

/// Parse an 8-byte part EQ dump into an [`EqState`].
///
/// Byte 0 is the EQ switch (0=off, 1=on), bytes 1–7 are the band params.
pub fn parse_part_eq_dump(data: &[u8]) -> EqState {
    let mut eq = EqState::default();
    if data.len() < PART_EQ_DUMP_SIZE {
        return eq;
    }
    eq.enabled = data[0] == 1;
    eq.low_freq = data[1];
    eq.low_gain = data[2];
    eq.mid_freq = data[3];
    eq.mid_gain = data[4];
    eq.mid_q = data[5];
    eq.high_freq = data[6];
    eq.high_gain = data[7];
    eq
}

/// Parse a 7-byte master EQ dump (no switch byte) into an [`EqState`].
///
/// The `enabled` field is left at its default (`true`); the caller should
/// set it from a separate switch read.
pub fn parse_master_eq_dump(data: &[u8]) -> EqState {
    let mut eq = EqState::default();
    if data.len() < MASTER_EQ_DUMP_SIZE {
        return eq;
    }
    eq.low_freq = data[0];
    eq.low_gain = data[1];
    eq.mid_freq = data[2];
    eq.mid_gain = data[3];
    eq.mid_q = data[4];
    eq.high_freq = data[5];
    eq.high_gain = data[6];
    eq
}

// ---------------------------------------------------------------------------
// Motional Surround parsing
// ---------------------------------------------------------------------------

/// Parse the 13-byte Motional Surround common dump.
pub fn parse_surround_common(data: &[u8]) -> super::SurroundState {
    use crate::block_field::BlockField;
    use crate::params::SurroundField as F;
    let mut s = super::SurroundState::default();
    if data.len() < crate::params::SURROUND_COMMON_SIZE.value() as usize {
        return s;
    }
    s.enabled = F::Switch.byte(data) == 1;
    s.room_type = F::RoomType.byte(data);
    s.ambience_level = F::AmbienceLevel.byte(data);
    s.room_size = F::RoomSize.byte(data);
    s.ambience_time = F::AmbienceTime.byte(data);
    s.ambience_density = F::AmbienceDensity.byte(data);
    s.ambience_hf_damp = F::AmbienceHfDamp.byte(data);
    s.ext.lr = F::ExtPartLr.byte(data);
    s.ext.fb = F::ExtPartFb.byte(data);
    s.ext.width = F::ExtPartWidth.byte(data);
    s.ext.ambience_send = F::ExtPartAmbienceSend.byte(data);
    s.ext_control_channel = F::ExtPartControlChannel.byte(data);
    s.depth = F::Depth.byte(data);
    s
}

/// Parse 4-byte per-part surround data (LR, FB, Width, AmbSend).
///
/// Note: the part block has LR at offset 0x44, FB at 0x46 (skipping 0x45),
/// Width at 0x48, AmbSend at 0x49. So if reading from offset 0x44, the data
/// at indices [0, 2, 4, 5] maps to [LR, FB, Width, AmbSend].
pub fn parse_surround_part(
    lr: u8,
    fb: u8,
    width: u8,
    ambience_send: u8,
) -> super::SurroundPartState {
    super::SurroundPartState {
        lr,
        fb,
        width,
        ambience_send,
    }
}

// ---------------------------------------------------------------------------
// Comp+EQ parsing
// ---------------------------------------------------------------------------

/// Bytes per Comp+EQ unit (6 comp + 8 eq = 14).
const COMP_EQ_UNIT_BYTES: usize = 14;

/// Parse a single 14-byte Comp+EQ unit.
pub fn parse_comp_eq_unit(data: &[u8]) -> super::CompEqUnit {
    let mut unit = super::CompEqUnit::default();
    if data.len() < COMP_EQ_UNIT_BYTES {
        return unit;
    }
    unit.comp_switch = data[0] == 1;
    unit.comp_attack = data[1];
    unit.comp_release = data[2];
    unit.comp_threshold = data[3];
    unit.comp_ratio = data[4];
    unit.comp_output_gain = data[5];
    unit.eq_switch = data[6] == 1;
    unit.eq_low_freq = data[7];
    unit.eq_low_gain = data[8];
    unit.eq_mid_freq = data[9];
    unit.eq_mid_gain = data[10];
    unit.eq_mid_q = data[11];
    unit.eq_high_freq = data[12];
    unit.eq_high_gain = data[13];
    unit
}

/// Parse the full 84-byte (0x54) Comp+EQ block into 6 units.
pub fn parse_comp_eq_block(data: &[u8]) -> [super::CompEqUnit; 6] {
    std::array::from_fn(|i| {
        let offset = i * COMP_EQ_UNIT_BYTES;
        if offset + COMP_EQ_UNIT_BYTES <= data.len() {
            parse_comp_eq_unit(&data[offset..])
        } else {
            super::CompEqUnit::default()
        }
    })
}

// ---------------------------------------------------------------------------
// Nibble codec
// ---------------------------------------------------------------------------

/// Decode nibblized FX parameters.
///
/// Each parameter is encoded as 4 consecutive 7-bit bytes (only the low
/// nibble of each byte is significant).  The resulting 16-bit unsigned
/// value is offset by 32768 to produce a signed display value.
///
/// Returns up to `count` decoded values.
pub fn decode_nib_params(data: &[u8], count: usize) -> Vec<i32> {
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let off = i * 4;
        if off + 3 >= data.len() {
            break;
        }
        let raw = (u32::from(data[off] & 0x0f) << 12)
            | (u32::from(data[off + 1] & 0x0f) << 8)
            | (u32::from(data[off + 2] & 0x0f) << 4)
            | u32::from(data[off + 3] & 0x0f);
        result.push(raw as i32 - 32768);
    }
    result
}

/// Encode a signed display value into 4 nibble bytes for a SysEx DT1.
///
/// This is the inverse of [`decode_nib_params`].
pub fn encode_nib_param(value: i32) -> [u8; 4] {
    let raw = (value + 32768) as u16;
    [
        ((raw >> 12) & 0x0f) as u8,
        ((raw >> 8) & 0x0f) as u8,
        ((raw >> 4) & 0x0f) as u8,
        (raw & 0x0f) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_surround_common_reads_every_field_at_its_position() {
        use crate::block_field::test_support::{assert_reads_at_positions, positional_block};
        use crate::params::SurroundField as F;
        let mut data = positional_block(crate::params::SURROUND_COMMON_SIZE.value() as usize);
        // The switch reads ON from a 1, not from its position (0).
        data[F::Switch as usize] = 1;
        let s = parse_surround_common(&data);
        assert!(s.enabled);
        assert_reads_at_positions!(s, {
            room_type: F::RoomType,
            ambience_level: F::AmbienceLevel,
            room_size: F::RoomSize,
            ambience_time: F::AmbienceTime,
            ambience_density: F::AmbienceDensity,
            ambience_hf_damp: F::AmbienceHfDamp,
            ext_control_channel: F::ExtPartControlChannel,
            depth: F::Depth,
        });
        assert_reads_at_positions!(s.ext, {
            lr: F::ExtPartLr,
            fb: F::ExtPartFb,
            width: F::ExtPartWidth,
            ambience_send: F::ExtPartAmbienceSend,
        });
    }

    #[test]
    fn parse_part_dump_extracts_fields() {
        let mut data = [0u8; PART_DUMP_SIZE];
        data[0x00] = 3; // receive_channel
        data[0x06] = 89; // tone_bank_msb
        data[0x07] = 64; // tone_bank_lsb
        data[0x08] = 12; // tone_pc
        data[0x09] = 100; // level
        data[0x0a] = 64; // pan
        data[0x25] = 1; // muted
        data[0x27] = 42; // chorus_send
        data[0x28] = 55; // reverb_send
        data[0x29] = 3; // output_assign (D)

        let part = parse_part_dump(&data);
        assert_eq!(part.receive_channel, 3);
        assert_eq!(part.tone_bank_msb, 89);
        assert_eq!(part.tone_bank_lsb, 64);
        assert_eq!(part.tone_pc, 12);
        assert_eq!(part.level, 100);
        assert_eq!(part.pan, 64);
        assert!(part.muted);
        assert_eq!(part.chorus_send, 42);
        assert_eq!(part.reverb_send, 55);
        assert_eq!(part.output_assign, 3);
    }

    #[test]
    fn parse_part_dump_short_data_returns_default() {
        let part = parse_part_dump(&[]);
        assert_eq!(part.level, 100); // default
    }

    #[test]
    fn parse_part_eq_dump_extracts_fields() {
        let data = [1, 0, 20, 8, 10, 2, 1, 25];
        let eq = parse_part_eq_dump(&data);
        assert!(eq.enabled);
        assert_eq!(eq.low_freq, 0);
        assert_eq!(eq.low_gain, 20);
        assert_eq!(eq.mid_freq, 8);
        assert_eq!(eq.mid_gain, 10);
        assert_eq!(eq.mid_q, 2);
        assert_eq!(eq.high_freq, 1);
        assert_eq!(eq.high_gain, 25);
    }

    #[test]
    fn parse_master_eq_dump_no_switch() {
        let data = [0, 15, 7, 15, 0, 1, 15];
        let eq = parse_master_eq_dump(&data);
        assert!(eq.enabled); // default, not from data
        assert_eq!(eq.low_freq, 0);
        assert_eq!(eq.low_gain, 15);
    }

    #[test]
    fn nibble_roundtrip_zero() {
        let encoded = encode_nib_param(0);
        let decoded = decode_nib_params(&encoded, 1);
        assert_eq!(decoded, vec![0]);
    }

    #[test]
    fn nibble_roundtrip_positive() {
        let encoded = encode_nib_param(500);
        let decoded = decode_nib_params(&encoded, 1);
        assert_eq!(decoded, vec![500]);
    }

    #[test]
    fn nibble_roundtrip_negative() {
        let encoded = encode_nib_param(-98);
        let decoded = decode_nib_params(&encoded, 1);
        assert_eq!(decoded, vec![-98]);
    }

    #[test]
    fn decode_multiple_params() {
        let mut data = Vec::new();
        data.extend_from_slice(&encode_nib_param(100));
        data.extend_from_slice(&encode_nib_param(-50));
        data.extend_from_slice(&encode_nib_param(0));

        let decoded = decode_nib_params(&data, 3);
        assert_eq!(decoded, vec![100, -50, 0]);
    }

    #[test]
    fn decode_truncated_data() {
        let data = encode_nib_param(42);
        // Ask for 5 but only have 1 param's worth of data.
        let decoded = decode_nib_params(&data, 5);
        assert_eq!(decoded, vec![42]);
    }
}
