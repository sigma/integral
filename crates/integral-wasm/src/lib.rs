//! WASM bindings for the Integral Integra-7 control surface.

use integral_core::address::{Address, DataSize};
use integral_core::sysex;
use integral_core::{params, params::part};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Returns the SysEx Identity Request message bytes.
///
/// Send these bytes to a MIDI output to request device identification.
#[wasm_bindgen]
pub fn identity_request() -> Vec<u8> {
    sysex::identity_request().to_vec()
}

/// Parsed device identity from a SysEx Identity Reply.
#[wasm_bindgen]
pub struct DeviceIdentity {
    #[wasm_bindgen(readonly)]
    pub device_id: u8,
    #[wasm_bindgen(readonly)]
    pub manufacturer_id: u8,
    #[wasm_bindgen(readonly)]
    pub family_code_0: u8,
    #[wasm_bindgen(readonly)]
    pub family_code_1: u8,
    #[wasm_bindgen(readonly)]
    pub family_number_0: u8,
    #[wasm_bindgen(readonly)]
    pub family_number_1: u8,
    #[wasm_bindgen(readonly)]
    pub revision_0: u8,
    #[wasm_bindgen(readonly)]
    pub revision_1: u8,
    #[wasm_bindgen(readonly)]
    pub revision_2: u8,
    #[wasm_bindgen(readonly)]
    pub revision_3: u8,
}

#[wasm_bindgen]
impl DeviceIdentity {
    /// Returns `true` if this device is a Roland INTEGRA-7.
    #[wasm_bindgen(js_name = isIntegra7)]
    pub fn is_integra7(&self) -> bool {
        self.manufacturer_id == sysex::ROLAND_ID
            && [self.family_code_0, self.family_code_1] == sysex::INTEGRA7_FAMILY
    }

    /// Returns the family code as a formatted hex string (e.g. "64 02").
    #[wasm_bindgen(js_name = familyCodeHex)]
    pub fn family_code_hex(&self) -> String {
        format!("{:02X} {:02X}", self.family_code_0, self.family_code_1)
    }

    /// Returns the family number as a formatted hex string (e.g. "00 00").
    #[wasm_bindgen(js_name = familyNumberHex)]
    pub fn family_number_hex(&self) -> String {
        format!("{:02X} {:02X}", self.family_number_0, self.family_number_1)
    }

    /// Returns the revision as a formatted hex string (e.g. "00 00 00 00").
    #[wasm_bindgen(js_name = revisionHex)]
    pub fn revision_hex(&self) -> String {
        format!(
            "{:02X} {:02X} {:02X} {:02X}",
            self.revision_0, self.revision_1, self.revision_2, self.revision_3
        )
    }
}

impl From<sysex::DeviceIdentity> for DeviceIdentity {
    fn from(id: sysex::DeviceIdentity) -> Self {
        Self {
            device_id: id.device_id,
            manufacturer_id: id.manufacturer_id,
            family_code_0: id.family_code[0],
            family_code_1: id.family_code[1],
            family_number_0: id.family_number[0],
            family_number_1: id.family_number[1],
            revision_0: id.revision[0],
            revision_1: id.revision[1],
            revision_2: id.revision[2],
            revision_3: id.revision[3],
        }
    }
}

/// Parse a SysEx Identity Reply message.
///
/// Returns a `DeviceIdentity` on success, or throws a JS error on failure.
#[wasm_bindgen]
pub fn parse_identity_reply(data: &[u8]) -> Result<DeviceIdentity, JsError> {
    sysex::parse_identity_reply(data)
        .map(DeviceIdentity::from)
        .map_err(|e| JsError::new(&e.to_string()))
}

// ---------------------------------------------------------------------------
// DT1 / RQ1 builders
// ---------------------------------------------------------------------------

/// Build a DT1 (Data Set 1) SysEx message.
///
/// Address is passed as 4 individual bytes (wasm-bindgen ABI constraint).
#[wasm_bindgen]
pub fn build_dt1(device_id: u8, a0: u8, a1: u8, a2: u8, a3: u8, data: &[u8]) -> Vec<u8> {
    let addr = Address::new(a0, a1, a2, a3);
    sysex::build_dt1(device_id, &addr, data)
}

/// Build an RQ1 (Data Request 1) SysEx message.
///
/// Address and size are passed as individual bytes.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn build_rq1(
    device_id: u8,
    a0: u8,
    a1: u8,
    a2: u8,
    a3: u8,
    s0: u8,
    s1: u8,
    s2: u8,
    s3: u8,
) -> Vec<u8> {
    let addr = Address::new(a0, a1, a2, a3);
    let size = DataSize::new(s0, s1, s2, s3);
    sysex::build_rq1(device_id, &addr, &size)
}

// ---------------------------------------------------------------------------
// DT1 parser
// ---------------------------------------------------------------------------

/// A parsed DT1 message received from the device.
#[wasm_bindgen]
pub struct Dt1Message {
    #[wasm_bindgen(readonly)]
    pub device_id: u8,
    #[wasm_bindgen(readonly)]
    pub addr_0: u8,
    #[wasm_bindgen(readonly)]
    pub addr_1: u8,
    #[wasm_bindgen(readonly)]
    pub addr_2: u8,
    #[wasm_bindgen(readonly)]
    pub addr_3: u8,
    data: Vec<u8>,
}

#[wasm_bindgen]
impl Dt1Message {
    /// Returns the data payload of the DT1 message.
    pub fn data(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Returns the address as a 4-byte array.
    pub fn address(&self) -> Vec<u8> {
        vec![self.addr_0, self.addr_1, self.addr_2, self.addr_3]
    }
}

impl From<sysex::Dt1Message> for Dt1Message {
    fn from(msg: sysex::Dt1Message) -> Self {
        let addr = msg.address.as_bytes();
        Self {
            device_id: msg.device_id,
            addr_0: addr[0],
            addr_1: addr[1],
            addr_2: addr[2],
            addr_3: addr[3],
            data: msg.data,
        }
    }
}

/// Parse a raw SysEx message as an INTEGRA-7 DT1.
///
/// Returns a `Dt1Message` on success, or throws a JS error on failure.
#[wasm_bindgen]
pub fn parse_dt1(raw: &[u8]) -> Result<Dt1Message, JsError> {
    sysex::parse_dt1(raw)
        .map(Dt1Message::from)
        .map_err(|e| JsError::new(&e.to_string()))
}

// ---------------------------------------------------------------------------
// Address helpers
// ---------------------------------------------------------------------------

/// Helper: returns an address as a `Vec<u8>` of 4 bytes for JS consumption.
fn addr_to_vec(addr: Address) -> Vec<u8> {
    addr.as_bytes().to_vec()
}

/// Returns the address for System Master Level.
#[wasm_bindgen]
pub fn master_level_address() -> Vec<u8> {
    addr_to_vec(params::SYSTEM_MASTER_LEVEL)
}

/// Returns the address for Studio Set Name (first of 16 bytes).
#[wasm_bindgen]
pub fn studio_set_name_address() -> Vec<u8> {
    addr_to_vec(params::STUDIO_SET_NAME)
}

/// Returns the address for a part's Level parameter.
/// `part` is 0-indexed (0 = Part 1).
#[wasm_bindgen]
pub fn part_level_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::LEVEL))
}

/// Returns the address for a part's Pan parameter.
#[wasm_bindgen]
pub fn part_pan_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::PAN))
}

/// Returns the address for a part's Mute Switch.
#[wasm_bindgen]
pub fn part_mute_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::MUTE))
}

/// Returns the address for a part's Tone Bank MSB.
#[wasm_bindgen]
pub fn part_tone_bank_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::TONE_BANK_MSB))
}

/// Returns the address for a part's Receive Channel.
#[wasm_bindgen]
pub fn part_receive_channel_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::RECEIVE_CHANNEL))
}

/// Returns the address for a part's Chorus Send Level.
#[wasm_bindgen]
pub fn part_chorus_send_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::CHORUS_SEND))
}

/// Returns the address for a part's Reverb Send Level.
#[wasm_bindgen]
pub fn part_reverb_send_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::REVERB_SEND))
}

/// Returns the RQ1 size for reading all mixer-relevant part parameters in one request.
/// Covers from Receive Channel (offset 00) through Reverb Send (offset 28).
#[wasm_bindgen]
pub fn part_mixer_size() -> Vec<u8> {
    params::PART_MIXER_SIZE.as_bytes().to_vec()
}

/// Returns the RQ1 size for reading the Studio Set name (16 bytes).
#[wasm_bindgen]
pub fn studio_set_name_size() -> Vec<u8> {
    params::STUDIO_SET_NAME_SIZE.as_bytes().to_vec()
}

/// Returns the RQ1 size for a single byte parameter.
#[wasm_bindgen]
pub fn single_byte_size() -> Vec<u8> {
    params::SINGLE_BYTE_SIZE.as_bytes().to_vec()
}
