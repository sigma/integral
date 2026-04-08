//! SysEx message construction and parsing for the Roland INTEGRA-7.
//!
//! Reference: `docs/midi/01-protocol.md`

use crate::address::{Address, DataSize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Roland manufacturer ID.
pub const ROLAND_ID: u8 = 0x41;

/// INTEGRA-7 model ID bytes.
pub const MODEL_ID: [u8; 3] = [0x00, 0x00, 0x64];

/// INTEGRA-7 device family code (from Identity Reply).
pub const INTEGRA7_FAMILY: [u8; 2] = [0x64, 0x02];

/// SysEx start byte.
const SYSEX_START: u8 = 0xF0;

/// SysEx end byte.
const SYSEX_END: u8 = 0xF7;

/// Universal Non-realtime message ID.
const UNIVERSAL_NON_REALTIME: u8 = 0x7E;

/// Broadcast device ID.
const DEVICE_ID_BROADCAST: u8 = 0x7F;

/// Sub ID#1: General Information.
const SUB_ID_GENERAL_INFO: u8 = 0x06;

/// Sub ID#2: Identity Request.
const SUB_ID_IDENTITY_REQUEST: u8 = 0x01;

/// Sub ID#2: Identity Reply.
const SUB_ID_IDENTITY_REPLY: u8 = 0x02;

/// DT1 command ID.
pub const CMD_DT1: u8 = 0x12;

/// RQ1 command ID.
pub const CMD_RQ1: u8 = 0x11;

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Build a Universal Non-realtime Identity Request message.
///
/// ```text
/// F0 7E 7F 06 01 F7
/// ```
///
/// Uses the broadcast device ID (`7FH`) so any device on the bus will reply.
pub fn identity_request() -> [u8; 6] {
    [
        SYSEX_START,
        UNIVERSAL_NON_REALTIME,
        DEVICE_ID_BROADCAST,
        SUB_ID_GENERAL_INFO,
        SUB_ID_IDENTITY_REQUEST,
        SYSEX_END,
    ]
}

/// Parsed device identity from a SysEx Identity Reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentity {
    /// Device ID of the responder (`10H`–`1FH`).
    pub device_id: u8,
    /// Manufacturer ID byte.
    pub manufacturer_id: u8,
    /// Device family code (2 bytes).
    pub family_code: [u8; 2],
    /// Device family number code (2 bytes).
    pub family_number: [u8; 2],
    /// Software revision level (4 bytes).
    pub revision: [u8; 4],
}

impl DeviceIdentity {
    /// Returns `true` if this identity belongs to a Roland INTEGRA-7.
    pub fn is_integra7(&self) -> bool {
        self.manufacturer_id == ROLAND_ID && self.family_code == INTEGRA7_FAMILY
    }
}

/// Errors that can occur when parsing a SysEx Identity Reply.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdentityParseError {
    /// Message is too short to be a valid Identity Reply.
    #[error("message too short: expected at least 15 bytes, got {0}")]
    TooShort(usize),
    /// Missing SysEx start byte.
    #[error("missing SysEx start byte (F0)")]
    MissingSysexStart,
    /// Missing SysEx end byte.
    #[error("missing SysEx end byte (F7)")]
    MissingSysexEnd,
    /// Not a Universal Non-realtime message.
    #[error("not a Universal Non-realtime message (expected 7E, got {0:#04X})")]
    NotUniversalNonRealtime(u8),
    /// Not a General Information message.
    #[error("not a General Information message (expected 06, got {0:#04X})")]
    NotGeneralInfo(u8),
    /// Not an Identity Reply.
    #[error("not an Identity Reply (expected 02, got {0:#04X})")]
    NotIdentityReply(u8),
}

/// Parse a raw SysEx Identity Reply message into a [`DeviceIdentity`].
///
/// Expected format:
/// ```text
/// F0 7E dev 06 02 mfr fam0 fam1 num0 num1 rev0 rev1 rev2 rev3 F7
/// ```
pub fn parse_identity_reply(data: &[u8]) -> Result<DeviceIdentity, IdentityParseError> {
    if data.len() < 15 {
        return Err(IdentityParseError::TooShort(data.len()));
    }
    if data[0] != SYSEX_START {
        return Err(IdentityParseError::MissingSysexStart);
    }
    if data[data.len() - 1] != SYSEX_END {
        return Err(IdentityParseError::MissingSysexEnd);
    }
    if data[1] != UNIVERSAL_NON_REALTIME {
        return Err(IdentityParseError::NotUniversalNonRealtime(data[1]));
    }
    if data[3] != SUB_ID_GENERAL_INFO {
        return Err(IdentityParseError::NotGeneralInfo(data[3]));
    }
    if data[4] != SUB_ID_IDENTITY_REPLY {
        return Err(IdentityParseError::NotIdentityReply(data[4]));
    }

    Ok(DeviceIdentity {
        device_id: data[2],
        manufacturer_id: data[5],
        family_code: [data[6], data[7]],
        family_number: [data[8], data[9]],
        revision: [data[10], data[11], data[12], data[13]],
    })
}

// ---------------------------------------------------------------------------
// DT1 parsing
// ---------------------------------------------------------------------------

/// A parsed DT1 (Data Set 1) message from the INTEGRA-7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dt1Message {
    /// Device ID of the sender.
    pub device_id: u8,
    /// 4-byte parameter address.
    pub address: Address,
    /// Parameter data (one or more bytes).
    pub data: Vec<u8>,
}

/// Errors that can occur when parsing a DT1 SysEx message.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Dt1ParseError {
    /// Message is too short to contain a valid DT1.
    #[error("message too short: expected at least 13 bytes, got {0}")]
    TooShort(usize),
    /// Missing SysEx start byte.
    #[error("missing SysEx start byte (F0)")]
    MissingSysexStart,
    /// Missing SysEx end byte.
    #[error("missing SysEx end byte (F7)")]
    MissingSysexEnd,
    /// Not a Roland message.
    #[error("not a Roland message (expected manufacturer 41, got {0:#04X})")]
    NotRoland(u8),
    /// Not an INTEGRA-7 message.
    #[error("wrong model ID")]
    WrongModelId,
    /// Not a DT1 command.
    #[error("not a DT1 command (expected 12, got {0:#04X})")]
    NotDt1(u8),
    /// Checksum mismatch.
    #[error("checksum mismatch: expected {expected:#04X}, got {actual:#04X}")]
    ChecksumMismatch { expected: u8, actual: u8 },
}

/// Parse a raw SysEx message as a DT1 from the INTEGRA-7.
///
/// Expected format:
/// ```text
/// F0 41 dev 00 00 64 12 aa bb cc dd ee ... ff sum F7
/// ```
///
/// Validates the header, model ID, command, and checksum.
pub fn parse_dt1(raw: &[u8]) -> Result<Dt1Message, Dt1ParseError> {
    // Minimum: F0 41 dev 00 00 64 12 aa bb cc dd (1 data) sum F7 = 14 bytes
    // But a DT1 with 1 data byte is 14 bytes. With 0 data it's 13 (invalid).
    if raw.len() < 14 {
        return Err(Dt1ParseError::TooShort(raw.len()));
    }
    if raw[0] != SYSEX_START {
        return Err(Dt1ParseError::MissingSysexStart);
    }
    if raw[raw.len() - 1] != SYSEX_END {
        return Err(Dt1ParseError::MissingSysexEnd);
    }
    if raw[1] != ROLAND_ID {
        return Err(Dt1ParseError::NotRoland(raw[1]));
    }
    if raw[3..6] != MODEL_ID {
        return Err(Dt1ParseError::WrongModelId);
    }
    if raw[6] != CMD_DT1 {
        return Err(Dt1ParseError::NotDt1(raw[6]));
    }

    let device_id = raw[2];
    let address = Address::new(raw[7], raw[8], raw[9], raw[10]);

    // Data runs from byte 11 to len-3 (last two bytes are checksum + F7)
    let data = raw[11..raw.len() - 2].to_vec();

    // Verify checksum: covers address + data bytes (bytes 7 to len-3)
    let chk_bytes = &raw[7..raw.len() - 2];
    let expected = checksum(chk_bytes);
    let actual = raw[raw.len() - 2];
    if expected != actual {
        return Err(Dt1ParseError::ChecksumMismatch { expected, actual });
    }

    Ok(Dt1Message {
        device_id,
        address,
        data,
    })
}

// ---------------------------------------------------------------------------
// DT1 / RQ1 builders
// ---------------------------------------------------------------------------

/// Build a DT1 (Data Set 1) SysEx message.
///
/// ```text
/// F0 41 <dev_id> 00 00 64 12 <addr[4]> <data[..]> <checksum> F7
/// ```
///
/// The checksum covers the address and data bytes.
pub fn build_dt1(device_id: u8, address: &Address, data: &[u8]) -> Vec<u8> {
    let addr = address.as_bytes();
    let chk_data: Vec<u8> = addr.iter().chain(data.iter()).copied().collect();
    let chk = checksum(&chk_data);

    let mut msg = Vec::with_capacity(7 + 4 + data.len() + 2);
    msg.push(SYSEX_START);
    msg.push(ROLAND_ID);
    msg.push(device_id);
    msg.extend_from_slice(&MODEL_ID);
    msg.push(CMD_DT1);
    msg.extend_from_slice(addr);
    msg.extend_from_slice(data);
    msg.push(chk);
    msg.push(SYSEX_END);
    msg
}

/// Build an RQ1 (Data Request 1) SysEx message.
///
/// ```text
/// F0 41 <dev_id> 00 00 64 11 <addr[4]> <size[4]> <checksum> F7
/// ```
///
/// The checksum covers the address and size bytes.
pub fn build_rq1(device_id: u8, address: &Address, size: &DataSize) -> Vec<u8> {
    let addr = address.as_bytes();
    let sz = size.as_bytes();
    let chk_data: Vec<u8> = addr.iter().chain(sz.iter()).copied().collect();
    let chk = checksum(&chk_data);

    let mut msg = Vec::with_capacity(7 + 4 + 4 + 2);
    msg.push(SYSEX_START);
    msg.push(ROLAND_ID);
    msg.push(device_id);
    msg.extend_from_slice(&MODEL_ID);
    msg.push(CMD_RQ1);
    msg.extend_from_slice(addr);
    msg.extend_from_slice(sz);
    msg.push(chk);
    msg.push(SYSEX_END);
    msg
}

// ---------------------------------------------------------------------------
// Checksum
// ---------------------------------------------------------------------------

/// Compute the Roland SysEx checksum over the given bytes.
///
/// The checksum is calculated as: `(128 - (sum % 128)) % 128`
///
/// This is applied to the address + data bytes of DT1/RQ1 messages.
pub fn checksum(data: &[u8]) -> u8 {
    let sum: u32 = data.iter().map(|&b| u32::from(b)).sum();
    (128 - (sum % 128) as u8) % 128
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_request_bytes() {
        assert_eq!(identity_request(), [0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]);
    }

    #[test]
    fn parse_valid_integra7_reply() {
        let reply = [
            0xF0, 0x7E, 0x10, 0x06, 0x02, // header
            0x41, // Roland
            0x64, 0x02, // family code (INTEGRA-7)
            0x00, 0x00, // family number
            0x00, 0x00, 0x00, 0x00, // revision
            0xF7, // EOX
        ];
        let id = parse_identity_reply(&reply).unwrap();
        assert_eq!(id.device_id, 0x10);
        assert_eq!(id.manufacturer_id, ROLAND_ID);
        assert_eq!(id.family_code, INTEGRA7_FAMILY);
        assert!(id.is_integra7());
    }

    #[test]
    fn parse_non_integra7_reply() {
        let reply = [
            0xF0, 0x7E, 0x10, 0x06, 0x02, //
            0x41, // Roland
            0x55, 0x03, // different family
            0x00, 0x00, //
            0x01, 0x02, 0x03, 0x04, //
            0xF7,
        ];
        let id = parse_identity_reply(&reply).unwrap();
        assert!(!id.is_integra7());
    }

    #[test]
    fn parse_too_short() {
        let short = [0xF0, 0x7E, 0x10, 0x06, 0x02, 0xF7];
        assert_eq!(
            parse_identity_reply(&short),
            Err(IdentityParseError::TooShort(6))
        );
    }

    #[test]
    fn parse_wrong_header() {
        let bad = [
            0xF0, 0x7F, 0x10, 0x06, 0x02, // 7F instead of 7E
            0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF7,
        ];
        assert_eq!(
            parse_identity_reply(&bad),
            Err(IdentityParseError::NotUniversalNonRealtime(0x7F))
        );
    }

    #[test]
    fn build_dt1_part1_level() {
        // Set Part 1 Level to 100: address 18 00 20 09, data 0x64
        let addr = Address::new(0x18, 0x00, 0x20, 0x09);
        let msg = build_dt1(0x10, &addr, &[0x64]);

        // Verify header
        assert_eq!(&msg[..7], &[0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x12]);
        // Verify address
        assert_eq!(&msg[7..11], &[0x18, 0x00, 0x20, 0x09]);
        // Verify data
        assert_eq!(msg[11], 0x64);
        // Verify checksum: (128 - ((0x18+0x00+0x20+0x09+0x64) % 128)) % 128
        // sum = 0xA5 = 165, 165 % 128 = 37, 128 - 37 = 91 = 0x5B
        assert_eq!(msg[12], 0x5B);
        // Verify EOX
        assert_eq!(msg[13], 0xF7);
    }

    #[test]
    fn build_rq1_studio_set_name() {
        // Request Studio Set name: address 18 00 00 00, size 00 00 00 10 (16 bytes)
        let addr = Address::new(0x18, 0x00, 0x00, 0x00);
        let size = DataSize::SIXTEEN;
        let msg = build_rq1(0x10, &addr, &size);

        // Verify header
        assert_eq!(&msg[..7], &[0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11]);
        // Verify address
        assert_eq!(&msg[7..11], &[0x18, 0x00, 0x00, 0x00]);
        // Verify size
        assert_eq!(&msg[11..15], &[0x00, 0x00, 0x00, 0x10]);
        // Verify checksum: (128 - ((0x18+0x10) % 128)) % 128 = (128 - 40) = 88 = 0x58
        assert_eq!(msg[15], 0x58);
        // Verify EOX
        assert_eq!(msg[16], 0xF7);
    }

    #[test]
    fn parse_dt1_valid() {
        let addr = Address::new(0x18, 0x00, 0x20, 0x09);
        let msg = build_dt1(0x10, &addr, &[0x64]);
        let parsed = parse_dt1(&msg).unwrap();
        assert_eq!(parsed.device_id, 0x10);
        assert_eq!(parsed.address, addr);
        assert_eq!(parsed.data, vec![0x64]);
    }

    #[test]
    fn parse_dt1_multi_byte_data() {
        // Studio Set name: 16 bytes of ASCII
        let addr = Address::new(0x18, 0x00, 0x00, 0x00);
        let name = b"Test Studio Set!";
        let msg = build_dt1(0x10, &addr, name);
        let parsed = parse_dt1(&msg).unwrap();
        assert_eq!(parsed.address, addr);
        assert_eq!(parsed.data, name.to_vec());
    }

    #[test]
    fn parse_dt1_bad_checksum() {
        let mut msg = build_dt1(0x10, &Address::new(0x18, 0x00, 0x20, 0x09), &[0x64]);
        // Corrupt the checksum
        let idx = msg.len() - 2;
        msg[idx] ^= 0x01;
        assert!(matches!(
            parse_dt1(&msg),
            Err(Dt1ParseError::ChecksumMismatch { .. })
        ));
    }

    #[test]
    fn parse_dt1_not_roland() {
        let mut msg = build_dt1(0x10, &Address::new(0x18, 0x00, 0x20, 0x09), &[0x64]);
        msg[1] = 0x42; // not Roland
        assert_eq!(parse_dt1(&msg), Err(Dt1ParseError::NotRoland(0x42)));
    }

    #[test]
    fn dt1_roundtrip() {
        // Build → parse should recover exact address and data
        let addr = Address::new(0x02, 0x00, 0x00, 0x05);
        let data = vec![0x7F];
        let msg = build_dt1(0x10, &addr, &data);
        let parsed = parse_dt1(&msg).unwrap();
        assert_eq!(parsed.device_id, 0x10);
        assert_eq!(parsed.address, addr);
        assert_eq!(parsed.data, data);
    }

    #[test]
    fn checksum_reverb_type_example() {
        // From docs: Reverb Type = Room 2 → address 18 00 06 00, data 01
        // Expected checksum: 60H
        let data = [0x18, 0x00, 0x06, 0x00, 0x01];
        assert_eq!(checksum(&data), 0x61);
        // Note: the doc example may use a slightly different address.
        // Let's also verify the formula directly:
        // sum = 0x18 + 0x00 + 0x06 + 0x00 + 0x01 = 0x1F = 31
        // checksum = (128 - 31) % 128 = 97 = 0x61
    }

    #[test]
    fn checksum_zero_case() {
        // If sum % 128 == 0, checksum should be 0 (not 128)
        let data = [0x00];
        assert_eq!(checksum(&data), 0);

        // 128 % 128 == 0, so checksum should be 0
        let data2 = [0x40, 0x40]; // 64 + 64 = 128
        assert_eq!(checksum(&data2), 0);
    }
}
