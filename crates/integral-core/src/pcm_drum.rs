//! PCM Drum Kit address map, state types, and parse functions.
//!
//! The PCM Drum Kit tone occupies offset `10 00 00` within the temporary tone
//! block. It contains: Common parameters, MFX (handled by `mfx.rs`), Comp+EQ
//! (handled by `params.rs`), 88 per-key Partial blocks, and Common2.

use crate::address::{Address, DataSize};
use crate::block_field::{BlockField, FieldIndexError};
use crate::device_spec::{KeyOutOfRange, KeyRange};
use crate::mfx::MfxState;
use crate::params;
use crate::parse_helpers::{
    encode_ascii_name, encode_nibble2, encode_nibble4, nibble2, nibble4, parse_ascii_name,
};
use core::fmt::Debug;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// PCM Drum Kit Common block size (18 bytes).
pub const PCMD_COMMON_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x12);

/// PCM Drum Kit Common parse size in bytes.
pub const PCMD_COMMON_SIZE: usize = 18;

/// PCM Drum Kit per-key Partial block size (195 bytes; 7-bit size `01 43`).
pub const PCMD_PARTIAL_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x01, 0x43);

/// PCM Drum Kit per-key Partial parse size in bytes (195 = 128 + 67).
pub const PCMD_PARTIAL_SIZE: usize = 195;

/// PCM Drum Kit Common2 block size (50 bytes).
pub const PCMD_COMMON2_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x32);

/// PCM Drum Kit Common2 parse size in bytes.
pub const PCMD_COMMON2_SIZE: usize = 50;

/// First key number in the PCM Drum Kit partial range.
pub const PCMD_FIRST_KEY: u8 = 21;

/// Last key number in the PCM Drum Kit partial range.
pub const PCMD_LAST_KEY: u8 = 108;

/// Keys a PCM Drum Kit has a Partial block for.
pub const PCMD_KEYS: KeyRange = KeyRange {
    first: PCMD_FIRST_KEY,
    last: PCMD_LAST_KEY,
};

/// Number of PCM Drum Kit key slots.
pub const PCMD_KEY_COUNT: usize = (PCMD_LAST_KEY - PCMD_FIRST_KEY + 1) as usize; // 88

/// Number of bytes per WMT layer in a PCM Drum Kit partial.
const WMT_LAYER_SIZE: usize = 29;

/// Byte index of the first WMT layer within a PCM Drum Kit partial; the
/// others follow [`WMT_LAYER_SIZE`] apart (WMT1 `00 21`, WMT2 `00 3E`,
/// WMT3 `00 5B`, WMT4 `00 78`).
const WMT1_START: usize = 0x21;

/// Number of WMT (Wave Mix Table) layers in a PCM Drum Kit partial.
pub const PCMD_WMT_COUNT: usize = 4;

/// A field of a PCM Drum Kit WMT layer. The discriminant is the field's byte
/// position within the 29-byte layer; as a [`BlockField`] of the Partial
/// block it holds one value per WMT layer ([`PCMD_WMT_COUNT`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmdWmtField {
    /// Wave Switch.
    Switch = 0,
    /// Wave Group Type.
    WaveGroupType = 1,
    /// Wave Group ID (nibblized, 4 bytes).
    WaveGroupId = 2,
    /// Wave Number L / Mono (nibblized, 4 bytes).
    WaveNumberL = 6,
    /// Wave Number R (nibblized, 4 bytes).
    WaveNumberR = 10,
    /// Wave Gain.
    WaveGain = 14,
    /// Wave FXM Switch.
    WaveFxmSwitch = 15,
    /// Wave FXM Color.
    WaveFxmColor = 16,
    /// Wave FXM Depth.
    WaveFxmDepth = 17,
    /// Wave Tempo Sync.
    WaveTempoSync = 18,
    /// Wave Coarse Tune.
    CoarseTune = 19,
    /// Wave Fine Tune.
    FineTune = 20,
    /// Wave Pan.
    Pan = 21,
    /// Wave Random Pan Switch.
    RandomPanSwitch = 22,
    /// Wave Alternate Pan Switch.
    AlternatePanSwitch = 23,
    /// Wave Level.
    Level = 24,
    /// Velocity Range Lower.
    VelocityRangeLower = 25,
    /// Velocity Range Upper.
    VelocityRangeUpper = 26,
    /// Velocity Fade Width Lower.
    VelocityFadeLower = 27,
    /// Velocity Fade Width Upper.
    VelocityFadeUpper = 28,
}

impl BlockField for PcmdWmtField {
    fn position(self) -> usize {
        WMT1_START + self as usize
    }

    fn count(self) -> usize {
        PCMD_WMT_COUNT
    }

    fn width(self) -> usize {
        match self {
            Self::WaveGroupId | Self::WaveNumberL | Self::WaveNumberR => 4,
            _ => 1,
        }
    }

    fn stride(self) -> usize {
        WMT_LAYER_SIZE
    }
}

/// A per-key field of a PCM Drum Kit Partial outside the WMT layers. The
/// discriminant is the field's first byte position within the partial;
/// envelope time and level fields span [`BlockField::count`] bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmdPartialField {
    /// Assign Type.
    AssignType = 0x0C,
    /// Mute Group.
    MuteGroup = 0x0D,
    /// Partial Level.
    Level = 0x0E,
    /// Partial Coarse Tune.
    CoarseTune = 0x0F,
    /// Partial Fine Tune.
    FineTune = 0x10,
    /// Partial Random Pitch Depth.
    RandomPitchDepth = 0x11,
    /// Partial Pan.
    Pan = 0x12,
    /// Partial Random Pan Depth.
    RandomPanDepth = 0x13,
    /// Partial Alternate Pan Depth.
    AlternatePanDepth = 0x14,
    /// Partial Env Mode.
    EnvMode = 0x15,
    /// Partial Output Level.
    OutputLevel = 0x16,
    /// Partial Chorus Send Level.
    ChorusSend = 0x19,
    /// Partial Reverb Send Level.
    ReverbSend = 0x1A,
    /// Partial Output Assign.
    OutputAssign = 0x1B,
    /// Partial Pitch Bend Range.
    PitchBendRange = 0x1C,
    /// Partial Receive Expression.
    ReceiveExpression = 0x1D,
    /// Partial Receive Hold-1.
    ReceiveHold1 = 0x1E,
    /// WMT Velocity Control.
    WmtVelocityControl = 0x20,
    /// Pitch Env Depth (`01 15`).
    PitchEnvDepth = 0x80 + 0x15,
    /// Pitch Env Velocity Sens.
    PitchEnvVelocitySens = 0x80 + 0x16,
    /// Pitch Env Time 1 Velocity Sens.
    PitchEnvT1VelocitySens = 0x80 + 0x17,
    /// Pitch Env Time 4 Velocity Sens.
    PitchEnvT4VelocitySens = 0x80 + 0x18,
    /// Pitch Env Time 1–4.
    PitchEnvTime = 0x80 + 0x19,
    /// Pitch Env Level 0–4.
    PitchEnvLevel = 0x80 + 0x1D,
    /// TVF Filter Type (`01 22`).
    TvfFilterType = 0x80 + 0x22,
    /// TVF Cutoff Frequency.
    TvfCutoffFrequency = 0x80 + 0x23,
    /// TVF Cutoff Velocity Curve.
    TvfCutoffVelocityCurve = 0x80 + 0x24,
    /// TVF Cutoff Velocity Sens.
    TvfCutoffVelocitySens = 0x80 + 0x25,
    /// TVF Resonance.
    TvfResonance = 0x80 + 0x26,
    /// TVF Resonance Velocity Sens.
    TvfResonanceVelocitySens = 0x80 + 0x27,
    /// TVF Env Depth.
    TvfEnvDepth = 0x80 + 0x28,
    /// TVF Env Velocity Curve Type.
    TvfEnvVelocityCurve = 0x80 + 0x29,
    /// TVF Env Velocity Sens.
    TvfEnvVelocitySens = 0x80 + 0x2A,
    /// TVF Env Time 1 Velocity Sens.
    TvfEnvT1VelocitySens = 0x80 + 0x2B,
    /// TVF Env Time 4 Velocity Sens.
    TvfEnvT4VelocitySens = 0x80 + 0x2C,
    /// TVF Env Time 1–4.
    TvfEnvTime = 0x80 + 0x2D,
    /// TVF Env Level 0–4.
    TvfEnvLevel = 0x80 + 0x31,
    /// TVA Level Velocity Curve (`01 36`).
    TvaLevelVelocityCurve = 0x80 + 0x36,
    /// TVA Level Velocity Sens.
    TvaLevelVelocitySens = 0x80 + 0x37,
    /// TVA Env Time 1 Velocity Sens.
    TvaEnvT1VelocitySens = 0x80 + 0x38,
    /// TVA Env Time 4 Velocity Sens.
    TvaEnvT4VelocitySens = 0x80 + 0x39,
    /// TVA Env Time 1–4.
    TvaEnvTime = 0x80 + 0x3A,
    /// TVA Env Level 1–3.
    TvaEnvLevel = 0x80 + 0x3E,
    /// One Shot Mode (`01 41`).
    OneShotMode = 0x80 + 0x41,
}

impl BlockField for PcmdPartialField {
    fn position(self) -> usize {
        self as usize
    }

    fn count(self) -> usize {
        match self {
            Self::PitchEnvTime | Self::TvfEnvTime | Self::TvaEnvTime => 4,
            Self::PitchEnvLevel | Self::TvfEnvLevel => 5,
            Self::TvaEnvLevel => 3,
            _ => 1,
        }
    }
}

/// Error returned for a PCM Drum Kit Partial field address that doesn't exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PcmdFieldError<F: Debug> {
    /// The key is outside the kit's range.
    #[error(transparent)]
    Key(#[from] KeyOutOfRange),
    /// The value index (WMT layer, envelope step) is not below the field's
    /// [`BlockField::count`].
    #[error(transparent)]
    Index(#[from] FieldIndexError<F>),
}

// ---------------------------------------------------------------------------
// Address functions
// ---------------------------------------------------------------------------

/// Compute the absolute PCM Drum Kit base address for a part.
pub const fn pcmd_base(part_index: u8) -> Address {
    params::ToneType::PcmDrum.base_address(part_index)
}

/// Compute the absolute PCM Drum Kit Common block address for a part.
pub const fn pcmd_common_address(part_index: u8) -> Address {
    pcmd_base(part_index)
}

/// A field of the PCM Drum Kit Common block. The discriminant is the
/// field's byte position within the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PcmdCommonField {
    /// Kit Level.
    KitLevel = 0x0C,
}

impl BlockField for PcmdCommonField {
    fn position(self) -> usize {
        self as usize
    }
}

/// Compute the absolute address for a single PCM Drum Kit Common parameter.
pub const fn pcmd_common_param_address(part_index: u8, offset: u8) -> Address {
    pcmd_common_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

/// Compute the absolute PCM Drum Kit Partial block address for a part and
/// key number.
///
/// Key 21 maps to offset `00 10 00`, key 22 to `00 12 00`, etc.
/// Each key is spaced `00 02 00` apart. Keys outside [`PCMD_KEYS`] are
/// rejected.
pub fn pcmd_partial_address(part_index: u8, key: u8) -> Result<Address, KeyOutOfRange> {
    let key_offset = (PCMD_KEYS.check(key)? - PCMD_FIRST_KEY) as u16;
    // Each key occupies 0x02 00 in the address space (i.e., byte2 increments by 2)
    // Starting at byte1=0x00, byte2=0x10
    let byte2_total = 0x10 + key_offset * 2;
    // Handle 7-bit carry: byte2 wraps at 128
    let byte1_add = (byte2_total / 128) as u8;
    let byte2 = (byte2_total % 128) as u8;
    Ok(pcmd_base(part_index).offset([0x00, byte1_add, byte2, 0x00]))
}

/// Compute the absolute address of the byte at `position` (0–194) in the
/// Partial for a part and key; positions from `0x80` are at offset `01 00`
/// and up.
pub fn pcmd_partial_param_address(
    part_index: u8,
    key: u8,
    position: u8,
) -> Result<Address, KeyOutOfRange> {
    Ok(pcmd_partial_address(part_index, key)?.offset([0x00, 0x00, 0x00, position]))
}

/// Byte position of `field` in WMT `wmt` (0-based) within a Partial block.
///
/// # Panics
///
/// If `wmt` is not below [`PCMD_WMT_COUNT`].
pub const fn pcmd_wmt_byte(wmt: usize, field: PcmdWmtField) -> usize {
    assert!(wmt < PCMD_WMT_COUNT, "WMT index out of range");
    WMT1_START + wmt * WMT_LAYER_SIZE + field as usize
}

/// Compute the absolute PCM Drum Kit Common2 block address for a part.
pub const fn pcmd_common2_address(part_index: u8) -> Address {
    pcmd_base(part_index).offset([0x00, 0x02, 0x00, 0x00])
}

/// PCM Drum Kit Common2 offset of Phrase Number (2 nibbles).
pub const COMMON2_PHRASE_NUMBER: u8 = 0x10;

/// PCM Drum Kit Common2 offset of TFX Switch (0–1).
pub const COMMON2_TFX_SWITCH: u8 = 0x31;

/// Compute the absolute address for a single PCM Drum Kit Common2 parameter.
pub const fn pcmd_common2_param_address(part_index: u8, offset: u8) -> Address {
    pcmd_common2_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// PCM Drum Kit Common parameters (18 bytes at offset `00 00 00`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmDrumCommon {
    /// Kit name (up to 12 ASCII characters).
    pub kit_name: String,
    /// Kit Level (0-127).
    pub kit_level: u8,
}

impl Default for PcmDrumCommon {
    fn default() -> Self {
        Self {
            kit_name: String::new(),
            kit_level: 127,
        }
    }
}

/// PCM Drum Kit WMT (Wave Mix Table) layer parameters (29 bytes each).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmDrumWmt {
    /// Wave Switch (0=OFF, 1=ON).
    #[cfg_attr(feature = "serde", serde(rename = "wmtSwitch"))]
    pub switch: u8,
    /// Wave Group Type (0-3: INT, SRX).
    pub wave_group_type: u8,
    /// Wave Group ID (0-16384, nibblized 4 bytes).
    pub wave_group_id: u16,
    /// Wave Number L / Mono (0-16384, nibblized 4 bytes).
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
    /// Wave Coarse Tune (16-112, display -48 to +48).
    pub coarse_tune: u8,
    /// Wave Fine Tune (14-114, display -50 to +50).
    pub fine_tune: u8,
    /// Wave Pan (0-127, display L64-63R).
    pub pan: u8,
    /// Wave Random Pan Switch (0=OFF, 1=ON).
    pub random_pan_switch: u8,
    /// Wave Alternate Pan Switch (0-2: OFF, ON, REVERSE).
    pub alternate_pan_switch: u8,
    /// Wave Level (0-127).
    pub level: u8,
    /// Velocity Range Lower (1-127).
    pub velocity_range_lower: u8,
    /// Velocity Range Upper (1-127).
    pub velocity_range_upper: u8,
    /// Velocity Fade Width Lower (0-127).
    pub velocity_fade_lower: u8,
    /// Velocity Fade Width Upper (0-127).
    pub velocity_fade_upper: u8,
}

impl Default for PcmDrumWmt {
    fn default() -> Self {
        Self {
            switch: 0,
            wave_group_type: 0,
            wave_group_id: 1, // INT, as `WaveGroup::Int.params()` writes
            wave_number_l: 0,
            wave_number_r: 0,
            wave_gain: 1, // 0 dB
            wave_fxm_switch: 0,
            wave_fxm_color: 0,
            wave_fxm_depth: 0,
            wave_tempo_sync: 0,
            coarse_tune: 64,
            fine_tune: 64,
            pan: 64,
            random_pan_switch: 0,
            alternate_pan_switch: 0,
            level: 127,
            velocity_range_lower: 1,
            velocity_range_upper: 127,
            velocity_fade_lower: 0,
            velocity_fade_upper: 0,
        }
    }
}

/// PCM Drum Kit per-key Partial parameters (195 bytes per key).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmDrumPartial {
    // -- Common per-key (offsets 00 00 - 00 1E) --
    /// Partial name (up to 12 ASCII characters).
    pub partial_name: String,
    /// Assign Type (0=MULTI, 1=SINGLE).
    pub assign_type: u8,
    /// Mute Group (0-31: OFF, 1-31).
    pub mute_group: u8,
    /// Partial Level (0-127).
    pub level: u8,
    /// Partial Coarse Tune (0-127, display C-1 to G9).
    pub coarse_tune: u8,
    /// Partial Fine Tune (14-114, display -50 to +50).
    pub fine_tune: u8,
    /// Random Pitch Depth (0-30).
    pub random_pitch_depth: u8,
    /// Partial Pan (0-127, display L64-63R).
    pub pan: u8,
    /// Random Pan Depth (0-63).
    pub random_pan_depth: u8,
    /// Alternate Pan Depth (1-127, display L63-63R).
    pub alternate_pan_depth: u8,
    /// Env Mode (0=NO-SUS, 1=SUSTAIN).
    pub env_mode: u8,
    /// Output Level (0-127).
    pub output_level: u8,
    /// Chorus Send Level (0-127).
    pub chorus_send: u8,
    /// Reverb Send Level (0-127).
    pub reverb_send: u8,
    /// Output Assign (0-6: PART, COMP+EQ1-6).
    pub output_assign: u8,
    /// Pitch Bend Range (0-48).
    pub pitch_bend_range: u8,
    /// Receive Expression (0=OFF, 1=ON).
    pub receive_expression: u8,
    /// Receive Hold-1 (0=OFF, 1=ON).
    pub receive_hold1: u8,

    // -- WMT (offsets 00 20 - 01 14) --
    /// WMT Velocity Control (0-2: OFF, ON, RANDOM).
    #[cfg_attr(feature = "serde", serde(rename = "wmtVelocityControl"))]
    pub wmt_velocity_control: u8,
    /// WMT layers (4 layers).
    #[cfg_attr(feature = "serde", serde(rename = "wmt"))]
    pub wmt: Vec<PcmDrumWmt>,

    // -- Pitch Envelope --
    /// Pitch Env Depth (52-76, display -12 to +12).
    pub pitch_env_depth: u8,
    /// Pitch Env Velocity Sens (1-127, display -63 to +63).
    pub pitch_env_velocity_sens: u8,
    /// Pitch Env Time 1 Velocity Sens (1-127, display -63 to +63).
    pub pitch_env_t1_velocity_sens: u8,
    /// Pitch Env Time 4 Velocity Sens (1-127, display -63 to +63).
    pub pitch_env_t4_velocity_sens: u8,
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

    // -- TVF Envelope --
    /// TVF Env Depth (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvDepth"))]
    pub tvf_env_depth: u8,
    /// TVF Env Velocity Curve (0-7: FIXED, 1-7).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvVelocityCurve"))]
    pub tvf_env_velocity_curve: u8,
    /// TVF Env Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvVelocitySens"))]
    pub tvf_env_velocity_sens: u8,
    /// TVF Env Time 1 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvT1VelocitySens"))]
    pub tvf_env_t1_velocity_sens: u8,
    /// TVF Env Time 4 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvT4VelocitySens"))]
    pub tvf_env_t4_velocity_sens: u8,
    /// TVF Env Times (4 values, each 0-127: T1-T4).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvTime"))]
    pub tvf_env_time: [u8; 4],
    /// TVF Env Levels (5 values, each 0-127: L0-L4).
    #[cfg_attr(feature = "serde", serde(rename = "tvfEnvLevel"))]
    pub tvf_env_level: [u8; 5],

    // -- TVA (Amplifier) --
    /// TVA Level Velocity Curve (0-7: FIXED, 1-7).
    #[cfg_attr(feature = "serde", serde(rename = "tvaLevelVelocityCurve"))]
    pub tva_level_velocity_curve: u8,
    /// TVA Level Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvaLevelVelocitySens"))]
    pub tva_level_velocity_sens: u8,
    /// TVA Env Time 1 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvT1VelocitySens"))]
    pub tva_env_t1_velocity_sens: u8,
    /// TVA Env Time 4 Velocity Sens (1-127, display -63 to +63).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvT4VelocitySens"))]
    pub tva_env_t4_velocity_sens: u8,
    /// TVA Env Times (4 values, each 0-127: T1-T4).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvTime"))]
    pub tva_env_time: [u8; 4],
    /// TVA Env Levels (3 values, each 0-127: L1-L3; L0 is always max).
    #[cfg_attr(feature = "serde", serde(rename = "tvaEnvLevel"))]
    pub tva_env_level: [u8; 3],

    // -- Final --
    /// One Shot Mode (0=OFF, 1=ON).
    pub one_shot_mode: u8,
}

impl Default for PcmDrumPartial {
    fn default() -> Self {
        Self {
            partial_name: String::new(),
            assign_type: 0,
            mute_group: 0,
            level: 127,
            coarse_tune: 64,
            fine_tune: 64,
            random_pitch_depth: 0,
            pan: 64,
            random_pan_depth: 0,
            alternate_pan_depth: 64,
            env_mode: 0,
            output_level: 127,
            chorus_send: 0,
            reverb_send: 0,
            output_assign: 0,
            pitch_bend_range: 0,
            receive_expression: 1,
            receive_hold1: 1,

            wmt_velocity_control: 0,
            wmt: vec![PcmDrumWmt::default(); PCMD_WMT_COUNT],

            pitch_env_depth: 64,
            pitch_env_velocity_sens: 64,
            pitch_env_t1_velocity_sens: 64,
            pitch_env_t4_velocity_sens: 64,
            pitch_env_time: [0; 4],
            pitch_env_level: [64; 5],

            tvf_filter_type: 1,
            tvf_cutoff_frequency: 127,
            tvf_cutoff_velocity_curve: 0,
            tvf_cutoff_velocity_sens: 64,
            tvf_resonance: 0,
            tvf_resonance_velocity_sens: 64,

            tvf_env_depth: 64,
            tvf_env_velocity_curve: 0,
            tvf_env_velocity_sens: 64,
            tvf_env_t1_velocity_sens: 64,
            tvf_env_t4_velocity_sens: 64,
            tvf_env_time: [0; 4],
            tvf_env_level: [0; 5],

            tva_level_velocity_curve: 0,
            tva_level_velocity_sens: 64,
            tva_env_t1_velocity_sens: 64,
            tva_env_t4_velocity_sens: 64,
            tva_env_time: [0; 4],
            tva_env_level: [0; 3],

            one_shot_mode: 0,
        }
    }
}

/// PCM Drum Kit Common2 parameters (50 bytes at offset `02 00 00`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmDrumCommon2 {
    /// Phrase Number (0-255, nibblized 2 bytes at offset 0x10).
    pub phrase_number: u16,
    /// TFX Switch (0=OFF, 1=ON, at offset 0x31).
    #[cfg_attr(feature = "serde", serde(rename = "tfxSwitch"))]
    pub tfx_switch: u8,
}

impl Default for PcmDrumCommon2 {
    #[allow(clippy::derivable_impls)]
    fn default() -> Self {
        Self {
            phrase_number: 0,
            tfx_switch: 0,
        }
    }
}

/// Full PCM Drum Kit state for one part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PcmDrumKitState {
    /// Common parameters.
    pub common: PcmDrumCommon,
    /// Common2 parameters.
    pub common2: PcmDrumCommon2,
    /// MFX parameters.
    pub mfx: MfxState,
}

// ---------------------------------------------------------------------------
// Parse helpers
// ---------------------------------------------------------------------------

/// Parse a single WMT layer from 29 contiguous bytes.
fn parse_pcmd_wmt(data: &[u8]) -> PcmDrumWmt {
    let mut w = PcmDrumWmt::default();
    if data.len() < WMT_LAYER_SIZE {
        return w;
    }
    use PcmdWmtField as F;
    let byte = |f: F| data[f as usize];
    w.switch = byte(F::Switch);
    w.wave_group_type = byte(F::WaveGroupType);
    w.wave_group_id = nibble4(data, F::WaveGroupId as usize);
    w.wave_number_l = nibble4(data, F::WaveNumberL as usize);
    w.wave_number_r = nibble4(data, F::WaveNumberR as usize);
    w.wave_gain = byte(F::WaveGain);
    w.wave_fxm_switch = byte(F::WaveFxmSwitch);
    w.wave_fxm_color = byte(F::WaveFxmColor);
    w.wave_fxm_depth = byte(F::WaveFxmDepth);
    w.wave_tempo_sync = byte(F::WaveTempoSync);
    w.coarse_tune = byte(F::CoarseTune);
    w.fine_tune = byte(F::FineTune);
    w.pan = byte(F::Pan);
    w.random_pan_switch = byte(F::RandomPanSwitch);
    w.alternate_pan_switch = byte(F::AlternatePanSwitch);
    w.level = byte(F::Level);
    w.velocity_range_lower = byte(F::VelocityRangeLower);
    w.velocity_range_upper = byte(F::VelocityRangeUpper);
    w.velocity_fade_lower = byte(F::VelocityFadeLower);
    w.velocity_fade_upper = byte(F::VelocityFadeUpper);
    w
}

// ---------------------------------------------------------------------------
// Parse functions
// ---------------------------------------------------------------------------

/// Parse a PCM Drum Kit Common dump (18 bytes).
///
/// Offsets:
/// - `00 00`-`00 0B`: Kit Name (12 ASCII chars)
/// - `00 0C`: Kit Level
/// - `00 0D`-`00 11`: reserved
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pcmd_common(data: &[u8]) -> Result<PcmDrumCommon, crate::ToneParseError> {
    if data.len() < PCMD_COMMON_SIZE {
        return Err(crate::ToneParseError {
            expected: PCMD_COMMON_SIZE,
            got: data.len(),
        });
    }
    let mut c = PcmDrumCommon::default();
    c.kit_name = parse_ascii_name(data);
    c.kit_level = PcmdCommonField::KitLevel.byte(data);
    Ok(c)
}

/// Parse a PCM Drum Kit per-key Partial dump (195 bytes).
///
/// The data array is a contiguous byte stream from the DT1 response.
/// Linear indices map to SysEx offsets: indices 0-127 = offsets `00 00`-`00 7F`,
/// index 128 = offset `01 00`, etc.
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pcmd_partial(data: &[u8]) -> Result<PcmDrumPartial, crate::ToneParseError> {
    if data.len() < PCMD_PARTIAL_SIZE {
        return Err(crate::ToneParseError {
            expected: PCMD_PARTIAL_SIZE,
            got: data.len(),
        });
    }
    let mut p = PcmDrumPartial::default();

    use PcmdPartialField as F;
    let byte = |f: F| data[f as usize];
    let bytes = |f: F| &data[f as usize..f as usize + f.count()];

    p.partial_name = parse_ascii_name(data);
    p.assign_type = byte(F::AssignType);
    p.mute_group = byte(F::MuteGroup);
    p.level = byte(F::Level);
    p.coarse_tune = byte(F::CoarseTune);
    p.fine_tune = byte(F::FineTune);
    p.random_pitch_depth = byte(F::RandomPitchDepth);
    p.pan = byte(F::Pan);
    p.random_pan_depth = byte(F::RandomPanDepth);
    p.alternate_pan_depth = byte(F::AlternatePanDepth);
    p.env_mode = byte(F::EnvMode);
    p.output_level = byte(F::OutputLevel);
    p.chorus_send = byte(F::ChorusSend);
    p.reverb_send = byte(F::ReverbSend);
    p.output_assign = byte(F::OutputAssign);
    p.pitch_bend_range = byte(F::PitchBendRange);
    p.receive_expression = byte(F::ReceiveExpression);
    p.receive_hold1 = byte(F::ReceiveHold1);

    p.wmt_velocity_control = byte(F::WmtVelocityControl);
    for (i, w) in p.wmt.iter_mut().enumerate() {
        let start = pcmd_wmt_byte(i, PcmdWmtField::Switch);
        *w = parse_pcmd_wmt(&data[start..start + WMT_LAYER_SIZE]);
    }

    p.pitch_env_depth = byte(F::PitchEnvDepth);
    p.pitch_env_velocity_sens = byte(F::PitchEnvVelocitySens);
    p.pitch_env_t1_velocity_sens = byte(F::PitchEnvT1VelocitySens);
    p.pitch_env_t4_velocity_sens = byte(F::PitchEnvT4VelocitySens);
    p.pitch_env_time.copy_from_slice(bytes(F::PitchEnvTime));
    p.pitch_env_level.copy_from_slice(bytes(F::PitchEnvLevel));

    p.tvf_filter_type = byte(F::TvfFilterType);
    p.tvf_cutoff_frequency = byte(F::TvfCutoffFrequency);
    p.tvf_cutoff_velocity_curve = byte(F::TvfCutoffVelocityCurve);
    p.tvf_cutoff_velocity_sens = byte(F::TvfCutoffVelocitySens);
    p.tvf_resonance = byte(F::TvfResonance);
    p.tvf_resonance_velocity_sens = byte(F::TvfResonanceVelocitySens);
    p.tvf_env_depth = byte(F::TvfEnvDepth);
    p.tvf_env_velocity_curve = byte(F::TvfEnvVelocityCurve);
    p.tvf_env_velocity_sens = byte(F::TvfEnvVelocitySens);
    p.tvf_env_t1_velocity_sens = byte(F::TvfEnvT1VelocitySens);
    p.tvf_env_t4_velocity_sens = byte(F::TvfEnvT4VelocitySens);
    p.tvf_env_time.copy_from_slice(bytes(F::TvfEnvTime));
    p.tvf_env_level.copy_from_slice(bytes(F::TvfEnvLevel));

    p.tva_level_velocity_curve = byte(F::TvaLevelVelocityCurve);
    p.tva_level_velocity_sens = byte(F::TvaLevelVelocitySens);
    p.tva_env_t1_velocity_sens = byte(F::TvaEnvT1VelocitySens);
    p.tva_env_t4_velocity_sens = byte(F::TvaEnvT4VelocitySens);
    p.tva_env_time.copy_from_slice(bytes(F::TvaEnvTime));
    p.tva_env_level.copy_from_slice(bytes(F::TvaEnvLevel));

    p.one_shot_mode = byte(F::OneShotMode);

    Ok(p)
}

/// Parse a PCM Drum Kit Common2 dump (50 bytes).
///
/// Offsets:
/// - `00 00`-`00 0F`: reserved
/// - `00 10`-`00 11`: Phrase Number (nibblized 2 bytes)
/// - `00 12`-`00 30`: reserved
/// - `00 31`: TFX Switch
#[allow(clippy::field_reassign_with_default)]
pub fn parse_pcmd_common2(data: &[u8]) -> Result<PcmDrumCommon2, crate::ToneParseError> {
    if data.len() < PCMD_COMMON2_SIZE {
        return Err(crate::ToneParseError {
            expected: PCMD_COMMON2_SIZE,
            got: data.len(),
        });
    }
    let mut c2 = PcmDrumCommon2::default();
    c2.phrase_number = nibble2(data, usize::from(COMMON2_PHRASE_NUMBER));
    c2.tfx_switch = data[usize::from(COMMON2_TFX_SWITCH)];
    Ok(c2)
}

// ---------------------------------------------------------------------------
// Encode functions
// ---------------------------------------------------------------------------

/// Common: the unnamed and reserve bytes after Kit Level, as `INIT RHYTHM`
/// holds them: `00 0D` 0, `00 0E` 120 (2 nibbles), `00 10` 0, `00 11` 13.
/// See `docs/svd/07-pcm-drum.md`.
const INIT_COMMON_TAIL: [u8; 5] = [0, 0x07, 0x08, 0, 13];

/// Partial: reserve `01 42`, which the MIDI Implementation prints as 64.
const PARTIAL_RESERVE_64: usize = 0x80 + 0x42;

/// Encode a PCM Drum Kit Common block (18 bytes), the inverse of
/// [`parse_pcmd_common`]. The bytes after Kit Level get the `INIT RHYTHM`
/// values.
pub fn encode_pcmd_common(c: &PcmDrumCommon) -> Vec<u8> {
    let mut data = encode_ascii_name(&c.kit_name, PcmdCommonField::KitLevel.position());
    data.push(c.kit_level);
    data.extend_from_slice(&INIT_COMMON_TAIL);
    data
}

/// Encode a single WMT layer (29 bytes), the inverse of `parse_pcmd_wmt`.
pub(crate) fn encode_pcmd_wmt(w: &PcmDrumWmt, data: &mut [u8]) {
    use PcmdWmtField as F;
    let mut set = |f: F, v: u8| data[f as usize] = v;
    set(F::Switch, w.switch);
    set(F::WaveGroupType, w.wave_group_type);
    set(F::WaveGain, w.wave_gain);
    set(F::WaveFxmSwitch, w.wave_fxm_switch);
    set(F::WaveFxmColor, w.wave_fxm_color);
    set(F::WaveFxmDepth, w.wave_fxm_depth);
    set(F::WaveTempoSync, w.wave_tempo_sync);
    set(F::CoarseTune, w.coarse_tune);
    set(F::FineTune, w.fine_tune);
    set(F::Pan, w.pan);
    set(F::RandomPanSwitch, w.random_pan_switch);
    set(F::AlternatePanSwitch, w.alternate_pan_switch);
    set(F::Level, w.level);
    set(F::VelocityRangeLower, w.velocity_range_lower);
    set(F::VelocityRangeUpper, w.velocity_range_upper);
    set(F::VelocityFadeLower, w.velocity_fade_lower);
    set(F::VelocityFadeUpper, w.velocity_fade_upper);
    for (f, v) in [
        (F::WaveGroupId, w.wave_group_id),
        (F::WaveNumberL, w.wave_number_l),
        (F::WaveNumberR, w.wave_number_r),
    ] {
        data[f as usize..f as usize + 4].copy_from_slice(&encode_nibble4(v));
    }
}

/// Encode a PCM Drum Kit per-key Partial block (195 bytes), the inverse of
/// [`parse_pcmd_partial`]. Reserve `01 42` gets 64, the value the MIDI
/// Implementation prints. The others (`00 17`, `00 18`, `00 1F`) get 0, a
/// stand-in: no documented INIT kit shows them.
pub fn encode_pcmd_partial(p: &PcmDrumPartial) -> Vec<u8> {
    let mut data = vec![0; PCMD_PARTIAL_SIZE];
    use PcmdPartialField as F;
    data[..F::AssignType as usize]
        .copy_from_slice(&encode_ascii_name(&p.partial_name, F::AssignType as usize));
    let mut set = |f: F, v: &[u8]| data[f as usize..f as usize + v.len()].copy_from_slice(v);
    set(F::AssignType, &[p.assign_type]);
    set(F::MuteGroup, &[p.mute_group]);
    set(F::Level, &[p.level]);
    set(F::CoarseTune, &[p.coarse_tune]);
    set(F::FineTune, &[p.fine_tune]);
    set(F::RandomPitchDepth, &[p.random_pitch_depth]);
    set(F::Pan, &[p.pan]);
    set(F::RandomPanDepth, &[p.random_pan_depth]);
    set(F::AlternatePanDepth, &[p.alternate_pan_depth]);
    set(F::EnvMode, &[p.env_mode]);
    set(F::OutputLevel, &[p.output_level]);
    set(F::ChorusSend, &[p.chorus_send]);
    set(F::ReverbSend, &[p.reverb_send]);
    set(F::OutputAssign, &[p.output_assign]);
    set(F::PitchBendRange, &[p.pitch_bend_range]);
    set(F::ReceiveExpression, &[p.receive_expression]);
    set(F::ReceiveHold1, &[p.receive_hold1]);
    set(F::WmtVelocityControl, &[p.wmt_velocity_control]);

    set(F::PitchEnvDepth, &[p.pitch_env_depth]);
    set(F::PitchEnvVelocitySens, &[p.pitch_env_velocity_sens]);
    set(F::PitchEnvT1VelocitySens, &[p.pitch_env_t1_velocity_sens]);
    set(F::PitchEnvT4VelocitySens, &[p.pitch_env_t4_velocity_sens]);
    set(F::PitchEnvTime, &p.pitch_env_time);
    set(F::PitchEnvLevel, &p.pitch_env_level);

    set(F::TvfFilterType, &[p.tvf_filter_type]);
    set(F::TvfCutoffFrequency, &[p.tvf_cutoff_frequency]);
    set(F::TvfCutoffVelocityCurve, &[p.tvf_cutoff_velocity_curve]);
    set(F::TvfCutoffVelocitySens, &[p.tvf_cutoff_velocity_sens]);
    set(F::TvfResonance, &[p.tvf_resonance]);
    set(
        F::TvfResonanceVelocitySens,
        &[p.tvf_resonance_velocity_sens],
    );
    set(F::TvfEnvDepth, &[p.tvf_env_depth]);
    set(F::TvfEnvVelocityCurve, &[p.tvf_env_velocity_curve]);
    set(F::TvfEnvVelocitySens, &[p.tvf_env_velocity_sens]);
    set(F::TvfEnvT1VelocitySens, &[p.tvf_env_t1_velocity_sens]);
    set(F::TvfEnvT4VelocitySens, &[p.tvf_env_t4_velocity_sens]);
    set(F::TvfEnvTime, &p.tvf_env_time);
    set(F::TvfEnvLevel, &p.tvf_env_level);

    set(F::TvaLevelVelocityCurve, &[p.tva_level_velocity_curve]);
    set(F::TvaLevelVelocitySens, &[p.tva_level_velocity_sens]);
    set(F::TvaEnvT1VelocitySens, &[p.tva_env_t1_velocity_sens]);
    set(F::TvaEnvT4VelocitySens, &[p.tva_env_t4_velocity_sens]);
    set(F::TvaEnvTime, &p.tva_env_time);
    set(F::TvaEnvLevel, &p.tva_env_level);

    set(F::OneShotMode, &[p.one_shot_mode]);

    for (i, w) in p.wmt.iter().enumerate() {
        let start = pcmd_wmt_byte(i, PcmdWmtField::Switch);
        encode_pcmd_wmt(w, &mut data[start..start + WMT_LAYER_SIZE]);
    }
    data[PARTIAL_RESERVE_64] = 64;
    data
}

/// Encode a PCM Drum Kit Common2 block (50 bytes), the inverse of
/// [`parse_pcmd_common2`]. The long-name reserves are left blank, and the
/// other reserves get 0, a stand-in: no documented INIT kit shows them.
pub fn encode_pcmd_common2(c2: &PcmDrumCommon2) -> Vec<u8> {
    let mut data = vec![0; PCMD_COMMON2_SIZE];
    let phrase_number = usize::from(COMMON2_PHRASE_NUMBER);
    data[phrase_number..phrase_number + 2].copy_from_slice(&encode_nibble2(c2.phrase_number));
    data[usize::from(COMMON2_TFX_SWITCH)] = c2.tfx_switch;
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_wmt_uses_int_group_id_1() {
        assert_eq!(PcmDrumWmt::default().wave_group_id, 1);
    }

    // -- Address tests --

    #[test]
    fn pcmd_base_part1() {
        let addr = pcmd_base(0);
        // Part 1 tone base = 19 00 00 00, + 10 00 00 = 19 10 00 00
        assert_eq!(addr, Address::new(0x19, 0x10, 0x00, 0x00));
    }

    #[test]
    fn pcmd_common_address_part1() {
        let addr = pcmd_common_address(0);
        assert_eq!(addr, Address::new(0x19, 0x10, 0x00, 0x00));
    }

    #[test]
    fn pcmd_common_param_address_kit_level() {
        let addr = pcmd_common_param_address(0, PcmdCommonField::KitLevel as u8);
        assert_eq!(addr, Address::new(0x19, 0x10, 0x00, 0x0C));
    }

    #[test]
    fn pcmd_partial_address_first_key() {
        let addr = pcmd_partial_address(0, 21);
        // Key 21 → offset 0x10 in byte 2
        assert_eq!(addr, Ok(Address::new(0x19, 0x10, 0x10, 0x00)));
    }

    #[test]
    fn pcmd_partial_address_second_key() {
        let addr = pcmd_partial_address(0, 22);
        // Key 22 → offset 0x12 in byte 2
        assert_eq!(addr, Ok(Address::new(0x19, 0x10, 0x12, 0x00)));
    }

    #[test]
    fn pcmd_partial_address_last_key() {
        let addr = pcmd_partial_address(0, 108);
        // Key 108: offset = 0x10 + (108-21)*2 = 0x10 + 174 = 16 + 174 = 190
        // 190 in 7-bit: 190/128=1 remainder 62, so byte1+=1, byte2=62=0x3E
        // Final: 19 11 3E 00
        assert_eq!(addr, Ok(Address::new(0x19, 0x11, 0x3E, 0x00)));
    }

    #[test]
    fn pcmd_partial_address_rejects_keys_outside_the_kit() {
        for key in [PCMD_FIRST_KEY - 1, PCMD_LAST_KEY + 1] {
            assert_eq!(
                pcmd_partial_address(0, key),
                Err(KeyOutOfRange {
                    key,
                    range: PCMD_KEYS
                })
            );
            assert!(pcmd_partial_param_address(0, key, 0x21).is_err());
        }
    }

    #[test]
    fn pcmd_partial_param_address_wmt1_switch() {
        let addr = pcmd_partial_param_address(0, 21, 0x21);
        assert_eq!(addr, Ok(Address::new(0x19, 0x10, 0x10, 0x21)));
    }

    #[test]
    fn pcmd_wmt_field_positions_follow_the_midi_implementation() {
        // WMT1 Wave Switch `00 21`, WMT4 Wave Number R `01 02`,
        // WMT4 Wave Level `01 10`.
        assert_eq!(PcmdWmtField::Switch.element_position(0), Ok(0x21));
        assert_eq!(PcmdWmtField::WaveNumberR.element_position(3), Ok(0x82));
        assert!(PcmdWmtField::WaveNumberR.is_nibblized());
        assert_eq!(PcmdWmtField::Level.element_position(3), Ok(0x90));
        assert_eq!(
            PcmdWmtField::Switch.element_position(4),
            Err(FieldIndexError {
                field: PcmdWmtField::Switch,
                index: 4
            })
        );
    }

    #[test]
    fn pcmd_partial_field_positions_follow_the_midi_implementation() {
        // Partial Pan `00 12`, TVF Env Time 3 `01 2F`, TVA Env Level 3 `01 40`,
        // One Shot Mode `01 41`.
        assert_eq!(PcmdPartialField::Pan.element_position(0), Ok(0x12));
        assert_eq!(PcmdPartialField::TvfEnvTime.element_position(2), Ok(0xAF));
        assert_eq!(PcmdPartialField::TvaEnvLevel.element_position(2), Ok(0xC0));
        assert_eq!(PcmdPartialField::OneShotMode.element_position(0), Ok(0xC1));
        assert_eq!(
            PcmdPartialField::TvaEnvLevel.element_position(3),
            Err(FieldIndexError {
                field: PcmdPartialField::TvaEnvLevel,
                index: 3
            })
        );
    }

    #[test]
    fn pcmd_partial_param_address_pitch_env_depth() {
        // Pitch Env Depth at offset 01 15
        let addr = pcmd_partial_param_address(0, 21, 0x95);
        assert_eq!(addr, Ok(Address::new(0x19, 0x10, 0x11, 0x15)));
    }

    #[test]
    fn pcmd_common2_address_part1() {
        let addr = pcmd_common2_address(0);
        assert_eq!(addr, Address::new(0x19, 0x12, 0x00, 0x00));
    }

    #[test]
    fn pcmd_common2_param_address_tfx_switch() {
        let addr = pcmd_common2_param_address(0, COMMON2_TFX_SWITCH);
        assert_eq!(addr, Address::new(0x19, 0x12, 0x00, 0x31));
    }

    #[test]
    fn pcmd_base_part16() {
        let addr = pcmd_base(15);
        // Part 16 tone base = 1C 60 00 00, + 10 00 00 = 1C 70 00 00
        assert_eq!(addr, Address::new(0x1C, 0x70, 0x00, 0x00));
    }

    // -- Parse tests --

    #[test]
    fn parse_common_basic() {
        let mut data = [0u8; PCMD_COMMON_SIZE];
        let name = b"Rock Kit     ";
        data[0x00..0x0C].copy_from_slice(&name[..12]);
        data[0x0C] = 100;

        let c = parse_pcmd_common(&data).unwrap();
        assert_eq!(c.kit_name, "Rock Kit");
        assert_eq!(c.kit_level, 100);
    }

    #[test]
    fn parse_common_short_data_returns_err() {
        assert!(parse_pcmd_common(&[0u8; 10]).is_err());
    }

    #[test]
    fn parse_wmt_basic() {
        let mut data = [0u8; WMT_LAYER_SIZE];
        data[0] = 1; // switch ON
        data[1] = 2; // SRX
        // wave_group_id = 42 → nibbles 0, 0, 2, A
        data[2] = 0x00;
        data[3] = 0x00;
        data[4] = 0x02;
        data[5] = 0x0A;
        data[14] = 2; // wave_gain = +6 dB
        data[24] = 100; // level

        let w = parse_pcmd_wmt(&data);
        assert_eq!(w.switch, 1);
        assert_eq!(w.wave_group_type, 2);
        assert_eq!(w.wave_group_id, 42);
        assert_eq!(w.wave_gain, 2);
        assert_eq!(w.level, 100);
    }

    #[test]
    fn parse_partial_basic() {
        let mut data = [0u8; PCMD_PARTIAL_SIZE];
        let name = b"Kick 1      ";
        data[0x00..0x0C].copy_from_slice(name);
        data[0x0C] = 1; // assign_type = SINGLE
        data[0x0D] = 5; // mute_group = 5
        data[0x0E] = 100; // level
        data[0x20] = 1; // wmt_velocity_control = ON

        // WMT1 switch at 0x21
        data[0x21] = 1;

        // Pitch Env Depth at linear 149
        data[149] = 64;
        // TVF Filter Type at linear 162
        data[162] = 3; // HPF
        // One Shot Mode at linear 193
        data[193] = 1;

        let p = parse_pcmd_partial(&data).unwrap();
        assert_eq!(p.partial_name, "Kick 1");
        assert_eq!(p.assign_type, 1);
        assert_eq!(p.mute_group, 5);
        assert_eq!(p.level, 100);
        assert_eq!(p.wmt_velocity_control, 1);
        assert_eq!(p.wmt[0].switch, 1);
        assert_eq!(p.pitch_env_depth, 64);
        assert_eq!(p.tvf_filter_type, 3);
        assert_eq!(p.one_shot_mode, 1);
    }

    #[test]
    fn parse_partial_short_data_returns_err() {
        assert!(parse_pcmd_partial(&[0u8; 50]).is_err());
    }

    #[test]
    fn parse_common2_basic() {
        let mut data = [0u8; PCMD_COMMON2_SIZE];
        // Phrase Number = 42 → nibbles: 2, A
        data[0x10] = 0x02;
        data[0x11] = 0x0A;
        data[0x31] = 1; // TFX on

        let c2 = parse_pcmd_common2(&data).unwrap();
        assert_eq!(c2.phrase_number, 42);
        assert_eq!(c2.tfx_switch, 1);
    }

    #[test]
    fn parse_common2_short_data_returns_err() {
        assert!(parse_pcmd_common2(&[0u8; 10]).is_err());
    }

    #[test]
    fn parse_partial_wmt4_crosses_boundary() {
        // WMT4 starts at linear index 0x78 = 120 and ends at 148
        // Verify WMT4 wave_number_l spans indices 126-129 (crossing the 128 boundary)
        let mut data = [0u8; PCMD_PARTIAL_SIZE];
        // Set WMT4 switch ON
        data[120] = 1;
        // WMT4 wave_number_l at 126-129: nibbles for value 1000
        // 1000 = 0x03E8 → nibbles: 0, 3, E, 8
        data[126] = 0x00;
        data[127] = 0x03;
        data[128] = 0x0E;
        data[129] = 0x08;

        let p = parse_pcmd_partial(&data).unwrap();
        assert_eq!(p.wmt[3].switch, 1);
        assert_eq!(p.wmt[3].wave_number_l, 1000);
    }

    #[test]
    fn blocks_round_trip_through_their_encoders() {
        let common = PcmDrumCommon {
            kit_name: "Round Trip".into(),
            kit_level: 100,
        };
        let block = encode_pcmd_common(&common);
        assert_eq!(block.len(), PCMD_COMMON_SIZE);
        assert_eq!(parse_pcmd_common(&block).unwrap(), common);
        assert_eq!(
            &block[PcmdCommonField::KitLevel.position() + 1..],
            [0, 7, 8, 0, 13]
        );

        let mut partial = PcmDrumPartial {
            partial_name: "Kick".into(),
            mute_group: 3,
            pitch_env_level: [1, 2, 3, 4, 5],
            tva_env_level: [6, 7, 8],
            one_shot_mode: 1,
            ..PcmDrumPartial::default()
        };
        partial.wmt[2] = PcmDrumWmt {
            switch: 1,
            wave_group_type: 1,
            wave_group_id: 6,
            wave_number_l: 300,
            wave_number_r: 301,
            velocity_fade_upper: 9,
            ..PcmDrumWmt::default()
        };
        let block = encode_pcmd_partial(&partial);
        assert_eq!(block.len(), PCMD_PARTIAL_SIZE);
        assert_eq!(parse_pcmd_partial(&block).unwrap(), partial);
        assert_eq!(block[PARTIAL_RESERVE_64], 64);
        assert_eq!(block[pcmd_wmt_byte(2, PcmdWmtField::VelocityFadeUpper)], 9);

        let common2 = PcmDrumCommon2 {
            phrase_number: 200,
            tfx_switch: 1,
        };
        let block = encode_pcmd_common2(&common2);
        assert_eq!(block.len(), PCMD_COMMON2_SIZE);
        assert_eq!(parse_pcmd_common2(&block).unwrap(), common2);
    }
}
