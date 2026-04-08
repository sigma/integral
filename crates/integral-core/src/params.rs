//! Parameter definitions for the INTEGRA-7.
//!
//! Provides well-known parameter offsets and convenience functions for
//! computing absolute SysEx addresses for mixer-relevant parameters.
//!
//! Reference: `docs/midi/05-studio-set.md`, `docs/midi/04-address-map.md`

use crate::address::{Address, DataSize};

// ---------------------------------------------------------------------------
// Setup (base: 01 00 00 00)
// ---------------------------------------------------------------------------

/// Setup: Studio Set Bank Select MSB (CC#0). Value 85 for Studio Sets.
pub const SETUP_STUDIO_SET_BS_MSB: Address = Address::new(0x01, 0x00, 0x00, 0x04);

/// Setup: Studio Set Bank Select LSB (CC#32). Value 0 for Studio Sets.
pub const SETUP_STUDIO_SET_BS_LSB: Address = Address::new(0x01, 0x00, 0x00, 0x05);

/// Setup: Studio Set Program Change (0–63 for Studio Sets 1–64).
pub const SETUP_STUDIO_SET_PC: Address = Address::new(0x01, 0x00, 0x00, 0x06);

// ---------------------------------------------------------------------------
// System Common (base: 02 00 00 00)
// ---------------------------------------------------------------------------

/// System Common: Master Level (0–127).
pub const SYSTEM_MASTER_LEVEL: Address = Address::new(0x02, 0x00, 0x00, 0x05);

// ---------------------------------------------------------------------------
// Studio Set Common (base: 18 00 00 00)
// ---------------------------------------------------------------------------

/// Studio Set Common: Name start (16 ASCII bytes at 00 00 – 00 0F).
pub const STUDIO_SET_NAME: Address = Address::new(0x18, 0x00, 0x00, 0x00);

/// Size for reading the full Studio Set name (16 bytes).
pub const STUDIO_SET_NAME_SIZE: DataSize = DataSize::SIXTEEN;

// ---------------------------------------------------------------------------
// Studio Set Part offsets (within a Part block)
// ---------------------------------------------------------------------------

/// Part parameter offsets within a Studio Set Part block.
///
/// Use [`part_address`] to compute the absolute address for a given part.
pub mod part {
    /// Receive Channel (0–15 → ch 1–16).
    pub const RECEIVE_CHANNEL: [u8; 3] = [0x00, 0x00, 0x00];
    /// Receive Switch (0=OFF, 1=ON).
    pub const RECEIVE_SWITCH: [u8; 3] = [0x00, 0x00, 0x01];
    /// Tone Bank Select MSB / CC#0 (0–127).
    pub const TONE_BANK_MSB: [u8; 3] = [0x00, 0x00, 0x06];
    /// Tone Bank Select LSB / CC#32 (0–127).
    pub const TONE_BANK_LSB: [u8; 3] = [0x00, 0x00, 0x07];
    /// Tone Program Number / PC (0–127).
    pub const TONE_PC: [u8; 3] = [0x00, 0x00, 0x08];
    /// Part Level / CC#7 (0–127).
    pub const LEVEL: [u8; 3] = [0x00, 0x00, 0x09];
    /// Part Pan / CC#10 (0–127, 64=center).
    pub const PAN: [u8; 3] = [0x00, 0x00, 0x0A];
    /// Mute Switch (0=OFF, 1=MUTE).
    pub const MUTE: [u8; 3] = [0x00, 0x00, 0x25];
    /// Chorus Send Level / CC#93 (0–127).
    pub const CHORUS_SEND: [u8; 3] = [0x00, 0x00, 0x27];
    /// Reverb Send Level / CC#91 (0–127).
    pub const REVERB_SEND: [u8; 3] = [0x00, 0x00, 0x28];
}

/// Compute the absolute address for a Studio Set Part parameter.
///
/// `part_index` is 0-based (0 = Part 1, 15 = Part 16).
pub const fn part_address(part_index: u8, param_offset: [u8; 3]) -> Address {
    crate::address::studio_set_part(part_index, param_offset)
}

// ---------------------------------------------------------------------------
// Temporary Tone (tone name reading)
// ---------------------------------------------------------------------------

/// Temporary Tone base addresses per part.
///
/// Part 1 = `19 00 00 00`, Part 2 = `19 20 00 00`, ..., Part 16 = `1C 60 00 00`.
/// Parts are spaced `00 20 00 00` apart starting at `19 00 00 00`.
pub const fn temporary_tone_base(part_index: u8) -> Address {
    // Each part is offset by 00 20 00 00 in the address space.
    // part_index 0 → 19 00 00 00 (no offset)
    // part_index 1 → 19 20 00 00 (+ 00 20 00 00)
    // part_index 4 → 1A 00 00 00 (+ 01 00 00 00, via 7-bit carry: 4*0x20=0x80=overflow)
    //
    // We add part_index * 0x20 to byte 1, with 7-bit arithmetic handling carry.
    let total = (part_index as u16) * 0x20;
    let byte0_add = (total / 128) as u8;
    let byte1_add = (total % 128) as u8;
    Address::new(0x19, 0x00, 0x00, 0x00).offset([byte0_add, byte1_add, 0x00, 0x00])
}

/// Tone type offsets within a Temporary Tone block.
pub mod tone_type {
    /// PCM Synth Tone.
    pub const PCM_SYNTH: [u8; 3] = [0x00, 0x00, 0x00];
    /// SuperNATURAL Synth Tone.
    pub const SN_SYNTH: [u8; 3] = [0x01, 0x00, 0x00];
    /// SuperNATURAL Acoustic Tone.
    pub const SN_ACOUSTIC: [u8; 3] = [0x02, 0x00, 0x00];
    /// SuperNATURAL Drum Kit.
    pub const SN_DRUM: [u8; 3] = [0x03, 0x00, 0x00];
    /// PCM Drum Kit.
    pub const PCM_DRUM: [u8; 3] = [0x10, 0x00, 0x00];
}

/// Determine the tone type offset from the Bank Select MSB.
///
/// Returns the tone type offset to use within the Temporary Tone block,
/// or `None` if the MSB doesn't map to a known type.
pub fn tone_type_from_bank_msb(msb: u8) -> Option<[u8; 3]> {
    match msb {
        87 | 93 | 97 | 121 => Some(tone_type::PCM_SYNTH),
        95 => Some(tone_type::SN_SYNTH),
        89 => Some(tone_type::SN_ACOUSTIC),
        88 => Some(tone_type::SN_DRUM),
        86 | 92 | 96 | 120 => Some(tone_type::PCM_DRUM),
        _ => None,
    }
}

/// Compute the address of the tone name for a given part and tone type.
///
/// Tone names are 12 ASCII bytes at offset `00 00`–`00 0B` in the tone's Common block.
pub const fn tone_name_address(part_index: u8, tone_type_offset: [u8; 3]) -> Address {
    temporary_tone_base(part_index).offset([
        0x00,
        tone_type_offset[0],
        tone_type_offset[1],
        tone_type_offset[2],
    ])
}

/// Size for reading a tone name (12 bytes).
pub const TONE_NAME_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x0C);

/// Size for reading a single-byte parameter.
pub const SINGLE_BYTE_SIZE: DataSize = DataSize::ONE;

/// Size for reading the tone bank + program (3 consecutive bytes: MSB, LSB, PC).
pub const TONE_SELECTION_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x03);

/// Size for reading the core mixer parameters of a part in one request.
///
/// From offset 00 00 (Receive Channel) through 00 28 (Reverb Send) = 0x29 bytes.
/// This allows fetching all mixer-relevant parameters in a single RQ1.
pub const PART_MIXER_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x29);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::Address;

    #[test]
    fn part1_level_address() {
        let addr = part_address(0, part::LEVEL);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x20, 0x09));
    }

    #[test]
    fn part1_pan_address() {
        let addr = part_address(0, part::PAN);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x20, 0x0A));
    }

    #[test]
    fn part16_mute_address() {
        let addr = part_address(15, part::MUTE);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x2F, 0x25));
    }

    #[test]
    fn part1_tone_bank_msb_address() {
        let addr = part_address(0, part::TONE_BANK_MSB);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x20, 0x06));
    }

    #[test]
    fn part8_chorus_send_address() {
        // Part 8 = index 7
        let addr = part_address(7, part::CHORUS_SEND);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x27, 0x27));
    }

    #[test]
    fn system_master_level() {
        assert_eq!(SYSTEM_MASTER_LEVEL, Address::new(0x02, 0x00, 0x00, 0x05));
    }

    #[test]
    fn temporary_tone_part1() {
        assert_eq!(temporary_tone_base(0), Address::new(0x19, 0x00, 0x00, 0x00));
    }

    #[test]
    fn temporary_tone_part2() {
        assert_eq!(temporary_tone_base(1), Address::new(0x19, 0x20, 0x00, 0x00));
    }

    #[test]
    fn temporary_tone_part5() {
        // Part 5 = index 4, 4*0x20 = 0x80 → overflow byte 1, carry to byte 0
        // 19 00 + 00 80 → 19 00 + 01 00 (7-bit carry) = 1A 00
        assert_eq!(temporary_tone_base(4), Address::new(0x1A, 0x00, 0x00, 0x00));
    }

    #[test]
    fn temporary_tone_part16() {
        // Part 16 = index 15, 15*0x20 = 0x1E0 = 3*128 + 96 = carry 3, remainder 0x60
        assert_eq!(
            temporary_tone_base(15),
            Address::new(0x1C, 0x60, 0x00, 0x00)
        );
    }

    #[test]
    fn tone_name_pcm_synth_part1() {
        let addr = tone_name_address(0, tone_type::PCM_SYNTH);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x00, 0x00));
    }

    #[test]
    fn tone_name_sn_acoustic_part1() {
        // SN Acoustic offset is 02 00 00, so byte 1 of the tone block gets +2
        let addr = tone_name_address(0, tone_type::SN_ACOUSTIC);
        assert_eq!(addr, Address::new(0x19, 0x02, 0x00, 0x00));
    }

    #[test]
    fn tone_type_bank_msb_mapping() {
        assert_eq!(tone_type_from_bank_msb(89), Some(tone_type::SN_ACOUSTIC));
        assert_eq!(tone_type_from_bank_msb(95), Some(tone_type::SN_SYNTH));
        assert_eq!(tone_type_from_bank_msb(87), Some(tone_type::PCM_SYNTH));
        assert_eq!(tone_type_from_bank_msb(86), Some(tone_type::PCM_DRUM));
        assert_eq!(tone_type_from_bank_msb(88), Some(tone_type::SN_DRUM));
        assert_eq!(tone_type_from_bank_msb(0), None);
    }
}
