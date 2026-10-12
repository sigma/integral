//! WASM bindings for the Integral Integra-7 control surface.

use integral_core::address::{Address, DataSize};
use integral_core::catalog;
use integral_core::device::DeviceState;
use integral_core::sysex;
use integral_core::{
    field_spec, fx_params, mfx, mfx_params, param_registry, params, pcm_drum, pcm_synth,
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

/// Whether a bank MSB selects a drum kit, the only tone type that can feed
/// the Drum Comp+EQ units.
#[wasm_bindgen]
pub fn is_drum_kit_bank(bank_msb: u8) -> bool {
    params::is_drum_kit_bank(bank_msb)
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

/// A Motional Surround common field (mirrors [`params::SurroundField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurroundField {
    /// See [`params::SurroundField::Switch`].
    Switch,
    /// See [`params::SurroundField::RoomType`].
    RoomType,
    /// See [`params::SurroundField::AmbienceLevel`].
    AmbienceLevel,
    /// See [`params::SurroundField::RoomSize`].
    RoomSize,
    /// See [`params::SurroundField::AmbienceTime`].
    AmbienceTime,
    /// See [`params::SurroundField::AmbienceDensity`].
    AmbienceDensity,
    /// See [`params::SurroundField::AmbienceHfDamp`].
    AmbienceHfDamp,
    /// See [`params::SurroundField::ExtPartLr`].
    ExtPartLr,
    /// See [`params::SurroundField::ExtPartFb`].
    ExtPartFb,
    /// See [`params::SurroundField::ExtPartWidth`].
    ExtPartWidth,
    /// See [`params::SurroundField::ExtPartAmbienceSend`].
    ExtPartAmbienceSend,
    /// See [`params::SurroundField::ExtPartControlChannel`].
    ExtPartControlChannel,
    /// See [`params::SurroundField::Depth`].
    Depth,
}

mirror_core_enum!(SurroundField <=> params::SurroundField {
    Switch,
    RoomType,
    AmbienceLevel,
    RoomSize,
    AmbienceTime,
    AmbienceDensity,
    AmbienceHfDamp,
    ExtPartLr,
    ExtPartFb,
    ExtPartWidth,
    ExtPartAmbienceSend,
    ExtPartControlChannel,
    Depth,
});

/// A field of an SN Synth Tone Common (mirrors [`sn_synth::SnsCommonField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnsCommonField {
    /// See [`sn_synth::SnsCommonField::ToneLevel`].
    ToneLevel,
    /// See [`sn_synth::SnsCommonField::PortamentoSwitch`].
    PortamentoSwitch,
    /// See [`sn_synth::SnsCommonField::PortamentoTime`].
    PortamentoTime,
    /// See [`sn_synth::SnsCommonField::MonoSwitch`].
    MonoSwitch,
    /// See [`sn_synth::SnsCommonField::OctaveShift`].
    OctaveShift,
    /// See [`sn_synth::SnsCommonField::PitchBendRangeUp`].
    PitchBendRangeUp,
    /// See [`sn_synth::SnsCommonField::PitchBendRangeDown`].
    PitchBendRangeDown,
    /// See [`sn_synth::SnsCommonField::Partial1Switch`].
    Partial1Switch,
    /// See [`sn_synth::SnsCommonField::Partial1Select`].
    Partial1Select,
    /// See [`sn_synth::SnsCommonField::Partial2Switch`].
    Partial2Switch,
    /// See [`sn_synth::SnsCommonField::Partial2Select`].
    Partial2Select,
    /// See [`sn_synth::SnsCommonField::Partial3Switch`].
    Partial3Switch,
    /// See [`sn_synth::SnsCommonField::Partial3Select`].
    Partial3Select,
    /// See [`sn_synth::SnsCommonField::RingSwitch`].
    RingSwitch,
    /// See [`sn_synth::SnsCommonField::TfxSwitch`].
    TfxSwitch,
    /// See [`sn_synth::SnsCommonField::UnisonSwitch`].
    UnisonSwitch,
    /// See [`sn_synth::SnsCommonField::PortamentoMode`].
    PortamentoMode,
    /// See [`sn_synth::SnsCommonField::LegatoSwitch`].
    LegatoSwitch,
    /// See [`sn_synth::SnsCommonField::AnalogFeel`].
    AnalogFeel,
    /// See [`sn_synth::SnsCommonField::WaveShape`].
    WaveShape,
    /// See [`sn_synth::SnsCommonField::ToneCategory`].
    ToneCategory,
    /// See [`sn_synth::SnsCommonField::PhraseNumber`].
    PhraseNumber,
    /// See [`sn_synth::SnsCommonField::PhraseOctaveShift`].
    PhraseOctaveShift,
    /// See [`sn_synth::SnsCommonField::UnisonSize`].
    UnisonSize,
}

mirror_core_enum!(SnsCommonField <=> sn_synth::SnsCommonField {
    ToneLevel,
    PortamentoSwitch,
    PortamentoTime,
    MonoSwitch,
    OctaveShift,
    PitchBendRangeUp,
    PitchBendRangeDown,
    Partial1Switch,
    Partial1Select,
    Partial2Switch,
    Partial2Select,
    Partial3Switch,
    Partial3Select,
    RingSwitch,
    TfxSwitch,
    UnisonSwitch,
    PortamentoMode,
    LegatoSwitch,
    AnalogFeel,
    WaveShape,
    ToneCategory,
    PhraseNumber,
    PhraseOctaveShift,
    UnisonSize,
});

/// A field of an SN Synth Tone Partial (mirrors [`sn_synth::SnsPartialField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnsPartialField {
    /// See [`sn_synth::SnsPartialField::OscWave`].
    OscWave,
    /// See [`sn_synth::SnsPartialField::OscWaveVariation`].
    OscWaveVariation,
    /// See [`sn_synth::SnsPartialField::OscPitch`].
    OscPitch,
    /// See [`sn_synth::SnsPartialField::OscDetune`].
    OscDetune,
    /// See [`sn_synth::SnsPartialField::OscPwModDepth`].
    OscPwModDepth,
    /// See [`sn_synth::SnsPartialField::OscPulseWidth`].
    OscPulseWidth,
    /// See [`sn_synth::SnsPartialField::OscPitchEnvAttack`].
    OscPitchEnvAttack,
    /// See [`sn_synth::SnsPartialField::OscPitchEnvDecay`].
    OscPitchEnvDecay,
    /// See [`sn_synth::SnsPartialField::OscPitchEnvDepth`].
    OscPitchEnvDepth,
    /// See [`sn_synth::SnsPartialField::FilterMode`].
    FilterMode,
    /// See [`sn_synth::SnsPartialField::FilterSlope`].
    FilterSlope,
    /// See [`sn_synth::SnsPartialField::FilterCutoff`].
    FilterCutoff,
    /// See [`sn_synth::SnsPartialField::FilterKeyfollow`].
    FilterKeyfollow,
    /// See [`sn_synth::SnsPartialField::FilterEnvVelSens`].
    FilterEnvVelSens,
    /// See [`sn_synth::SnsPartialField::FilterResonance`].
    FilterResonance,
    /// See [`sn_synth::SnsPartialField::FilterEnvAttack`].
    FilterEnvAttack,
    /// See [`sn_synth::SnsPartialField::FilterEnvDecay`].
    FilterEnvDecay,
    /// See [`sn_synth::SnsPartialField::FilterEnvSustain`].
    FilterEnvSustain,
    /// See [`sn_synth::SnsPartialField::FilterEnvRelease`].
    FilterEnvRelease,
    /// See [`sn_synth::SnsPartialField::FilterEnvDepth`].
    FilterEnvDepth,
    /// See [`sn_synth::SnsPartialField::AmpLevel`].
    AmpLevel,
    /// See [`sn_synth::SnsPartialField::AmpVelSens`].
    AmpVelSens,
    /// See [`sn_synth::SnsPartialField::AmpEnvAttack`].
    AmpEnvAttack,
    /// See [`sn_synth::SnsPartialField::AmpEnvDecay`].
    AmpEnvDecay,
    /// See [`sn_synth::SnsPartialField::AmpEnvSustain`].
    AmpEnvSustain,
    /// See [`sn_synth::SnsPartialField::AmpEnvRelease`].
    AmpEnvRelease,
    /// See [`sn_synth::SnsPartialField::AmpPan`].
    AmpPan,
    /// See [`sn_synth::SnsPartialField::LfoShape`].
    LfoShape,
    /// See [`sn_synth::SnsPartialField::LfoRate`].
    LfoRate,
    /// See [`sn_synth::SnsPartialField::LfoTempoSync`].
    LfoTempoSync,
    /// See [`sn_synth::SnsPartialField::LfoTempoSyncNote`].
    LfoTempoSyncNote,
    /// See [`sn_synth::SnsPartialField::LfoFadeTime`].
    LfoFadeTime,
    /// See [`sn_synth::SnsPartialField::LfoKeyTrigger`].
    LfoKeyTrigger,
    /// See [`sn_synth::SnsPartialField::LfoPitchDepth`].
    LfoPitchDepth,
    /// See [`sn_synth::SnsPartialField::LfoFilterDepth`].
    LfoFilterDepth,
    /// See [`sn_synth::SnsPartialField::LfoAmpDepth`].
    LfoAmpDepth,
    /// See [`sn_synth::SnsPartialField::LfoPanDepth`].
    LfoPanDepth,
    /// See [`sn_synth::SnsPartialField::ModLfoShape`].
    ModLfoShape,
    /// See [`sn_synth::SnsPartialField::ModLfoRate`].
    ModLfoRate,
    /// See [`sn_synth::SnsPartialField::ModLfoTempoSync`].
    ModLfoTempoSync,
    /// See [`sn_synth::SnsPartialField::ModLfoTempoSyncNote`].
    ModLfoTempoSyncNote,
    /// See [`sn_synth::SnsPartialField::PwShift`].
    PwShift,
    /// See [`sn_synth::SnsPartialField::ModLfoPitchDepth`].
    ModLfoPitchDepth,
    /// See [`sn_synth::SnsPartialField::ModLfoFilterDepth`].
    ModLfoFilterDepth,
    /// See [`sn_synth::SnsPartialField::ModLfoAmpDepth`].
    ModLfoAmpDepth,
    /// See [`sn_synth::SnsPartialField::ModLfoPanDepth`].
    ModLfoPanDepth,
    /// See [`sn_synth::SnsPartialField::AftertouchCutoff`].
    AftertouchCutoff,
    /// See [`sn_synth::SnsPartialField::AftertouchLevel`].
    AftertouchLevel,
    /// See [`sn_synth::SnsPartialField::WaveGain`].
    WaveGain,
    /// See [`sn_synth::SnsPartialField::WaveNumber`].
    WaveNumber,
    /// See [`sn_synth::SnsPartialField::HpfCutoff`].
    HpfCutoff,
    /// See [`sn_synth::SnsPartialField::SuperSawDetune`].
    SuperSawDetune,
    /// See [`sn_synth::SnsPartialField::ModLfoRateControl`].
    ModLfoRateControl,
    /// See [`sn_synth::SnsPartialField::AmpLevelKeyfollow`].
    AmpLevelKeyfollow,
}

mirror_core_enum!(SnsPartialField <=> sn_synth::SnsPartialField {
    OscWave,
    OscWaveVariation,
    OscPitch,
    OscDetune,
    OscPwModDepth,
    OscPulseWidth,
    OscPitchEnvAttack,
    OscPitchEnvDecay,
    OscPitchEnvDepth,
    FilterMode,
    FilterSlope,
    FilterCutoff,
    FilterKeyfollow,
    FilterEnvVelSens,
    FilterResonance,
    FilterEnvAttack,
    FilterEnvDecay,
    FilterEnvSustain,
    FilterEnvRelease,
    FilterEnvDepth,
    AmpLevel,
    AmpVelSens,
    AmpEnvAttack,
    AmpEnvDecay,
    AmpEnvSustain,
    AmpEnvRelease,
    AmpPan,
    LfoShape,
    LfoRate,
    LfoTempoSync,
    LfoTempoSyncNote,
    LfoFadeTime,
    LfoKeyTrigger,
    LfoPitchDepth,
    LfoFilterDepth,
    LfoAmpDepth,
    LfoPanDepth,
    ModLfoShape,
    ModLfoRate,
    ModLfoTempoSync,
    ModLfoTempoSyncNote,
    PwShift,
    ModLfoPitchDepth,
    ModLfoFilterDepth,
    ModLfoAmpDepth,
    ModLfoPanDepth,
    AftertouchCutoff,
    AftertouchLevel,
    WaveGain,
    WaveNumber,
    HpfCutoff,
    SuperSawDetune,
    ModLfoRateControl,
    AmpLevelKeyfollow,
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

/// The SN Synth Common field switching Partial `partial` (0-based) on or
/// off.
#[wasm_bindgen]
pub fn sns_partial_switch_field(partial: u8) -> Result<SnsCommonField, JsError> {
    check_partial(partial, sn_synth::SNS_PARTIAL_COUNT as u8 - 1)?;
    Ok(sn_synth::SnsCommonField::PARTIAL_SWITCHES[partial as usize].into())
}

/// The SN Synth Partial OSC Wave value that selects a PCM Wave.
#[wasm_bindgen]
pub fn sns_osc_wave_pcm() -> u8 {
    sn_synth::OSC_WAVE_PCM
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

// ---------------------------------------------------------------------------
// Chorus / Reverb addresses
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Drum Comp+EQ and Motional Surround addresses
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Ext Part addresses
// ---------------------------------------------------------------------------

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

/// Range, default and display format of a tone block field.
#[wasm_bindgen]
pub struct WasmFieldSpec {
    /// Minimum raw value.
    #[wasm_bindgen(readonly)]
    pub min: i32,
    /// Maximum raw value.
    #[wasm_bindgen(readonly)]
    pub max: i32,
    /// Default raw value.
    #[wasm_bindgen(readonly, js_name = defaultValue)]
    pub default_value: i32,
    format: param_registry::ParamFormat,
}

#[wasm_bindgen]
impl WasmFieldSpec {
    /// Format a raw value for display.
    #[wasm_bindgen(js_name = formatValue)]
    pub fn format_value(&self, value: i32) -> String {
        self.format.format(value)
    }

    /// The raw value a knob centres on, for signed formats.
    #[wasm_bindgen(getter)]
    pub fn center(&self) -> Option<i32> {
        self.format.center()
    }
}

fn wrap_field_spec(spec: field_spec::FieldSpec) -> WasmFieldSpec {
    WasmFieldSpec {
        min: spec.min,
        max: spec.max,
        default_value: spec.default,
        format: spec.format,
    }
}

/// Spec of a PCM Synth Tone Common field.
#[wasm_bindgen]
pub fn pcms_common_spec(field: PcmsCommonField) -> WasmFieldSpec {
    wrap_field_spec(field_spec::pcms_common_spec(field.into()))
}

/// Spec of a PCM Synth Tone PMT field.
#[wasm_bindgen]
pub fn pcms_pmt_spec(field: PcmsPmtField) -> WasmFieldSpec {
    wrap_field_spec(field_spec::pcms_pmt_spec(field.into()))
}

/// Spec of a PCM Synth Tone Partial field.
#[wasm_bindgen]
pub fn pcms_partial_spec(field: PcmsPartialField) -> WasmFieldSpec {
    wrap_field_spec(field_spec::pcms_partial_spec(field.into()))
}

/// Spec of a PCM Drum Kit Partial field; `undefined` for fields not
/// specified yet.
#[wasm_bindgen]
pub fn pcmd_partial_spec(field: PcmdPartialField) -> Option<WasmFieldSpec> {
    field_spec::pcmd_partial_spec(field.into()).map(wrap_field_spec)
}

/// How strongly velocity `v` (0.0-1.0) acts under V-Curve `curve` (0 =
/// FIXED, 1-7), from 0.0 to 1.0.
#[wasm_bindgen]
pub fn velocity_curve(curve: u8, v: f64) -> f64 {
    integral_core::velocity_curve::velocity_curve(curve, v)
}

/// Signal routing of PCM Synth Structure Type `structure` (0-9), as
/// `{ upper, lower, join? }`; see [`integral_core::pcm_structure::Routing`].
///
/// # Errors
/// `structure` is out of range.
#[wasm_bindgen]
pub fn pcms_structure_routing(structure: u8) -> Result<JsValue, JsError> {
    let routing = integral_core::pcm_structure::routing(structure)
        .ok_or_else(|| JsError::new("structure type out of range"))?;
    serde_wasm_bindgen::to_value(&routing).map_err(|e| JsError::new(&e.to_string()))
}

/// Whether a tone from bank `msb` can be edited (not GM2 or ExPCM, for
/// instance); see [`params::is_editable_bank`].
#[wasm_bindgen]
pub fn is_editable_bank(msb: u8) -> bool {
    params::is_editable_bank(msb)
}

/// The tone type a Bank Select MSB selects (whose Temporary Tone block an
/// editor reads), or `undefined` for an MSB that isn't a tone bank.
#[wasm_bindgen]
pub fn tone_type_from_bank_msb(msb: u8) -> Option<ToneType> {
    params::tone_type_from_bank_msb(msb).map(Into::into)
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

    // -- Motional Surround -------------------------------------------------

    /// Set a Motional Surround common field.
    #[wasm_bindgen(js_name = setSurroundParam)]
    pub fn set_surround_param(&mut self, field: SurroundField, value: u8) {
        self.inner.set_surround_param(field.into(), value);
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

    // -- SN-S Tone Edit ----------------------------------------------------

    /// Set an SN-S Common field, sent nibblized when the field is.
    #[wasm_bindgen(js_name = setSnsCommonParam)]
    pub fn set_sns_common_param(
        &mut self,
        part: u8,
        field: SnsCommonField,
        value: u16,
    ) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.set_sns_common_param(part, field.into(), value);
        Ok(())
    }

    /// Set an SN-S Partial field, sent nibblized when the field is (e.g.
    /// the Wave Number).
    #[wasm_bindgen(js_name = setSnsPartialParam)]
    pub fn set_sns_partial_param(
        &mut self,
        part: u8,
        partial: u8,
        field: SnsPartialField,
        value: u16,
    ) -> Result<(), JsError> {
        check_part(part)?;
        check_partial(partial, sn_synth::SNS_PARTIAL_COUNT as u8 - 1)?;
        self.inner
            .set_sns_partial_param(part, partial, field.into(), value);
        Ok(())
    }

    /// Start reading the whole mixer state from the device, dropping any
    /// read in progress. Replies update the state as they arrive; lost
    /// replies are requested again by `drain`.
    #[wasm_bindgen(js_name = loadMixer)]
    pub fn load_mixer(&mut self) {
        self.inner.load_mixer();
    }

    /// Whether a mixer load still waits for a block the mixer can't be
    /// shown without.
    #[wasm_bindgen(js_name = mixerLoading)]
    pub fn mixer_loading(&self) -> bool {
        self.inner.mixer_loading()
    }

    /// Drop a part's cached SN-S tone and read it again from the device,
    /// e.g. when its editor opens. Lost replies are requested again by
    /// `drain`.
    #[wasm_bindgen(js_name = reloadSnsTone)]
    pub fn reload_sns_tone(&mut self, part: u8) -> Result<(), JsError> {
        check_part(part)?;
        self.inner.reload_sns_tone(part);
        Ok(())
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

    /// Change the MFX type of the part's tone of type `tone_type` and load
    /// the type's default parameters. Returns the new slot values.
    #[wasm_bindgen(js_name = setMfxType)]
    pub fn set_mfx_type(
        &mut self,
        part: u8,
        tone_type: ToneType,
        mfx_type: u8,
    ) -> Result<Vec<i32>, JsError> {
        self.inner
            .set_mfx_type(part, tone_type.into(), mfx_type)
            .map(Vec::from)
            .ok_or_else(|| JsError::new(&format!("unknown MFX type {mfx_type}")))
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

/// Returns the MFX Control Assign destinations of the given type: the
/// stored value is the index, and 0 is `OFF`.
#[wasm_bindgen]
pub fn mfx_control_targets(mfx_type: u8) -> Vec<String> {
    mfx_params::control_targets(mfx_type)
        .unwrap_or_default()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Returns MFX parameter definitions for the given type index.
#[wasm_bindgen]
pub fn mfx_params(mfx_type: u8) -> Vec<WasmFxParamDef> {
    mfx_params::mfx_type_def(mfx_type).map_or_else(Vec::new, |d| wrap_fx_params(d.params))
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
