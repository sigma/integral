//! PCM Synth Tone address map and state types.
//!
//! The PCM Synth tone occupies offset `00 00 00 00` within the temporary tone
//! block (it IS the base tone type). It contains: Common, PMT (Partial Mix
//! Table), four Partial blocks, Common2, and MFX (handled by `mfx.rs`).

use crate::address::{Address, DataSize};
use crate::block_field::BlockField;
use crate::mfx::MfxState;
use crate::params;
use crate::parse_helpers::{encode_ascii_name, parse_ascii_name};

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// PCM Synth Common block offset (relative to PCMS base).
const PCMS_COMMON_OFFSET: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// Bytes of the PCM Synth Tone Name, at Common `00 00`.
const PCMS_NAME_LEN: usize = 12;

/// PCM Synth Common block size (80 bytes).
pub const PCMS_COMMON_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x50);

/// PCM Synth Common parse size in bytes.
pub const PCMS_COMMON_SIZE: usize = 0x50;

/// PCM Synth PMT block offset (relative to PCMS base).
const PCMS_PMT_OFFSET: [u8; 4] = [0x00, 0x00, 0x10, 0x00];

/// PCM Synth PMT block size (41 bytes).
pub const PCMS_PMT_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x29);

/// PCM Synth PMT parse size in bytes.
pub const PCMS_PMT_SIZE: usize = 0x29;

/// PCM Synth Partial block size (154 bytes; 7-bit size `01 1A` = 128 + 26).
pub const PCMS_PARTIAL_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x01, 0x1A);

/// PCM Synth Partial parse size in bytes (154 = 128 + 26).
pub const PCMS_PARTIAL_SIZE: usize = 154;

/// Partial block offsets (relative to PCMS base), indexed 0-3.
const PCMS_PARTIAL_OFFSETS: [[u8; 4]; 4] = [
    [0x00, 0x00, 0x20, 0x00],
    [0x00, 0x00, 0x22, 0x00],
    [0x00, 0x00, 0x24, 0x00],
    [0x00, 0x00, 0x26, 0x00],
];

/// Number of Partials in a PCM Synth Tone.
pub const PCMS_PARTIAL_COUNT: usize = PCMS_PARTIAL_OFFSETS.len();

/// PCM Synth Common2 block offset (relative to PCMS base).
const PCMS_COMMON2_OFFSET: [u8; 4] = [0x00, 0x00, 0x30, 0x00];

/// PCM Synth Common2 block size (60 bytes).
pub const PCMS_COMMON2_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x3C);

/// PCM Synth Common2 parse size in bytes.
pub const PCMS_COMMON2_SIZE: usize = 0x3C;

// ---------------------------------------------------------------------------
// Address functions
// ---------------------------------------------------------------------------

/// Compute the absolute PCM Synth base address for a part.
pub const fn pcms_base(part_index: u8) -> Address {
    params::ToneType::PcmSynth.base_address(part_index)
}

/// Compute the absolute PCM Synth Common block address for a part.
pub const fn pcms_common_address(part_index: u8) -> Address {
    pcms_base(part_index).offset(PCMS_COMMON_OFFSET)
}

/// Compute the absolute address for a single PCM Synth Common parameter.
pub const fn pcms_common_param_address(part_index: u8, offset: u8) -> Address {
    pcms_common_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

/// Compute the absolute PCM Synth PMT block address for a part.
pub const fn pcms_pmt_address(part_index: u8) -> Address {
    pcms_base(part_index).offset(PCMS_PMT_OFFSET)
}

/// Compute the absolute address for a single PCM Synth PMT parameter.
pub const fn pcms_pmt_param_address(part_index: u8, offset: u8) -> Address {
    pcms_pmt_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

/// Compute the absolute PCM Synth Partial block address for a part and
/// partial index (0-3).
pub const fn pcms_partial_address(part_index: u8, partial: u8) -> Address {
    pcms_base(part_index).offset(PCMS_PARTIAL_OFFSETS[partial as usize])
}

/// Compute the absolute address for a single PCM Synth Partial parameter
/// at byte `position` within the block.
///
/// Partial blocks are 154 bytes, so positions from `0x80` carry into the
/// next address byte (position `0x80` is offset `01 00`).
pub const fn pcms_partial_param_address(part_index: u8, partial: u8, position: u8) -> Address {
    pcms_partial_address(part_index, partial).offset([0x00, 0x00, 0x00, position])
}

/// Compute the absolute PCM Synth Common2 block address for a part.
pub const fn pcms_common2_address(part_index: u8) -> Address {
    pcms_base(part_index).offset(PCMS_COMMON2_OFFSET)
}

/// Compute the absolute address for a single PCM Synth Common2 parameter.
pub const fn pcms_common2_param_address(part_index: u8, offset: u8) -> Address {
    pcms_common2_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

// ---------------------------------------------------------------------------
// Fields
// ---------------------------------------------------------------------------

/// Number of Matrix Controls in a PCM Synth Tone Common.
pub const PCMS_MATRIX_CONTROL_COUNT: usize = 4;

/// Bytes from one Matrix Control (or PMT entry) to the next.
const PCMS_ENTRY_STRIDE: usize = 9;

/// A field of a PCM Synth Tone Common block. The discriminant is the
/// field's byte position within the block; the Matrix Control fields hold
/// one value per Matrix Control ([`PCMS_MATRIX_CONTROL_COUNT`]), 9 bytes
/// apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmsCommonField {
    /// PCM Synth Tone Level.
    ToneLevel = 0x0E,
    /// PCM Synth Tone Pan.
    TonePan = 0x0F,
    /// PCM Synth Tone Priority.
    TonePriority = 0x10,
    /// PCM Synth Tone Coarse Tune.
    CoarseTune = 0x11,
    /// PCM Synth Tone Fine Tune.
    FineTune = 0x12,
    /// Octave Shift.
    OctaveShift = 0x13,
    /// Stretch Tune Depth.
    StretchTuneDepth = 0x14,
    /// Analog Feel.
    AnalogFeel = 0x15,
    /// Mono/Poly.
    MonoPoly = 0x16,
    /// Legato Switch.
    LegatoSwitch = 0x17,
    /// Legato Retrigger.
    LegatoRetrigger = 0x18,
    /// Portamento Switch.
    PortamentoSwitch = 0x19,
    /// Portamento Mode.
    PortamentoMode = 0x1A,
    /// Portamento Type.
    PortamentoType = 0x1B,
    /// Portamento Start.
    PortamentoStart = 0x1C,
    /// Portamento Time.
    PortamentoTime = 0x1D,
    /// Cutoff Offset.
    CutoffOffset = 0x22,
    /// Resonance Offset.
    ResonanceOffset = 0x23,
    /// Attack Time Offset.
    AttackTimeOffset = 0x24,
    /// Release Time Offset.
    ReleaseTimeOffset = 0x25,
    /// Velocity Sens Offset.
    VelocitySensOffset = 0x26,
    /// PMT Control Switch.
    PmtControlSwitch = 0x28,
    /// Pitch Bend Range Up.
    PitchBendRangeUp = 0x29,
    /// Pitch Bend Range Down.
    PitchBendRangeDown = 0x2A,
    /// Matrix Control 1–4 Source.
    MatrixControlSource = 0x2B,
    /// Matrix Control 1–4 Destination 1.
    MatrixControlDestination1 = 0x2C,
    /// Matrix Control 1–4 Sens 1.
    MatrixControlSens1 = 0x2D,
    /// Matrix Control 1–4 Destination 2.
    MatrixControlDestination2 = 0x2E,
    /// Matrix Control 1–4 Sens 2.
    MatrixControlSens2 = 0x2F,
    /// Matrix Control 1–4 Destination 3.
    MatrixControlDestination3 = 0x30,
    /// Matrix Control 1–4 Sens 3.
    MatrixControlSens3 = 0x31,
    /// Matrix Control 1–4 Destination 4.
    MatrixControlDestination4 = 0x32,
    /// Matrix Control 1–4 Sens 4.
    MatrixControlSens4 = 0x33,
}

impl PcmsCommonField {
    /// The Destination 1–4 fields of a Matrix Control, in order.
    pub const MATRIX_CONTROL_DESTINATIONS: [Self; 4] = [
        Self::MatrixControlDestination1,
        Self::MatrixControlDestination2,
        Self::MatrixControlDestination3,
        Self::MatrixControlDestination4,
    ];

    /// The Sens 1–4 fields of a Matrix Control, in order.
    pub const MATRIX_CONTROL_SENS: [Self; 4] = [
        Self::MatrixControlSens1,
        Self::MatrixControlSens2,
        Self::MatrixControlSens3,
        Self::MatrixControlSens4,
    ];

    /// Whether the field holds one value per Matrix Control.
    const fn is_matrix_control(self) -> bool {
        self as u8 >= Self::MatrixControlSource as u8
    }
}

impl BlockField for PcmsCommonField {
    fn position(self) -> usize {
        self as usize
    }

    fn count(self) -> usize {
        if self.is_matrix_control() {
            PCMS_MATRIX_CONTROL_COUNT
        } else {
            1
        }
    }

    fn stride(self) -> usize {
        if self.is_matrix_control() {
            PCMS_ENTRY_STRIDE
        } else {
            1
        }
    }
}

/// A field of a PCM Synth Tone PMT (Partial Mix Table) block. The
/// discriminant is the field's byte position within the block; the
/// per-Partial fields (from [`PcmsPmtField::PartialSwitch`]) hold one value
/// per Partial ([`PCMS_PARTIAL_COUNT`]), 9 bytes apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmsPmtField {
    /// Structure Type 1 & 2.
    StructureType12 = 0x00,
    /// Booster 1 & 2.
    Booster12 = 0x01,
    /// Structure Type 3 & 4.
    StructureType34 = 0x02,
    /// Booster 3 & 4.
    Booster34 = 0x03,
    /// PMT Velocity Control.
    PmtVelocityControl = 0x04,
    /// PMT1–4 Partial Switch.
    PartialSwitch = 0x05,
    /// PMT1–4 Keyboard Range Lower.
    KeyRangeLower = 0x06,
    /// PMT1–4 Keyboard Range Upper.
    KeyRangeUpper = 0x07,
    /// PMT1–4 Keyboard Fade Width Lower.
    KeyFadeLower = 0x08,
    /// PMT1–4 Keyboard Fade Width Upper.
    KeyFadeUpper = 0x09,
    /// PMT1–4 Velocity Range Lower.
    VelocityRangeLower = 0x0A,
    /// PMT1–4 Velocity Range Upper.
    VelocityRangeUpper = 0x0B,
    /// PMT1–4 Velocity Fade Width Lower.
    VelocityFadeLower = 0x0C,
    /// PMT1–4 Velocity Fade Width Upper.
    VelocityFadeUpper = 0x0D,
}

impl PcmsPmtField {
    /// Whether the field holds one value per Partial.
    const fn is_per_partial(self) -> bool {
        self as u8 >= Self::PartialSwitch as u8
    }
}

impl BlockField for PcmsPmtField {
    fn position(self) -> usize {
        self as usize
    }

    fn count(self) -> usize {
        if self.is_per_partial() {
            PCMS_PARTIAL_COUNT
        } else {
            1
        }
    }

    fn stride(self) -> usize {
        if self.is_per_partial() {
            PCMS_ENTRY_STRIDE
        } else {
            1
        }
    }
}

/// A field of a PCM Synth Tone Partial block. The discriminant is the
/// field's first byte position within the block (positions from `0x80` are
/// at offset `01 00` and up); [`BlockField`] gives the envelope, control
/// switch and step fields' value count and the nibblized fields' width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmsPartialField {
    /// Partial Level.
    Level = 0x00,
    /// Partial Coarse Tune.
    CoarseTune = 0x01,
    /// Partial Fine Tune.
    FineTune = 0x02,
    /// Partial Random Pitch Depth.
    RandomPitchDepth = 0x03,
    /// Partial Pan.
    Pan = 0x04,
    /// Partial Pan Keyfollow.
    PanKeyfollow = 0x05,
    /// Partial Random Pan Depth.
    RandomPanDepth = 0x06,
    /// Partial Alternate Pan Depth.
    AlternatePanDepth = 0x07,
    /// Partial Env Mode.
    EnvMode = 0x08,
    /// Partial Delay Mode.
    DelayMode = 0x09,
    /// Partial Delay Time (nibblized, 2 bytes).
    DelayTime = 0x0A,
    /// Partial Output Level.
    OutputLevel = 0x0C,
    /// Partial Chorus Send Level.
    ChorusSend = 0x0F,
    /// Partial Reverb Send Level.
    ReverbSend = 0x10,
    /// Partial Receive Bender.
    ReceiveBender = 0x12,
    /// Partial Receive Expression.
    ReceiveExpression = 0x13,
    /// Partial Receive Hold-1.
    ReceiveHold1 = 0x14,
    /// Partial Redamper Switch.
    RedamperSwitch = 0x16,
    /// Partial Control 1 Switch 1–4.
    Control1Switch = 0x17,
    /// Partial Control 2 Switch 1–4.
    Control2Switch = 0x1B,
    /// Partial Control 3 Switch 1–4.
    Control3Switch = 0x1F,
    /// Partial Control 4 Switch 1–4.
    Control4Switch = 0x23,
    /// Wave Group Type.
    WaveGroupType = 0x27,
    /// Wave Group ID (nibblized, 4 bytes).
    WaveGroupId = 0x28,
    /// Wave Number L / Mono (nibblized, 4 bytes).
    WaveNumberL = 0x2C,
    /// Wave Number R (nibblized, 4 bytes).
    WaveNumberR = 0x30,
    /// Wave Gain.
    WaveGain = 0x34,
    /// Wave FXM Switch.
    WaveFxmSwitch = 0x35,
    /// Wave FXM Color.
    WaveFxmColor = 0x36,
    /// Wave FXM Depth.
    WaveFxmDepth = 0x37,
    /// Wave Tempo Sync.
    WaveTempoSync = 0x38,
    /// Wave Pitch Keyfollow.
    WavePitchKeyfollow = 0x39,
    /// Pitch Env Depth.
    PitchEnvDepth = 0x3A,
    /// Pitch Env Velocity Sens.
    PitchEnvVelocitySens = 0x3B,
    /// Pitch Env Time 1 Velocity Sens.
    PitchEnvT1VelocitySens = 0x3C,
    /// Pitch Env Time 4 Velocity Sens.
    PitchEnvT4VelocitySens = 0x3D,
    /// Pitch Env Time Keyfollow.
    PitchEnvTimeKeyfollow = 0x3E,
    /// Pitch Env Time 1–4.
    PitchEnvTime = 0x3F,
    /// Pitch Env Level 0–4.
    PitchEnvLevel = 0x43,
    /// TVF Filter Type.
    TvfFilterType = 0x48,
    /// TVF Cutoff Frequency.
    TvfCutoffFrequency = 0x49,
    /// TVF Cutoff Keyfollow.
    TvfCutoffKeyfollow = 0x4A,
    /// TVF Cutoff Velocity Curve.
    TvfCutoffVelocityCurve = 0x4B,
    /// TVF Cutoff Velocity Sens.
    TvfCutoffVelocitySens = 0x4C,
    /// TVF Resonance.
    TvfResonance = 0x4D,
    /// TVF Resonance Velocity Sens.
    TvfResonanceVelocitySens = 0x4E,
    /// TVF Env Depth.
    TvfEnvDepth = 0x4F,
    /// TVF Env Velocity Curve.
    TvfEnvVelocityCurve = 0x50,
    /// TVF Env Velocity Sens.
    TvfEnvVelocitySens = 0x51,
    /// TVF Env Time 1 Velocity Sens.
    TvfEnvT1VelocitySens = 0x52,
    /// TVF Env Time 4 Velocity Sens.
    TvfEnvT4VelocitySens = 0x53,
    /// TVF Env Time Keyfollow.
    TvfEnvTimeKeyfollow = 0x54,
    /// TVF Env Time 1–4.
    TvfEnvTime = 0x55,
    /// TVF Env Level 0–4.
    TvfEnvLevel = 0x59,
    /// Bias Level.
    TvaBiasLevel = 0x5E,
    /// Bias Position.
    TvaBiasPosition = 0x5F,
    /// Bias Direction.
    TvaBiasDirection = 0x60,
    /// TVA Level Velocity Curve.
    TvaLevelVelocityCurve = 0x61,
    /// TVA Level Velocity Sens.
    TvaLevelVelocitySens = 0x62,
    /// TVA Env Time 1 Velocity Sens.
    TvaEnvT1VelocitySens = 0x63,
    /// TVA Env Time 4 Velocity Sens.
    TvaEnvT4VelocitySens = 0x64,
    /// TVA Env Time Keyfollow.
    TvaEnvTimeKeyfollow = 0x65,
    /// TVA Env Time 1–4.
    TvaEnvTime = 0x66,
    /// TVA Env Level 1–3.
    TvaEnvLevel = 0x6A,
    /// LFO1 Waveform.
    Lfo1Waveform = 0x6D,
    /// LFO1 Rate (nibblized, 2 bytes).
    Lfo1Rate = 0x6E,
    /// LFO1 Offset.
    Lfo1Offset = 0x70,
    /// LFO1 Rate Detune.
    Lfo1RateDetune = 0x71,
    /// LFO1 Delay Time.
    Lfo1DelayTime = 0x72,
    /// LFO1 Delay Time Keyfollow.
    Lfo1DelayTimeKeyfollow = 0x73,
    /// LFO1 Fade Mode.
    Lfo1FadeMode = 0x74,
    /// LFO1 Fade Time.
    Lfo1FadeTime = 0x75,
    /// LFO1 Key Trigger.
    Lfo1KeyTrigger = 0x76,
    /// LFO1 Pitch Depth.
    Lfo1PitchDepth = 0x77,
    /// LFO1 TVF Depth.
    Lfo1TvfDepth = 0x78,
    /// LFO1 TVA Depth.
    Lfo1TvaDepth = 0x79,
    /// LFO1 Pan Depth.
    Lfo1PanDepth = 0x7A,
    /// LFO2 Waveform.
    Lfo2Waveform = 0x7B,
    /// LFO2 Rate (nibblized, 2 bytes).
    Lfo2Rate = 0x7C,
    /// LFO2 Offset.
    Lfo2Offset = 0x7E,
    /// LFO2 Rate Detune.
    Lfo2RateDetune = 0x7F,
    /// LFO2 Delay Time (`01 00`).
    Lfo2DelayTime = 0x80,
    /// LFO2 Delay Time Keyfollow.
    Lfo2DelayTimeKeyfollow = 0x80 + 0x01,
    /// LFO2 Fade Mode.
    Lfo2FadeMode = 0x80 + 0x02,
    /// LFO2 Fade Time.
    Lfo2FadeTime = 0x80 + 0x03,
    /// LFO2 Key Trigger.
    Lfo2KeyTrigger = 0x80 + 0x04,
    /// LFO2 Pitch Depth.
    Lfo2PitchDepth = 0x80 + 0x05,
    /// LFO2 TVF Depth.
    Lfo2TvfDepth = 0x80 + 0x06,
    /// LFO2 TVA Depth.
    Lfo2TvaDepth = 0x80 + 0x07,
    /// LFO2 Pan Depth.
    Lfo2PanDepth = 0x80 + 0x08,
    /// LFO Step Type (`01 09`).
    LfoStepType = 0x80 + 0x09,
    /// LFO Step 1–16 (`01 0A`).
    LfoStep = 0x80 + 0x0A,
}

impl BlockField for PcmsPartialField {
    fn position(self) -> usize {
        self as usize
    }

    fn count(self) -> usize {
        match self {
            Self::Control1Switch
            | Self::Control2Switch
            | Self::Control3Switch
            | Self::Control4Switch
            | Self::PitchEnvTime
            | Self::TvfEnvTime
            | Self::TvaEnvTime => 4,
            Self::PitchEnvLevel | Self::TvfEnvLevel => 5,
            Self::TvaEnvLevel => 3,
            Self::LfoStep => 16,
            _ => 1,
        }
    }

    fn width(self) -> usize {
        match self {
            Self::DelayTime | Self::Lfo1Rate | Self::Lfo2Rate => 2,
            Self::WaveGroupId | Self::WaveNumberL | Self::WaveNumberR => 4,
            _ => 1,
        }
    }
}

/// A field of a PCM Synth Tone Common 2 block. The discriminant is the
/// field's first byte position within the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmsCommon2Field {
    /// Tone Category.
    ToneCategory = 0x10,
    /// Tone Number (nibblized, 2 bytes).
    ToneNumber = 0x11,
    /// Phrase Octave Shift.
    PhraseOctaveShift = 0x13,
    /// TFX Switch.
    TfxSwitch = 0x33,
    /// Phrase Number (nibblized, 4 bytes).
    PhraseNumber = 0x38,
}

impl BlockField for PcmsCommon2Field {
    fn position(self) -> usize {
        self as usize
    }

    fn width(self) -> usize {
        match self {
            Self::ToneNumber => 2,
            Self::PhraseNumber => 4,
            _ => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// Matrix Control routing entry (source + 4 destination/sensitivity pairs).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MatrixControl {
    /// Source controller (0-109).
    pub source: u8,
    /// Destination assignments (4 slots, each 0-33).
    pub destinations: [u8; 4],
    /// Sensitivity values (4 slots, each 1-127, display -63 to +63).
    pub sensitivities: [u8; 4],
}

impl Default for MatrixControl {
    fn default() -> Self {
        Self {
            source: 0,
            destinations: [0; 4],
            sensitivities: [64; 4], // center (0)
        }
    }
}

/// PCM Synth Tone Common parameters (80 bytes at offset `00 00 00`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmSynthCommon {
    /// Tone name (up to 12 ASCII characters).
    pub tone_name: String,
    /// Tone Level (0-127).
    pub tone_level: u8,
    /// Tone Pan (0-127, display L64-63R).
    pub tone_pan: u8,
    /// Tone Priority (0=LAST, 1=LOUDEST).
    pub tone_priority: u8,
    /// Coarse Tune (16-112, display -48 to +48).
    pub coarse_tune: u8,
    /// Fine Tune (14-114, display -50 to +50).
    pub fine_tune: u8,
    /// Octave Shift (61-67, display -3 to +3).
    pub octave_shift: u8,
    /// Stretch Tune Depth (0-3: OFF, 1-3).
    pub stretch_tune_depth: u8,
    /// Analog Feel (0-127).
    pub analog_feel: u8,
    /// Mono/Poly (0=MONO, 1=POLY).
    pub mono_poly: u8,
    /// Legato Switch (0=OFF, 1=ON).
    pub legato_switch: u8,
    /// Legato Retrigger (0=OFF, 1=ON).
    pub legato_retrigger: u8,
    /// Portamento Switch (0=OFF, 1=ON).
    pub portamento_switch: u8,
    /// Portamento Mode (0=NORMAL, 1=LEGATO).
    pub portamento_mode: u8,
    /// Portamento Type (0=RATE, 1=TIME).
    pub portamento_type: u8,
    /// Portamento Start (0=PITCH, 1=NOTE).
    pub portamento_start: u8,
    /// Portamento Time (0-127).
    pub portamento_time: u8,
    /// Cutoff Offset (1-127, display -63 to +63).
    pub cutoff_offset: u8,
    /// Resonance Offset (1-127, display -63 to +63).
    pub resonance_offset: u8,
    /// Attack Time Offset (1-127, display -63 to +63).
    pub attack_time_offset: u8,
    /// Release Time Offset (1-127, display -63 to +63).
    pub release_time_offset: u8,
    /// Velocity Sens Offset (1-127, display -63 to +63).
    pub velocity_sens_offset: u8,
    /// PMT Control Switch (0=OFF, 1=ON).
    pub pmt_control_switch: u8,
    /// Pitch Bend Range Up (0-48).
    pub pitch_bend_range_up: u8,
    /// Pitch Bend Range Down (0-48).
    pub pitch_bend_range_down: u8,
    /// Matrix Controls (4 slots).
    pub matrix_controls: Vec<MatrixControl>,
}

impl Default for PcmSynthCommon {
    fn default() -> Self {
        Self {
            tone_name: String::new(),
            tone_level: 127,
            tone_pan: 64,     // center
            tone_priority: 0, // LAST
            coarse_tune: 64,  // 0
            fine_tune: 64,    // 0
            octave_shift: 64, // 0
            stretch_tune_depth: 0,
            analog_feel: 0,
            mono_poly: 1, // POLY
            legato_switch: 0,
            legato_retrigger: 0,
            portamento_switch: 0,
            portamento_mode: 0,
            portamento_type: 0,
            portamento_start: 0,
            portamento_time: 0,
            cutoff_offset: 64,        // 0
            resonance_offset: 64,     // 0
            attack_time_offset: 64,   // 0
            release_time_offset: 64,  // 0
            velocity_sens_offset: 64, // 0
            pmt_control_switch: 0,
            pitch_bend_range_up: 2,
            pitch_bend_range_down: 2,
            matrix_controls: vec![
                MatrixControl::default(),
                MatrixControl::default(),
                MatrixControl::default(),
                MatrixControl::default(),
            ],
        }
    }
}

/// PMT (Partial Mix Table) per-partial entry (9 bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PmtPartialEntry {
    /// Partial Switch (0=OFF, 1=ON).
    pub partial_switch: u8,
    /// Key Range Lower (0-127).
    pub key_range_lower: u8,
    /// Key Range Upper (0-127).
    pub key_range_upper: u8,
    /// Key Fade Width Lower (0-127).
    pub key_fade_lower: u8,
    /// Key Fade Width Upper (0-127).
    pub key_fade_upper: u8,
    /// Velocity Range Lower (1-127).
    pub velocity_range_lower: u8,
    /// Velocity Range Upper (1-127).
    pub velocity_range_upper: u8,
    /// Velocity Fade Width Lower (0-127).
    pub velocity_fade_lower: u8,
    /// Velocity Fade Width Upper (0-127).
    pub velocity_fade_upper: u8,
}

impl Default for PmtPartialEntry {
    fn default() -> Self {
        Self {
            partial_switch: 1,
            key_range_lower: 0,
            key_range_upper: 127,
            key_fade_lower: 0,
            key_fade_upper: 0,
            velocity_range_lower: 1,
            velocity_range_upper: 127,
            velocity_fade_lower: 0,
            velocity_fade_upper: 0,
        }
    }
}

/// PCM Synth Tone PMT (Partial Mix Table) parameters (41 bytes at offset
/// `00 10 00`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmSynthPmt {
    /// Structure Type 1&2 (0-9, display 1-10).
    pub structure_type_12: u8,
    /// Booster 1&2 (0-3: 0, +6, +12, +18 dB).
    pub booster_12: u8,
    /// Structure Type 3&4 (0-9, display 1-10).
    pub structure_type_34: u8,
    /// Booster 3&4 (0-3: 0, +6, +12, +18 dB).
    pub booster_34: u8,
    /// PMT Velocity Control (0-3: OFF, ON, RANDOM, CYCLE).
    pub pmt_velocity_control: u8,
    /// Per-partial PMT entries (4 partials).
    pub partial_entries: [PmtPartialEntry; 4],
}

impl Default for PcmSynthPmt {
    #[allow(clippy::derivable_impls)]
    fn default() -> Self {
        Self {
            structure_type_12: 0,
            booster_12: 0,
            structure_type_34: 0,
            booster_34: 0,
            pmt_velocity_control: 0,
            partial_entries: [
                PmtPartialEntry::default(),
                PmtPartialEntry::default(),
                PmtPartialEntry::default(),
                PmtPartialEntry::default(),
            ],
        }
    }
}

/// PCM Synth Tone Partial parameters (282 bytes at offsets `00 20 00` etc).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmSynthPartial {
    // -- General --
    /// Level (0-127).
    pub level: u8,
    /// Coarse Tune (16-112, display -48 to +48).
    pub coarse_tune: u8,
    /// Fine Tune (14-114, display -50 to +50).
    pub fine_tune: u8,
    /// Random Pitch Depth (0-30).
    pub random_pitch_depth: u8,
    /// Pan (0-127, display L64-63R).
    pub pan: u8,
    /// Pan Keyfollow (54-74, display -100 to +100).
    pub pan_keyfollow: u8,
    /// Random Pan Depth (0-63).
    pub random_pan_depth: u8,
    /// Alternate Pan Depth (1-127, display -63 to +63).
    pub alternate_pan_depth: u8,
    /// Env Mode (0=NO-SUS, 1=SUSTAIN).
    pub env_mode: u8,
    /// Delay Mode (0-3).
    pub delay_mode: u8,
    /// Delay Time (0-149, nibblized 2 bytes).
    pub delay_time: u16,

    // -- Output --
    /// Output Level (0-127).
    pub output_level: u8,
    /// Chorus Send Level (0-127).
    pub chorus_send: u8,
    /// Reverb Send Level (0-127).
    pub reverb_send: u8,

    // -- Receive switches --
    /// Receive Bender (0=OFF, 1=ON).
    pub receive_bender: u8,
    /// Receive Expression (0=OFF, 1=ON).
    pub receive_expression: u8,
    /// Receive Hold-1 (0=OFF, 1=ON).
    pub receive_hold1: u8,
    /// Redamper Switch (0=OFF, 1=ON).
    pub redamper_switch: u8,

    // -- Partial control switches --
    /// Control 1 switches (4 values, each 0-2: OFF, ON, REVERSE).
    pub control1_switches: [u8; 4],
    /// Control 2 switches (4 values, each 0-2: OFF, ON, REVERSE).
    pub control2_switches: [u8; 4],
    /// Control 3 switches (4 values, each 0-2: OFF, ON, REVERSE).
    pub control3_switches: [u8; 4],
    /// Control 4 switches (4 values, each 0-2: OFF, ON, REVERSE).
    pub control4_switches: [u8; 4],

    // -- Wave --
    /// Wave Group Type (0-3: INT, SRX).
    pub wave_group_type: u8,
    /// Wave Group ID (0-16384, nibblized 4 bytes).
    pub wave_group_id: u16,
    /// Wave Number L (0-16384, nibblized 4 bytes).
    pub wave_number_l: u16,
    /// Wave Number R (0-16384, nibblized 4 bytes).
    pub wave_number_r: u16,
    /// Wave Gain (0-3: -6, 0, +6, +12 dB).
    pub wave_gain: u8,
    /// Wave FXM Switch (0=OFF, 1=ON).
    #[cfg_attr(feature = "serde", serde(rename = "waveFxmSwitch"))]
    pub wave_fxm_switch: u8,
    /// Wave FXM Color (0-3, display 1-4).
    #[cfg_attr(feature = "serde", serde(rename = "waveFxmColor"))]
    pub wave_fxm_color: u8,
    /// Wave FXM Depth (0-16).
    #[cfg_attr(feature = "serde", serde(rename = "waveFxmDepth"))]
    pub wave_fxm_depth: u8,
    /// Wave Tempo Sync (0=OFF, 1=ON).
    pub wave_tempo_sync: u8,
    /// Wave Pitch Keyfollow (44-84, display -200 to +200).
    pub wave_pitch_keyfollow: u8,

    // -- Pitch Envelope --
    /// Pitch Env Depth (52-76, display -12 to +12).
    pub pitch_env_depth: u8,
    /// Pitch Env Velocity Sens (1-127, display -63 to +63).
    pub pitch_env_velocity_sens: u8,
    /// Pitch Env T1 Velocity Sens (1-127, display -63 to +63).
    pub pitch_env_t1_velocity_sens: u8,
    /// Pitch Env T4 Velocity Sens (1-127, display -63 to +63).
    pub pitch_env_t4_velocity_sens: u8,
    /// Pitch Env Time Keyfollow (54-74, display -100 to +100).
    pub pitch_env_time_keyfollow: u8,
    /// Pitch Env Times (4 values, each 0-127: T1-T4).
    pub pitch_env_time: [u8; 4],
    /// Pitch Env Levels (5 values, each 1-127: L0-L4, display -63 to +63).
    pub pitch_env_level: [u8; 5],

    // -- TVF (Filter) --
    /// TVF Filter Type (0-6: OFF, LPF, BPF, HPF, PKG, LPF2, LPF3).
    #[cfg_attr(feature = "serde", serde(rename = "tvfFilterType"))]
    pub tvf_filter_type: u8,
    /// TVF Cutoff Frequency (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "tvfCutoffFrequency"))]
    pub tvf_cutoff_frequency: u8,
    /// TVF Cutoff Keyfollow (44-84, display -200 to +200).
    #[cfg_attr(feature = "serde", serde(rename = "tvfCutoffKeyfollow"))]
    pub tvf_cutoff_keyfollow: u8,
    /// TVF Cutoff Velocity Curve (0-7: FIXED, 1-7).
    #[cfg_attr(feature = "serde", serde(rename = "tvfCutoffVelocityCurve"))]
    pub tvf_cutoff_velocity_curve: u8,
    /// TVF Cutoff Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfCutoffVelocitySens"))]
    pub tvf_cutoff_velocity_sens: u8,
    /// TVF Resonance (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "tvfResonance"))]
    pub tvf_resonance: u8,
    /// TVF Resonance Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfResonanceVelocitySens"))]
    pub tvf_resonance_velocity_sens: u8,
    /// TVF Env Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvDepth"))]
    pub tvf_env_depth: u8,
    /// TVF Env Velocity Curve (0-7: FIXED, 1-7).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvVelocityCurve"))]
    pub tvf_env_velocity_curve: u8,
    /// TVF Env Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvVelocitySens"))]
    pub tvf_env_velocity_sens: u8,
    /// TVF Env T1 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvT1VelocitySens"))]
    pub tvf_env_t1_velocity_sens: u8,
    /// TVF Env T4 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvT4VelocitySens"))]
    pub tvf_env_t4_velocity_sens: u8,
    /// TVF Env Time Keyfollow (54-74, display -100 to +100).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvTimeKeyfollow"))]
    pub tvf_env_time_keyfollow: u8,
    /// TVF Env Times (4 values, each 0-127: T1-T4).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvTime"))]
    pub tvf_env_time: [u8; 4],
    /// TVF Env Levels (5 values, each 0-127: L0-L4).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvLevel"))]
    pub tvf_env_level: [u8; 5],

    // -- TVA (Amplifier) --
    /// TVA Bias Level (54-74, display -100 to +100).
    #[cfg_attr(feature = "serde", serde(rename = "tvaBiasLevel"))]
    pub tva_bias_level: u8,
    /// TVA Bias Position (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "tvaBiasPosition"))]
    pub tva_bias_position: u8,
    /// TVA Bias Direction (0-3: LOWER, UPPER, LOWER&UPPER, ALL).
    #[cfg_attr(feature = "serde", serde(rename = "tvaBiasDirection"))]
    pub tva_bias_direction: u8,
    /// TVA Level Velocity Curve (0-7: FIXED, 1-7).
    #[cfg_attr(feature = "serde", serde(rename = "tvaLevelVelocityCurve"))]
    pub tva_level_velocity_curve: u8,
    /// TVA Level Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvaLevelVelocitySens"))]
    pub tva_level_velocity_sens: u8,
    /// TVA Env T1 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvT1VelocitySens"))]
    pub tva_env_t1_velocity_sens: u8,
    /// TVA Env T4 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvT4VelocitySens"))]
    pub tva_env_t4_velocity_sens: u8,
    /// TVA Env Time Keyfollow (54-74, display -100 to +100).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvTimeKeyfollow"))]
    pub tva_env_time_keyfollow: u8,
    /// TVA Env Times (4 values, each 0-127: T1-T4).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvTime"))]
    pub tva_env_time: [u8; 4],
    /// TVA Env Levels (3 values, each 0-127: L1-L3; L0 is always max).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvLevel"))]
    pub tva_env_level: [u8; 3],

    // -- LFO1 --
    /// LFO1 Waveform (0-12).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1Waveform"))]
    pub lfo1_waveform: u8,
    /// LFO1 Rate (0-149, nibblized 2 bytes).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1Rate"))]
    pub lfo1_rate: u16,
    /// LFO1 Offset (0-4).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1Offset"))]
    pub lfo1_offset: u8,
    /// LFO1 Rate Detune (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1RateDetune"))]
    pub lfo1_rate_detune: u8,
    /// LFO1 Delay Time (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1DelayTime"))]
    pub lfo1_delay_time: u8,
    /// LFO1 Delay Time Keyfollow (54-74, display -100 to +100).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1DelayTimeKeyfollow"))]
    pub lfo1_delay_time_keyfollow: u8,
    /// LFO1 Fade Mode (0-3).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1FadeMode"))]
    pub lfo1_fade_mode: u8,
    /// LFO1 Fade Time (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1FadeTime"))]
    pub lfo1_fade_time: u8,
    /// LFO1 Key Trigger (0=OFF, 1=ON).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1KeyTrigger"))]
    pub lfo1_key_trigger: u8,
    /// LFO1 Pitch Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1PitchDepth"))]
    pub lfo1_pitch_depth: u8,
    /// LFO1 TVF Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1TvfDepth"))]
    pub lfo1_tvf_depth: u8,
    /// LFO1 TVA Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1TvaDepth"))]
    pub lfo1_tva_depth: u8,
    /// LFO1 Pan Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo1PanDepth"))]
    pub lfo1_pan_depth: u8,

    // -- LFO2 --
    /// LFO2 Waveform (0-12).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2Waveform"))]
    pub lfo2_waveform: u8,
    /// LFO2 Rate (0-149, nibblized 2 bytes).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2Rate"))]
    pub lfo2_rate: u16,
    /// LFO2 Offset (0-4).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2Offset"))]
    pub lfo2_offset: u8,
    /// LFO2 Rate Detune (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2RateDetune"))]
    pub lfo2_rate_detune: u8,
    /// LFO2 Delay Time (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2DelayTime"))]
    pub lfo2_delay_time: u8,
    /// LFO2 Delay Time Keyfollow (54-74, display -100 to +100).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2DelayTimeKeyfollow"))]
    pub lfo2_delay_time_keyfollow: u8,
    /// LFO2 Fade Mode (0-3).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2FadeMode"))]
    pub lfo2_fade_mode: u8,
    /// LFO2 Fade Time (0-127).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2FadeTime"))]
    pub lfo2_fade_time: u8,
    /// LFO2 Key Trigger (0=OFF, 1=ON).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2KeyTrigger"))]
    pub lfo2_key_trigger: u8,
    /// LFO2 Pitch Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2PitchDepth"))]
    pub lfo2_pitch_depth: u8,
    /// LFO2 TVF Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2TvfDepth"))]
    pub lfo2_tvf_depth: u8,
    /// LFO2 TVA Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2TvaDepth"))]
    pub lfo2_tva_depth: u8,
    /// LFO2 Pan Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "lfo2PanDepth"))]
    pub lfo2_pan_depth: u8,

    // -- LFO Step Sequencer --
    /// LFO Step Type (0-1).
    #[cfg_attr(feature = "serde", serde(rename = "lfoStepType"))]
    pub lfo_step_type: u8,
    /// LFO Step Values (16 values, each 28-100, display -36 to +36).
    #[cfg_attr(feature = "serde", serde(rename = "lfoStepValues"))]
    pub lfo_step_values: [u8; 16],
}

impl Default for PcmSynthPartial {
    fn default() -> Self {
        Self {
            // General
            level: 127,
            coarse_tune: 64, // 0
            fine_tune: 64,   // 0
            random_pitch_depth: 0,
            pan: 64,           // center
            pan_keyfollow: 64, // 0
            random_pan_depth: 0,
            alternate_pan_depth: 64, // 0
            env_mode: 1,             // SUSTAIN
            delay_mode: 0,
            delay_time: 0,

            // Output
            output_level: 127,
            chorus_send: 0,
            reverb_send: 0,

            // Receive switches
            receive_bender: 1,
            receive_expression: 1,
            receive_hold1: 1,
            redamper_switch: 1,

            // Partial control switches
            control1_switches: [0; 4],
            control2_switches: [0; 4],
            control3_switches: [0; 4],
            control4_switches: [0; 4],

            // Wave
            wave_group_type: 0,
            wave_group_id: 1, // INT, as `WaveGroup::Int.params()` writes
            wave_number_l: 0,
            wave_number_r: 0,
            wave_gain: 1, // 0 dB
            wave_fxm_switch: 0,
            wave_fxm_color: 0,
            wave_fxm_depth: 0,
            wave_tempo_sync: 0,
            wave_pitch_keyfollow: 64, // 0

            // Pitch Envelope
            pitch_env_depth: 64,            // 0
            pitch_env_velocity_sens: 64,    // 0
            pitch_env_t1_velocity_sens: 64, // 0
            pitch_env_t4_velocity_sens: 64, // 0
            pitch_env_time_keyfollow: 64,   // 0
            pitch_env_time: [0; 4],
            pitch_env_level: [64; 5], // all center

            // TVF
            tvf_filter_type: 1, // LPF
            tvf_cutoff_frequency: 127,
            tvf_cutoff_keyfollow: 64, // 0
            tvf_cutoff_velocity_curve: 0,
            tvf_cutoff_velocity_sens: 64,
            tvf_resonance: 0,
            tvf_resonance_velocity_sens: 64,
            tvf_env_depth: 64,
            tvf_env_velocity_curve: 0,
            tvf_env_velocity_sens: 64,
            tvf_env_t1_velocity_sens: 64,
            tvf_env_t4_velocity_sens: 64,
            tvf_env_time_keyfollow: 64,
            tvf_env_time: [0; 4],
            tvf_env_level: [0; 5],

            // TVA
            tva_bias_level: 64,
            tva_bias_position: 64,
            tva_bias_direction: 0,
            tva_level_velocity_curve: 0,
            tva_level_velocity_sens: 64,
            tva_env_t1_velocity_sens: 64,
            tva_env_t4_velocity_sens: 64,
            tva_env_time_keyfollow: 64,
            tva_env_time: [0; 4],
            tva_env_level: [127; 3], // full level until release (not yet checked on the device: #250)

            // LFO1
            lfo1_waveform: 0,
            lfo1_rate: 0,
            lfo1_offset: 0,
            lfo1_rate_detune: 0,
            lfo1_delay_time: 0,
            lfo1_delay_time_keyfollow: 64,
            lfo1_fade_mode: 0,
            lfo1_fade_time: 0,
            lfo1_key_trigger: 0,
            lfo1_pitch_depth: 64,
            lfo1_tvf_depth: 64,
            lfo1_tva_depth: 64,
            lfo1_pan_depth: 64,

            // LFO2
            lfo2_waveform: 0,
            lfo2_rate: 0,
            lfo2_offset: 0,
            lfo2_rate_detune: 0,
            lfo2_delay_time: 0,
            lfo2_delay_time_keyfollow: 64,
            lfo2_fade_mode: 0,
            lfo2_fade_time: 0,
            lfo2_key_trigger: 0,
            lfo2_pitch_depth: 64,
            lfo2_tvf_depth: 64,
            lfo2_tva_depth: 64,
            lfo2_pan_depth: 64,

            // LFO Step Sequencer
            lfo_step_type: 0,
            lfo_step_values: [64; 16], // center (0)
        }
    }
}

/// PCM Synth Tone Common2 parameters (60 bytes at offset `00 30 00`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmSynthCommon2 {
    /// Tone Category (0-127).
    pub tone_category: u8,
    /// Tone Number (0-255, nibblized 2 bytes).
    pub tone_number: u16,
    /// Phrase Octave Shift (61-67, display -3 to +3).
    pub phrase_octave_shift: u8,
    /// TFX Switch (0=OFF, 1=ON).
    #[cfg_attr(feature = "serde", serde(rename = "tfxSwitch"))]
    pub tfx_switch: u8,
    /// Phrase Number (0-65535, nibblized 4 bytes).
    pub phrase_number: u16,
}

impl Default for PcmSynthCommon2 {
    fn default() -> Self {
        Self {
            tone_category: 0,
            tone_number: 0,
            phrase_octave_shift: 64, // 0
            tfx_switch: 0,
            phrase_number: 0,
        }
    }
}

/// Full PCM Synth Tone state for one part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmSynthState {
    /// Common parameters.
    pub common: PcmSynthCommon,
    /// Partial Mix Table parameters.
    pub pmt: PcmSynthPmt,
    /// Per-partial parameters (4 partials).
    pub partials: [PcmSynthPartial; 4],
    /// Common2 parameters.
    pub common2: PcmSynthCommon2,
    /// MFX parameters.
    pub mfx: MfxState,
}

// ---------------------------------------------------------------------------
// Parse functions
// ---------------------------------------------------------------------------

/// Parse a PCM Synth Tone Common dump (0x50 = 80 bytes).
///
/// Offsets follow the MIDI implementation doc (`docs/midi/06-pcm-synth-tone.md`).
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pcms_common(data: &[u8]) -> Result<PcmSynthCommon, crate::ToneParseError> {
    if data.len() < PCMS_COMMON_SIZE {
        return Err(crate::ToneParseError {
            expected: PCMS_COMMON_SIZE,
            got: data.len(),
        });
    }
    let mut c = PcmSynthCommon::default();

    use PcmsCommonField as F;
    c.tone_name = parse_ascii_name(data);
    c.tone_level = F::ToneLevel.byte(data);
    c.tone_pan = F::TonePan.byte(data);
    c.tone_priority = F::TonePriority.byte(data);
    c.coarse_tune = F::CoarseTune.byte(data);
    c.fine_tune = F::FineTune.byte(data);
    c.octave_shift = F::OctaveShift.byte(data);
    c.stretch_tune_depth = F::StretchTuneDepth.byte(data);
    c.analog_feel = F::AnalogFeel.byte(data);
    c.mono_poly = F::MonoPoly.byte(data);
    c.legato_switch = F::LegatoSwitch.byte(data);
    c.legato_retrigger = F::LegatoRetrigger.byte(data);
    c.portamento_switch = F::PortamentoSwitch.byte(data);
    c.portamento_mode = F::PortamentoMode.byte(data);
    c.portamento_type = F::PortamentoType.byte(data);
    c.portamento_start = F::PortamentoStart.byte(data);
    c.portamento_time = F::PortamentoTime.byte(data);
    c.cutoff_offset = F::CutoffOffset.byte(data);
    c.resonance_offset = F::ResonanceOffset.byte(data);
    c.attack_time_offset = F::AttackTimeOffset.byte(data);
    c.release_time_offset = F::ReleaseTimeOffset.byte(data);
    c.velocity_sens_offset = F::VelocitySensOffset.byte(data);
    c.pmt_control_switch = F::PmtControlSwitch.byte(data);
    c.pitch_bend_range_up = F::PitchBendRangeUp.byte(data);
    c.pitch_bend_range_down = F::PitchBendRangeDown.byte(data);

    for (i, mc) in c.matrix_controls.iter_mut().enumerate() {
        mc.source = F::MatrixControlSource.element(data, i);
        mc.destinations = F::MATRIX_CONTROL_DESTINATIONS.map(|f| f.element(data, i));
        mc.sensitivities = F::MATRIX_CONTROL_SENS.map(|f| f.element(data, i));
    }

    Ok(c)
}

/// Parse a PCM Synth Tone PMT dump (0x29 = 41 bytes).
///
/// Offsets follow the MIDI implementation doc.
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pcms_pmt(data: &[u8]) -> Result<PcmSynthPmt, crate::ToneParseError> {
    if data.len() < PCMS_PMT_SIZE {
        return Err(crate::ToneParseError {
            expected: PCMS_PMT_SIZE,
            got: data.len(),
        });
    }
    let mut pmt = PcmSynthPmt::default();

    use PcmsPmtField as F;
    pmt.structure_type_12 = F::StructureType12.byte(data);
    pmt.booster_12 = F::Booster12.byte(data);
    pmt.structure_type_34 = F::StructureType34.byte(data);
    pmt.booster_34 = F::Booster34.byte(data);
    pmt.pmt_velocity_control = F::PmtVelocityControl.byte(data);

    for (i, e) in pmt.partial_entries.iter_mut().enumerate() {
        e.partial_switch = F::PartialSwitch.element(data, i);
        e.key_range_lower = F::KeyRangeLower.element(data, i);
        e.key_range_upper = F::KeyRangeUpper.element(data, i);
        e.key_fade_lower = F::KeyFadeLower.element(data, i);
        e.key_fade_upper = F::KeyFadeUpper.element(data, i);
        e.velocity_range_lower = F::VelocityRangeLower.element(data, i);
        e.velocity_range_upper = F::VelocityRangeUpper.element(data, i);
        e.velocity_fade_lower = F::VelocityFadeLower.element(data, i);
        e.velocity_fade_upper = F::VelocityFadeUpper.element(data, i);
    }

    Ok(pmt)
}

/// Parse a PCM Synth Tone Partial dump (0x011A = 282 bytes).
///
/// The data array is a contiguous byte stream from the DT1 response.
/// Linear indices map directly to the SysEx offsets:
/// indices 0-127 = offsets `00 00`-`00 7F`, index 128 = offset `01 00`, etc.
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pcms_partial(data: &[u8]) -> Result<PcmSynthPartial, crate::ToneParseError> {
    if data.len() < PCMS_PARTIAL_SIZE {
        return Err(crate::ToneParseError {
            expected: PCMS_PARTIAL_SIZE,
            got: data.len(),
        });
    }
    let mut p = PcmSynthPartial::default();

    use PcmsPartialField as F;

    // -- General --
    p.level = F::Level.byte(data);
    p.coarse_tune = F::CoarseTune.byte(data);
    p.fine_tune = F::FineTune.byte(data);
    p.random_pitch_depth = F::RandomPitchDepth.byte(data);
    p.pan = F::Pan.byte(data);
    p.pan_keyfollow = F::PanKeyfollow.byte(data);
    p.random_pan_depth = F::RandomPanDepth.byte(data);
    p.alternate_pan_depth = F::AlternatePanDepth.byte(data);
    p.env_mode = F::EnvMode.byte(data);
    p.delay_mode = F::DelayMode.byte(data);
    p.delay_time = F::DelayTime.nibbles(data);

    // -- Output --
    p.output_level = F::OutputLevel.byte(data);
    p.chorus_send = F::ChorusSend.byte(data);
    p.reverb_send = F::ReverbSend.byte(data);

    // -- Receive switches --
    p.receive_bender = F::ReceiveBender.byte(data);
    p.receive_expression = F::ReceiveExpression.byte(data);
    p.receive_hold1 = F::ReceiveHold1.byte(data);
    p.redamper_switch = F::RedamperSwitch.byte(data);

    // -- Partial control switches --
    p.control1_switches = F::Control1Switch.elements(data);
    p.control2_switches = F::Control2Switch.elements(data);
    p.control3_switches = F::Control3Switch.elements(data);
    p.control4_switches = F::Control4Switch.elements(data);

    // -- Wave --
    p.wave_group_type = F::WaveGroupType.byte(data);
    p.wave_group_id = F::WaveGroupId.nibbles(data);
    p.wave_number_l = F::WaveNumberL.nibbles(data);
    p.wave_number_r = F::WaveNumberR.nibbles(data);
    p.wave_gain = F::WaveGain.byte(data);
    p.wave_fxm_switch = F::WaveFxmSwitch.byte(data);
    p.wave_fxm_color = F::WaveFxmColor.byte(data);
    p.wave_fxm_depth = F::WaveFxmDepth.byte(data);
    p.wave_tempo_sync = F::WaveTempoSync.byte(data);
    p.wave_pitch_keyfollow = F::WavePitchKeyfollow.byte(data);

    // -- Pitch Envelope --
    p.pitch_env_depth = F::PitchEnvDepth.byte(data);
    p.pitch_env_velocity_sens = F::PitchEnvVelocitySens.byte(data);
    p.pitch_env_t1_velocity_sens = F::PitchEnvT1VelocitySens.byte(data);
    p.pitch_env_t4_velocity_sens = F::PitchEnvT4VelocitySens.byte(data);
    p.pitch_env_time_keyfollow = F::PitchEnvTimeKeyfollow.byte(data);
    p.pitch_env_time = F::PitchEnvTime.elements(data);
    p.pitch_env_level = F::PitchEnvLevel.elements(data);

    // -- TVF (Filter) --
    p.tvf_filter_type = F::TvfFilterType.byte(data);
    p.tvf_cutoff_frequency = F::TvfCutoffFrequency.byte(data);
    p.tvf_cutoff_keyfollow = F::TvfCutoffKeyfollow.byte(data);
    p.tvf_cutoff_velocity_curve = F::TvfCutoffVelocityCurve.byte(data);
    p.tvf_cutoff_velocity_sens = F::TvfCutoffVelocitySens.byte(data);
    p.tvf_resonance = F::TvfResonance.byte(data);
    p.tvf_resonance_velocity_sens = F::TvfResonanceVelocitySens.byte(data);
    p.tvf_env_depth = F::TvfEnvDepth.byte(data);
    p.tvf_env_velocity_curve = F::TvfEnvVelocityCurve.byte(data);
    p.tvf_env_velocity_sens = F::TvfEnvVelocitySens.byte(data);
    p.tvf_env_t1_velocity_sens = F::TvfEnvT1VelocitySens.byte(data);
    p.tvf_env_t4_velocity_sens = F::TvfEnvT4VelocitySens.byte(data);
    p.tvf_env_time_keyfollow = F::TvfEnvTimeKeyfollow.byte(data);
    p.tvf_env_time = F::TvfEnvTime.elements(data);
    p.tvf_env_level = F::TvfEnvLevel.elements(data);

    // -- TVA (Amplifier) --
    p.tva_bias_level = F::TvaBiasLevel.byte(data);
    p.tva_bias_position = F::TvaBiasPosition.byte(data);
    p.tva_bias_direction = F::TvaBiasDirection.byte(data);
    p.tva_level_velocity_curve = F::TvaLevelVelocityCurve.byte(data);
    p.tva_level_velocity_sens = F::TvaLevelVelocitySens.byte(data);
    p.tva_env_t1_velocity_sens = F::TvaEnvT1VelocitySens.byte(data);
    p.tva_env_t4_velocity_sens = F::TvaEnvT4VelocitySens.byte(data);
    p.tva_env_time_keyfollow = F::TvaEnvTimeKeyfollow.byte(data);
    p.tva_env_time = F::TvaEnvTime.elements(data);
    p.tva_env_level = F::TvaEnvLevel.elements(data);

    // -- LFO1 --
    p.lfo1_waveform = F::Lfo1Waveform.byte(data);
    p.lfo1_rate = F::Lfo1Rate.nibbles(data);
    p.lfo1_offset = F::Lfo1Offset.byte(data);
    p.lfo1_rate_detune = F::Lfo1RateDetune.byte(data);
    p.lfo1_delay_time = F::Lfo1DelayTime.byte(data);
    p.lfo1_delay_time_keyfollow = F::Lfo1DelayTimeKeyfollow.byte(data);
    p.lfo1_fade_mode = F::Lfo1FadeMode.byte(data);
    p.lfo1_fade_time = F::Lfo1FadeTime.byte(data);
    p.lfo1_key_trigger = F::Lfo1KeyTrigger.byte(data);
    p.lfo1_pitch_depth = F::Lfo1PitchDepth.byte(data);
    p.lfo1_tvf_depth = F::Lfo1TvfDepth.byte(data);
    p.lfo1_tva_depth = F::Lfo1TvaDepth.byte(data);
    p.lfo1_pan_depth = F::Lfo1PanDepth.byte(data);

    // -- LFO2 --
    p.lfo2_waveform = F::Lfo2Waveform.byte(data);
    p.lfo2_rate = F::Lfo2Rate.nibbles(data);
    p.lfo2_offset = F::Lfo2Offset.byte(data);
    p.lfo2_rate_detune = F::Lfo2RateDetune.byte(data);
    p.lfo2_delay_time = F::Lfo2DelayTime.byte(data);
    p.lfo2_delay_time_keyfollow = F::Lfo2DelayTimeKeyfollow.byte(data);
    p.lfo2_fade_mode = F::Lfo2FadeMode.byte(data);
    p.lfo2_fade_time = F::Lfo2FadeTime.byte(data);
    p.lfo2_key_trigger = F::Lfo2KeyTrigger.byte(data);
    p.lfo2_pitch_depth = F::Lfo2PitchDepth.byte(data);
    p.lfo2_tvf_depth = F::Lfo2TvfDepth.byte(data);
    p.lfo2_tva_depth = F::Lfo2TvaDepth.byte(data);
    p.lfo2_pan_depth = F::Lfo2PanDepth.byte(data);

    // -- LFO Step Sequencer --
    p.lfo_step_type = F::LfoStepType.byte(data);
    p.lfo_step_values = F::LfoStep.elements(data);

    Ok(p)
}

/// Parse a PCM Synth Tone Common2 dump (0x3C = 60 bytes).
///
/// Offsets follow the MIDI implementation doc.
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pcms_common2(data: &[u8]) -> Result<PcmSynthCommon2, crate::ToneParseError> {
    if data.len() < PCMS_COMMON2_SIZE {
        return Err(crate::ToneParseError {
            expected: PCMS_COMMON2_SIZE,
            got: data.len(),
        });
    }
    let mut c2 = PcmSynthCommon2::default();

    use PcmsCommon2Field as F;
    c2.tone_category = F::ToneCategory.byte(data);
    c2.tone_number = F::ToneNumber.nibbles(data);
    c2.phrase_octave_shift = F::PhraseOctaveShift.byte(data);
    c2.tfx_switch = F::TfxSwitch.byte(data);
    c2.phrase_number = F::PhraseNumber.nibbles(data);

    Ok(c2)
}

// ---------------------------------------------------------------------------
// Encode functions: the inverse of the parse functions. Reserve bytes are 0.
// ---------------------------------------------------------------------------

/// Encode a PCM Synth Tone Common block (0x50 = 80 bytes), the inverse of
/// [`parse_pcms_common`].
pub fn encode_pcms_common(c: &PcmSynthCommon) -> Vec<u8> {
    let mut data = vec![0; PCMS_COMMON_SIZE];
    data[..PCMS_NAME_LEN].copy_from_slice(&encode_ascii_name(&c.tone_name, PCMS_NAME_LEN));

    use PcmsCommonField as F;
    F::ToneLevel.set_byte(&mut data, c.tone_level);
    F::TonePan.set_byte(&mut data, c.tone_pan);
    F::TonePriority.set_byte(&mut data, c.tone_priority);
    F::CoarseTune.set_byte(&mut data, c.coarse_tune);
    F::FineTune.set_byte(&mut data, c.fine_tune);
    F::OctaveShift.set_byte(&mut data, c.octave_shift);
    F::StretchTuneDepth.set_byte(&mut data, c.stretch_tune_depth);
    F::AnalogFeel.set_byte(&mut data, c.analog_feel);
    F::MonoPoly.set_byte(&mut data, c.mono_poly);
    F::LegatoSwitch.set_byte(&mut data, c.legato_switch);
    F::LegatoRetrigger.set_byte(&mut data, c.legato_retrigger);
    F::PortamentoSwitch.set_byte(&mut data, c.portamento_switch);
    F::PortamentoMode.set_byte(&mut data, c.portamento_mode);
    F::PortamentoType.set_byte(&mut data, c.portamento_type);
    F::PortamentoStart.set_byte(&mut data, c.portamento_start);
    F::PortamentoTime.set_byte(&mut data, c.portamento_time);
    F::CutoffOffset.set_byte(&mut data, c.cutoff_offset);
    F::ResonanceOffset.set_byte(&mut data, c.resonance_offset);
    F::AttackTimeOffset.set_byte(&mut data, c.attack_time_offset);
    F::ReleaseTimeOffset.set_byte(&mut data, c.release_time_offset);
    F::VelocitySensOffset.set_byte(&mut data, c.velocity_sens_offset);
    F::PmtControlSwitch.set_byte(&mut data, c.pmt_control_switch);
    F::PitchBendRangeUp.set_byte(&mut data, c.pitch_bend_range_up);
    F::PitchBendRangeDown.set_byte(&mut data, c.pitch_bend_range_down);

    for (i, mc) in c.matrix_controls.iter().enumerate() {
        F::MatrixControlSource.set_element(&mut data, i, mc.source);
        for (f, &value) in F::MATRIX_CONTROL_DESTINATIONS.iter().zip(&mc.destinations) {
            f.set_element(&mut data, i, value);
        }
        for (f, &value) in F::MATRIX_CONTROL_SENS.iter().zip(&mc.sensitivities) {
            f.set_element(&mut data, i, value);
        }
    }
    data
}

/// Encode a PCM Synth Tone PMT block (0x29 = 41 bytes), the inverse of
/// [`parse_pcms_pmt`].
pub fn encode_pcms_pmt(pmt: &PcmSynthPmt) -> Vec<u8> {
    let mut data = vec![0; PCMS_PMT_SIZE];
    use PcmsPmtField as F;
    F::StructureType12.set_byte(&mut data, pmt.structure_type_12);
    F::Booster12.set_byte(&mut data, pmt.booster_12);
    F::StructureType34.set_byte(&mut data, pmt.structure_type_34);
    F::Booster34.set_byte(&mut data, pmt.booster_34);
    F::PmtVelocityControl.set_byte(&mut data, pmt.pmt_velocity_control);

    for (i, e) in pmt.partial_entries.iter().enumerate() {
        F::PartialSwitch.set_element(&mut data, i, e.partial_switch);
        F::KeyRangeLower.set_element(&mut data, i, e.key_range_lower);
        F::KeyRangeUpper.set_element(&mut data, i, e.key_range_upper);
        F::KeyFadeLower.set_element(&mut data, i, e.key_fade_lower);
        F::KeyFadeUpper.set_element(&mut data, i, e.key_fade_upper);
        F::VelocityRangeLower.set_element(&mut data, i, e.velocity_range_lower);
        F::VelocityRangeUpper.set_element(&mut data, i, e.velocity_range_upper);
        F::VelocityFadeLower.set_element(&mut data, i, e.velocity_fade_lower);
        F::VelocityFadeUpper.set_element(&mut data, i, e.velocity_fade_upper);
    }
    data
}

/// Encode a PCM Synth Tone Partial block (154 bytes), the inverse of
/// [`parse_pcms_partial`].
pub fn encode_pcms_partial(p: &PcmSynthPartial) -> Vec<u8> {
    let mut data = vec![0; PCMS_PARTIAL_SIZE];
    use PcmsPartialField as F;

    // -- General --
    F::Level.set_byte(&mut data, p.level);
    F::CoarseTune.set_byte(&mut data, p.coarse_tune);
    F::FineTune.set_byte(&mut data, p.fine_tune);
    F::RandomPitchDepth.set_byte(&mut data, p.random_pitch_depth);
    F::Pan.set_byte(&mut data, p.pan);
    F::PanKeyfollow.set_byte(&mut data, p.pan_keyfollow);
    F::RandomPanDepth.set_byte(&mut data, p.random_pan_depth);
    F::AlternatePanDepth.set_byte(&mut data, p.alternate_pan_depth);
    F::EnvMode.set_byte(&mut data, p.env_mode);
    F::DelayMode.set_byte(&mut data, p.delay_mode);
    F::DelayTime.set_nibbles(&mut data, p.delay_time);

    // -- Output --
    F::OutputLevel.set_byte(&mut data, p.output_level);
    F::ChorusSend.set_byte(&mut data, p.chorus_send);
    F::ReverbSend.set_byte(&mut data, p.reverb_send);

    // -- Receive switches --
    F::ReceiveBender.set_byte(&mut data, p.receive_bender);
    F::ReceiveExpression.set_byte(&mut data, p.receive_expression);
    F::ReceiveHold1.set_byte(&mut data, p.receive_hold1);
    F::RedamperSwitch.set_byte(&mut data, p.redamper_switch);

    // -- Partial control switches --
    F::Control1Switch.set_elements(&mut data, &p.control1_switches);
    F::Control2Switch.set_elements(&mut data, &p.control2_switches);
    F::Control3Switch.set_elements(&mut data, &p.control3_switches);
    F::Control4Switch.set_elements(&mut data, &p.control4_switches);

    // -- Wave --
    F::WaveGroupType.set_byte(&mut data, p.wave_group_type);
    F::WaveGroupId.set_nibbles(&mut data, p.wave_group_id);
    F::WaveNumberL.set_nibbles(&mut data, p.wave_number_l);
    F::WaveNumberR.set_nibbles(&mut data, p.wave_number_r);
    F::WaveGain.set_byte(&mut data, p.wave_gain);
    F::WaveFxmSwitch.set_byte(&mut data, p.wave_fxm_switch);
    F::WaveFxmColor.set_byte(&mut data, p.wave_fxm_color);
    F::WaveFxmDepth.set_byte(&mut data, p.wave_fxm_depth);
    F::WaveTempoSync.set_byte(&mut data, p.wave_tempo_sync);
    F::WavePitchKeyfollow.set_byte(&mut data, p.wave_pitch_keyfollow);

    // -- Pitch Envelope --
    F::PitchEnvDepth.set_byte(&mut data, p.pitch_env_depth);
    F::PitchEnvVelocitySens.set_byte(&mut data, p.pitch_env_velocity_sens);
    F::PitchEnvT1VelocitySens.set_byte(&mut data, p.pitch_env_t1_velocity_sens);
    F::PitchEnvT4VelocitySens.set_byte(&mut data, p.pitch_env_t4_velocity_sens);
    F::PitchEnvTimeKeyfollow.set_byte(&mut data, p.pitch_env_time_keyfollow);
    F::PitchEnvTime.set_elements(&mut data, &p.pitch_env_time);
    F::PitchEnvLevel.set_elements(&mut data, &p.pitch_env_level);

    // -- TVF (Filter) --
    F::TvfFilterType.set_byte(&mut data, p.tvf_filter_type);
    F::TvfCutoffFrequency.set_byte(&mut data, p.tvf_cutoff_frequency);
    F::TvfCutoffKeyfollow.set_byte(&mut data, p.tvf_cutoff_keyfollow);
    F::TvfCutoffVelocityCurve.set_byte(&mut data, p.tvf_cutoff_velocity_curve);
    F::TvfCutoffVelocitySens.set_byte(&mut data, p.tvf_cutoff_velocity_sens);
    F::TvfResonance.set_byte(&mut data, p.tvf_resonance);
    F::TvfResonanceVelocitySens.set_byte(&mut data, p.tvf_resonance_velocity_sens);
    F::TvfEnvDepth.set_byte(&mut data, p.tvf_env_depth);
    F::TvfEnvVelocityCurve.set_byte(&mut data, p.tvf_env_velocity_curve);
    F::TvfEnvVelocitySens.set_byte(&mut data, p.tvf_env_velocity_sens);
    F::TvfEnvT1VelocitySens.set_byte(&mut data, p.tvf_env_t1_velocity_sens);
    F::TvfEnvT4VelocitySens.set_byte(&mut data, p.tvf_env_t4_velocity_sens);
    F::TvfEnvTimeKeyfollow.set_byte(&mut data, p.tvf_env_time_keyfollow);
    F::TvfEnvTime.set_elements(&mut data, &p.tvf_env_time);
    F::TvfEnvLevel.set_elements(&mut data, &p.tvf_env_level);

    // -- TVA (Amplifier) --
    F::TvaBiasLevel.set_byte(&mut data, p.tva_bias_level);
    F::TvaBiasPosition.set_byte(&mut data, p.tva_bias_position);
    F::TvaBiasDirection.set_byte(&mut data, p.tva_bias_direction);
    F::TvaLevelVelocityCurve.set_byte(&mut data, p.tva_level_velocity_curve);
    F::TvaLevelVelocitySens.set_byte(&mut data, p.tva_level_velocity_sens);
    F::TvaEnvT1VelocitySens.set_byte(&mut data, p.tva_env_t1_velocity_sens);
    F::TvaEnvT4VelocitySens.set_byte(&mut data, p.tva_env_t4_velocity_sens);
    F::TvaEnvTimeKeyfollow.set_byte(&mut data, p.tva_env_time_keyfollow);
    F::TvaEnvTime.set_elements(&mut data, &p.tva_env_time);
    F::TvaEnvLevel.set_elements(&mut data, &p.tva_env_level);

    // -- LFO1 --
    F::Lfo1Waveform.set_byte(&mut data, p.lfo1_waveform);
    F::Lfo1Rate.set_nibbles(&mut data, p.lfo1_rate);
    F::Lfo1Offset.set_byte(&mut data, p.lfo1_offset);
    F::Lfo1RateDetune.set_byte(&mut data, p.lfo1_rate_detune);
    F::Lfo1DelayTime.set_byte(&mut data, p.lfo1_delay_time);
    F::Lfo1DelayTimeKeyfollow.set_byte(&mut data, p.lfo1_delay_time_keyfollow);
    F::Lfo1FadeMode.set_byte(&mut data, p.lfo1_fade_mode);
    F::Lfo1FadeTime.set_byte(&mut data, p.lfo1_fade_time);
    F::Lfo1KeyTrigger.set_byte(&mut data, p.lfo1_key_trigger);
    F::Lfo1PitchDepth.set_byte(&mut data, p.lfo1_pitch_depth);
    F::Lfo1TvfDepth.set_byte(&mut data, p.lfo1_tvf_depth);
    F::Lfo1TvaDepth.set_byte(&mut data, p.lfo1_tva_depth);
    F::Lfo1PanDepth.set_byte(&mut data, p.lfo1_pan_depth);

    // -- LFO2 --
    F::Lfo2Waveform.set_byte(&mut data, p.lfo2_waveform);
    F::Lfo2Rate.set_nibbles(&mut data, p.lfo2_rate);
    F::Lfo2Offset.set_byte(&mut data, p.lfo2_offset);
    F::Lfo2RateDetune.set_byte(&mut data, p.lfo2_rate_detune);
    F::Lfo2DelayTime.set_byte(&mut data, p.lfo2_delay_time);
    F::Lfo2DelayTimeKeyfollow.set_byte(&mut data, p.lfo2_delay_time_keyfollow);
    F::Lfo2FadeMode.set_byte(&mut data, p.lfo2_fade_mode);
    F::Lfo2FadeTime.set_byte(&mut data, p.lfo2_fade_time);
    F::Lfo2KeyTrigger.set_byte(&mut data, p.lfo2_key_trigger);
    F::Lfo2PitchDepth.set_byte(&mut data, p.lfo2_pitch_depth);
    F::Lfo2TvfDepth.set_byte(&mut data, p.lfo2_tvf_depth);
    F::Lfo2TvaDepth.set_byte(&mut data, p.lfo2_tva_depth);
    F::Lfo2PanDepth.set_byte(&mut data, p.lfo2_pan_depth);

    // -- LFO Step Sequencer --
    F::LfoStepType.set_byte(&mut data, p.lfo_step_type);
    F::LfoStep.set_elements(&mut data, &p.lfo_step_values);

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoders_invert_the_parsers() {
        let mut state = PcmSynthState::default();
        state.common.tone_name = "INIT TONE".into();
        state.common.matrix_controls[3].destinations[2] = 5;
        state.pmt.partial_entries[2].velocity_fade_upper = 9;
        state.partials[1].delay_time = 140;
        state.partials[1].lfo2_rate = 131;
        state.partials[1].wave_number_l = 300;
        let common = encode_pcms_common(&state.common);
        assert_eq!(common.len(), PCMS_COMMON_SIZE);
        assert_eq!(parse_pcms_common(&common).unwrap(), state.common);
        let pmt = encode_pcms_pmt(&state.pmt);
        assert_eq!(parse_pcms_pmt(&pmt).unwrap(), state.pmt);
        let partial = encode_pcms_partial(&state.partials[1]);
        assert_eq!(partial.len(), PCMS_PARTIAL_SIZE);
        assert_eq!(parse_pcms_partial(&partial).unwrap(), state.partials[1]);
    }

    #[test]
    fn default_partial_uses_int_group_id_1() {
        assert_eq!(PcmSynthPartial::default().wave_group_id, 1);
    }

    #[test]
    fn pcms_common_address_part1() {
        let addr = pcms_common_address(0);
        // Part 1 tone base = 19 00 00 00, PCMS has no tone type offset
        assert_eq!(addr, Address::new(0x19, 0x00, 0x00, 0x00));
    }

    #[test]
    fn pcms_common_param_address_tone_level() {
        let addr = pcms_common_param_address(0, 0x0E);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x00, 0x0E));
    }

    #[test]
    fn pcms_pmt_address_part1() {
        let addr = pcms_pmt_address(0);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x10, 0x00));
    }

    #[test]
    fn pcms_pmt_param_address_structure_type() {
        let addr = pcms_pmt_param_address(0, 0x00);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x10, 0x00));
    }

    #[test]
    fn pcms_partial_address_part1_partial1() {
        let addr = pcms_partial_address(0, 0);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x20, 0x00));
    }

    #[test]
    fn pcms_partial_address_part1_partial4() {
        let addr = pcms_partial_address(0, 3);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x26, 0x00));
    }

    #[test]
    fn pcms_partial_param_address_single_byte_offset() {
        // level at offset 0x00
        let addr = pcms_partial_param_address(0, 0, 0x00);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x20, 0x00));
    }

    #[test]
    fn pcms_partial_param_address_two_byte_offset() {
        // lfo2_delay_time at position 0x80 (offset 01 00)
        let addr = pcms_partial_param_address(0, 0, 0x80);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x21, 0x00));
    }

    #[test]
    fn pcms_partial_param_address_lfo_step_last() {
        // Last LFO step value at position 0x99 (offset 01 19)
        let addr = pcms_partial_param_address(0, 0, 0x99);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x21, 0x19));
    }

    #[test]
    fn pcms_partial_param_address_partial2_high_offset() {
        // partial 2 (offset 00 22 00) + position 0x88 (offset 01 08)
        let addr = pcms_partial_param_address(0, 1, 0x88);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x23, 0x08));
    }

    #[test]
    fn pcms_common2_address_part1() {
        let addr = pcms_common2_address(0);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x30, 0x00));
    }

    #[test]
    fn pcms_common2_param_address_tone_category() {
        let addr = pcms_common2_param_address(0, 0x10);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x30, 0x10));
    }

    #[test]
    fn pcms_common_address_part16() {
        let addr = pcms_common_address(15);
        // Part 16 tone base = 1C 60 00 00
        assert_eq!(addr, Address::new(0x1C, 0x60, 0x00, 0x00));
    }

    #[test]
    fn pcms_partial_address_part16_partial4() {
        let addr = pcms_partial_address(15, 3);
        // 1C 60 00 00 + 00 00 26 00 = 1C 60 26 00
        assert_eq!(addr, Address::new(0x1C, 0x60, 0x26, 0x00));
    }

    #[test]
    fn pcms_base_equals_temporary_tone_base() {
        for i in 0..16 {
            assert_eq!(pcms_base(i), params::temporary_tone_base(i));
        }
    }

    #[test]
    fn default_matrix_controls_count() {
        let common = PcmSynthCommon::default();
        assert_eq!(common.matrix_controls.len(), 4);
    }

    #[test]
    fn default_pmt_partial_entries_count() {
        let pmt = PcmSynthPmt::default();
        assert_eq!(pmt.partial_entries.len(), 4);
    }

    // -- Parse function tests --

    #[test]
    fn parse_common_basic() {
        let mut data = [0u8; PCMS_COMMON_SIZE];
        // Tone name "BrightPad   "
        let name = b"BrightPad   ";
        data[0x00..0x0C].copy_from_slice(name);
        data[0x0E] = 100; // tone level
        data[0x0F] = 32; // tone pan (L32)
        data[0x10] = 1; // priority = LOUDEST
        data[0x11] = 76; // coarse tune = +12
        data[0x12] = 78; // fine tune = +14
        data[0x13] = 65; // octave shift = +1
        data[0x14] = 2; // stretch tune depth
        data[0x15] = 50; // analog feel
        data[0x16] = 1; // POLY
        data[0x17] = 1; // legato on
        data[0x18] = 1; // legato retrigger on
        data[0x19] = 1; // portamento on
        data[0x1A] = 1; // portamento mode = LEGATO
        data[0x1B] = 1; // portamento type = TIME
        data[0x1C] = 1; // portamento start = NOTE
        data[0x1D] = 64; // portamento time
        data[0x22] = 70; // cutoff offset
        data[0x23] = 80; // resonance offset
        data[0x24] = 90; // attack time offset
        data[0x25] = 40; // release time offset
        data[0x26] = 50; // velocity sens offset
        data[0x28] = 1; // PMT control on
        data[0x29] = 12; // pitch bend up
        data[0x2A] = 24; // pitch bend down
        // Matrix Control 1: source=5, dest1=2, sens1=70
        data[0x2B] = 5;
        data[0x2C] = 2;
        data[0x2D] = 70;

        let c = parse_pcms_common(&data).unwrap();
        assert_eq!(c.tone_name, "BrightPad");
        assert_eq!(c.tone_level, 100);
        assert_eq!(c.tone_pan, 32);
        assert_eq!(c.tone_priority, 1);
        assert_eq!(c.coarse_tune, 76);
        assert_eq!(c.fine_tune, 78);
        assert_eq!(c.octave_shift, 65);
        assert_eq!(c.stretch_tune_depth, 2);
        assert_eq!(c.analog_feel, 50);
        assert_eq!(c.mono_poly, 1);
        assert_eq!(c.legato_switch, 1);
        assert_eq!(c.legato_retrigger, 1);
        assert_eq!(c.portamento_switch, 1);
        assert_eq!(c.portamento_mode, 1);
        assert_eq!(c.portamento_type, 1);
        assert_eq!(c.portamento_start, 1);
        assert_eq!(c.portamento_time, 64);
        assert_eq!(c.cutoff_offset, 70);
        assert_eq!(c.resonance_offset, 80);
        assert_eq!(c.attack_time_offset, 90);
        assert_eq!(c.release_time_offset, 40);
        assert_eq!(c.velocity_sens_offset, 50);
        assert_eq!(c.pmt_control_switch, 1);
        assert_eq!(c.pitch_bend_range_up, 12);
        assert_eq!(c.pitch_bend_range_down, 24);
        assert_eq!(c.matrix_controls[0].source, 5);
        assert_eq!(c.matrix_controls[0].destinations[0], 2);
        assert_eq!(c.matrix_controls[0].sensitivities[0], 70);
    }

    #[test]
    fn parse_common_short_returns_err() {
        assert!(parse_pcms_common(&[0u8; 10]).is_err());
    }

    #[test]
    fn parse_pmt_basic() {
        let mut data = [0u8; PCMS_PMT_SIZE];
        data[0x00] = 3; // structure_type_12
        data[0x01] = 2; // booster_12
        data[0x02] = 5; // structure_type_34
        data[0x03] = 1; // booster_34
        data[0x04] = 2; // pmt_velocity_control = RANDOM
        // Partial 1 entry at offset 0x05
        data[0x05] = 1; // switch ON
        data[0x06] = 24; // key range lower
        data[0x07] = 96; // key range upper
        data[0x08] = 10; // key fade lower
        data[0x09] = 15; // key fade upper
        data[0x0A] = 20; // velocity range lower
        data[0x0B] = 110; // velocity range upper
        data[0x0C] = 5; // velocity fade lower
        data[0x0D] = 8; // velocity fade upper
        // Partial 2 entry at offset 0x0E
        data[0x0E] = 0; // switch OFF

        let pmt = parse_pcms_pmt(&data).unwrap();
        assert_eq!(pmt.structure_type_12, 3);
        assert_eq!(pmt.booster_12, 2);
        assert_eq!(pmt.structure_type_34, 5);
        assert_eq!(pmt.booster_34, 1);
        assert_eq!(pmt.pmt_velocity_control, 2);
        assert_eq!(pmt.partial_entries[0].partial_switch, 1);
        assert_eq!(pmt.partial_entries[0].key_range_lower, 24);
        assert_eq!(pmt.partial_entries[0].key_range_upper, 96);
        assert_eq!(pmt.partial_entries[0].key_fade_lower, 10);
        assert_eq!(pmt.partial_entries[0].key_fade_upper, 15);
        assert_eq!(pmt.partial_entries[0].velocity_range_lower, 20);
        assert_eq!(pmt.partial_entries[0].velocity_range_upper, 110);
        assert_eq!(pmt.partial_entries[0].velocity_fade_lower, 5);
        assert_eq!(pmt.partial_entries[0].velocity_fade_upper, 8);
        assert_eq!(pmt.partial_entries[1].partial_switch, 0);
    }

    #[test]
    fn parse_partial_basic() {
        let mut data = [0u8; PCMS_PARTIAL_SIZE];
        data[0x00] = 100; // level
        data[0x01] = 76; // coarse tune
        data[0x02] = 78; // fine tune
        data[0x04] = 64; // pan center
        data[0x0C] = 120; // output level

        // Wave Number L = 0x1234 via nibblized 4 bytes
        data[0x2C] = 0x01;
        data[0x2D] = 0x02;
        data[0x2E] = 0x03;
        data[0x2F] = 0x04;
        // Wave Number R = 0x5678
        data[0x30] = 0x05;
        data[0x31] = 0x06;
        data[0x32] = 0x07;
        data[0x33] = 0x08;

        // TVF envelope times
        data[0x55] = 10;
        data[0x56] = 20;
        data[0x57] = 30;
        data[0x58] = 40;
        // TVF envelope levels
        data[0x59] = 50;
        data[0x5A] = 60;
        data[0x5B] = 70;
        data[0x5C] = 80;
        data[0x5D] = 90;

        // TVA envelope times
        data[0x66] = 11;
        data[0x67] = 22;
        data[0x68] = 33;
        data[0x69] = 44;
        // TVA envelope levels (L1-L3)
        data[0x6A] = 100;
        data[0x6B] = 110;
        data[0x6C] = 120;

        // LFO1 rate (nibblized 2 bytes): 0x09 0x05 = 0x95 = 149
        data[0x6E] = 0x09;
        data[0x6F] = 0x05;

        let p = parse_pcms_partial(&data).unwrap();
        assert_eq!(p.level, 100);
        assert_eq!(p.coarse_tune, 76);
        assert_eq!(p.fine_tune, 78);
        assert_eq!(p.pan, 64);
        assert_eq!(p.output_level, 120);
        assert_eq!(p.wave_number_l, 0x1234);
        assert_eq!(p.wave_number_r, 0x5678);
        assert_eq!(p.tvf_env_time, [10, 20, 30, 40]);
        assert_eq!(p.tvf_env_level, [50, 60, 70, 80, 90]);
        assert_eq!(p.tva_env_time, [11, 22, 33, 44]);
        assert_eq!(p.tva_env_level, [100, 110, 120]);
        assert_eq!(p.lfo1_rate, 149);
    }

    #[test]
    fn parse_partial_lfo2_and_step() {
        let mut data = [0u8; PCMS_PARTIAL_SIZE];

        // LFO2 waveform at 0x7B
        data[0x7B] = 5;
        // LFO2 rate (nibblized): 0x08 0x03 = 0x83 = 131
        data[0x7C] = 0x08;
        data[0x7D] = 0x03;
        data[0x7E] = 2; // lfo2 offset
        data[0x7F] = 42; // lfo2 rate detune

        // Past the 0x7F boundary — linear index 0x80+
        data[0x80] = 99; // lfo2 delay time
        data[0x81] = 60; // lfo2 delay time keyfollow
        data[0x82] = 3; // lfo2 fade mode
        data[0x83] = 77; // lfo2 fade time
        data[0x84] = 1; // lfo2 key trigger
        data[0x85] = 30; // lfo2 pitch depth
        data[0x86] = 40; // lfo2 tvf depth
        data[0x87] = 50; // lfo2 tva depth
        data[0x88] = 60; // lfo2 pan depth

        // LFO Step Sequencer
        data[0x89] = 1; // lfo step type
        for i in 0..16u8 {
            data[0x8A + i as usize] = 28 + i * 4; // distinct values
        }

        let p = parse_pcms_partial(&data).unwrap();
        assert_eq!(p.lfo2_waveform, 5);
        assert_eq!(p.lfo2_rate, 131);
        assert_eq!(p.lfo2_offset, 2);
        assert_eq!(p.lfo2_rate_detune, 42);
        assert_eq!(p.lfo2_delay_time, 99);
        assert_eq!(p.lfo2_delay_time_keyfollow, 60);
        assert_eq!(p.lfo2_fade_mode, 3);
        assert_eq!(p.lfo2_fade_time, 77);
        assert_eq!(p.lfo2_key_trigger, 1);
        assert_eq!(p.lfo2_pitch_depth, 30);
        assert_eq!(p.lfo2_tvf_depth, 40);
        assert_eq!(p.lfo2_tva_depth, 50);
        assert_eq!(p.lfo2_pan_depth, 60);
        assert_eq!(p.lfo_step_type, 1);
        assert_eq!(p.lfo_step_values[0], 28);
        assert_eq!(p.lfo_step_values[15], 28 + 15 * 4);
    }

    #[test]
    fn parse_common2_basic() {
        let mut data = [0u8; PCMS_COMMON2_SIZE];
        data[0x10] = 42; // tone category
        // Tone number = 0xAB via nibblized 2 bytes: 0x0A 0x0B
        data[0x11] = 0x0A;
        data[0x12] = 0x0B;
        data[0x13] = 63; // phrase octave shift = -1
        data[0x33] = 1; // TFX switch ON
        // Phrase number = 0x1234 via nibblized 4 bytes
        data[0x38] = 0x01;
        data[0x39] = 0x02;
        data[0x3A] = 0x03;
        data[0x3B] = 0x04;

        let c2 = parse_pcms_common2(&data).unwrap();
        assert_eq!(c2.tone_category, 42);
        assert_eq!(c2.tone_number, 0xAB);
        assert_eq!(c2.phrase_octave_shift, 63);
        assert_eq!(c2.tfx_switch, 1);
        assert_eq!(c2.phrase_number, 0x1234);
    }

    // -- Field tests --

    use crate::block_field::test_support::{
        EntryField, assert_entries_read_at_positions, assert_reads_at_positions, positional_block,
    };

    #[test]
    fn field_positions_follow_the_midi_implementation() {
        // Cutoff Offset `00 22`, Matrix Control 4 Sens 4 `00 4E`.
        assert_eq!(PcmsCommonField::CutoffOffset.element_position(0), Ok(0x22));
        assert_eq!(
            PcmsCommonField::MatrixControlSens4.element_position(3),
            Ok(0x4E)
        );
        // PMT4 Velocity Fade Width Upper `00 28`.
        assert_eq!(
            PcmsPmtField::VelocityFadeUpper.element_position(3),
            Ok(0x28)
        );
        // Chorus Send `00 0F`, LFO Step 16 `01 19`.
        assert_eq!(PcmsPartialField::ChorusSend.element_position(0), Ok(0x0F));
        assert_eq!(PcmsPartialField::LfoStep.element_position(15), Ok(0x99));
        assert_eq!(
            pcms_partial_param_address(0, 0, 0x99),
            Address::new(0x19, 0x00, 0x21, 0x19)
        );
        assert!(PcmsPartialField::LfoStep.element_position(16).is_err());
        assert!(PcmsPmtField::Booster12.element_position(1).is_err());
    }

    #[test]
    fn parse_common_reads_every_field_at_its_position() {
        use PcmsCommonField as F;
        let c = parse_pcms_common(&positional_block(PCMS_COMMON_SIZE)).unwrap();
        assert_reads_at_positions!(c, {
            tone_level: F::ToneLevel,
            tone_pan: F::TonePan,
            tone_priority: F::TonePriority,
            coarse_tune: F::CoarseTune,
            fine_tune: F::FineTune,
            octave_shift: F::OctaveShift,
            stretch_tune_depth: F::StretchTuneDepth,
            analog_feel: F::AnalogFeel,
            mono_poly: F::MonoPoly,
            legato_switch: F::LegatoSwitch,
            legato_retrigger: F::LegatoRetrigger,
            portamento_switch: F::PortamentoSwitch,
            portamento_mode: F::PortamentoMode,
            portamento_type: F::PortamentoType,
            portamento_start: F::PortamentoStart,
            portamento_time: F::PortamentoTime,
            cutoff_offset: F::CutoffOffset,
            resonance_offset: F::ResonanceOffset,
            attack_time_offset: F::AttackTimeOffset,
            release_time_offset: F::ReleaseTimeOffset,
            velocity_sens_offset: F::VelocitySensOffset,
            pmt_control_switch: F::PmtControlSwitch,
            pitch_bend_range_up: F::PitchBendRangeUp,
            pitch_bend_range_down: F::PitchBendRangeDown,
        });
        let per_control: [EntryField<F, MatrixControl>; 9] = [
            (F::MatrixControlSource, |m| m.source),
            (F::MatrixControlDestination1, |m| m.destinations[0]),
            (F::MatrixControlSens1, |m| m.sensitivities[0]),
            (F::MatrixControlDestination2, |m| m.destinations[1]),
            (F::MatrixControlSens2, |m| m.sensitivities[1]),
            (F::MatrixControlDestination3, |m| m.destinations[2]),
            (F::MatrixControlSens3, |m| m.sensitivities[2]),
            (F::MatrixControlDestination4, |m| m.destinations[3]),
            (F::MatrixControlSens4, |m| m.sensitivities[3]),
        ];
        assert_entries_read_at_positions(&c.matrix_controls, &per_control);
    }

    #[test]
    fn parse_pmt_reads_every_field_at_its_position() {
        use PcmsPmtField as F;
        let pmt = parse_pcms_pmt(&positional_block(PCMS_PMT_SIZE)).unwrap();
        assert_reads_at_positions!(pmt, {
            structure_type_12: F::StructureType12,
            booster_12: F::Booster12,
            structure_type_34: F::StructureType34,
            booster_34: F::Booster34,
            pmt_velocity_control: F::PmtVelocityControl,
        });
        let per_partial: [EntryField<F, PmtPartialEntry>; 9] = [
            (F::PartialSwitch, |e| e.partial_switch),
            (F::KeyRangeLower, |e| e.key_range_lower),
            (F::KeyRangeUpper, |e| e.key_range_upper),
            (F::KeyFadeLower, |e| e.key_fade_lower),
            (F::KeyFadeUpper, |e| e.key_fade_upper),
            (F::VelocityRangeLower, |e| e.velocity_range_lower),
            (F::VelocityRangeUpper, |e| e.velocity_range_upper),
            (F::VelocityFadeLower, |e| e.velocity_fade_lower),
            (F::VelocityFadeUpper, |e| e.velocity_fade_upper),
        ];
        assert_entries_read_at_positions(&pmt.partial_entries, &per_partial);
    }

    #[test]
    fn parse_partial_reads_every_field_at_its_position() {
        use PcmsPartialField as F;
        let p = parse_pcms_partial(&positional_block(PCMS_PARTIAL_SIZE)).unwrap();
        assert_reads_at_positions!(p, {
            level: F::Level,
            coarse_tune: F::CoarseTune,
            fine_tune: F::FineTune,
            random_pitch_depth: F::RandomPitchDepth,
            pan: F::Pan,
            pan_keyfollow: F::PanKeyfollow,
            random_pan_depth: F::RandomPanDepth,
            alternate_pan_depth: F::AlternatePanDepth,
            env_mode: F::EnvMode,
            delay_mode: F::DelayMode,
            delay_time: F::DelayTime,
            output_level: F::OutputLevel,
            chorus_send: F::ChorusSend,
            reverb_send: F::ReverbSend,
            receive_bender: F::ReceiveBender,
            receive_expression: F::ReceiveExpression,
            receive_hold1: F::ReceiveHold1,
            redamper_switch: F::RedamperSwitch,
            control1_switches: F::Control1Switch,
            control2_switches: F::Control2Switch,
            control3_switches: F::Control3Switch,
            control4_switches: F::Control4Switch,
            wave_group_type: F::WaveGroupType,
            wave_group_id: F::WaveGroupId,
            wave_number_l: F::WaveNumberL,
            wave_number_r: F::WaveNumberR,
            wave_gain: F::WaveGain,
            wave_fxm_switch: F::WaveFxmSwitch,
            wave_fxm_color: F::WaveFxmColor,
            wave_fxm_depth: F::WaveFxmDepth,
            wave_tempo_sync: F::WaveTempoSync,
            wave_pitch_keyfollow: F::WavePitchKeyfollow,
            pitch_env_depth: F::PitchEnvDepth,
            pitch_env_velocity_sens: F::PitchEnvVelocitySens,
            pitch_env_t1_velocity_sens: F::PitchEnvT1VelocitySens,
            pitch_env_t4_velocity_sens: F::PitchEnvT4VelocitySens,
            pitch_env_time_keyfollow: F::PitchEnvTimeKeyfollow,
            pitch_env_time: F::PitchEnvTime,
            pitch_env_level: F::PitchEnvLevel,
            tvf_filter_type: F::TvfFilterType,
            tvf_cutoff_frequency: F::TvfCutoffFrequency,
            tvf_cutoff_keyfollow: F::TvfCutoffKeyfollow,
            tvf_cutoff_velocity_curve: F::TvfCutoffVelocityCurve,
            tvf_cutoff_velocity_sens: F::TvfCutoffVelocitySens,
            tvf_resonance: F::TvfResonance,
            tvf_resonance_velocity_sens: F::TvfResonanceVelocitySens,
            tvf_env_depth: F::TvfEnvDepth,
            tvf_env_velocity_curve: F::TvfEnvVelocityCurve,
            tvf_env_velocity_sens: F::TvfEnvVelocitySens,
            tvf_env_t1_velocity_sens: F::TvfEnvT1VelocitySens,
            tvf_env_t4_velocity_sens: F::TvfEnvT4VelocitySens,
            tvf_env_time_keyfollow: F::TvfEnvTimeKeyfollow,
            tvf_env_time: F::TvfEnvTime,
            tvf_env_level: F::TvfEnvLevel,
            tva_bias_level: F::TvaBiasLevel,
            tva_bias_position: F::TvaBiasPosition,
            tva_bias_direction: F::TvaBiasDirection,
            tva_level_velocity_curve: F::TvaLevelVelocityCurve,
            tva_level_velocity_sens: F::TvaLevelVelocitySens,
            tva_env_t1_velocity_sens: F::TvaEnvT1VelocitySens,
            tva_env_t4_velocity_sens: F::TvaEnvT4VelocitySens,
            tva_env_time_keyfollow: F::TvaEnvTimeKeyfollow,
            tva_env_time: F::TvaEnvTime,
            tva_env_level: F::TvaEnvLevel,
            lfo1_waveform: F::Lfo1Waveform,
            lfo1_rate: F::Lfo1Rate,
            lfo1_offset: F::Lfo1Offset,
            lfo1_rate_detune: F::Lfo1RateDetune,
            lfo1_delay_time: F::Lfo1DelayTime,
            lfo1_delay_time_keyfollow: F::Lfo1DelayTimeKeyfollow,
            lfo1_fade_mode: F::Lfo1FadeMode,
            lfo1_fade_time: F::Lfo1FadeTime,
            lfo1_key_trigger: F::Lfo1KeyTrigger,
            lfo1_pitch_depth: F::Lfo1PitchDepth,
            lfo1_tvf_depth: F::Lfo1TvfDepth,
            lfo1_tva_depth: F::Lfo1TvaDepth,
            lfo1_pan_depth: F::Lfo1PanDepth,
            lfo2_waveform: F::Lfo2Waveform,
            lfo2_rate: F::Lfo2Rate,
            lfo2_offset: F::Lfo2Offset,
            lfo2_rate_detune: F::Lfo2RateDetune,
            lfo2_delay_time: F::Lfo2DelayTime,
            lfo2_delay_time_keyfollow: F::Lfo2DelayTimeKeyfollow,
            lfo2_fade_mode: F::Lfo2FadeMode,
            lfo2_fade_time: F::Lfo2FadeTime,
            lfo2_key_trigger: F::Lfo2KeyTrigger,
            lfo2_pitch_depth: F::Lfo2PitchDepth,
            lfo2_tvf_depth: F::Lfo2TvfDepth,
            lfo2_tva_depth: F::Lfo2TvaDepth,
            lfo2_pan_depth: F::Lfo2PanDepth,
            lfo_step_type: F::LfoStepType,
            lfo_step_values: F::LfoStep,
        });
    }

    #[test]
    fn parse_common2_reads_every_field_at_its_position() {
        use PcmsCommon2Field as F;
        let c2 = parse_pcms_common2(&positional_block(PCMS_COMMON2_SIZE)).unwrap();
        assert_reads_at_positions!(c2, {
            tone_category: F::ToneCategory,
            tone_number: F::ToneNumber,
            phrase_octave_shift: F::PhraseOctaveShift,
            tfx_switch: F::TfxSwitch,
            phrase_number: F::PhraseNumber,
        });
    }
}
