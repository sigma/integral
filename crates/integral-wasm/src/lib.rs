//! WASM bindings for the Integral Integra-7 control surface.

use integral_core::address::{Address, DataSize};
use integral_core::catalog;
use integral_core::device::DeviceState;
use integral_core::state::parse as state_parse;
use integral_core::sysex;
use integral_core::{
    fx_params, mfx, mfx_params, param_registry, params, params::part, pcm_drum, pcm_synth,
    sn_acoustic, sn_drum, sn_synth, tone_banks, user_write,
};
use wasm_bindgen::prelude::*;

mod expansion;
mod librarian;
mod sequencer;
mod waves;
pub use expansion::WasmExpansion;
pub use sequencer::WasmSequencer;
pub use waves::WasmWaveGroup;

/// Maximum valid part index (0–15 for 16 parts).
const MAX_PART: u8 = 15;

/// Validate a part index, returning a JsError if out of range.
fn check_part(part: u8) -> Result<(), JsError> {
    if part > MAX_PART {
        Err(JsError::new(&format!(
            "part index {part} out of range (0–{MAX_PART})"
        )))
    } else {
        Ok(())
    }
}

/// Validate a partial index (0–2 for SN-S, 0–3 for PCMS).
fn check_partial(partial: u8, max: u8) -> Result<(), JsError> {
    if partial > max {
        Err(JsError::new(&format!(
            "partial index {partial} out of range (0–{max})"
        )))
    } else {
        Ok(())
    }
}

/// Validate a Comp+EQ unit index (0–5).
fn check_comp_eq_unit(unit: u8) -> Result<(), JsError> {
    if unit > 5 {
        Err(JsError::new(&format!(
            "comp+eq unit {unit} out of range (0–5)"
        )))
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Returns the INTEGRA-7 device specification as a JSON string.
///
/// Contains all static device constants (part count, output names,
/// FX type labels, surround options, etc.).
#[wasm_bindgen(js_name = "deviceSpecJson")]
pub fn device_spec_json() -> Result<String, JsError> {
    serde_json::to_string(&integral_core::device_spec::INTEGRA7)
        .map_err(|e| JsError::new(&e.to_string()))
}

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
    manufacturer_id: u8,
    family_code_0: u8,
    family_code_1: u8,
    family_number_0: u8,
    family_number_1: u8,
    revision_0: u8,
    revision_1: u8,
    revision_2: u8,
    revision_3: u8,
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
    addr_0: u8,
    addr_1: u8,
    addr_2: u8,
    addr_3: u8,
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

/// Returns the address for a part's Receive Channel.
#[wasm_bindgen]
pub fn part_receive_channel_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, part::RECEIVE_CHANNEL))
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

/// Returns the address of the Solo Part param.
#[wasm_bindgen]
pub fn solo_part_address() -> Vec<u8> {
    addr_to_vec(params::SOLO_PART)
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

/// Returns the address of the Tone Category byte for a part's current tone,
/// or an empty vec for drum kits and unknown banks (no category byte).
#[wasm_bindgen]
pub fn tone_category_address(part_index: u8, bank_msb: u8) -> Vec<u8> {
    params::tone_category_address(part_index, bank_msb)
        .map(addr_to_vec)
        .unwrap_or_default()
}

/// Whether a bank MSB selects a drum kit, the only tone type that can feed
/// the Drum Comp+EQ units.
#[wasm_bindgen]
pub fn is_drum_kit_bank(bank_msb: u8) -> bool {
    params::is_drum_kit_bank(bank_msb)
}

/// Returns the RQ1 size for reading a tone name (12 bytes).
#[wasm_bindgen]
pub fn tone_name_size() -> Vec<u8> {
    params::TONE_NAME_SIZE.as_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// Tone edit blocks
// ---------------------------------------------------------------------------

/// Returns the RQ1 size for reading a tone's whole MFX block.
#[wasm_bindgen]
pub fn mfx_block_size() -> Vec<u8> {
    mfx::MFX_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s PCM Synth Common block.
#[wasm_bindgen]
pub fn pcms_common_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(pcm_synth::pcms_common_address(part_index))
}

/// Returns the RQ1 size for reading a PCM Synth Common block.
#[wasm_bindgen]
pub fn pcms_common_size() -> Vec<u8> {
    pcm_synth::PCMS_COMMON_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s PCM Synth PMT block.
#[wasm_bindgen]
pub fn pcms_pmt_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(pcm_synth::pcms_pmt_address(part_index))
}

/// Returns the RQ1 size for reading a PCM Synth PMT block.
#[wasm_bindgen]
pub fn pcms_pmt_size() -> Vec<u8> {
    pcm_synth::PCMS_PMT_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s PCM Synth Partial `partial` (0–3).
#[wasm_bindgen]
pub fn pcms_partial_address(part_index: u8, partial: u8) -> Vec<u8> {
    addr_to_vec(pcm_synth::pcms_partial_address(part_index, partial))
}

/// Returns the RQ1 size for reading a PCM Synth Partial block.
#[wasm_bindgen]
pub fn pcms_partial_size() -> Vec<u8> {
    pcm_synth::PCMS_PARTIAL_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s PCM Synth Common2 block.
#[wasm_bindgen]
pub fn pcms_common2_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(pcm_synth::pcms_common2_address(part_index))
}

/// Returns the RQ1 size for reading a PCM Synth Common2 block.
#[wasm_bindgen]
pub fn pcms_common2_size() -> Vec<u8> {
    pcm_synth::PCMS_COMMON2_BLOCK_SIZE.as_bytes().to_vec()
}

/// Converts between a wasm enum and the core enum it mirrors, both ways.
/// The core-to-wasm `match` is exhaustive over the core enum, so a variant
/// added in core fails the build until the wasm mirror lists it.
macro_rules! mirror_core_enum {
    ($wasm:ident <=> $core:ty { $($variant:ident),+ $(,)? }) => {
        impl From<$wasm> for $core {
            fn from(value: $wasm) -> Self {
                match value {
                    $($wasm::$variant => Self::$variant,)+
                }
            }
        }

        impl From<$core> for $wasm {
            fn from(value: $core) -> Self {
                type Core = $core;
                match value {
                    $(Core::$variant => Self::$variant,)+
                }
            }
        }
    };
}

/// Tone type of a Part's Temporary Tone, selecting which tone type block
/// (and so which MFX block) an edit addresses (mirrors [`params::ToneType`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToneType {
    /// PCM Synth Tone.
    PcmSynth,
    /// SuperNATURAL Synth Tone.
    SnSynth,
    /// SuperNATURAL Acoustic Tone.
    SnAcoustic,
    /// SuperNATURAL Drum Kit.
    SnDrum,
    /// PCM Drum Kit.
    PcmDrum,
}

mirror_core_enum!(ToneType <=> params::ToneType {
    PcmSynth,
    SnSynth,
    SnAcoustic,
    SnDrum,
    PcmDrum,
});

/// A field of a PCM Drum Kit WMT layer (mirrors [`pcm_drum::PcmdWmtField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmdWmtField {
    /// See [`pcm_drum::PcmdWmtField::Switch`].
    Switch,
    /// See [`pcm_drum::PcmdWmtField::WaveGroupType`].
    WaveGroupType,
    /// See [`pcm_drum::PcmdWmtField::WaveGroupId`].
    WaveGroupId,
    /// See [`pcm_drum::PcmdWmtField::WaveNumberL`].
    WaveNumberL,
    /// See [`pcm_drum::PcmdWmtField::WaveNumberR`].
    WaveNumberR,
    /// See [`pcm_drum::PcmdWmtField::WaveGain`].
    WaveGain,
    /// See [`pcm_drum::PcmdWmtField::WaveFxmSwitch`].
    WaveFxmSwitch,
    /// See [`pcm_drum::PcmdWmtField::WaveFxmColor`].
    WaveFxmColor,
    /// See [`pcm_drum::PcmdWmtField::WaveFxmDepth`].
    WaveFxmDepth,
    /// See [`pcm_drum::PcmdWmtField::WaveTempoSync`].
    WaveTempoSync,
    /// See [`pcm_drum::PcmdWmtField::CoarseTune`].
    CoarseTune,
    /// See [`pcm_drum::PcmdWmtField::FineTune`].
    FineTune,
    /// See [`pcm_drum::PcmdWmtField::Pan`].
    Pan,
    /// See [`pcm_drum::PcmdWmtField::RandomPanSwitch`].
    RandomPanSwitch,
    /// See [`pcm_drum::PcmdWmtField::AlternatePanSwitch`].
    AlternatePanSwitch,
    /// See [`pcm_drum::PcmdWmtField::Level`].
    Level,
    /// See [`pcm_drum::PcmdWmtField::VelocityRangeLower`].
    VelocityRangeLower,
    /// See [`pcm_drum::PcmdWmtField::VelocityRangeUpper`].
    VelocityRangeUpper,
    /// See [`pcm_drum::PcmdWmtField::VelocityFadeLower`].
    VelocityFadeLower,
    /// See [`pcm_drum::PcmdWmtField::VelocityFadeUpper`].
    VelocityFadeUpper,
}

mirror_core_enum!(PcmdWmtField <=> pcm_drum::PcmdWmtField {
    Switch,
    WaveGroupType,
    WaveGroupId,
    WaveNumberL,
    WaveNumberR,
    WaveGain,
    WaveFxmSwitch,
    WaveFxmColor,
    WaveFxmDepth,
    WaveTempoSync,
    CoarseTune,
    FineTune,
    Pan,
    RandomPanSwitch,
    AlternatePanSwitch,
    Level,
    VelocityRangeLower,
    VelocityRangeUpper,
    VelocityFadeLower,
    VelocityFadeUpper,
});

/// A non-WMT field of a PCM Drum Kit Partial (mirrors [`pcm_drum::PcmdPartialField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmdPartialField {
    /// See [`pcm_drum::PcmdPartialField::AssignType`].
    AssignType,
    /// See [`pcm_drum::PcmdPartialField::MuteGroup`].
    MuteGroup,
    /// See [`pcm_drum::PcmdPartialField::Level`].
    Level,
    /// See [`pcm_drum::PcmdPartialField::CoarseTune`].
    CoarseTune,
    /// See [`pcm_drum::PcmdPartialField::FineTune`].
    FineTune,
    /// See [`pcm_drum::PcmdPartialField::RandomPitchDepth`].
    RandomPitchDepth,
    /// See [`pcm_drum::PcmdPartialField::Pan`].
    Pan,
    /// See [`pcm_drum::PcmdPartialField::RandomPanDepth`].
    RandomPanDepth,
    /// See [`pcm_drum::PcmdPartialField::AlternatePanDepth`].
    AlternatePanDepth,
    /// See [`pcm_drum::PcmdPartialField::EnvMode`].
    EnvMode,
    /// See [`pcm_drum::PcmdPartialField::OutputLevel`].
    OutputLevel,
    /// See [`pcm_drum::PcmdPartialField::ChorusSend`].
    ChorusSend,
    /// See [`pcm_drum::PcmdPartialField::ReverbSend`].
    ReverbSend,
    /// See [`pcm_drum::PcmdPartialField::OutputAssign`].
    OutputAssign,
    /// See [`pcm_drum::PcmdPartialField::PitchBendRange`].
    PitchBendRange,
    /// See [`pcm_drum::PcmdPartialField::ReceiveExpression`].
    ReceiveExpression,
    /// See [`pcm_drum::PcmdPartialField::ReceiveHold1`].
    ReceiveHold1,
    /// See [`pcm_drum::PcmdPartialField::WmtVelocityControl`].
    WmtVelocityControl,
    /// See [`pcm_drum::PcmdPartialField::PitchEnvDepth`].
    PitchEnvDepth,
    /// See [`pcm_drum::PcmdPartialField::PitchEnvVelocitySens`].
    PitchEnvVelocitySens,
    /// See [`pcm_drum::PcmdPartialField::PitchEnvT1VelocitySens`].
    PitchEnvT1VelocitySens,
    /// See [`pcm_drum::PcmdPartialField::PitchEnvT4VelocitySens`].
    PitchEnvT4VelocitySens,
    /// See [`pcm_drum::PcmdPartialField::PitchEnvTime`].
    PitchEnvTime,
    /// See [`pcm_drum::PcmdPartialField::PitchEnvLevel`].
    PitchEnvLevel,
    /// See [`pcm_drum::PcmdPartialField::TvfFilterType`].
    TvfFilterType,
    /// See [`pcm_drum::PcmdPartialField::TvfCutoffFrequency`].
    TvfCutoffFrequency,
    /// See [`pcm_drum::PcmdPartialField::TvfCutoffVelocityCurve`].
    TvfCutoffVelocityCurve,
    /// See [`pcm_drum::PcmdPartialField::TvfCutoffVelocitySens`].
    TvfCutoffVelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvfResonance`].
    TvfResonance,
    /// See [`pcm_drum::PcmdPartialField::TvfResonanceVelocitySens`].
    TvfResonanceVelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvfEnvDepth`].
    TvfEnvDepth,
    /// See [`pcm_drum::PcmdPartialField::TvfEnvVelocityCurve`].
    TvfEnvVelocityCurve,
    /// See [`pcm_drum::PcmdPartialField::TvfEnvVelocitySens`].
    TvfEnvVelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvfEnvT1VelocitySens`].
    TvfEnvT1VelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvfEnvT4VelocitySens`].
    TvfEnvT4VelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvfEnvTime`].
    TvfEnvTime,
    /// See [`pcm_drum::PcmdPartialField::TvfEnvLevel`].
    TvfEnvLevel,
    /// See [`pcm_drum::PcmdPartialField::TvaLevelVelocityCurve`].
    TvaLevelVelocityCurve,
    /// See [`pcm_drum::PcmdPartialField::TvaLevelVelocitySens`].
    TvaLevelVelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvaEnvT1VelocitySens`].
    TvaEnvT1VelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvaEnvT4VelocitySens`].
    TvaEnvT4VelocitySens,
    /// See [`pcm_drum::PcmdPartialField::TvaEnvTime`].
    TvaEnvTime,
    /// See [`pcm_drum::PcmdPartialField::TvaEnvLevel`].
    TvaEnvLevel,
    /// See [`pcm_drum::PcmdPartialField::OneShotMode`].
    OneShotMode,
}

mirror_core_enum!(PcmdPartialField <=> pcm_drum::PcmdPartialField {
    AssignType,
    MuteGroup,
    Level,
    CoarseTune,
    FineTune,
    RandomPitchDepth,
    Pan,
    RandomPanDepth,
    AlternatePanDepth,
    EnvMode,
    OutputLevel,
    ChorusSend,
    ReverbSend,
    OutputAssign,
    PitchBendRange,
    ReceiveExpression,
    ReceiveHold1,
    WmtVelocityControl,
    PitchEnvDepth,
    PitchEnvVelocitySens,
    PitchEnvT1VelocitySens,
    PitchEnvT4VelocitySens,
    PitchEnvTime,
    PitchEnvLevel,
    TvfFilterType,
    TvfCutoffFrequency,
    TvfCutoffVelocityCurve,
    TvfCutoffVelocitySens,
    TvfResonance,
    TvfResonanceVelocitySens,
    TvfEnvDepth,
    TvfEnvVelocityCurve,
    TvfEnvVelocitySens,
    TvfEnvT1VelocitySens,
    TvfEnvT4VelocitySens,
    TvfEnvTime,
    TvfEnvLevel,
    TvaLevelVelocityCurve,
    TvaLevelVelocitySens,
    TvaEnvT1VelocitySens,
    TvaEnvT4VelocitySens,
    TvaEnvTime,
    TvaEnvLevel,
    OneShotMode,
});

/// A field of a PCM Synth Tone Common (mirrors [`pcm_synth::PcmsCommonField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmsCommonField {
    /// See [`pcm_synth::PcmsCommonField::ToneLevel`].
    ToneLevel,
    /// See [`pcm_synth::PcmsCommonField::TonePan`].
    TonePan,
    /// See [`pcm_synth::PcmsCommonField::TonePriority`].
    TonePriority,
    /// See [`pcm_synth::PcmsCommonField::CoarseTune`].
    CoarseTune,
    /// See [`pcm_synth::PcmsCommonField::FineTune`].
    FineTune,
    /// See [`pcm_synth::PcmsCommonField::OctaveShift`].
    OctaveShift,
    /// See [`pcm_synth::PcmsCommonField::StretchTuneDepth`].
    StretchTuneDepth,
    /// See [`pcm_synth::PcmsCommonField::AnalogFeel`].
    AnalogFeel,
    /// See [`pcm_synth::PcmsCommonField::MonoPoly`].
    MonoPoly,
    /// See [`pcm_synth::PcmsCommonField::LegatoSwitch`].
    LegatoSwitch,
    /// See [`pcm_synth::PcmsCommonField::LegatoRetrigger`].
    LegatoRetrigger,
    /// See [`pcm_synth::PcmsCommonField::PortamentoSwitch`].
    PortamentoSwitch,
    /// See [`pcm_synth::PcmsCommonField::PortamentoMode`].
    PortamentoMode,
    /// See [`pcm_synth::PcmsCommonField::PortamentoType`].
    PortamentoType,
    /// See [`pcm_synth::PcmsCommonField::PortamentoStart`].
    PortamentoStart,
    /// See [`pcm_synth::PcmsCommonField::PortamentoTime`].
    PortamentoTime,
    /// See [`pcm_synth::PcmsCommonField::CutoffOffset`].
    CutoffOffset,
    /// See [`pcm_synth::PcmsCommonField::ResonanceOffset`].
    ResonanceOffset,
    /// See [`pcm_synth::PcmsCommonField::AttackTimeOffset`].
    AttackTimeOffset,
    /// See [`pcm_synth::PcmsCommonField::ReleaseTimeOffset`].
    ReleaseTimeOffset,
    /// See [`pcm_synth::PcmsCommonField::VelocitySensOffset`].
    VelocitySensOffset,
    /// See [`pcm_synth::PcmsCommonField::PmtControlSwitch`].
    PmtControlSwitch,
    /// See [`pcm_synth::PcmsCommonField::PitchBendRangeUp`].
    PitchBendRangeUp,
    /// See [`pcm_synth::PcmsCommonField::PitchBendRangeDown`].
    PitchBendRangeDown,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlSource`].
    MatrixControlSource,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlDestination1`].
    MatrixControlDestination1,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlSens1`].
    MatrixControlSens1,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlDestination2`].
    MatrixControlDestination2,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlSens2`].
    MatrixControlSens2,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlDestination3`].
    MatrixControlDestination3,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlSens3`].
    MatrixControlSens3,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlDestination4`].
    MatrixControlDestination4,
    /// See [`pcm_synth::PcmsCommonField::MatrixControlSens4`].
    MatrixControlSens4,
}

mirror_core_enum!(PcmsCommonField <=> pcm_synth::PcmsCommonField {
    ToneLevel,
    TonePan,
    TonePriority,
    CoarseTune,
    FineTune,
    OctaveShift,
    StretchTuneDepth,
    AnalogFeel,
    MonoPoly,
    LegatoSwitch,
    LegatoRetrigger,
    PortamentoSwitch,
    PortamentoMode,
    PortamentoType,
    PortamentoStart,
    PortamentoTime,
    CutoffOffset,
    ResonanceOffset,
    AttackTimeOffset,
    ReleaseTimeOffset,
    VelocitySensOffset,
    PmtControlSwitch,
    PitchBendRangeUp,
    PitchBendRangeDown,
    MatrixControlSource,
    MatrixControlDestination1,
    MatrixControlSens1,
    MatrixControlDestination2,
    MatrixControlSens2,
    MatrixControlDestination3,
    MatrixControlSens3,
    MatrixControlDestination4,
    MatrixControlSens4,
});

/// A field of a PCM Synth Tone PMT (mirrors [`pcm_synth::PcmsPmtField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmsPmtField {
    /// See [`pcm_synth::PcmsPmtField::StructureType12`].
    StructureType12,
    /// See [`pcm_synth::PcmsPmtField::Booster12`].
    Booster12,
    /// See [`pcm_synth::PcmsPmtField::StructureType34`].
    StructureType34,
    /// See [`pcm_synth::PcmsPmtField::Booster34`].
    Booster34,
    /// See [`pcm_synth::PcmsPmtField::PmtVelocityControl`].
    PmtVelocityControl,
    /// See [`pcm_synth::PcmsPmtField::PartialSwitch`].
    PartialSwitch,
    /// See [`pcm_synth::PcmsPmtField::KeyRangeLower`].
    KeyRangeLower,
    /// See [`pcm_synth::PcmsPmtField::KeyRangeUpper`].
    KeyRangeUpper,
    /// See [`pcm_synth::PcmsPmtField::KeyFadeLower`].
    KeyFadeLower,
    /// See [`pcm_synth::PcmsPmtField::KeyFadeUpper`].
    KeyFadeUpper,
    /// See [`pcm_synth::PcmsPmtField::VelocityRangeLower`].
    VelocityRangeLower,
    /// See [`pcm_synth::PcmsPmtField::VelocityRangeUpper`].
    VelocityRangeUpper,
    /// See [`pcm_synth::PcmsPmtField::VelocityFadeLower`].
    VelocityFadeLower,
    /// See [`pcm_synth::PcmsPmtField::VelocityFadeUpper`].
    VelocityFadeUpper,
}

mirror_core_enum!(PcmsPmtField <=> pcm_synth::PcmsPmtField {
    StructureType12,
    Booster12,
    StructureType34,
    Booster34,
    PmtVelocityControl,
    PartialSwitch,
    KeyRangeLower,
    KeyRangeUpper,
    KeyFadeLower,
    KeyFadeUpper,
    VelocityRangeLower,
    VelocityRangeUpper,
    VelocityFadeLower,
    VelocityFadeUpper,
});

/// A field of a PCM Synth Tone Partial (mirrors [`pcm_synth::PcmsPartialField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmsPartialField {
    /// See [`pcm_synth::PcmsPartialField::Level`].
    Level,
    /// See [`pcm_synth::PcmsPartialField::CoarseTune`].
    CoarseTune,
    /// See [`pcm_synth::PcmsPartialField::FineTune`].
    FineTune,
    /// See [`pcm_synth::PcmsPartialField::RandomPitchDepth`].
    RandomPitchDepth,
    /// See [`pcm_synth::PcmsPartialField::Pan`].
    Pan,
    /// See [`pcm_synth::PcmsPartialField::PanKeyfollow`].
    PanKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::RandomPanDepth`].
    RandomPanDepth,
    /// See [`pcm_synth::PcmsPartialField::AlternatePanDepth`].
    AlternatePanDepth,
    /// See [`pcm_synth::PcmsPartialField::EnvMode`].
    EnvMode,
    /// See [`pcm_synth::PcmsPartialField::DelayMode`].
    DelayMode,
    /// See [`pcm_synth::PcmsPartialField::DelayTime`].
    DelayTime,
    /// See [`pcm_synth::PcmsPartialField::OutputLevel`].
    OutputLevel,
    /// See [`pcm_synth::PcmsPartialField::ChorusSend`].
    ChorusSend,
    /// See [`pcm_synth::PcmsPartialField::ReverbSend`].
    ReverbSend,
    /// See [`pcm_synth::PcmsPartialField::ReceiveBender`].
    ReceiveBender,
    /// See [`pcm_synth::PcmsPartialField::ReceiveExpression`].
    ReceiveExpression,
    /// See [`pcm_synth::PcmsPartialField::ReceiveHold1`].
    ReceiveHold1,
    /// See [`pcm_synth::PcmsPartialField::RedamperSwitch`].
    RedamperSwitch,
    /// See [`pcm_synth::PcmsPartialField::Control1Switch`].
    Control1Switch,
    /// See [`pcm_synth::PcmsPartialField::Control2Switch`].
    Control2Switch,
    /// See [`pcm_synth::PcmsPartialField::Control3Switch`].
    Control3Switch,
    /// See [`pcm_synth::PcmsPartialField::Control4Switch`].
    Control4Switch,
    /// See [`pcm_synth::PcmsPartialField::WaveGroupType`].
    WaveGroupType,
    /// See [`pcm_synth::PcmsPartialField::WaveGroupId`].
    WaveGroupId,
    /// See [`pcm_synth::PcmsPartialField::WaveNumberL`].
    WaveNumberL,
    /// See [`pcm_synth::PcmsPartialField::WaveNumberR`].
    WaveNumberR,
    /// See [`pcm_synth::PcmsPartialField::WaveGain`].
    WaveGain,
    /// See [`pcm_synth::PcmsPartialField::WaveFxmSwitch`].
    WaveFxmSwitch,
    /// See [`pcm_synth::PcmsPartialField::WaveFxmColor`].
    WaveFxmColor,
    /// See [`pcm_synth::PcmsPartialField::WaveFxmDepth`].
    WaveFxmDepth,
    /// See [`pcm_synth::PcmsPartialField::WaveTempoSync`].
    WaveTempoSync,
    /// See [`pcm_synth::PcmsPartialField::WavePitchKeyfollow`].
    WavePitchKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::PitchEnvDepth`].
    PitchEnvDepth,
    /// See [`pcm_synth::PcmsPartialField::PitchEnvVelocitySens`].
    PitchEnvVelocitySens,
    /// See [`pcm_synth::PcmsPartialField::PitchEnvT1VelocitySens`].
    PitchEnvT1VelocitySens,
    /// See [`pcm_synth::PcmsPartialField::PitchEnvT4VelocitySens`].
    PitchEnvT4VelocitySens,
    /// See [`pcm_synth::PcmsPartialField::PitchEnvTimeKeyfollow`].
    PitchEnvTimeKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::PitchEnvTime`].
    PitchEnvTime,
    /// See [`pcm_synth::PcmsPartialField::PitchEnvLevel`].
    PitchEnvLevel,
    /// See [`pcm_synth::PcmsPartialField::TvfFilterType`].
    TvfFilterType,
    /// See [`pcm_synth::PcmsPartialField::TvfCutoffFrequency`].
    TvfCutoffFrequency,
    /// See [`pcm_synth::PcmsPartialField::TvfCutoffKeyfollow`].
    TvfCutoffKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::TvfCutoffVelocityCurve`].
    TvfCutoffVelocityCurve,
    /// See [`pcm_synth::PcmsPartialField::TvfCutoffVelocitySens`].
    TvfCutoffVelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvfResonance`].
    TvfResonance,
    /// See [`pcm_synth::PcmsPartialField::TvfResonanceVelocitySens`].
    TvfResonanceVelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvDepth`].
    TvfEnvDepth,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvVelocityCurve`].
    TvfEnvVelocityCurve,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvVelocitySens`].
    TvfEnvVelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvT1VelocitySens`].
    TvfEnvT1VelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvT4VelocitySens`].
    TvfEnvT4VelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvTimeKeyfollow`].
    TvfEnvTimeKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvTime`].
    TvfEnvTime,
    /// See [`pcm_synth::PcmsPartialField::TvfEnvLevel`].
    TvfEnvLevel,
    /// See [`pcm_synth::PcmsPartialField::TvaBiasLevel`].
    TvaBiasLevel,
    /// See [`pcm_synth::PcmsPartialField::TvaBiasPosition`].
    TvaBiasPosition,
    /// See [`pcm_synth::PcmsPartialField::TvaBiasDirection`].
    TvaBiasDirection,
    /// See [`pcm_synth::PcmsPartialField::TvaLevelVelocityCurve`].
    TvaLevelVelocityCurve,
    /// See [`pcm_synth::PcmsPartialField::TvaLevelVelocitySens`].
    TvaLevelVelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvaEnvT1VelocitySens`].
    TvaEnvT1VelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvaEnvT4VelocitySens`].
    TvaEnvT4VelocitySens,
    /// See [`pcm_synth::PcmsPartialField::TvaEnvTimeKeyfollow`].
    TvaEnvTimeKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::TvaEnvTime`].
    TvaEnvTime,
    /// See [`pcm_synth::PcmsPartialField::TvaEnvLevel`].
    TvaEnvLevel,
    /// See [`pcm_synth::PcmsPartialField::Lfo1Waveform`].
    Lfo1Waveform,
    /// See [`pcm_synth::PcmsPartialField::Lfo1Rate`].
    Lfo1Rate,
    /// See [`pcm_synth::PcmsPartialField::Lfo1Offset`].
    Lfo1Offset,
    /// See [`pcm_synth::PcmsPartialField::Lfo1RateDetune`].
    Lfo1RateDetune,
    /// See [`pcm_synth::PcmsPartialField::Lfo1DelayTime`].
    Lfo1DelayTime,
    /// See [`pcm_synth::PcmsPartialField::Lfo1DelayTimeKeyfollow`].
    Lfo1DelayTimeKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::Lfo1FadeMode`].
    Lfo1FadeMode,
    /// See [`pcm_synth::PcmsPartialField::Lfo1FadeTime`].
    Lfo1FadeTime,
    /// See [`pcm_synth::PcmsPartialField::Lfo1KeyTrigger`].
    Lfo1KeyTrigger,
    /// See [`pcm_synth::PcmsPartialField::Lfo1PitchDepth`].
    Lfo1PitchDepth,
    /// See [`pcm_synth::PcmsPartialField::Lfo1TvfDepth`].
    Lfo1TvfDepth,
    /// See [`pcm_synth::PcmsPartialField::Lfo1TvaDepth`].
    Lfo1TvaDepth,
    /// See [`pcm_synth::PcmsPartialField::Lfo1PanDepth`].
    Lfo1PanDepth,
    /// See [`pcm_synth::PcmsPartialField::Lfo2Waveform`].
    Lfo2Waveform,
    /// See [`pcm_synth::PcmsPartialField::Lfo2Rate`].
    Lfo2Rate,
    /// See [`pcm_synth::PcmsPartialField::Lfo2Offset`].
    Lfo2Offset,
    /// See [`pcm_synth::PcmsPartialField::Lfo2RateDetune`].
    Lfo2RateDetune,
    /// See [`pcm_synth::PcmsPartialField::Lfo2DelayTime`].
    Lfo2DelayTime,
    /// See [`pcm_synth::PcmsPartialField::Lfo2DelayTimeKeyfollow`].
    Lfo2DelayTimeKeyfollow,
    /// See [`pcm_synth::PcmsPartialField::Lfo2FadeMode`].
    Lfo2FadeMode,
    /// See [`pcm_synth::PcmsPartialField::Lfo2FadeTime`].
    Lfo2FadeTime,
    /// See [`pcm_synth::PcmsPartialField::Lfo2KeyTrigger`].
    Lfo2KeyTrigger,
    /// See [`pcm_synth::PcmsPartialField::Lfo2PitchDepth`].
    Lfo2PitchDepth,
    /// See [`pcm_synth::PcmsPartialField::Lfo2TvfDepth`].
    Lfo2TvfDepth,
    /// See [`pcm_synth::PcmsPartialField::Lfo2TvaDepth`].
    Lfo2TvaDepth,
    /// See [`pcm_synth::PcmsPartialField::Lfo2PanDepth`].
    Lfo2PanDepth,
    /// See [`pcm_synth::PcmsPartialField::LfoStepType`].
    LfoStepType,
    /// See [`pcm_synth::PcmsPartialField::LfoStep`].
    LfoStep,
}

mirror_core_enum!(PcmsPartialField <=> pcm_synth::PcmsPartialField {
    Level,
    CoarseTune,
    FineTune,
    RandomPitchDepth,
    Pan,
    PanKeyfollow,
    RandomPanDepth,
    AlternatePanDepth,
    EnvMode,
    DelayMode,
    DelayTime,
    OutputLevel,
    ChorusSend,
    ReverbSend,
    ReceiveBender,
    ReceiveExpression,
    ReceiveHold1,
    RedamperSwitch,
    Control1Switch,
    Control2Switch,
    Control3Switch,
    Control4Switch,
    WaveGroupType,
    WaveGroupId,
    WaveNumberL,
    WaveNumberR,
    WaveGain,
    WaveFxmSwitch,
    WaveFxmColor,
    WaveFxmDepth,
    WaveTempoSync,
    WavePitchKeyfollow,
    PitchEnvDepth,
    PitchEnvVelocitySens,
    PitchEnvT1VelocitySens,
    PitchEnvT4VelocitySens,
    PitchEnvTimeKeyfollow,
    PitchEnvTime,
    PitchEnvLevel,
    TvfFilterType,
    TvfCutoffFrequency,
    TvfCutoffKeyfollow,
    TvfCutoffVelocityCurve,
    TvfCutoffVelocitySens,
    TvfResonance,
    TvfResonanceVelocitySens,
    TvfEnvDepth,
    TvfEnvVelocityCurve,
    TvfEnvVelocitySens,
    TvfEnvT1VelocitySens,
    TvfEnvT4VelocitySens,
    TvfEnvTimeKeyfollow,
    TvfEnvTime,
    TvfEnvLevel,
    TvaBiasLevel,
    TvaBiasPosition,
    TvaBiasDirection,
    TvaLevelVelocityCurve,
    TvaLevelVelocitySens,
    TvaEnvT1VelocitySens,
    TvaEnvT4VelocitySens,
    TvaEnvTimeKeyfollow,
    TvaEnvTime,
    TvaEnvLevel,
    Lfo1Waveform,
    Lfo1Rate,
    Lfo1Offset,
    Lfo1RateDetune,
    Lfo1DelayTime,
    Lfo1DelayTimeKeyfollow,
    Lfo1FadeMode,
    Lfo1FadeTime,
    Lfo1KeyTrigger,
    Lfo1PitchDepth,
    Lfo1TvfDepth,
    Lfo1TvaDepth,
    Lfo1PanDepth,
    Lfo2Waveform,
    Lfo2Rate,
    Lfo2Offset,
    Lfo2RateDetune,
    Lfo2DelayTime,
    Lfo2DelayTimeKeyfollow,
    Lfo2FadeMode,
    Lfo2FadeTime,
    Lfo2KeyTrigger,
    Lfo2PitchDepth,
    Lfo2TvfDepth,
    Lfo2TvaDepth,
    Lfo2PanDepth,
    LfoStepType,
    LfoStep,
});

/// A field of an SN Acoustic Tone Common (mirrors [`sn_acoustic::SnaCommonField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnaCommonField {
    /// See [`sn_acoustic::SnaCommonField::ToneLevel`].
    ToneLevel,
    /// See [`sn_acoustic::SnaCommonField::MonoPoly`].
    MonoPoly,
    /// See [`sn_acoustic::SnaCommonField::PortamentoTimeOffset`].
    PortamentoTimeOffset,
    /// See [`sn_acoustic::SnaCommonField::CutoffOffset`].
    CutoffOffset,
    /// See [`sn_acoustic::SnaCommonField::ResonanceOffset`].
    ResonanceOffset,
    /// See [`sn_acoustic::SnaCommonField::AttackTimeOffset`].
    AttackTimeOffset,
    /// See [`sn_acoustic::SnaCommonField::ReleaseTimeOffset`].
    ReleaseTimeOffset,
    /// See [`sn_acoustic::SnaCommonField::VibratoRate`].
    VibratoRate,
    /// See [`sn_acoustic::SnaCommonField::VibratoDepth`].
    VibratoDepth,
    /// See [`sn_acoustic::SnaCommonField::VibratoDelay`].
    VibratoDelay,
    /// See [`sn_acoustic::SnaCommonField::OctaveShift`].
    OctaveShift,
    /// See [`sn_acoustic::SnaCommonField::Category`].
    Category,
    /// See [`sn_acoustic::SnaCommonField::PhraseNumber`].
    PhraseNumber,
    /// See [`sn_acoustic::SnaCommonField::PhraseOctaveShift`].
    PhraseOctaveShift,
    /// See [`sn_acoustic::SnaCommonField::TfxSwitch`].
    TfxSwitch,
    /// See [`sn_acoustic::SnaCommonField::InstVariation`].
    InstVariation,
    /// See [`sn_acoustic::SnaCommonField::InstNumber`].
    InstNumber,
    /// See [`sn_acoustic::SnaCommonField::ModifyParameter`].
    ModifyParameter,
}

mirror_core_enum!(SnaCommonField <=> sn_acoustic::SnaCommonField {
    ToneLevel,
    MonoPoly,
    PortamentoTimeOffset,
    CutoffOffset,
    ResonanceOffset,
    AttackTimeOffset,
    ReleaseTimeOffset,
    VibratoRate,
    VibratoDepth,
    VibratoDelay,
    OctaveShift,
    Category,
    PhraseNumber,
    PhraseOctaveShift,
    TfxSwitch,
    InstVariation,
    InstNumber,
    ModifyParameter,
});

/// A field of an SN Drum Kit Common (mirrors [`sn_drum::SndCommonField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SndCommonField {
    /// See [`sn_drum::SndCommonField::KitLevel`].
    KitLevel,
    /// See [`sn_drum::SndCommonField::AmbienceLevel`].
    AmbienceLevel,
    /// See [`sn_drum::SndCommonField::PhraseNumber`].
    PhraseNumber,
    /// See [`sn_drum::SndCommonField::TfxSwitch`].
    TfxSwitch,
}

mirror_core_enum!(SndCommonField <=> sn_drum::SndCommonField {
    KitLevel,
    AmbienceLevel,
    PhraseNumber,
    TfxSwitch,
});

/// A field of an SN Drum Kit Note (mirrors [`sn_drum::SndNoteField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SndNoteField {
    /// See [`sn_drum::SndNoteField::InstNumber`].
    InstNumber,
    /// See [`sn_drum::SndNoteField::Level`].
    Level,
    /// See [`sn_drum::SndNoteField::Pan`].
    Pan,
    /// See [`sn_drum::SndNoteField::ChorusSend`].
    ChorusSend,
    /// See [`sn_drum::SndNoteField::ReverbSend`].
    ReverbSend,
    /// See [`sn_drum::SndNoteField::Tune`].
    Tune,
    /// See [`sn_drum::SndNoteField::Attack`].
    Attack,
    /// See [`sn_drum::SndNoteField::Decay`].
    Decay,
    /// See [`sn_drum::SndNoteField::Brilliance`].
    Brilliance,
    /// See [`sn_drum::SndNoteField::Variation`].
    Variation,
    /// See [`sn_drum::SndNoteField::DynamicRange`].
    DynamicRange,
    /// See [`sn_drum::SndNoteField::StereoWidth`].
    StereoWidth,
    /// See [`sn_drum::SndNoteField::OutputAssign`].
    OutputAssign,
}

mirror_core_enum!(SndNoteField <=> sn_drum::SndNoteField {
    InstNumber,
    Level,
    Pan,
    ChorusSend,
    ReverbSend,
    Tune,
    Attack,
    Decay,
    Brilliance,
    Variation,
    DynamicRange,
    StereoWidth,
    OutputAssign,
});

/// A header field of an MFX block (mirrors [`mfx::MfxHeaderField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MfxHeaderField {
    /// See [`mfx::MfxHeaderField::Type`].
    Type,
    /// See [`mfx::MfxHeaderField::ChorusSend`].
    ChorusSend,
    /// See [`mfx::MfxHeaderField::ReverbSend`].
    ReverbSend,
    /// See [`mfx::MfxHeaderField::ControlSource`].
    ControlSource,
    /// See [`mfx::MfxHeaderField::ControlSens`].
    ControlSens,
    /// See [`mfx::MfxHeaderField::ControlAssign`].
    ControlAssign,
}

mirror_core_enum!(MfxHeaderField <=> mfx::MfxHeaderField {
    Type,
    ChorusSend,
    ReverbSend,
    ControlSource,
    ControlSens,
    ControlAssign,
});

/// Returns the address of part `part_index`'s PCM Synth MFX block.
#[wasm_bindgen]
pub fn pcms_mfx_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(mfx::tone_mfx_block_address(
        part_index,
        params::ToneType::PcmSynth,
    ))
}

/// Returns the address of part `part_index`'s SN Synth MFX block.
#[wasm_bindgen]
pub fn sns_mfx_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(mfx::tone_mfx_block_address(
        part_index,
        params::ToneType::SnSynth,
    ))
}

/// Returns the address of part `part_index`'s SN Acoustic Common block.
#[wasm_bindgen]
pub fn sna_common_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(sn_acoustic::sna_common_address(part_index))
}

/// Returns the RQ1 size for reading an SN Acoustic Common block.
#[wasm_bindgen]
pub fn sna_common_size() -> Vec<u8> {
    sn_acoustic::SNA_COMMON_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s SN Acoustic MFX block.
#[wasm_bindgen]
pub fn sna_mfx_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(mfx::tone_mfx_block_address(
        part_index,
        params::ToneType::SnAcoustic,
    ))
}

/// Returns the address of part `part_index`'s SN Drum Kit Common block.
#[wasm_bindgen]
pub fn snd_common_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(sn_drum::snd_common_address(part_index))
}

/// Returns the RQ1 size for reading an SN Drum Kit Common block.
#[wasm_bindgen]
pub fn snd_common_size() -> Vec<u8> {
    sn_drum::SND_COMMON_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s SN Drum Kit MFX block.
#[wasm_bindgen]
pub fn snd_mfx_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(mfx::tone_mfx_block_address(
        part_index,
        params::ToneType::SnDrum,
    ))
}

/// Returns the address of part `part_index`'s SN Drum Kit Note block for `key`.
#[wasm_bindgen]
pub fn snd_note_address(part_index: u8, key: u8) -> Result<Vec<u8>, JsError> {
    Ok(addr_to_vec(sn_drum::snd_note_address(part_index, key)?))
}

/// Returns the RQ1 size for reading an SN Drum Kit Note block.
#[wasm_bindgen]
pub fn snd_note_size() -> Vec<u8> {
    sn_drum::SND_NOTE_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s PCM Drum Kit Common block.
#[wasm_bindgen]
pub fn pcmd_common_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(pcm_drum::pcmd_common_address(part_index))
}

/// Returns the RQ1 size for reading a PCM Drum Kit Common block.
#[wasm_bindgen]
pub fn pcmd_common_size() -> Vec<u8> {
    pcm_drum::PCMD_COMMON_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s PCM Drum Kit Common2 block.
#[wasm_bindgen]
pub fn pcmd_common2_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(pcm_drum::pcmd_common2_address(part_index))
}

/// Returns the RQ1 size for reading a PCM Drum Kit Common2 block.
#[wasm_bindgen]
pub fn pcmd_common2_size() -> Vec<u8> {
    pcm_drum::PCMD_COMMON2_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s PCM Drum Kit MFX block.
#[wasm_bindgen]
pub fn pcmd_mfx_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(mfx::tone_mfx_block_address(
        part_index,
        params::ToneType::PcmDrum,
    ))
}

/// Returns the address of part `part_index`'s PCM Drum Kit Partial block for `key`.
#[wasm_bindgen]
pub fn pcmd_partial_address(part_index: u8, key: u8) -> Result<Vec<u8>, JsError> {
    Ok(addr_to_vec(pcm_drum::pcmd_partial_address(
        part_index, key,
    )?))
}

/// Returns the RQ1 size for reading a PCM Drum Kit Partial block.
#[wasm_bindgen]
pub fn pcmd_partial_size() -> Vec<u8> {
    pcm_drum::PCMD_PARTIAL_BLOCK_SIZE.as_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// Catalog queries (undocumented)
// ---------------------------------------------------------------------------

/// Build a catalog query for all User Studio Set names.
///
/// Returns the raw SysEx bytes to send. The device responds with one DT1
/// message per User Studio Set, as many as the device spec declares.
#[wasm_bindgen]
pub fn build_studio_set_catalog_request(device_id: u8) -> Vec<u8> {
    let count = integral_core::device_spec::INTEGRA7.user_studio_set_count();
    catalog::build_studio_set_catalog_request(device_id, 0, u8::try_from(count).unwrap_or(u8::MAX))
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
// User-memory write (undocumented)
// ---------------------------------------------------------------------------

/// Build the request that saves a part's Temporary tone into a user slot.
/// `part` is 0-indexed.
#[wasm_bindgen]
pub fn build_tone_write_request(
    device_id: u8,
    part: u8,
    msb: u8,
    lsb: u8,
    pc: u8,
) -> Result<Vec<u8>, JsError> {
    check_part(part)?;
    Ok(user_write::build_write_request(
        device_id,
        user_write::BankSelect { msb, lsb, pc },
        user_write::WriteSource::Part(part),
    ))
}

/// Build the request that saves the Temporary Studio Set into user slot `pc`
/// (0-indexed).
#[wasm_bindgen]
pub fn build_studio_set_write_request(device_id: u8, pc: u8) -> Vec<u8> {
    user_write::build_write_request(
        device_id,
        user_write::BankSelect {
            msb: catalog::STUDIO_SET_MSB,
            lsb: catalog::STUDIO_SET_LSB,
            pc,
        },
        user_write::WriteSource::StudioSet,
    )
}

/// How long a write waits for its "write finished" reply once sent (ms).
#[wasm_bindgen]
pub fn write_timeout_ms() -> f64 {
    integral_core::librarian::import::WRITE_TIMEOUT_MS
}

/// A parsed write reply (see `parse_write_reply`).
#[wasm_bindgen]
pub struct WriteReply {
    /// True for the "write finished" reply, false for "write started".
    #[wasm_bindgen(readonly)]
    pub finished: bool,
    /// Destination Bank Select MSB.
    #[wasm_bindgen(readonly)]
    pub msb: u8,
    /// Destination Bank Select LSB.
    #[wasm_bindgen(readonly)]
    pub lsb: u8,
    /// Destination program number, 0-indexed.
    #[wasm_bindgen(readonly)]
    pub pc: u8,
    /// Source part (0-indexed), or null when the Studio Set was saved.
    #[wasm_bindgen(readonly)]
    pub part: Option<u8>,
}

/// Parse a DT1 (address + data) as a write reply; null if it is not one.
#[wasm_bindgen]
pub fn parse_write_reply(addr: &[u8], data: &[u8]) -> Option<WriteReply> {
    let [a, b, c, d] = *addr else {
        return None;
    };
    let reply = user_write::parse_write_reply(&Address::new(a, b, c, d), data)?;
    Some(WriteReply {
        finished: reply.phase == user_write::WritePhase::Finished,
        msb: reply.target.msb,
        lsb: reply.target.lsb,
        pc: reply.target.pc,
        part: match reply.source {
            user_write::WriteSource::Part(part) => Some(part),
            user_write::WriteSource::StudioSet => None,
        },
    })
}

// ---------------------------------------------------------------------------
// EQ addresses
// ---------------------------------------------------------------------------

/// Returns the address of part `part_index`'s EQ params (the switch, first).
#[wasm_bindgen]
pub fn part_eq_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_eq_address(part_index, params::part_eq::SWITCH))
}

// ---------------------------------------------------------------------------
// Chorus / Reverb addresses
// ---------------------------------------------------------------------------

/// Returns the address of the Chorus core params (the type, first).
#[wasm_bindgen]
pub fn chorus_core_address() -> Vec<u8> {
    addr_to_vec(params::chorus_address(params::chorus::TYPE))
}

/// Returns the address of the first nibblized Chorus parameter.
#[wasm_bindgen]
pub fn chorus_params_address() -> Vec<u8> {
    addr_to_vec(params::chorus_address(params::chorus::PARAM_BASE))
}

/// Returns the RQ1 size for reading all nibblized Chorus parameters.
#[wasm_bindgen]
pub fn chorus_params_size() -> Vec<u8> {
    params::CHORUS_PARAMS_SIZE.as_bytes().to_vec()
}

/// Decodes the nibblized Chorus parameters read at [`chorus_params_address`].
#[wasm_bindgen]
pub fn decode_chorus_params(data: &[u8]) -> Vec<i32> {
    state_parse::decode_nib_params(data, params::CHORUS_PARAM_COUNT)
}

#[wasm_bindgen]
pub fn chorus_switch_address() -> Vec<u8> {
    addr_to_vec(params::CHORUS_SWITCH)
}

#[wasm_bindgen]
pub fn chorus_core_size() -> Vec<u8> {
    params::CHORUS_CORE_SIZE.as_bytes().to_vec()
}

/// Returns the address of the Reverb core params (the type, first).
#[wasm_bindgen]
pub fn reverb_core_address() -> Vec<u8> {
    addr_to_vec(params::reverb_address(params::reverb::TYPE))
}

/// Returns the address of the first nibblized Reverb parameter.
#[wasm_bindgen]
pub fn reverb_params_address() -> Vec<u8> {
    addr_to_vec(params::reverb_address(params::reverb::PARAM_BASE))
}

/// Returns the RQ1 size for reading all nibblized Reverb parameters.
#[wasm_bindgen]
pub fn reverb_params_size() -> Vec<u8> {
    params::REVERB_PARAMS_SIZE.as_bytes().to_vec()
}

/// Decodes the nibblized Reverb parameters read at [`reverb_params_address`].
#[wasm_bindgen]
pub fn decode_reverb_params(data: &[u8]) -> Vec<i32> {
    state_parse::decode_nib_params(data, params::REVERB_PARAM_COUNT)
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
// Drum Comp+EQ and Motional Surround addresses
// ---------------------------------------------------------------------------

/// Returns the address of the Drum Comp/EQ common params (the switch, first).
#[wasm_bindgen]
pub fn drum_comp_eq_common_address() -> Vec<u8> {
    addr_to_vec(params::DRUM_COMP_EQ_SWITCH)
}

/// Returns the RQ1 size for reading the Drum Comp/EQ common params.
#[wasm_bindgen]
pub fn drum_comp_eq_common_size() -> Vec<u8> {
    params::DRUM_COMP_EQ_COMMON_SIZE.as_bytes().to_vec()
}

/// Returns the address of the Comp+EQ block of part `part_index`'s tone
/// (bank MSB `bank_msb`), or empty if that tone isn't a drum kit.
#[wasm_bindgen]
pub fn comp_eq_block_address(part_index: u8, bank_msb: u8) -> Vec<u8> {
    params::comp_eq_block_address_for_bank(part_index, bank_msb)
        .map(addr_to_vec)
        .unwrap_or_default()
}

/// Returns the RQ1 size for reading a whole Comp+EQ block.
#[wasm_bindgen]
pub fn comp_eq_block_size() -> Vec<u8> {
    params::COMP_EQ_BLOCK_SIZE.as_bytes().to_vec()
}

/// Returns the address of the Motional Surround common block.
#[wasm_bindgen]
pub fn surround_common_address() -> Vec<u8> {
    addr_to_vec(params::SURROUND_BASE)
}

/// Returns the RQ1 size for reading the Motional Surround common block.
#[wasm_bindgen]
pub fn surround_common_size() -> Vec<u8> {
    params::SURROUND_COMMON_SIZE.as_bytes().to_vec()
}

/// Returns the address of part `part_index`'s surround params (L-R, first).
#[wasm_bindgen]
pub fn part_surround_address(part_index: u8) -> Vec<u8> {
    addr_to_vec(params::part_address(part_index, params::part_surround::LR))
}

/// Returns the RQ1 size for reading a part's surround params.
#[wasm_bindgen]
pub fn part_surround_size() -> Vec<u8> {
    params::PART_SURROUND_SIZE.as_bytes().to_vec()
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

/// Returns the address of the Master EQ params (Low Freq, first).
#[wasm_bindgen]
pub fn master_eq_address() -> Vec<u8> {
    addr_to_vec(params::master_eq_address(params::master_eq::LOW_FREQ))
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

// ---------------------------------------------------------------------------
// Tone bank definitions
// ---------------------------------------------------------------------------

/// A tone bank exposed to JS.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmToneBank {
    /// Bank Select MSB.
    #[wasm_bindgen(readonly)]
    pub msb: u8,
    label: String,
    lsbs: Vec<u8>,
    group: String,
    badge: String,
    expansion: Option<WasmExpansion>,
}

impl From<&'static tone_banks::ToneBank> for WasmToneBank {
    fn from(b: &'static tone_banks::ToneBank) -> Self {
        Self {
            msb: b.msb,
            label: b.label.to_string(),
            lsbs: b.lsbs.to_vec(),
            group: tone_banks::group_of(b).map_or("", |g| g.label).to_string(),
            badge: b.badge().to_string(),
            expansion: b.expansion.map(WasmExpansion::from),
        }
    }
}

#[wasm_bindgen]
impl WasmToneBank {
    /// Bank name within its group (e.g. "Preset", "SRX-05 Drum").
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> String {
        self.label.clone()
    }

    /// Bank Select LSBs; each holds up to 128 tones.
    #[wasm_bindgen(getter)]
    pub fn lsbs(&self) -> Vec<u8> {
        self.lsbs.clone()
    }

    /// Label of the group holding this bank (e.g. "SN Acoustic").
    #[wasm_bindgen(getter)]
    pub fn group(&self) -> String {
        self.group.clone()
    }

    /// Short tag for this bank's tones in mixed lists (e.g. "SN-A",
    /// "SRX-05").
    #[wasm_bindgen(getter)]
    pub fn badge(&self) -> String {
        self.badge.clone()
    }

    /// The Expansion providing this bank, or `undefined` for built-in banks.
    #[wasm_bindgen(getter)]
    pub fn expansion(&self) -> Option<WasmExpansion> {
        self.expansion.clone()
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
    /// Group display name.
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
        self.banks.get(index).cloned()
    }
}

/// Returns all tone bank groups, in display order. Banks of the Expansion
/// group follow the Expansion catalog order, tone bank before drum bank.
#[wasm_bindgen]
pub fn tone_bank_groups() -> Vec<WasmToneBankGroup> {
    tone_banks::TONE_BANK_GROUPS
        .iter()
        .map(|g| WasmToneBankGroup {
            label: g.label.to_string(),
            banks: g.banks.iter().map(WasmToneBank::from).collect(),
        })
        .collect()
}

/// The user bank a tone selected by `msb` saves into (e.g. ExPCM and GM2
/// tones save into the PCM Synth user bank). Returns null for unknown MSBs.
#[wasm_bindgen]
pub fn user_tone_bank(msb: u8) -> Option<WasmToneBank> {
    tone_banks::user_bank_for(msb).map(WasmToneBank::from)
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

    #[derive(serde::Serialize)]
    struct Entry<'a> {
        msb: u8,
        lsb: u8,
        pc: u8,
        name: &'a str,
        category: u8,
    }

    let tones = factory_tones(msb, lsb);
    let entries: Vec<Entry<'_>> = tones
        .iter()
        .map(|t| Entry {
            msb: t.msb,
            lsb: t.lsb,
            pc: t.pc,
            name: t.name,
            category: t.category,
        })
        .collect();
    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
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
    /// Nibblized slot index this knob reads and writes.
    #[wasm_bindgen(readonly)]
    pub index: u8,
    /// Minimum stored value.
    #[wasm_bindgen(readonly)]
    pub min: i32,
    /// Maximum stored value.
    #[wasm_bindgen(readonly)]
    pub max: i32,
    /// Default stored value.
    #[wasm_bindgen(readonly, js_name = defaultValue)]
    pub default_value: i32,
    inner: fx_params::FxParamDef,
}

#[wasm_bindgen]
impl WasmFxParamDef {
    /// Display name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.inner.name.to_string()
    }

    /// Format a stored value for display.
    #[wasm_bindgen(js_name = formatValue)]
    pub fn format_value(&self, value: i32) -> String {
        self.inner.format_value(value)
    }

    /// Slot of the msec/Hz-vs-note switch, if this parameter has one.
    #[wasm_bindgen(getter, js_name = syncIndex)]
    pub fn sync_index(&self) -> Option<u8> {
        self.inner.sync.map(|s| s.switch_index)
    }

    /// Switch position labels (`[own slot, note]`); empty without a switch.
    #[wasm_bindgen(getter, js_name = syncLabels)]
    pub fn sync_labels(&self) -> Vec<String> {
        self.inner
            .sync
            .map(|s| s.labels.iter().map(|l| l.to_string()).collect())
            .unwrap_or_default()
    }

    /// Whether the block's stored `slots` put this parameter in note mode.
    #[wasm_bindgen(js_name = noteMode)]
    pub fn note_mode(&self, slots: &[i32]) -> bool {
        self.inner.note_mode(slots)
    }

    /// The note-mode view of this parameter (edits the note slot).
    #[wasm_bindgen(js_name = noteParam)]
    pub fn note_param(&self) -> Option<WasmFxParamDef> {
        self.inner.note_param().map(wrap_fx_param)
    }
}

fn wrap_fx_param(p: fx_params::FxParamDef) -> WasmFxParamDef {
    WasmFxParamDef {
        index: p.index,
        min: p.min,
        max: p.max,
        default_value: p.default_value,
        inner: p,
    }
}

fn wrap_fx_params(params: &[fx_params::FxParamDef]) -> Vec<WasmFxParamDef> {
    params.iter().cloned().map(wrap_fx_param).collect()
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
    pub fn set_part_level(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_level(part, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartPan)]
    pub fn set_part_pan(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_pan(part, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartMute)]
    pub fn set_part_mute(&mut self, part: u8, muted: bool) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_mute(part, muted);
        Ok(())
    }

    #[wasm_bindgen(js_name = togglePartMute)]
    pub fn toggle_part_mute(&mut self, part: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.toggle_part_mute(part);
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartChorusSend)]
    pub fn set_part_chorus_send(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_chorus_send(part, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartReverbSend)]
    pub fn set_part_reverb_send(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_reverb_send(part, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartReceiveChannel)]
    pub fn set_part_receive_channel(&mut self, part: u8, channel: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_receive_channel(part, channel);
        Ok(())
    }

    /// Set a part's output assign (0=A, 1=B, 2=C, 3=D, 4–11=1–8).
    #[wasm_bindgen(js_name = setPartOutputAssign)]
    pub fn set_part_output_assign(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_output_assign(part, value);
        Ok(())
    }

    /// Set a part's Temporary tone name (12 ASCII chars) on the device.
    /// Returns false if the part's bank doesn't map to a known tone type.
    #[wasm_bindgen(js_name = renameTone)]
    pub fn rename_tone(&mut self, part: u8, name: &str) -> Result<bool, JsError> {
        check_part(part)?;
        Ok(self.inner.rename_tone(part, name))
    }

    /// Set the Temporary Studio Set name (16 ASCII chars) on the device.
    #[wasm_bindgen(js_name = renameStudioSet)]
    pub fn rename_studio_set(&mut self, name: &str) {
        self.inner.rename_studio_set(name);
    }

    #[wasm_bindgen(js_name = changePartTone)]
    pub fn change_part_tone(&mut self, part: u8, msb: u8, lsb: u8, pc: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.change_part_tone(part, msb, lsb, pc);
        Ok(())
    }

    #[wasm_bindgen(js_name = setMasterLevel)]
    pub fn set_master_level(&mut self, value: u8) {
        self.inner.set_master_level(value);
    }

    // -- EQ setters --------------------------------------------------------

    #[wasm_bindgen(js_name = setPartEqParam)]
    pub fn set_part_eq_param(
        &mut self,
        part: u8,
        param_offset: u8,
        value: u8,
    ) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_eq_param(part, param_offset, value);
        Ok(())
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

    // -- Direct state mutation (for load responses) ------------------------

    /// Patch the mixer state with a parsed part dump.
    #[wasm_bindgen(js_name = applyPartDump)]
    pub fn apply_part_dump(&mut self, part: u8, data: &[u8]) -> Result<(), JsError> {
        check_part(part)?;
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
        Ok(())
    }

    /// Patch the part EQ with a parsed dump.
    #[wasm_bindgen(js_name = applyPartEqDump)]
    pub fn apply_part_eq_dump(&mut self, part: u8, data: &[u8]) -> Result<(), JsError> {
        check_part(part)?;
        let eq = state_parse::parse_part_eq_dump(data);
        self.inner.state_mut().parts[part as usize].eq = eq;
        Ok(())
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
    #[wasm_bindgen(js_name = setPartToneCategory)]
    pub fn set_part_tone_category(&mut self, part: u8, category: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.state_mut().parts[part as usize].tone_category = category;
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartToneName)]
    pub fn set_part_tone_name(&mut self, part: u8, name: &str) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.state_mut().parts[part as usize].tone_name = name.to_string();
        Ok(())
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
    pub fn set_drum_comp_eq_output_assign(&mut self, unit: u8, value: u8) -> Result<(), JsError> {
        check_comp_eq_unit(unit)?;
        self.inner.set_drum_comp_eq_output_assign(unit, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = setCompEqParam)]
    pub fn set_comp_eq_param(
        &mut self,
        unit: u8,
        param_offset: u8,
        value: u8,
    ) -> Result<(), JsError> {
        check_comp_eq_unit(unit)?;
        self.inner.set_comp_eq_param(unit, param_offset, value);
        Ok(())
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
    pub fn set_part_surround_lr(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner
            .set_part_surround_param(part, integral_core::params::part_surround::LR, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartSurroundFb)]
    pub fn set_part_surround_fb(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner
            .set_part_surround_param(part, integral_core::params::part_surround::FB, value);
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartSurroundWidth)]
    pub fn set_part_surround_width(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_surround_param(
            part,
            integral_core::params::part_surround::WIDTH,
            value,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name = setPartSurroundAmbienceSend)]
    pub fn set_part_surround_ambience_send(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_part_surround_param(
            part,
            integral_core::params::part_surround::AMBIENCE_SEND,
            value,
        );
        Ok(())
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
    pub fn apply_part_surround(
        &mut self,
        part: u8,
        lr: u8,
        fb: u8,
        width: u8,
        ambience_send: u8,
    ) -> Result<(), JsError> {
        check_part(part)?;
        let ps = &mut self.inner.state_mut().surround.parts[part as usize];
        ps.lr = lr;
        ps.fb = fb;
        ps.width = width;
        ps.ambience_send = ambience_send;
        Ok(())
    }

    // -- SN-S Tone Edit ----------------------------------------------------

    /// Set a single SN-S Common parameter.
    #[wasm_bindgen(js_name = setSnsCommonParam)]
    pub fn set_sns_common_param(&mut self, part: u8, offset: u8, value: u8) {
        self.inner.set_sns_common_param(part, offset, value);
    }

    /// Set a single SN-S Partial parameter.
    #[wasm_bindgen(js_name = setSnsPartialParam)]
    pub fn set_sns_partial_param(
        &mut self,
        part: u8,
        partial: u8,
        offset: u8,
        value: u8,
    ) -> Result<(), JsError> {
        check_part(part)?;
        check_partial(partial, sn_synth::SNS_PARTIAL_COUNT as u8 - 1)?;
        self.inner
            .set_sns_partial_param(part, partial, offset, value);
        Ok(())
    }

    /// Set a nibblized SN-S Partial parameter (4 bytes, e.g. wave number).
    #[wasm_bindgen(js_name = setSnsPartialNibParam)]
    pub fn set_sns_partial_nib_param(
        &mut self,
        part: u8,
        partial: u8,
        offset: u8,
        value: u16,
    ) -> Result<(), JsError> {
        check_part(part)?;
        check_partial(partial, sn_synth::SNS_PARTIAL_COUNT as u8 - 1)?;
        self.inner
            .set_sns_partial_nib_param(part, partial, offset, value);
        Ok(())
    }

    /// Start reading a part's SN-S tone into the core cache, unless it is
    /// already read or being read for the part's current tone. Returns
    /// whether a new read started. Lost replies are requested again by
    /// `drain`.
    #[wasm_bindgen(js_name = loadSnsTone)]
    pub fn load_sns_tone(&mut self, part: u8) -> Result<bool, JsError> {
        check_part(part)?;
        Ok(self.inner.load_sns_tone(part))
    }

    /// A part's cached SN-S tone (`{ common, partials }`), or `null` until
    /// every block of it has arrived.
    #[wasm_bindgen(js_name = snsTone)]
    pub fn sns_tone(&self, part: u8) -> Result<JsValue, JsError> {
        check_part(part)?;
        let Some(tone) = self.inner.sns_tone(part) else {
            return Ok(JsValue::NULL);
        };
        serde_wasm_bindgen::to_value(&tone)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    // -- SN-A Tone Edit ----------------------------------------------------

    /// Set value `index` (0-based; 0 unless the field is the Modify
    /// Parameters) of an SN-A Common field, sent nibblized when the field
    /// is.
    #[wasm_bindgen(js_name = setSnaCommonParam)]
    pub fn set_sna_common_param(
        &mut self,
        part: u8,
        field: SnaCommonField,
        index: u8,
        value: u16,
    ) -> Result<(), JsError> {
        check_part(part)?;
        Ok(self
            .inner
            .set_sna_common_param(part, field.into(), index, value)?)
    }

    /// Parse an SN-A Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applySnaCommon)]
    pub fn apply_sna_common(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = integral_core::sn_acoustic::parse_sna_common(data)
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    // -- SN-D Tone Edit ----------------------------------------------------

    /// Set an SN-D Common field.
    #[wasm_bindgen(js_name = setSndCommonParam)]
    pub fn set_snd_common_param(
        &mut self,
        part: u8,
        field: SndCommonField,
        value: u8,
    ) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_snd_common_param(part, field.into(), value);
        Ok(())
    }

    /// Set an SN-D Note field, sent nibblized when the field is.
    #[wasm_bindgen(js_name = setSndNoteParam)]
    pub fn set_snd_note_param(
        &mut self,
        part: u8,
        key: u8,
        field: SndNoteField,
        value: u16,
    ) -> Result<(), JsError> {
        check_part(part)?;
        Ok(self
            .inner
            .set_snd_note_param(part, key, field.into(), value)?)
    }

    /// Parse an SN-D Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applySndCommon)]
    pub fn apply_snd_common(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = sn_drum::parse_snd_common(data).map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    /// Parse an SN-D Note dump and return as a JS object.
    #[wasm_bindgen(js_name = applySndNote)]
    pub fn apply_snd_note(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = sn_drum::parse_snd_note(data).map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    // -- PCM Synth Tone Edit -----------------------------------------------

    /// Set value `index` (0-based; 0 unless the field holds one value per
    /// Matrix Control) of a PCM Synth Common field.
    #[wasm_bindgen(js_name = setPcmsCommonParam)]
    pub fn set_pcms_common_param(
        &mut self,
        part: u8,
        field: PcmsCommonField,
        index: u8,
        value: u8,
    ) -> Result<(), JsError> {
        check_part(part)?;
        Ok(self
            .inner
            .set_pcms_common_param(part, field.into(), index, value)?)
    }

    /// Set value `index` (0-based; the Partial for the per-Partial fields,
    /// else 0) of a PCM Synth PMT field.
    #[wasm_bindgen(js_name = setPcmsPmtParam)]
    pub fn set_pcms_pmt_param(
        &mut self,
        part: u8,
        field: PcmsPmtField,
        index: u8,
        value: u8,
    ) -> Result<(), JsError> {
        check_part(part)?;
        Ok(self
            .inner
            .set_pcms_pmt_param(part, field.into(), index, value)?)
    }

    /// Set value `index` (0-based; 0 unless the field is an envelope's
    /// times or levels, a control's switches or the LFO steps) of a PCM
    /// Synth Partial field, sent nibblized when the field is.
    #[wasm_bindgen(js_name = setPcmsPartialParam)]
    pub fn set_pcms_partial_param(
        &mut self,
        part: u8,
        partial: u8,
        field: PcmsPartialField,
        index: u8,
        value: u16,
    ) -> Result<(), JsError> {
        check_part(part)?;
        check_partial(partial, pcm_synth::PCMS_PARTIAL_COUNT as u8 - 1)?;
        Ok(self
            .inner
            .set_pcms_partial_param(part, partial, field.into(), index, value)?)
    }

    /// Parse a PCM Synth Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsCommon)]
    pub fn apply_pcms_common(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = integral_core::pcm_synth::parse_pcms_common(data)
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    /// Parse a PCM Synth PMT dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsPmt)]
    pub fn apply_pcms_pmt(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = integral_core::pcm_synth::parse_pcms_pmt(data)
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    /// Parse a PCM Synth Partial dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsPartial)]
    pub fn apply_pcms_partial(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = integral_core::pcm_synth::parse_pcms_partial(data)
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    /// Parse a PCM Synth Common2 dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmsCommon2)]
    pub fn apply_pcms_common2(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = integral_core::pcm_synth::parse_pcms_common2(data)
            .map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    // -- PCM Drum Kit ------------------------------------------------------

    /// Set a part's PCM Drum Kit Level (0–127).
    #[wasm_bindgen(js_name = setPcmdKitLevel)]
    pub fn set_pcmd_kit_level(&mut self, part: u8, value: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_pcmd_kit_level(part, value);
        Ok(())
    }

    /// Set a field of WMT `wmt` (0-based) in the PCM Drum Kit Partial for `key`.
    #[wasm_bindgen(js_name = setPcmdWmtParam)]
    pub fn set_pcmd_wmt_param(
        &mut self,
        part: u8,
        key: u8,
        wmt: u8,
        field: PcmdWmtField,
        value: u16,
    ) -> Result<(), JsError> {
        check_part(part)?;
        Ok(self
            .inner
            .set_pcmd_wmt_param(part, key, wmt, field.into(), value)?)
    }

    /// Set element `index` (0-based; 0 unless the field is an envelope's
    /// times or levels) of a non-WMT field in the PCM Drum Kit Partial for `key`.
    #[wasm_bindgen(js_name = setPcmdPartialField)]
    pub fn set_pcmd_partial_field(
        &mut self,
        part: u8,
        key: u8,
        field: PcmdPartialField,
        index: u8,
        value: u8,
    ) -> Result<(), JsError> {
        check_part(part)?;
        Ok(self
            .inner
            .set_pcmd_partial_field(part, key, field.into(), index, value)?)
    }

    /// Parse a PCM Drum Kit Common dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmdCommon)]
    pub fn apply_pcmd_common(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed = pcm_drum::parse_pcmd_common(data).map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    /// Parse a PCM Drum Kit Partial dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmdPartial)]
    pub fn apply_pcmd_partial(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed =
            pcm_drum::parse_pcmd_partial(data).map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    /// Parse a PCM Drum Kit Common2 dump and return as a JS object.
    #[wasm_bindgen(js_name = applyPcmdCommon2)]
    pub fn apply_pcmd_common2(&mut self, data: &[u8]) -> Result<JsValue, JsError> {
        let parsed =
            pcm_drum::parse_pcmd_common2(data).map_err(|e| JsError::new(&e.to_string()))?;
        serde_wasm_bindgen::to_value(&parsed)
            .map_err(|e| JsError::new(&format!("serialization failed: {e}")))
    }

    // -- MFX ---------------------------------------------------------------

    /// Set instance `index` (the 0-based control slot for the per-control
    /// fields, 0 otherwise) of an MFX header field of the part's tone of type
    /// `tone_type`.
    #[wasm_bindgen(js_name = setMfxHeaderParam)]
    pub fn set_mfx_header_param(
        &mut self,
        part: u8,
        tone_type: ToneType,
        field: MfxHeaderField,
        index: u8,
        value: u8,
    ) -> Result<(), JsError> {
        Ok(self
            .inner
            .set_mfx_header_param(part, tone_type.into(), field.into(), index, value)?)
    }

    /// Set a nibblized MFX parameter (0-based index) of the part's tone of
    /// type `tone_type`.
    #[wasm_bindgen(js_name = setMfxNibParam)]
    pub fn set_mfx_nib_param(
        &mut self,
        part: u8,
        tone_type: ToneType,
        param_index: u8,
        value: i32,
    ) {
        self.inner
            .set_mfx_nib_param(part, tone_type.into(), param_index, value);
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

/// Which Control Source list a value is read against (mirrors
/// [`mfx_params::ControlSourceKind`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlSourceKind {
    /// See [`mfx_params::ControlSourceKind::Mfx`].
    Mfx,
    /// See [`mfx_params::ControlSourceKind::Matrix`].
    Matrix,
}

mirror_core_enum!(ControlSourceKind <=> mfx_params::ControlSourceKind { Mfx, Matrix });

/// One Control Source a picker may offer.
#[wasm_bindgen(getter_with_clone)]
pub struct WasmControlSource {
    /// Device value written to the Control Source parameter.
    pub value: u8,
    /// Display name (e.g. "CC33", "BEND").
    pub name: String,
}

/// Every Control Source a picker may offer for `kind`, in value order.
/// The unused value 32 is skipped.
#[wasm_bindgen]
pub fn control_source_choices(kind: ControlSourceKind) -> Vec<WasmControlSource> {
    mfx_params::control_source_choices(kind.into())
        .map(|(value, name)| WasmControlSource {
            value,
            name: name.to_string(),
        })
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
    unit: fx_params::FxUnit,
}

#[wasm_bindgen]
impl WasmMfxParamDef {
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Format a stored value for display.
    #[wasm_bindgen(js_name = formatValue)]
    pub fn format_value(&self, value: i32) -> String {
        self.unit.format(value)
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
        unit: p.unit,
    })
}

// ---------------------------------------------------------------------------
// Parameter registry JSON
// ---------------------------------------------------------------------------

/// Return all mixer parameter definitions as a JSON string.
///
/// Returns a JSON array of `ParamDef` objects.
#[wasm_bindgen(js_name = mixerParamsJson)]
pub fn mixer_params_json() -> String {
    let params = param_registry::mixer_params();
    serde_json::to_string(&params).unwrap_or_else(|_| "[]".to_string())
}

/// Format a raw value of the mixer parameter `id` for display.
///
/// Unknown ids fall back to the raw number.
#[wasm_bindgen(js_name = formatMixerParam)]
pub fn format_mixer_param(id: &str, value: i32) -> String {
    static PARAMS: std::sync::LazyLock<Vec<param_registry::ParamDef>> =
        std::sync::LazyLock::new(param_registry::mixer_params);
    param_registry::lookup(&PARAMS, id)
        .map(|def| param_registry::format_value(def, value))
        .unwrap_or_else(|| value.to_string())
}
