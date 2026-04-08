//! Parameter definitions for the INTEGRA-7.
//!
//! Provides well-known parameter offsets and convenience functions for
//! computing absolute SysEx addresses for mixer-relevant parameters.
//!
//! Reference: `docs/midi/05-studio-set.md`, `docs/midi/04-address-map.md`

use crate::address::{Address, DataSize};

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
        assert_eq!(addr, Address::new(0x18, 0x20, 0x00, 0x09));
    }

    #[test]
    fn part1_pan_address() {
        let addr = part_address(0, part::PAN);
        assert_eq!(addr, Address::new(0x18, 0x20, 0x00, 0x0A));
    }

    #[test]
    fn part16_mute_address() {
        let addr = part_address(15, part::MUTE);
        assert_eq!(addr, Address::new(0x18, 0x2F, 0x00, 0x25));
    }

    #[test]
    fn part1_tone_bank_msb_address() {
        let addr = part_address(0, part::TONE_BANK_MSB);
        assert_eq!(addr, Address::new(0x18, 0x20, 0x00, 0x06));
    }

    #[test]
    fn part8_chorus_send_address() {
        // Part 8 = index 7
        let addr = part_address(7, part::CHORUS_SEND);
        assert_eq!(addr, Address::new(0x18, 0x27, 0x00, 0x27));
    }

    #[test]
    fn system_master_level() {
        assert_eq!(SYSTEM_MASTER_LEVEL, Address::new(0x02, 0x00, 0x00, 0x05));
    }
}
