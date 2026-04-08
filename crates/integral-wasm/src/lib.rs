//! WASM bindings for the Integral Integra-7 control surface.

use integral_core::sysex;
use wasm_bindgen::prelude::*;

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
