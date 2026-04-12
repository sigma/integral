//! WASM bindings for the Integral Integra-7 control surface.

use integral_core::address::{Address, DataSize};
use integral_core::catalog;
use integral_core::device::DeviceState;
use integral_core::state::parse as state_parse;
use integral_core::sysex;
use integral_core::{
    fx_params, mfx, mfx_params, params, params::part, params::part_eq, tone_banks,
};
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

/// Returns the address for Setup Studio Set Program Change.
#[wasm_bindgen]
pub fn setup_studio_set_pc_address() -> Vec<u8> {
    addr_to_vec(params::SETUP_STUDIO_SET_PC)
}

/// Returns the address for Setup Studio Set BS MSB.
#[wasm_bindgen]
pub fn setup_studio_set_bs_msb_address() -> Vec<u8> {
    addr_to_vec(params::SETUP_STUDIO_SET_BS_MSB)
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

/// Returns the address for reading a tone name, given a part and bank MSB.
///
/// The bank MSB determines the tone type (PCM Synth, SN Acoustic, etc.)
/// which determines the address block to read from.
/// Returns null (empty vec) if the bank MSB doesn't map to a known type.
#[wasm_bindgen]
pub fn tone_name_address(part_index: u8, bank_msb: u8) -> Vec<u8> {
    match params::tone_type_from_bank_msb(bank_msb) {
        Some(tt) => addr_to_vec(params::tone_name_address(part_index, tt)),
        None => vec![],
    }
}

/// Returns the RQ1 size for reading a tone name (12 bytes).
#[wasm_bindgen]
pub fn tone_name_size() -> Vec<u8> {
    params::TONE_NAME_SIZE.as_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// Catalog queries (undocumented)
// ---------------------------------------------------------------------------

/// Build a catalog query for all Studio Set names.
///
/// Returns the raw SysEx bytes to send. The device responds with multiple
/// DT1 messages containing all 64 Studio Set names.
#[wasm_bindgen]
pub fn build_studio_set_catalog_request(device_id: u8, start: u8, count: u8) -> Vec<u8> {
    catalog::build_studio_set_catalog_request(device_id, start, count)
}

/// Build a catalog query for tone names in a specific bank.
#[wasm_bindgen]
pub fn build_tone_catalog_request(
    device_id: u8,
    msb: u8,
    lsb: u8,
    start_pc: u8,
    count: u8,
) -> Vec<u8> {
    catalog::build_tone_catalog_request(device_id, msb, lsb, start_pc, count)
}

/// A parsed catalog entry (name for a preset/user slot).
#[wasm_bindgen]
pub struct CatalogEntry {
    #[wasm_bindgen(readonly)]
    pub bank_msb: u8,
    #[wasm_bindgen(readonly)]
    pub bank_lsb: u8,
    #[wasm_bindgen(readonly)]
    pub pc: u8,
    name: String,
}

#[wasm_bindgen]
impl CatalogEntry {
    /// Returns the entry name.
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

/// Parse a DT1 response as a catalog entry.
///
/// Returns null for delimiter messages (all-zero data) or invalid data.
#[wasm_bindgen]
pub fn parse_catalog_entry(data: &[u8]) -> Option<CatalogEntry> {
    catalog::parse_catalog_entry(data).map(|e| CatalogEntry {
        bank_msb: e.bank_msb,
        bank_lsb: e.bank_lsb,
        pc: e.pc,
        name: e.name,
    })
}

// ---------------------------------------------------------------------------
// EQ addresses
// ---------------------------------------------------------------------------

/// Returns the address for a Part EQ parameter.
/// `part_index` is 0-indexed, `param_offset` is a part_eq constant (0-7).
#[wasm_bindgen]
pub fn part_eq_address(part_index: u8, param_offset: u8) -> Vec<u8> {
    addr_to_vec(params::part_eq_address(part_index, param_offset))
}

// ---------------------------------------------------------------------------
// Chorus / Reverb addresses
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn chorus_address(param_offset: u8) -> Vec<u8> {
    addr_to_vec(params::chorus_address(param_offset))
}

#[wasm_bindgen]
pub fn chorus_switch_address() -> Vec<u8> {
    addr_to_vec(params::CHORUS_SWITCH)
}

#[wasm_bindgen]
pub fn chorus_core_size() -> Vec<u8> {
    params::CHORUS_CORE_SIZE.as_bytes().to_vec()
}

#[wasm_bindgen]
pub fn reverb_address(param_offset: u8) -> Vec<u8> {
    addr_to_vec(params::reverb_address(param_offset))
}

#[wasm_bindgen]
pub fn reverb_switch_address() -> Vec<u8> {
    addr_to_vec(params::REVERB_SWITCH)
}

#[wasm_bindgen]
pub fn reverb_core_size() -> Vec<u8> {
    params::REVERB_CORE_SIZE.as_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// Ext Part addresses
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn ext_part_level_address() -> Vec<u8> {
    addr_to_vec(params::EXT_PART_LEVEL)
}

#[wasm_bindgen]
pub fn ext_part_mute_address() -> Vec<u8> {
    addr_to_vec(params::EXT_PART_MUTE)
}

/// Returns the RQ1 size for reading all Part EQ parameters (8 bytes).
#[wasm_bindgen]
pub fn part_eq_size() -> Vec<u8> {
    params::PART_EQ_SIZE.as_bytes().to_vec()
}

/// Returns the address for a Master EQ parameter.
/// `param_offset` is a master_eq constant (0-6).
#[wasm_bindgen]
pub fn master_eq_address(param_offset: u8) -> Vec<u8> {
    addr_to_vec(params::master_eq_address(param_offset))
}

/// Returns the RQ1 size for reading all Master EQ parameters (7 bytes).
#[wasm_bindgen]
pub fn master_eq_size() -> Vec<u8> {
    params::MASTER_EQ_SIZE.as_bytes().to_vec()
}

/// Returns the address for the Master EQ Switch.
#[wasm_bindgen]
pub fn master_eq_switch_address() -> Vec<u8> {
    addr_to_vec(params::MASTER_EQ_SWITCH)
}

// Re-export EQ parameter offset constants for JS use.

#[wasm_bindgen]
pub fn eq_switch_offset() -> u8 {
    part_eq::SWITCH
}
#[wasm_bindgen]
pub fn eq_low_freq_offset() -> u8 {
    part_eq::LOW_FREQ
}
#[wasm_bindgen]
pub fn eq_low_gain_offset() -> u8 {
    part_eq::LOW_GAIN
}
#[wasm_bindgen]
pub fn eq_mid_freq_offset() -> u8 {
    part_eq::MID_FREQ
}
#[wasm_bindgen]
pub fn eq_mid_gain_offset() -> u8 {
    part_eq::MID_GAIN
}
#[wasm_bindgen]
pub fn eq_mid_q_offset() -> u8 {
    part_eq::MID_Q
}
#[wasm_bindgen]
pub fn eq_high_freq_offset() -> u8 {
    part_eq::HIGH_FREQ
}
#[wasm_bindgen]
pub fn eq_high_gain_offset() -> u8 {
    part_eq::HIGH_GAIN
}

// ---------------------------------------------------------------------------
// State parsing
// ---------------------------------------------------------------------------

/// Parsed part mixer state from a 0x29-byte dump.
#[wasm_bindgen]
pub struct WasmPartState {
    #[wasm_bindgen(readonly, js_name = receiveChannel)]
    pub receive_channel: u8,
    #[wasm_bindgen(readonly, js_name = toneBankMsb)]
    pub tone_bank_msb: u8,
    #[wasm_bindgen(readonly, js_name = toneBankLsb)]
    pub tone_bank_lsb: u8,
    #[wasm_bindgen(readonly, js_name = tonePC)]
    pub tone_pc: u8,
    #[wasm_bindgen(readonly)]
    pub level: u8,
    #[wasm_bindgen(readonly)]
    pub pan: u8,
    #[wasm_bindgen(readonly)]
    pub muted: bool,
    #[wasm_bindgen(readonly, js_name = chorusSend)]
    pub chorus_send: u8,
    #[wasm_bindgen(readonly, js_name = reverbSend)]
    pub reverb_send: u8,
}

/// Parse a 0x29-byte part mixer dump.
#[wasm_bindgen]
pub fn parse_part_dump(data: &[u8]) -> WasmPartState {
    let p = state_parse::parse_part_dump(data);
    WasmPartState {
        receive_channel: p.receive_channel,
        tone_bank_msb: p.tone_bank_msb,
        tone_bank_lsb: p.tone_bank_lsb,
        tone_pc: p.tone_pc,
        level: p.level,
        pan: p.pan,
        muted: p.muted,
        chorus_send: p.chorus_send,
        reverb_send: p.reverb_send,
    }
}

/// Parsed EQ state.
#[wasm_bindgen]
pub struct WasmEqState {
    #[wasm_bindgen(readonly)]
    pub enabled: bool,
    #[wasm_bindgen(readonly, js_name = lowFreq)]
    pub low_freq: u8,
    #[wasm_bindgen(readonly, js_name = lowGain)]
    pub low_gain: u8,
    #[wasm_bindgen(readonly, js_name = midFreq)]
    pub mid_freq: u8,
    #[wasm_bindgen(readonly, js_name = midGain)]
    pub mid_gain: u8,
    #[wasm_bindgen(readonly, js_name = midQ)]
    pub mid_q: u8,
    #[wasm_bindgen(readonly, js_name = highFreq)]
    pub high_freq: u8,
    #[wasm_bindgen(readonly, js_name = highGain)]
    pub high_gain: u8,
}

impl From<integral_core::state::EqState> for WasmEqState {
    fn from(eq: integral_core::state::EqState) -> Self {
        Self {
            enabled: eq.enabled,
            low_freq: eq.low_freq,
            low_gain: eq.low_gain,
            mid_freq: eq.mid_freq,
            mid_gain: eq.mid_gain,
            mid_q: eq.mid_q,
            high_freq: eq.high_freq,
            high_gain: eq.high_gain,
        }
    }
}

/// Parse an 8-byte Part EQ dump.
#[wasm_bindgen]
pub fn parse_part_eq_dump(data: &[u8]) -> WasmEqState {
    state_parse::parse_part_eq_dump(data).into()
}

/// Parse a 7-byte Master EQ dump (no switch byte).
#[wasm_bindgen]
pub fn parse_master_eq_dump(data: &[u8]) -> WasmEqState {
    state_parse::parse_master_eq_dump(data).into()
}

// ---------------------------------------------------------------------------
// Nibble codec
// ---------------------------------------------------------------------------

/// Decode nibblized FX parameters from raw SysEx data.
///
/// Each param is 4 nibble bytes → signed display value.
#[wasm_bindgen]
pub fn decode_nib_params(data: &[u8], count: usize) -> Vec<i32> {
    state_parse::decode_nib_params(data, count)
}

/// Encode a signed display value into 4 nibble bytes for SysEx DT1.
#[wasm_bindgen]
pub fn encode_nib_param(value: i32) -> Vec<u8> {
    state_parse::encode_nib_param(value).to_vec()
}

// ---------------------------------------------------------------------------
// Tone bank definitions
// ---------------------------------------------------------------------------

/// A tone bank exposed to JS.
#[wasm_bindgen]
pub struct WasmToneBank {
    #[wasm_bindgen(readonly)]
    pub msb: u8,
    label: String,
    lsbs: Vec<u8>,
}

#[wasm_bindgen]
impl WasmToneBank {
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> String {
        self.label.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn lsbs(&self) -> Vec<u8> {
        self.lsbs.clone()
    }
}

/// A tone bank group exposed to JS.
#[wasm_bindgen]
pub struct WasmToneBankGroup {
    label: String,
    banks: Vec<WasmToneBank>,
}

#[wasm_bindgen]
impl WasmToneBankGroup {
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> String {
        self.label.clone()
    }

    /// Number of banks in this group.
    #[wasm_bindgen(getter, js_name = bankCount)]
    pub fn bank_count(&self) -> usize {
        self.banks.len()
    }

    /// Get bank at index.
    #[wasm_bindgen(js_name = getBank)]
    pub fn get_bank(&self, index: usize) -> Option<WasmToneBank> {
        self.banks.get(index).map(|b| WasmToneBank {
            msb: b.msb,
            label: b.label.clone(),
            lsbs: b.lsbs.clone(),
        })
    }
}

/// Returns all tone bank groups.
#[wasm_bindgen]
pub fn tone_bank_groups() -> Vec<WasmToneBankGroup> {
    tone_banks::TONE_BANK_GROUPS
        .iter()
        .map(|g| WasmToneBankGroup {
            label: g.label.to_string(),
            banks: g
                .banks
                .iter()
                .map(|b| WasmToneBank {
                    msb: b.msb,
                    label: b.label.to_string(),
                    lsbs: b.lsbs.to_vec(),
                })
                .collect(),
        })
        .collect()
}

/// Find the bank containing a given (MSB, LSB) pair.
/// Returns null if not found.
#[wasm_bindgen]
pub fn find_tone_bank(msb: u8, lsb: u8) -> Option<WasmToneBank> {
    tone_banks::find_bank(msb, lsb).map(|b| WasmToneBank {
        msb: b.msb,
        label: b.label.to_string(),
        lsbs: b.lsbs.to_vec(),
    })
}

// ---------------------------------------------------------------------------
// Factory catalog
// ---------------------------------------------------------------------------

/// Return factory preset tones for a bank as a JSON string.
///
/// Returns a JSON array of `{msb, lsb, pc, name, category}` objects,
/// or `"[]"` for unknown banks. The `pc` values are 0-indexed.
#[wasm_bindgen(js_name = factoryTonesJson)]
pub fn factory_tones_json(msb: u8, lsb: u8) -> String {
    use integral_core::factory_catalog::factory_tones;

    let tones = factory_tones(msb, lsb);
    if tones.is_empty() {
        return "[]".to_string();
    }

    // Manual JSON serialization to avoid serde dependency in the main crate.
    let mut json = String::from("[");
    for (i, t) in tones.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        let name = t.name.replace('\\', "\\\\").replace('"', "\\\"");
        json.push_str(&format!(
            r#"{{"msb":{},"lsb":{},"pc":{},"name":"{}","category":{}}}"#,
            t.msb, t.lsb, t.pc, name, t.category
        ));
    }
    json.push(']');
    json
}

/// Look up a single factory tone name by bank and PC.
///
/// Returns the tone name, or an empty string if not found.
#[wasm_bindgen(js_name = factoryToneName)]
pub fn factory_tone_name(msb: u8, lsb: u8, pc: u8) -> String {
    use integral_core::factory_catalog::factory_tones;

    factory_tones(msb, lsb)
        .iter()
        .find(|t| t.pc == pc)
        .map(|t| t.name.to_string())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// FX parameter definitions
// ---------------------------------------------------------------------------

/// A single FX parameter definition exposed to JS.
#[wasm_bindgen]
pub struct WasmFxParamDef {
    #[wasm_bindgen(readonly)]
    pub index: u8,
    #[wasm_bindgen(readonly)]
    pub min: i32,
    #[wasm_bindgen(readonly)]
    pub max: i32,
    #[wasm_bindgen(readonly, js_name = defaultValue)]
    pub default_value: i32,
    inner: &'static fx_params::FxParamDef,
}

#[wasm_bindgen]
impl WasmFxParamDef {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.to_string()
    }

    /// Format a raw value for display.
    #[wasm_bindgen(js_name = formatValue)]
    pub fn format_value(&self, value: i32) -> String {
        self.inner.format_value(value)
    }
}

fn wrap_fx_params(params: &'static [fx_params::FxParamDef]) -> Vec<WasmFxParamDef> {
    params
        .iter()
        .map(|p| WasmFxParamDef {
            index: p.index,
            min: p.min,
            max: p.max,
            default_value: p.default_value,
            inner: p,
        })
        .collect()
}

/// Returns chorus parameter definitions for the given type index.
#[wasm_bindgen]
pub fn chorus_params(chorus_type: u8) -> Vec<WasmFxParamDef> {
    wrap_fx_params(fx_params::chorus_params(chorus_type))
}

/// Returns reverb parameter definitions for the given type index.
#[wasm_bindgen]
pub fn reverb_params(reverb_type: u8) -> Vec<WasmFxParamDef> {
    wrap_fx_params(fx_params::reverb_params(reverb_type))
}

/// Returns chorus type display names.
#[wasm_bindgen]
pub fn chorus_type_names() -> Vec<String> {
    fx_params::CHORUS_TYPE_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Returns reverb type display names.
#[wasm_bindgen]
pub fn reverb_type_names() -> Vec<String> {
    fx_params::REVERB_TYPE_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Returns chorus output routing names.
#[wasm_bindgen]
pub fn chorus_output_names() -> Vec<String> {
    fx_params::CHORUS_OUTPUT_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Returns reverb output routing names.
#[wasm_bindgen]
pub fn reverb_output_names() -> Vec<String> {
    fx_params::REVERB_OUTPUT_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// DeviceState — stateful device manager
// ---------------------------------------------------------------------------

/// Stateful device manager exposed to JS.
///
/// Owns the mixer state, send queue (with coalescing/throttle), and echo
/// suppression.  The JS layer feeds incoming MIDI and drains outbound
/// messages; all business logic lives in Rust.
#[wasm_bindgen]
pub struct WasmDeviceState {
    inner: DeviceState,
}

#[wasm_bindgen]
impl WasmDeviceState {
    /// Create a new device state with the given SysEx device ID.
    #[wasm_bindgen(constructor)]
    pub fn new(device_id: u8) -> Self {
        Self {
            inner: DeviceState::new(device_id),
        }
    }

    // -- Send queue --------------------------------------------------------

    /// Drain the next outbound SysEx message if the throttle has elapsed.
    ///
    /// `now_ms` is a monotonic timestamp (e.g. `performance.now()`).
    /// Returns `undefined` if nothing to send.
    pub fn drain(&mut self, now_ms: f64) -> Option<Vec<u8>> {
        self.inner.drain(now_ms)
    }

    /// Whether there are queued messages waiting to be sent.
    #[wasm_bindgen(js_name = hasPending)]
    pub fn has_pending(&self) -> bool {
        self.inner.has_pending()
    }

    /// Queue a raw SysEx message (for catalog requests etc.).
    #[wasm_bindgen(js_name = sendRaw)]
    pub fn send_raw(&mut self, key: &str, bytes: Vec<u8>) {
        self.inner.send_raw(key, bytes);
    }

    // -- Incoming DT1 ------------------------------------------------------

    /// Process an incoming DT1.  Returns true if state changed.
    #[wasm_bindgen(js_name = handleDt1)]
    pub fn handle_dt1(&mut self, addr: &[u8], data: &[u8], now_ms: f64) -> bool {
        if addr.len() < 4 {
            return false;
        }
        let address = Address::new(addr[0], addr[1], addr[2], addr[3]);
        self.inner.handle_dt1(&address, data, now_ms)
    }

    // -- State read --------------------------------------------------------

    /// Read a snapshot of the full mixer state as a JS object.
    ///
    /// Returns a JsValue that mirrors the TypeScript `MixerState` shape.
    #[wasm_bindgen(js_name = readState)]
    pub fn read_state(&self) -> Result<JsValue, JsError> {
        let s = self.inner.state();
        serde_wasm_bindgen::to_value(s).map_err(|e| JsError::new(&e.to_string()))
    }

    // -- Part setters ------------------------------------------------------

    #[wasm_bindgen(js_name = setPartLevel)]
    pub fn set_part_level(&mut self, part: u8, value: u8) {
        self.inner.set_part_level(part, value);
    }

    #[wasm_bindgen(js_name = setPartPan)]
    pub fn set_part_pan(&mut self, part: u8, value: u8) {
        self.inner.set_part_pan(part, value);
    }

    #[wasm_bindgen(js_name = setPartMute)]
    pub fn set_part_mute(&mut self, part: u8, muted: bool) {
        self.inner.set_part_mute(part, muted);
    }

    #[wasm_bindgen(js_name = togglePartMute)]
    pub fn toggle_part_mute(&mut self, part: u8) {
        self.inner.toggle_part_mute(part);
    }

    #[wasm_bindgen(js_name = setPartChorusSend)]
    pub fn set_part_chorus_send(&mut self, part: u8, value: u8) {
        self.inner.set_part_chorus_send(part, value);
    }

    #[wasm_bindgen(js_name = setPartReverbSend)]
    pub fn set_part_reverb_send(&mut self, part: u8, value: u8) {
        self.inner.set_part_reverb_send(part, value);
    }

    #[wasm_bindgen(js_name = setPartReceiveChannel)]
    pub fn set_part_receive_channel(&mut self, part: u8, channel: u8) {
        self.inner.set_part_receive_channel(part, channel);
    }

    #[wasm_bindgen(js_name = changePartTone)]
    pub fn change_part_tone(&mut self, part: u8, msb: u8, lsb: u8, pc: u8) {
        self.inner.change_part_tone(part, msb, lsb, pc);
    }

    #[wasm_bindgen(js_name = setMasterLevel)]
    pub fn set_master_level(&mut self, value: u8) {
        self.inner.set_master_level(value);
    }

    // -- EQ setters --------------------------------------------------------

    #[wasm_bindgen(js_name = setPartEqParam)]
    pub fn set_part_eq_param(&mut self, part: u8, param_offset: u8, value: u8) {
        self.inner.set_part_eq_param(part, param_offset, value);
    }

    #[wasm_bindgen(js_name = setMasterEqParam)]
    pub fn set_master_eq_param(&mut self, param_offset: u8, value: u8) {
        self.inner.set_master_eq_param(param_offset, value);
    }

    #[wasm_bindgen(js_name = toggleMasterEqSwitch)]
    pub fn toggle_master_eq_switch(&mut self) {
        self.inner.toggle_master_eq_switch();
    }

    // -- FX setters --------------------------------------------------------

    #[wasm_bindgen(js_name = setChorusParam)]
    pub fn set_chorus_param(&mut self, offset: u8, value: u8) {
        self.inner.set_chorus_param(offset, value);
    }

    #[wasm_bindgen(js_name = setChorusNibParam)]
    pub fn set_chorus_nib_param(&mut self, param_index: u32, value: i32) {
        self.inner.set_chorus_nib_param(param_index as usize, value);
    }

    #[wasm_bindgen(js_name = toggleChorusSwitch)]
    pub fn toggle_chorus_switch(&mut self) {
        self.inner.toggle_chorus_switch();
    }

    #[wasm_bindgen(js_name = setReverbParam)]
    pub fn set_reverb_param(&mut self, offset: u8, value: u8) {
        self.inner.set_reverb_param(offset, value);
    }

    #[wasm_bindgen(js_name = setReverbNibParam)]
    pub fn set_reverb_nib_param(&mut self, param_index: u32, value: i32) {
        self.inner.set_reverb_nib_param(param_index as usize, value);
    }

    #[wasm_bindgen(js_name = toggleReverbSwitch)]
    pub fn toggle_reverb_switch(&mut self) {
        self.inner.toggle_reverb_switch();
    }

    // -- Ext part ----------------------------------------------------------

    #[wasm_bindgen(js_name = setExtLevel)]
    pub fn set_ext_level(&mut self, value: u8) {
        self.inner.set_ext_level(value);
    }

    #[wasm_bindgen(js_name = toggleExtMute)]
    pub fn toggle_ext_mute(&mut self) {
        self.inner.toggle_ext_mute();
    }

    // -- Studio Set --------------------------------------------------------

    #[wasm_bindgen(js_name = switchStudioSet)]
    pub fn switch_studio_set(&mut self, pc: u8) {
        self.inner.switch_studio_set(pc);
    }

    #[wasm_bindgen(js_name = toggleSolo)]
    pub fn toggle_solo(&mut self, part: u8) {
        self.inner.toggle_solo(part);
    }

    #[wasm_bindgen(js_name = setSoloPart)]
    pub fn set_solo_part(&mut self, value: u8) {
        self.inner.set_solo_part(value);
    }

    // -- Preview -------------------------------------------------------------

    /// Start phrase preview on the given part (1–16).
    #[wasm_bindgen(js_name = previewStart)]
    pub fn preview_start(&mut self, part: u8) {
        self.inner.preview_start(part);
    }

    /// Stop any active phrase preview.
    #[wasm_bindgen(js_name = previewStop)]
    pub fn preview_stop(&mut self) {
        self.inner.preview_stop();
    }

    /// Toggle phrase preview for the given part (1–16).
    #[wasm_bindgen(js_name = previewToggle)]
    pub fn preview_toggle(&mut self, part: u8) {
        self.inner.preview_toggle(part);
    }

    /// Returns the currently previewing part (0 = off, 1–16 = part).
    #[wasm_bindgen(js_name = previewPart)]
    pub fn preview_part(&self) -> u8 {
        self.inner.state().preview_part
    }

    // -- RQ1 builders (return bytes to send) -------------------------------

    #[wasm_bindgen(js_name = buildPartMixerRequest)]
    pub fn build_part_mixer_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_part_mixer_request(part)
    }

    #[wasm_bindgen(js_name = buildMasterLevelRequest)]
    pub fn build_master_level_request(&self) -> Vec<u8> {
        self.inner.build_master_level_request()
    }

    #[wasm_bindgen(js_name = buildStudioSetNameRequest)]
    pub fn build_studio_set_name_request(&self) -> Vec<u8> {
        self.inner.build_studio_set_name_request()
    }

    #[wasm_bindgen(js_name = buildStudioSetPcRequest)]
    pub fn build_studio_set_pc_request(&self) -> Vec<u8> {
        self.inner.build_studio_set_pc_request()
    }

    // -- Direct state mutation (for load responses) ------------------------

    /// Patch the mixer state with a parsed part dump.
    #[wasm_bindgen(js_name = applyPartDump)]
    pub fn apply_part_dump(&mut self, part: u8, data: &[u8]) {
        let parsed = state_parse::parse_part_dump(data);
        let p = &mut self.inner.state_mut().parts[part as usize];
        p.receive_channel = parsed.receive_channel;
        p.tone_bank_msb = parsed.tone_bank_msb;
        p.tone_bank_lsb = parsed.tone_bank_lsb;
        p.tone_pc = parsed.tone_pc;
        p.level = parsed.level;
        p.pan = parsed.pan;
        p.muted = parsed.muted;
        p.chorus_send = parsed.chorus_send;
        p.reverb_send = parsed.reverb_send;
    }

    /// Patch the part EQ with a parsed dump.
    #[wasm_bindgen(js_name = applyPartEqDump)]
    pub fn apply_part_eq_dump(&mut self, part: u8, data: &[u8]) {
        let eq = state_parse::parse_part_eq_dump(data);
        self.inner.state_mut().parts[part as usize].eq = eq;
    }

    /// Patch the master EQ with a parsed dump.
    #[wasm_bindgen(js_name = applyMasterEqDump)]
    pub fn apply_master_eq_dump(&mut self, data: &[u8]) {
        let eq = state_parse::parse_master_eq_dump(data);
        // Preserve the enabled flag (read separately).
        let enabled = self.inner.state().master_eq.enabled;
        self.inner.state_mut().master_eq = eq;
        self.inner.state_mut().master_eq.enabled = enabled;
    }

    /// Set master EQ enabled state.
    #[wasm_bindgen(js_name = setMasterEqEnabled)]
    pub fn set_master_eq_enabled(&mut self, enabled: bool) {
        self.inner.state_mut().master_eq.enabled = enabled;
    }

    /// Set the studio set name.
    #[wasm_bindgen(js_name = setStudioSetName)]
    pub fn set_studio_set_name(&mut self, name: &str) {
        self.inner.state_mut().studio_set_name = name.to_string();
    }

    /// Set the studio set PC.
    #[wasm_bindgen(js_name = setStudioSetPc)]
    pub fn set_studio_set_pc(&mut self, pc: u8) {
        self.inner.state_mut().studio_set_pc = pc;
    }

    /// Set the master level.
    #[wasm_bindgen(js_name = applyMasterLevel)]
    pub fn apply_master_level(&mut self, value: u8) {
        self.inner.state_mut().master_level = value;
    }

    /// Set a part's tone name.
    #[wasm_bindgen(js_name = setPartToneName)]
    pub fn set_part_tone_name(&mut self, part: u8, name: &str) {
        self.inner.state_mut().parts[part as usize].tone_name = name.to_string();
    }

    /// Set chorus state from core bytes.
    #[wasm_bindgen(js_name = applyChorusCore)]
    pub fn apply_chorus_core(&mut self, data: &[u8]) {
        if data.len() >= 3 {
            let fx = &mut self.inner.state_mut().chorus;
            fx.fx_type = data[0];
            fx.level = data[1];
            fx.output = data[2];
        }
    }

    /// Set chorus enabled.
    #[wasm_bindgen(js_name = setChorusEnabled)]
    pub fn set_chorus_enabled(&mut self, enabled: bool) {
        self.inner.state_mut().chorus.enabled = enabled;
    }

    /// Set chorus params from decoded nib values.
    #[wasm_bindgen(js_name = applyChorusParams)]
    pub fn apply_chorus_params(&mut self, params: Vec<i32>) {
        self.inner.state_mut().chorus.params = params;
    }

    /// Set reverb state from core bytes.
    #[wasm_bindgen(js_name = applyReverbCore)]
    pub fn apply_reverb_core(&mut self, data: &[u8]) {
        if data.len() >= 3 {
            let fx = &mut self.inner.state_mut().reverb;
            fx.fx_type = data[0];
            fx.level = data[1];
            fx.output = data[2];
        }
    }

    /// Set reverb enabled.
    #[wasm_bindgen(js_name = setReverbEnabled)]
    pub fn set_reverb_enabled(&mut self, enabled: bool) {
        self.inner.state_mut().reverb.enabled = enabled;
    }

    /// Set reverb params from decoded nib values.
    #[wasm_bindgen(js_name = applyReverbParams)]
    pub fn apply_reverb_params(&mut self, params: Vec<i32>) {
        self.inner.state_mut().reverb.params = params;
    }

    /// Set ext part level.
    #[wasm_bindgen(js_name = applyExtLevel)]
    pub fn apply_ext_level(&mut self, value: u8) {
        self.inner.state_mut().ext_level = value;
    }

    /// Set ext part muted.
    #[wasm_bindgen(js_name = applyExtMuted)]
    pub fn apply_ext_muted(&mut self, muted: bool) {
        self.inner.state_mut().ext_muted = muted;
    }

    // -- Drum Comp+EQ --------------------------------------------------

    #[wasm_bindgen(js_name = setDrumCompEqSwitch)]
    pub fn set_drum_comp_eq_switch(&mut self, enabled: bool) {
        self.inner.set_drum_comp_eq_switch(enabled);
    }

    #[wasm_bindgen(js_name = setDrumCompEqPart)]
    pub fn set_drum_comp_eq_part(&mut self, part: u8) {
        self.inner.set_drum_comp_eq_part(part);
    }

    #[wasm_bindgen(js_name = setDrumCompEqOutputAssign)]
    pub fn set_drum_comp_eq_output_assign(&mut self, unit: u8, value: u8) {
        self.inner.set_drum_comp_eq_output_assign(unit, value);
    }

    #[wasm_bindgen(js_name = setCompEqParam)]
    pub fn set_comp_eq_param(&mut self, unit: u8, param_offset: u8, value: u8) {
        self.inner.set_comp_eq_param(unit, param_offset, value);
    }

    #[wasm_bindgen(js_name = buildCompEqBlockRequest)]
    pub fn build_comp_eq_block_request(&self) -> Vec<u8> {
        self.inner.build_comp_eq_block_request()
    }

    /// Apply a parsed 84-byte Comp+EQ block dump to the state.
    #[wasm_bindgen(js_name = applyCompEqBlock)]
    pub fn apply_comp_eq_block(&mut self, data: &[u8]) {
        let units = state_parse::parse_comp_eq_block(data);
        self.inner.state_mut().drum_comp_eq.units = units;
    }

    /// Apply Studio Set common Comp+EQ settings.
    #[wasm_bindgen(js_name = applyDrumCompEqCommon)]
    pub fn apply_drum_comp_eq_common(&mut self, enabled: bool, part: u8, output_assigns: &[u8]) {
        let dce = &mut self.inner.state_mut().drum_comp_eq;
        dce.enabled = enabled;
        dce.part = part;
        for (i, &v) in output_assigns.iter().take(6).enumerate() {
            dce.output_assigns[i] = v;
        }
    }

    // -- Motional Surround -------------------------------------------------

    #[wasm_bindgen(js_name = setSurroundParam)]
    pub fn set_surround_param(&mut self, param_offset: u8, value: u8) {
        self.inner.set_surround_param(param_offset, value);
    }

    #[wasm_bindgen(js_name = setPartSurroundLr)]
    pub fn set_part_surround_lr(&mut self, part: u8, value: u8) {
        self.inner
            .set_part_surround_param(part, integral_core::params::part_surround::LR, value);
    }

    #[wasm_bindgen(js_name = setPartSurroundFb)]
    pub fn set_part_surround_fb(&mut self, part: u8, value: u8) {
        self.inner
            .set_part_surround_param(part, integral_core::params::part_surround::FB, value);
    }

    #[wasm_bindgen(js_name = setPartSurroundWidth)]
    pub fn set_part_surround_width(&mut self, part: u8, value: u8) {
        self.inner.set_part_surround_param(
            part,
            integral_core::params::part_surround::WIDTH,
            value,
        );
    }

    #[wasm_bindgen(js_name = setPartSurroundAmbienceSend)]
    pub fn set_part_surround_ambience_send(&mut self, part: u8, value: u8) {
        self.inner.set_part_surround_param(
            part,
            integral_core::params::part_surround::AMBIENCE_SEND,
            value,
        );
    }

    /// Apply a parsed 13-byte Motional Surround common dump.
    #[wasm_bindgen(js_name = applySurroundCommon)]
    pub fn apply_surround_common(&mut self, data: &[u8]) {
        let parsed = state_parse::parse_surround_common(data);
        // Preserve per-part data (loaded separately).
        let parts = self.inner.state().surround.parts.clone();
        self.inner.state_mut().surround = parsed;
        self.inner.state_mut().surround.parts = parts;
    }

    /// Set per-part surround positioning directly.
    #[wasm_bindgen(js_name = applyPartSurround)]
    pub fn apply_part_surround(&mut self, part: u8, lr: u8, fb: u8, width: u8, ambience_send: u8) {
        let ps = &mut self.inner.state_mut().surround.parts[part as usize];
        ps.lr = lr;
        ps.fb = fb;
        ps.width = width;
        ps.ambience_send = ambience_send;
    }

    #[wasm_bindgen(js_name = buildSurroundCommonRequest)]
    pub fn build_surround_common_request(&self) -> Vec<u8> {
        self.inner.build_surround_common_request()
    }

    // -- SN-S Tone Edit ----------------------------------------------------

    /// Set a single SN-S Common parameter.
    #[wasm_bindgen(js_name = setSnsCommonParam)]
    pub fn set_sns_common_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_sns_common_param(part, offset, value);
    }

    /// Set a single SN-S Partial parameter.
    #[wasm_bindgen(js_name = setSnsPartialParam)]
    pub fn set_sns_partial_param(&mut self, part: u8, partial: u8, offset: u8, value: u8) {
        self.inner
            .set_sns_partial_param(part, partial, offset, value);
    }

    /// Set a nibblized SN-S Partial parameter (4 bytes, e.g. wave number).
    #[wasm_bindgen(js_name = setSnsPartialNibParam)]
    pub fn set_sns_partial_nib_param(&mut self, part: u8, partial: u8, offset: u8, value: u16) {
        self.inner
            .set_sns_partial_nib_param(part, partial, offset, value);
    }

    /// Set a nibblized SN-S Common parameter (4 bytes, e.g. phrase number).
    #[wasm_bindgen(js_name = setSnsCommonNibParam)]
    pub fn set_sns_common_nib_param(&mut self, part: u8, offset: u8, value: u16) {
        self.inner.set_sns_common_nib_param(part, offset, value);
    }

    /// Parse an SN-S Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applySnsCommon)]
    pub fn apply_sns_common(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::sn_synth::parse_sns_common(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Parse an SN-S Partial dump and return as a JS object.
    #[wasm_bindgen(js_name = applySnsPartial)]
    pub fn apply_sns_partial(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::sn_synth::parse_sns_partial(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Build an RQ1 to read the SN-S Common block for a part.
    #[wasm_bindgen(js_name = buildSnsCommonRequest)]
    pub fn build_sns_common_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_sns_common_request(part)
    }

    /// Build an RQ1 to read an SN-S Partial block for a part.
    #[wasm_bindgen(js_name = buildSnsPartialRequest)]
    pub fn build_sns_partial_request(&self, part: u8, partial: u8) -> Vec<u8> {
        self.inner.build_sns_partial_request(part, partial)
    }

    // -- SN-A Tone Edit ----------------------------------------------------

    /// Set a single SN-A Common parameter.
    #[wasm_bindgen(js_name = setSnaCommonParam)]
    pub fn set_sna_common_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_sna_common_param(part, offset, value);
    }

    /// Parse an SN-A Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applySnaCommon)]
    pub fn apply_sna_common(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::sn_acoustic::parse_sna_common(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Build an RQ1 to read the SN-A Common block for a part.
    #[wasm_bindgen(js_name = buildSnaCommonRequest)]
    pub fn build_sna_common_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_sna_common_request(part)
    }

    // -- SN-D Tone Edit ----------------------------------------------------

    /// Set a single SN-D Common parameter.
    #[wasm_bindgen(js_name = setSndCommonParam)]
    pub fn set_snd_common_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_snd_common_param(part, offset, value);
    }

    /// Set a single SN-D Note parameter.
    #[wasm_bindgen(js_name = setSndNoteParam)]
    pub fn set_snd_note_param(&mut self, part: u8, key: u8, offset: u8, value: u8) {
        self.inner.set_snd_note_param(part, key, offset, value);
    }

    /// Set a nibblized SN-D Note parameter (4 bytes).
    #[wasm_bindgen(js_name = setSndNoteNibParam)]
    pub fn set_snd_note_nib_param(&mut self, part: u8, key: u8, offset: u8, value: u16) {
        self.inner.set_snd_note_nib_param(part, key, offset, value);
    }

    /// Parse an SN-D Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applySndCommon)]
    pub fn apply_snd_common(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::sn_drum::parse_snd_common(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Parse an SN-D Note dump and return as a JS object.
    #[wasm_bindgen(js_name = applySndNote)]
    pub fn apply_snd_note(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::sn_drum::parse_snd_note(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Build an RQ1 to read the SN-D Common block for a part.
    #[wasm_bindgen(js_name = buildSndCommonRequest)]
    pub fn build_snd_common_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_snd_common_request(part)
    }

    /// Build an RQ1 to read an SN-D Note block for a part and key.
    #[wasm_bindgen(js_name = buildSndNoteRequest)]
    pub fn build_snd_note_request(&self, part: u8, key: u8) -> Vec<u8> {
        self.inner.build_snd_note_request(part, key)
    }

    // -- PCM Synth Tone Edit -----------------------------------------------

    /// Set a single PCM Synth Common parameter.
    #[wasm_bindgen(js_name = setPcmsCommonParam)]
    pub fn set_pcms_common_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_pcms_common_param(part, offset, value);
    }

    /// Set a nibblized PCM Synth Common parameter (4 bytes).
    #[wasm_bindgen(js_name = setPcmsCommonNibParam)]
    pub fn set_pcms_common_nib_param(&mut self, part: u8, offset: u8, value: u16) {
        self.inner.set_pcms_common_nib_param(part, offset, value);
    }

    /// Set a single PCM Synth PMT parameter.
    #[wasm_bindgen(js_name = setPcmsPmtParam)]
    pub fn set_pcms_pmt_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_pcms_pmt_param(part, offset, value);
    }

    /// Set a single PCM Synth Partial parameter.
    #[wasm_bindgen(js_name = setPcmsPartialParam)]
    pub fn set_pcms_partial_param(&mut self, part: u8, partial: u8, offset: u16, value: u8) {
        self.inner
            .set_pcms_partial_param(part, partial, offset, value);
    }

    /// Set a nibblized PCM Synth Partial parameter (4 bytes, e.g. wave number).
    #[wasm_bindgen(js_name = setPcmsPartialNibParam)]
    pub fn set_pcms_partial_nib_param(&mut self, part: u8, partial: u8, offset: u16, value: u16) {
        self.inner
            .set_pcms_partial_nib_param(part, partial, offset, value);
    }

    /// Set a nibblized PCM Synth Partial parameter (2 bytes, e.g. delay time).
    #[wasm_bindgen(js_name = setPcmsPartialNib2Param)]
    pub fn set_pcms_partial_nib2_param(&mut self, part: u8, partial: u8, offset: u16, value: u8) {
        self.inner
            .set_pcms_partial_nib2_param(part, partial, offset, value);
    }

    /// Set a single PCM Synth Common2 parameter.
    #[wasm_bindgen(js_name = setPcmsCommon2Param)]
    pub fn set_pcms_common2_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_pcms_common2_param(part, offset, value);
    }

    /// Set a nibblized PCM Synth Common2 parameter (4 bytes).
    #[wasm_bindgen(js_name = setPcmsCommon2NibParam)]
    pub fn set_pcms_common2_nib_param(&mut self, part: u8, offset: u8, value: u16) {
        self.inner.set_pcms_common2_nib_param(part, offset, value);
    }

    /// Parse a PCM Synth Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsCommon)]
    pub fn apply_pcms_common(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::pcm_synth::parse_pcms_common(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Parse a PCM Synth PMT dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsPmt)]
    pub fn apply_pcms_pmt(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::pcm_synth::parse_pcms_pmt(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Parse a PCM Synth Partial dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsPartial)]
    pub fn apply_pcms_partial(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::pcm_synth::parse_pcms_partial(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Parse a PCM Synth Common2 dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsCommon2)]
    pub fn apply_pcms_common2(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::pcm_synth::parse_pcms_common2(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Build an RQ1 to read the PCM Synth Common block for a part.
    #[wasm_bindgen(js_name = buildPcmsCommonRequest)]
    pub fn build_pcms_common_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_pcms_common_request(part)
    }

    /// Build an RQ1 to read the PCM Synth PMT block for a part.
    #[wasm_bindgen(js_name = buildPcmsPmtRequest)]
    pub fn build_pcms_pmt_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_pcms_pmt_request(part)
    }

    /// Build an RQ1 to read a PCM Synth Partial block for a part.
    #[wasm_bindgen(js_name = buildPcmsPartialRequest)]
    pub fn build_pcms_partial_request(&self, part: u8, partial: u8) -> Vec<u8> {
        self.inner.build_pcms_partial_request(part, partial)
    }

    /// Build an RQ1 to read the PCM Synth Common2 block for a part.
    #[wasm_bindgen(js_name = buildPcmsCommon2Request)]
    pub fn build_pcms_common2_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_pcms_common2_request(part)
    }

    // -- PCM Drum Kit ------------------------------------------------------

    /// Set a single PCM Drum Kit Common parameter.
    #[wasm_bindgen(js_name = setPcmdCommonParam)]
    pub fn set_pcmd_common_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_pcmd_common_param(part, offset, value);
    }

    /// Set a single PCM Drum Kit Partial parameter.
    #[wasm_bindgen(js_name = setPcmdPartialParam)]
    pub fn set_pcmd_partial_param(&mut self, part: u8, key: u8, offset: u16, value: u8) {
        self.inner.set_pcmd_partial_param(part, key, offset, value);
    }

    /// Set a nibblized PCM Drum Kit Partial parameter (4 bytes, e.g. wave number).
    #[wasm_bindgen(js_name = setPcmdPartialNibParam)]
    pub fn set_pcmd_partial_nib_param(&mut self, part: u8, key: u8, offset: u16, value: u16) {
        self.inner
            .set_pcmd_partial_nib_param(part, key, offset, value);
    }

    /// Set a single PCM Drum Kit Common2 parameter.
    #[wasm_bindgen(js_name = setPcmdCommon2Param)]
    pub fn set_pcmd_common2_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_pcmd_common2_param(part, offset, value);
    }

    /// Parse a PCM Drum Kit Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmdCommon)]
    pub fn apply_pcmd_common(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::pcm_drum::parse_pcmd_common(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Parse a PCM Drum Kit Partial dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmdPartial)]
    pub fn apply_pcmd_partial(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::pcm_drum::parse_pcmd_partial(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Parse a PCM Drum Kit Common2 dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmdCommon2)]
    pub fn apply_pcmd_common2(&mut self, data: &[u8]) -> JsValue {
        let parsed = integral_core::pcm_drum::parse_pcmd_common2(data);
        serde_wasm_bindgen::to_value(&parsed).unwrap_or(JsValue::NULL)
    }

    /// Build an RQ1 to read the PCM Drum Kit Common block for a part.
    #[wasm_bindgen(js_name = buildPcmdCommonRequest)]
    pub fn build_pcmd_common_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_pcmd_common_request(part)
    }

    /// Build an RQ1 to read a PCM Drum Kit Partial block for a part and key.
    #[wasm_bindgen(js_name = buildPcmdPartialRequest)]
    pub fn build_pcmd_partial_request(&self, part: u8, key: u8) -> Vec<u8> {
        self.inner.build_pcmd_partial_request(part, key)
    }

    /// Build an RQ1 to read the PCM Drum Kit Common2 block for a part.
    #[wasm_bindgen(js_name = buildPcmdCommon2Request)]
    pub fn build_pcmd_common2_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_pcmd_common2_request(part)
    }

    // -- MFX ---------------------------------------------------------------

    #[wasm_bindgen(js_name = setMfxParam)]
    pub fn set_mfx_param(&mut self, part: u8, param_offset: u8, value: u8) {
        self.inner.set_mfx_param(part, param_offset, value);
    }

    #[wasm_bindgen(js_name = setMfxNibParam)]
    pub fn set_mfx_nib_param(&mut self, part: u8, param_index: u8, value: i32) {
        self.inner.set_mfx_nib_param(part, param_index, value);
    }

    #[wasm_bindgen(js_name = buildMfxRequest)]
    pub fn build_mfx_request(&self, part: u8) -> Vec<u8> {
        self.inner.build_mfx_request(part)
    }

    /// Apply a full MFX block dump to the part's MFX state.
    #[wasm_bindgen(js_name = applyMfxBlock)]
    pub fn apply_mfx_block(&mut self, _part: u8, data: &[u8]) -> JsValue {
        let state = mfx::parse_mfx_block(data);
        serde_wasm_bindgen::to_value(&state).unwrap_or(JsValue::NULL)
    }
}

// ---------------------------------------------------------------------------
// SN-A instrument parameter table (standalone functions)
// ---------------------------------------------------------------------------

/// Get the instrument-specific parameter definitions for an SN-A category.
///
/// Returns a JS array of tuples `[index, name, min, max, default]`, or null
/// if the category name is not recognized.
#[wasm_bindgen]
pub fn sna_inst_params_by_category(category: &str) -> JsValue {
    match integral_core::sna_inst_params::sna_inst_type_by_category(category) {
        Some(def) => {
            let params: Vec<(u8, &str, u8, u8, u8)> = def
                .params
                .iter()
                .map(|p| (p.index, p.name, p.min, p.max, p.default_value))
                .collect();
            serde_wasm_bindgen::to_value(&params).unwrap_or(JsValue::NULL)
        }
        None => JsValue::NULL,
    }
}

/// Get all known SN-A instrument type names.
#[wasm_bindgen]
pub fn sna_all_inst_type_names() -> Vec<String> {
    integral_core::sna_inst_params::sna_all_inst_types()
        .iter()
        .map(|t| t.name.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// MFX parameter table (standalone functions)
// ---------------------------------------------------------------------------

/// Get MFX type names as an array.
#[wasm_bindgen]
pub fn mfx_type_names() -> Vec<String> {
    mfx_params::MFX_TYPE_NAMES
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Get the number of parameters for a given MFX type.
#[wasm_bindgen]
pub fn mfx_type_param_count(mfx_type: u8) -> u32 {
    mfx_params::mfx_type_def(mfx_type)
        .map(|d| d.params.len() as u32)
        .unwrap_or(0)
}

/// Get MFX parameter definition by type and parameter index (0-based).
/// Returns null if type or index is invalid.
#[wasm_bindgen]
pub struct WasmMfxParamDef {
    #[wasm_bindgen(readonly)]
    pub index: u8,
    #[wasm_bindgen(readonly)]
    pub min: i32,
    #[wasm_bindgen(readonly)]
    pub max: i32,
    #[wasm_bindgen(readonly, js_name = defaultValue)]
    pub default_value: i32,
    name: String,
}

#[wasm_bindgen]
impl WasmMfxParamDef {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
}

#[wasm_bindgen]
pub fn mfx_param_def(mfx_type: u8, param_idx: u32) -> Option<WasmMfxParamDef> {
    let def = mfx_params::mfx_type_def(mfx_type)?;
    let p = def.params.get(param_idx as usize)?;
    Some(WasmMfxParamDef {
        index: p.index,
        min: p.min,
        max: p.max,
        default_value: p.default_value,
        name: p.name.to_string(),
    })
}
