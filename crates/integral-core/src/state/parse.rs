//! Parsing functions for SysEx dump data into state structs.

use super::{EqState, PartState};

/// Part mixer dump size (0x29 = 41 bytes).
pub const PART_DUMP_SIZE: usize = 0x29;

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
