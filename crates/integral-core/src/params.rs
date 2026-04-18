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
// Studio Set Common: Ext Part parameters
// ---------------------------------------------------------------------------

/// Ext Part Level (0–127). Studio Set Common offset `00 4C`.
pub const EXT_PART_LEVEL: Address = Address::new(0x18, 0x00, 0x00, 0x4C);

/// Ext Part Chorus Send Level (0–127). Studio Set Common offset `00 4D`.
pub const EXT_PART_CHORUS_SEND: Address = Address::new(0x18, 0x00, 0x00, 0x4D);

/// Ext Part Reverb Send Level (0–127). Studio Set Common offset `00 4E`.
pub const EXT_PART_REVERB_SEND: Address = Address::new(0x18, 0x00, 0x00, 0x4E);

/// Ext Part Mute Switch (0=OFF, 1=ON). Studio Set Common offset `00 4F`.
pub const EXT_PART_MUTE: Address = Address::new(0x18, 0x00, 0x00, 0x4F);

// ---------------------------------------------------------------------------
// Studio Set Common: Chorus (offset 00 04 00)
// ---------------------------------------------------------------------------

/// Chorus block base address.
pub const CHORUS_BASE: Address = Address::new(0x18, 0x00, 0x04, 0x00);

/// Chorus Switch (in Studio Set Common at offset `00 41`).
pub const CHORUS_SWITCH: Address = Address::new(0x18, 0x00, 0x00, 0x41);

/// Chorus parameter offsets within the Chorus block.
pub mod chorus {
    /// Chorus Type (0=OFF, 1=Chorus, 2=Delay, 3=GM2 Chorus).
    pub const TYPE: u8 = 0x00;
    /// Chorus Level (0–127).
    pub const LEVEL: u8 = 0x01;
    /// Chorus Output Select (0=MAIN, 1=REV, 2=MAIN+REV).
    pub const OUTPUT_SELECT: u8 = 0x03;
    /// First nibblized parameter offset (Param 1 at 0x04, each is 4 bytes).
    pub const PARAM_BASE: u8 = 0x04;
}

/// Compute the absolute address for a Chorus parameter.
pub const fn chorus_address(param_offset: u8) -> Address {
    CHORUS_BASE.offset([0x00, 0x00, 0x00, param_offset])
}

/// Size for reading Chorus core params (type + level + reserved + output = 4 bytes).
pub const CHORUS_CORE_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x04);

// ---------------------------------------------------------------------------
// Studio Set Common: Reverb (offset 00 06 00)
// ---------------------------------------------------------------------------

/// Reverb block base address.
pub const REVERB_BASE: Address = Address::new(0x18, 0x00, 0x06, 0x00);

/// Reverb Switch (in Studio Set Common at offset `00 40`).
pub const REVERB_SWITCH: Address = Address::new(0x18, 0x00, 0x00, 0x40);

/// Reverb parameter offsets within the Reverb block.
pub mod reverb {
    /// Reverb Type (0=OFF, 1=Room1, 2=Room2, 3=Hall1, 4=Hall2, 5=Plate, 6=GM2 Reverb).
    pub const TYPE: u8 = 0x00;
    /// Reverb Level (0–127).
    pub const LEVEL: u8 = 0x01;
    /// Reverb Output Assign (0=A, 1=B, 2=C, 3=D).
    pub const OUTPUT_ASSIGN: u8 = 0x02;
    /// First nibblized parameter offset (Param 1 at 0x03, each is 4 bytes).
    pub const PARAM_BASE: u8 = 0x03;
}

/// Compute the absolute address for a Reverb parameter.
pub const fn reverb_address(param_offset: u8) -> Address {
    REVERB_BASE.offset([0x00, 0x00, 0x00, param_offset])
}

/// Size for reading Reverb core params (type + level + output = 3 bytes).
pub const REVERB_CORE_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x03);

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
    /// Output Assign (0=A, 1=B, 2=C, 3=D, 4–11=1–8).
    pub const OUTPUT_ASSIGN: [u8; 3] = [0x00, 0x00, 0x29];
}

/// Compute the absolute address for a Studio Set Part parameter.
///
/// `part_index` is 0-based (0 = Part 1, 15 = Part 16).
pub const fn part_address(part_index: u8, param_offset: [u8; 3]) -> Address {
    crate::address::studio_set_part(part_index, param_offset)
}

// ---------------------------------------------------------------------------
// Studio Set Part EQ (offset 00 50 00 to 00 5F 00, one per part)
// ---------------------------------------------------------------------------

/// Per-part EQ parameter offsets within the Part EQ block.
///
/// Part EQ *n* (0-indexed) is at Studio Set base + `[0x00, 0x00, 0x50 + n, 0x00]`.
/// Use [`part_eq_address`] to compute the absolute address.
pub mod part_eq {
    /// EQ Switch (0=OFF, 1=ON).
    pub const SWITCH: u8 = 0x00;
    /// Low Freq (0=200Hz, 1=400Hz).
    pub const LOW_FREQ: u8 = 0x01;
    /// Low Gain (0–30, display: -15 to +15 dB).
    pub const LOW_GAIN: u8 = 0x02;
    /// Mid Freq (0–16, 17 values from 200Hz to 8000Hz).
    pub const MID_FREQ: u8 = 0x03;
    /// Mid Gain (0–30, display: -15 to +15 dB).
    pub const MID_GAIN: u8 = 0x04;
    /// Mid Q (0–4, display: 0.5, 1.0, 2.0, 4.0, 8.0).
    pub const MID_Q: u8 = 0x05;
    /// High Freq (0=2000Hz, 1=4000Hz, 2=8000Hz).
    pub const HIGH_FREQ: u8 = 0x06;
    /// High Gain (0–30, display: -15 to +15 dB).
    pub const HIGH_GAIN: u8 = 0x07;
}

/// Compute the absolute address for a Part EQ parameter.
///
/// `part_index` is 0-based (0 = Part 1, 15 = Part 16).
/// `param_offset` is one of the `part_eq::*` constants.
pub const fn part_eq_address(part_index: u8, param_offset: u8) -> Address {
    crate::address::STUDIO_SET.offset([0x00, 0x00, 0x50 + part_index, param_offset])
}

/// Size for reading all Part EQ parameters in one RQ1 (8 bytes).
pub const PART_EQ_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x08);

// ---------------------------------------------------------------------------
// Studio Set Master EQ (offset 00 09 00)
// ---------------------------------------------------------------------------

/// Master EQ parameter offsets.
///
/// Base address: Studio Set + `[0x00, 0x00, 0x09, 0x00]` = `18 00 09 00`.
pub mod master_eq {
    /// Low Freq (0=200Hz, 1=400Hz).
    pub const LOW_FREQ: u8 = 0x00;
    /// Low Gain (0–30, display: -15 to +15 dB).
    pub const LOW_GAIN: u8 = 0x01;
    /// Mid Freq (0–16, 17 values from 200Hz to 8000Hz).
    pub const MID_FREQ: u8 = 0x02;
    /// Mid Gain (0–30, display: -15 to +15 dB).
    pub const MID_GAIN: u8 = 0x03;
    /// Mid Q (0–4, display: 0.5, 1.0, 2.0, 4.0, 8.0).
    pub const MID_Q: u8 = 0x04;
    /// High Freq (0=2000Hz, 1=4000Hz, 2=8000Hz).
    pub const HIGH_FREQ: u8 = 0x05;
    /// High Gain (0–30, display: -15 to +15 dB).
    pub const HIGH_GAIN: u8 = 0x06;
}

/// Compute the absolute address for a Master EQ parameter.
pub const fn master_eq_address(param_offset: u8) -> Address {
    crate::address::STUDIO_SET.offset([0x00, 0x00, 0x09, param_offset])
}

/// Size for reading all Master EQ parameters in one RQ1 (7 bytes).
pub const MASTER_EQ_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x07);

/// Master EQ Switch — in the Studio Set Common block at offset `00 42`.
pub const MASTER_EQ_SWITCH: Address = Address::new(0x18, 0x00, 0x00, 0x42);

/// Solo Part — Studio Set Common offset `00 3F`. Value 0=OFF, 1–16=Part 1–16.
pub const SOLO_PART: Address = Address::new(0x18, 0x00, 0x00, 0x3F);

// ---------------------------------------------------------------------------
// Drum Comp/EQ (Studio Set Common + Temporary Tone)
// ---------------------------------------------------------------------------

/// Drum Comp/EQ Switch — Studio Set Common offset `00 43`.
pub const DRUM_COMP_EQ_SWITCH: Address = Address::new(0x18, 0x00, 0x00, 0x43);

/// Drum Comp/EQ Part assignment — Studio Set Common offset `00 44` (0–15 → Part 1–16).
pub const DRUM_COMP_EQ_PART: Address = Address::new(0x18, 0x00, 0x00, 0x44);

/// Drum Comp/EQ Output Assign for unit N (0-based) — Studio Set Common offsets `00 45`–`00 4A`.
pub const fn drum_comp_eq_output_assign(unit: u8) -> Address {
    Address::new(0x18, 0x00, 0x00, 0x45 + unit)
}

/// Number of Comp+EQ units.
pub const COMP_EQ_UNIT_COUNT: usize = 6;

/// Bytes per Comp+EQ unit in the tone block (comp 6 + eq 8 = 14).
pub const COMP_EQ_UNIT_SIZE: u8 = 0x0E;

/// Total size of the Comp+EQ block (6 units × 14 bytes = 84 = 0x54).
pub const COMP_EQ_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x54);

/// Per-unit compressor parameter offsets (within the 14-byte unit).
pub mod comp {
    /// Compressor Switch (0=OFF, 1=ON).
    pub const SWITCH: u8 = 0x00;
    /// Attack Time (0–31).
    pub const ATTACK: u8 = 0x01;
    /// Release Time (0–23).
    pub const RELEASE: u8 = 0x02;
    /// Threshold (0–127).
    pub const THRESHOLD: u8 = 0x03;
    /// Ratio (0–19: 1:1, 2:1 ... 100:1, inf:1).
    pub const RATIO: u8 = 0x04;
    /// Output Gain (0–24: 0 to +24 dB).
    pub const OUTPUT_GAIN: u8 = 0x05;
}

/// Per-unit EQ parameter offsets (within the 14-byte unit, starting at 0x06).
pub mod comp_eq {
    /// EQ Switch (0=OFF, 1=ON).
    pub const EQ_SWITCH: u8 = 0x06;
    /// EQ Low Freq (0=200Hz, 1=400Hz).
    pub const EQ_LOW_FREQ: u8 = 0x07;
    /// EQ Low Gain (0–30, display: -15 to +15 dB).
    pub const EQ_LOW_GAIN: u8 = 0x08;
    /// EQ Mid Freq (0–16).
    pub const EQ_MID_FREQ: u8 = 0x09;
    /// EQ Mid Gain (0–30).
    pub const EQ_MID_GAIN: u8 = 0x0A;
    /// EQ Mid Q (0–4).
    pub const EQ_MID_Q: u8 = 0x0B;
    /// EQ High Freq (0–2).
    pub const EQ_HIGH_FREQ: u8 = 0x0C;
    /// EQ High Gain (0–30).
    pub const EQ_HIGH_GAIN: u8 = 0x0D;
}

/// Compute the absolute address of the Comp+EQ block for a part's tone.
///
/// The block starts at offset `00 08 00` within the tone type block.
/// `tone_type_offset` should be [`tone_type::PCM_DRUM`] or [`tone_type::SN_DRUM`].
pub const fn comp_eq_block_address(part_index: u8, tone_type_offset: [u8; 3]) -> Address {
    temporary_tone_base(part_index)
        .offset([0x00, tone_type_offset[0], tone_type_offset[1], tone_type_offset[2]])
        .offset([0x00, 0x00, 0x08, 0x00])
}

/// Compute the absolute address for a specific parameter within a
/// Comp+EQ unit.
///
/// `unit` is 0-based (0–5), `param_offset` is a `comp::` or `comp_eq::` constant.
/// `tone_type_offset` should be [`tone_type::PCM_DRUM`] or [`tone_type::SN_DRUM`].
pub const fn comp_eq_param_address(
    part_index: u8,
    tone_type_offset: [u8; 3],
    unit: u8,
    param_offset: u8,
) -> Address {
    let unit_byte_offset = unit * COMP_EQ_UNIT_SIZE + param_offset;
    comp_eq_block_address(part_index, tone_type_offset)
        .offset([0x00, 0x00, 0x00, unit_byte_offset])
}

// ---------------------------------------------------------------------------
// Motional Surround (Studio Set Common offset 00 08 00)
// ---------------------------------------------------------------------------

/// Motional Surround Common base address.
pub const SURROUND_BASE: Address = Address::new(0x18, 0x00, 0x08, 0x00);

/// Motional Surround common parameter offsets.
pub mod surround {
    /// Switch (0=OFF, 1=ON).
    pub const SWITCH: u8 = 0x00;
    /// Room Type (0–3: ROOM1, ROOM2, HALL1, HALL2).
    pub const ROOM_TYPE: u8 = 0x01;
    /// Ambience Level (0–127).
    pub const AMBIENCE_LEVEL: u8 = 0x02;
    /// Room Size (0–2: SMALL, MEDIUM, LARGE).
    pub const ROOM_SIZE: u8 = 0x03;
    /// Ambience Time (0–100).
    pub const AMBIENCE_TIME: u8 = 0x04;
    /// Ambience Density (0–100).
    pub const AMBIENCE_DENSITY: u8 = 0x05;
    /// Ambience HF Damp (0–100).
    pub const AMBIENCE_HF_DAMP: u8 = 0x06;
    /// Ext Part L-R (0–127, display: -64 to +63).
    pub const EXT_LR: u8 = 0x07;
    /// Ext Part F-B (0–127, display: -64 to +63).
    pub const EXT_FB: u8 = 0x08;
    /// Ext Part Width (0–32).
    pub const EXT_WIDTH: u8 = 0x09;
    /// Ext Part Ambience Send Level (0–127).
    pub const EXT_AMBIENCE_SEND: u8 = 0x0A;
    /// Ext Part Control Channel (0–16: 1–16, OFF).
    pub const EXT_CONTROL_CHANNEL: u8 = 0x0B;
    /// Motional Surround Depth (0–100).
    pub const DEPTH: u8 = 0x0C;
}

/// Compute absolute address for a Motional Surround common parameter.
pub const fn surround_address(param_offset: u8) -> Address {
    SURROUND_BASE.offset([0x00, 0x00, 0x00, param_offset])
}

/// Size for reading all Motional Surround common parameters (13 bytes).
pub const SURROUND_COMMON_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x0D);

/// Per-part Motional Surround parameter offsets (within the Part block).
pub mod part_surround {
    /// L-R position (0–127, display: -64 to +63).
    pub const LR: [u8; 3] = [0x00, 0x00, 0x44];
    /// F-B position (0–127, display: -64 to +63).
    pub const FB: [u8; 3] = [0x00, 0x00, 0x46];
    /// Width (0–32).
    pub const WIDTH: [u8; 3] = [0x00, 0x00, 0x48];
    /// Ambience Send Level (0–127).
    pub const AMBIENCE_SEND: [u8; 3] = [0x00, 0x00, 0x49];
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
/// From offset 00 00 (Receive Channel) through 00 29 (Output Assign) = 0x2A bytes.
/// This allows fetching all mixer-relevant parameters in a single RQ1.
pub const PART_MIXER_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x2A);

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
    fn chorus_addresses() {
        assert_eq!(
            chorus_address(chorus::TYPE),
            Address::new(0x18, 0x00, 0x04, 0x00)
        );
        assert_eq!(
            chorus_address(chorus::LEVEL),
            Address::new(0x18, 0x00, 0x04, 0x01)
        );
        assert_eq!(
            chorus_address(chorus::OUTPUT_SELECT),
            Address::new(0x18, 0x00, 0x04, 0x03)
        );
        assert_eq!(CHORUS_SWITCH, Address::new(0x18, 0x00, 0x00, 0x41));
    }

    #[test]
    fn reverb_addresses() {
        assert_eq!(
            reverb_address(reverb::TYPE),
            Address::new(0x18, 0x00, 0x06, 0x00)
        );
        assert_eq!(
            reverb_address(reverb::LEVEL),
            Address::new(0x18, 0x00, 0x06, 0x01)
        );
        assert_eq!(
            reverb_address(reverb::OUTPUT_ASSIGN),
            Address::new(0x18, 0x00, 0x06, 0x02)
        );
        assert_eq!(REVERB_SWITCH, Address::new(0x18, 0x00, 0x00, 0x40));
    }

    #[test]
    fn ext_part_addresses() {
        assert_eq!(EXT_PART_LEVEL, Address::new(0x18, 0x00, 0x00, 0x4C));
        assert_eq!(EXT_PART_MUTE, Address::new(0x18, 0x00, 0x00, 0x4F));
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

    #[test]
    fn part1_eq_switch_address() {
        let addr = part_eq_address(0, part_eq::SWITCH);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x50, 0x00));
    }

    #[test]
    fn part1_eq_low_gain_address() {
        let addr = part_eq_address(0, part_eq::LOW_GAIN);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x50, 0x02));
    }

    #[test]
    fn part16_eq_high_gain_address() {
        let addr = part_eq_address(15, part_eq::HIGH_GAIN);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x5F, 0x07));
    }

    #[test]
    fn master_eq_low_freq_address() {
        let addr = master_eq_address(master_eq::LOW_FREQ);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x09, 0x00));
    }

    #[test]
    fn master_eq_high_gain_address() {
        let addr = master_eq_address(master_eq::HIGH_GAIN);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x09, 0x06));
    }

    #[test]
    fn master_eq_switch_address() {
        assert_eq!(MASTER_EQ_SWITCH, Address::new(0x18, 0x00, 0x00, 0x42));
    }
}
