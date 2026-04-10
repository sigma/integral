//! PCM Synth Tone address map and state types.
//!
//! The PCM Synth tone occupies offset `00 00 00 00` within the temporary tone
//! block (it IS the base tone type). It contains: Common, PMT (Partial Mix
//! Table), four Partial blocks, Common2, and MFX (handled by `mfx.rs`).

use crate::address::{Address, DataSize};
use crate::mfx::MfxState;
use crate::params;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// PCM Synth Common block offset (relative to PCMS base).
const PCMS_COMMON_OFFSET: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

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

/// PCM Synth Partial block size (282 bytes).
pub const PCMS_PARTIAL_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x01, 0x1A);

/// PCM Synth Partial parse size in bytes.
pub const PCMS_PARTIAL_SIZE: usize = 0x011A;

/// Partial block offsets (relative to PCMS base), indexed 0-3.
const PCMS_PARTIAL_OFFSETS: [[u8; 4]; 4] = [
    [0x00, 0x00, 0x20, 0x00],
    [0x00, 0x00, 0x22, 0x00],
    [0x00, 0x00, 0x24, 0x00],
    [0x00, 0x00, 0x26, 0x00],
];

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
///
/// PCM Synth has no tone type offset (`00 00 00 00`), so this is the same
/// as `temporary_tone_base(part_index)`.
pub const fn pcms_base(part_index: u8) -> Address {
    params::temporary_tone_base(part_index)
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

/// Compute the absolute address for a single PCM Synth Partial parameter.
///
/// The offset is `u16` because partial blocks are 282 bytes (offsets go up
/// to `0x0119`). The offset is split into two address bytes:
/// `byte2 = (offset >> 8)`, `byte3 = (offset & 0xFF)`.
pub const fn pcms_partial_param_address(part_index: u8, partial: u8, offset: u16) -> Address {
    let byte2 = (offset >> 8) as u8;
    let byte3 = (offset & 0xFF) as u8;
    pcms_partial_address(part_index, partial).offset([0x00, 0x00, byte2, byte3])
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
            wave_group_id: 0,
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
            tva_env_level: [0; 3],

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

/// Helper: decode a nibblized 2-byte value from a data slice.
///
/// The two consecutive bytes each carry 4 bits of the result.
fn nibble2(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16 & 0x0F) << 4) | (data[offset + 1] as u16 & 0x0F)
}

/// Helper: decode a nibblized 4-byte value from a data slice.
///
/// The four consecutive bytes each carry 4 bits of the result.
fn nibble4(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16 & 0x0F) << 12)
        | ((data[offset + 1] as u16 & 0x0F) << 8)
        | ((data[offset + 2] as u16 & 0x0F) << 4)
        | (data[offset + 3] as u16 & 0x0F)
}

/// Helper: parse a tone name from 12 ASCII bytes, trimming trailing spaces.
fn parse_tone_name(data: &[u8]) -> String {
    data[0x00..0x0C]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Parse a PCM Synth Tone Common dump (0x50 = 80 bytes).
///
/// Offsets follow the MIDI implementation doc (`docs/midi/06-pcm-synth-tone.md`).
pub fn parse_pcms_common(data: &[u8]) -> PcmSynthCommon {
    let mut c = PcmSynthCommon::default();
    if data.len() < PCMS_COMMON_SIZE {
        return c;
    }

    c.tone_name = parse_tone_name(data);
    // 0x0C–0x0D: reserved
    c.tone_level = data[0x0E];
    c.tone_pan = data[0x0F];
    c.tone_priority = data[0x10];
    c.coarse_tune = data[0x11];
    c.fine_tune = data[0x12];
    c.octave_shift = data[0x13];
    c.stretch_tune_depth = data[0x14];
    c.analog_feel = data[0x15];
    c.mono_poly = data[0x16];
    c.legato_switch = data[0x17];
    c.legato_retrigger = data[0x18];
    c.portamento_switch = data[0x19];
    c.portamento_mode = data[0x1A];
    c.portamento_type = data[0x1B];
    c.portamento_start = data[0x1C];
    c.portamento_time = data[0x1D];
    // 0x1E–0x21: reserved
    c.cutoff_offset = data[0x22];
    c.resonance_offset = data[0x23];
    c.attack_time_offset = data[0x24];
    c.release_time_offset = data[0x25];
    c.velocity_sens_offset = data[0x26];
    // 0x27: reserved
    c.pmt_control_switch = data[0x28];
    c.pitch_bend_range_up = data[0x29];
    c.pitch_bend_range_down = data[0x2A];

    // Matrix Controls 1-4: each 9 bytes starting at 0x2B, stride 9
    for i in 0..4 {
        let base = 0x2B + i * 9;
        let mc = &mut c.matrix_controls[i];
        mc.source = data[base];
        mc.destinations[0] = data[base + 1];
        mc.sensitivities[0] = data[base + 2];
        mc.destinations[1] = data[base + 3];
        mc.sensitivities[1] = data[base + 4];
        mc.destinations[2] = data[base + 5];
        mc.sensitivities[2] = data[base + 6];
        mc.destinations[3] = data[base + 7];
        mc.sensitivities[3] = data[base + 8];
    }

    c
}

/// Parse a PCM Synth Tone PMT dump (0x29 = 41 bytes).
///
/// Offsets follow the MIDI implementation doc.
pub fn parse_pcms_pmt(data: &[u8]) -> PcmSynthPmt {
    let mut pmt = PcmSynthPmt::default();
    if data.len() < PCMS_PMT_SIZE {
        return pmt;
    }

    pmt.structure_type_12 = data[0x00];
    pmt.booster_12 = data[0x01];
    pmt.structure_type_34 = data[0x02];
    pmt.booster_34 = data[0x03];
    pmt.pmt_velocity_control = data[0x04];

    // 4 PMT partial entries at offsets 0x05, 0x0E, 0x17, 0x20 (each 9 bytes)
    let entry_offsets = [0x05, 0x0E, 0x17, 0x20];
    for (i, &off) in entry_offsets.iter().enumerate() {
        let e = &mut pmt.partial_entries[i];
        e.partial_switch = data[off];
        e.key_range_lower = data[off + 1];
        e.key_range_upper = data[off + 2];
        e.key_fade_lower = data[off + 3];
        e.key_fade_upper = data[off + 4];
        e.velocity_range_lower = data[off + 5];
        e.velocity_range_upper = data[off + 6];
        e.velocity_fade_lower = data[off + 7];
        e.velocity_fade_upper = data[off + 8];
    }

    pmt
}

/// Parse a PCM Synth Tone Partial dump (0x011A = 282 bytes).
///
/// The data array is a contiguous byte stream from the DT1 response.
/// Linear indices map directly to the SysEx offsets:
/// indices 0-127 = offsets `00 00`-`00 7F`, index 128 = offset `01 00`, etc.
pub fn parse_pcms_partial(data: &[u8]) -> PcmSynthPartial {
    let mut p = PcmSynthPartial::default();
    if data.len() < PCMS_PARTIAL_SIZE {
        return p;
    }

    // -- General --
    p.level = data[0x00];
    p.coarse_tune = data[0x01];
    p.fine_tune = data[0x02];
    p.random_pitch_depth = data[0x03];
    p.pan = data[0x04];
    p.pan_keyfollow = data[0x05];
    p.random_pan_depth = data[0x06];
    p.alternate_pan_depth = data[0x07];
    p.env_mode = data[0x08];
    p.delay_mode = data[0x09];
    p.delay_time = nibble2(data, 0x0A);

    // -- Output --
    p.output_level = data[0x0C];
    // 0x0D-0x0E: reserved
    p.chorus_send = data[0x0F];
    p.reverb_send = data[0x10];
    // 0x11: reserved

    // -- Receive switches --
    p.receive_bender = data[0x12];
    p.receive_expression = data[0x13];
    p.receive_hold1 = data[0x14];
    // 0x15: reserved
    p.redamper_switch = data[0x16];

    // -- Partial control switches --
    p.control1_switches.copy_from_slice(&data[0x17..0x1B]);
    p.control2_switches.copy_from_slice(&data[0x1B..0x1F]);
    p.control3_switches.copy_from_slice(&data[0x1F..0x23]);
    p.control4_switches.copy_from_slice(&data[0x23..0x27]);

    // -- Wave --
    p.wave_group_type = data[0x27];
    p.wave_group_id = nibble4(data, 0x28);
    p.wave_number_l = nibble4(data, 0x2C);
    p.wave_number_r = nibble4(data, 0x30);
    p.wave_gain = data[0x34];
    p.wave_fxm_switch = data[0x35];
    p.wave_fxm_color = data[0x36];
    p.wave_fxm_depth = data[0x37];
    p.wave_tempo_sync = data[0x38];
    p.wave_pitch_keyfollow = data[0x39];

    // -- Pitch Envelope --
    p.pitch_env_depth = data[0x3A];
    p.pitch_env_velocity_sens = data[0x3B];
    p.pitch_env_t1_velocity_sens = data[0x3C];
    p.pitch_env_t4_velocity_sens = data[0x3D];
    p.pitch_env_time_keyfollow = data[0x3E];
    p.pitch_env_time.copy_from_slice(&data[0x3F..0x43]);
    p.pitch_env_level.copy_from_slice(&data[0x43..0x48]);

    // -- TVF (Filter) --
    p.tvf_filter_type = data[0x48];
    p.tvf_cutoff_frequency = data[0x49];
    p.tvf_cutoff_keyfollow = data[0x4A];
    p.tvf_cutoff_velocity_curve = data[0x4B];
    p.tvf_cutoff_velocity_sens = data[0x4C];
    p.tvf_resonance = data[0x4D];
    p.tvf_resonance_velocity_sens = data[0x4E];
    p.tvf_env_depth = data[0x4F];
    p.tvf_env_velocity_curve = data[0x50];
    p.tvf_env_velocity_sens = data[0x51];
    p.tvf_env_t1_velocity_sens = data[0x52];
    p.tvf_env_t4_velocity_sens = data[0x53];
    p.tvf_env_time_keyfollow = data[0x54];
    p.tvf_env_time.copy_from_slice(&data[0x55..0x59]);
    p.tvf_env_level.copy_from_slice(&data[0x59..0x5E]);

    // -- TVA (Amplifier) --
    p.tva_bias_level = data[0x5E];
    p.tva_bias_position = data[0x5F];
    p.tva_bias_direction = data[0x60];
    p.tva_level_velocity_curve = data[0x61];
    p.tva_level_velocity_sens = data[0x62];
    p.tva_env_t1_velocity_sens = data[0x63];
    p.tva_env_t4_velocity_sens = data[0x64];
    p.tva_env_time_keyfollow = data[0x65];
    p.tva_env_time.copy_from_slice(&data[0x66..0x6A]);
    p.tva_env_level.copy_from_slice(&data[0x6A..0x6D]);

    // -- LFO1 --
    p.lfo1_waveform = data[0x6D];
    p.lfo1_rate = nibble2(data, 0x6E);
    p.lfo1_offset = data[0x70];
    p.lfo1_rate_detune = data[0x71];
    p.lfo1_delay_time = data[0x72];
    p.lfo1_delay_time_keyfollow = data[0x73];
    p.lfo1_fade_mode = data[0x74];
    p.lfo1_fade_time = data[0x75];
    p.lfo1_key_trigger = data[0x76];
    p.lfo1_pitch_depth = data[0x77];
    p.lfo1_tvf_depth = data[0x78];
    p.lfo1_tva_depth = data[0x79];
    p.lfo1_pan_depth = data[0x7A];

    // -- LFO2 --
    // LFO2 starts at offset 0x7B in the linear data array
    p.lfo2_waveform = data[0x7B];
    p.lfo2_rate = nibble2(data, 0x7C);
    p.lfo2_offset = data[0x7E];
    p.lfo2_rate_detune = data[0x7F];
    // Bytes beyond 0x7F: linear data index continues at 0x80 (= SysEx offset 01 00)
    p.lfo2_delay_time = data[0x80];
    p.lfo2_delay_time_keyfollow = data[0x81];
    p.lfo2_fade_mode = data[0x82];
    p.lfo2_fade_time = data[0x83];
    p.lfo2_key_trigger = data[0x84];
    p.lfo2_pitch_depth = data[0x85];
    p.lfo2_tvf_depth = data[0x86];
    p.lfo2_tva_depth = data[0x87];
    p.lfo2_pan_depth = data[0x88];

    // -- LFO Step Sequencer --
    p.lfo_step_type = data[0x89];
    p.lfo_step_values.copy_from_slice(&data[0x8A..0x9A]);

    p
}

/// Parse a PCM Synth Tone Common2 dump (0x3C = 60 bytes).
///
/// Offsets follow the MIDI implementation doc.
pub fn parse_pcms_common2(data: &[u8]) -> PcmSynthCommon2 {
    let mut c2 = PcmSynthCommon2::default();
    if data.len() < PCMS_COMMON2_SIZE {
        return c2;
    }

    // 0x00–0x0F: reserved
    c2.tone_category = data[0x10];
    c2.tone_number = nibble2(data, 0x11);
    c2.phrase_octave_shift = data[0x13];
    // 0x14–0x32: reserved
    c2.tfx_switch = data[0x33];
    // 0x34–0x37: reserved
    c2.phrase_number = nibble4(data, 0x38);

    c2
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let addr = pcms_partial_param_address(0, 0, 0x0000);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x20, 0x00));
    }

    #[test]
    fn pcms_partial_param_address_two_byte_offset() {
        // lfo2_delay_time at offset 0x0100
        let addr = pcms_partial_param_address(0, 0, 0x0100);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x21, 0x00));
    }

    #[test]
    fn pcms_partial_param_address_lfo_step_last() {
        // Last LFO step value at offset 0x0119
        let addr = pcms_partial_param_address(0, 0, 0x0119);
        assert_eq!(addr, Address::new(0x19, 0x00, 0x21, 0x19));
    }

    #[test]
    fn pcms_partial_param_address_partial2_high_offset() {
        // partial 2 (offset 00 22 00) + param offset 0x0108
        let addr = pcms_partial_param_address(0, 1, 0x0108);
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

        let c = parse_pcms_common(&data);
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
    fn parse_common_short_returns_default() {
        let data = [0u8; 10]; // way too short
        let c = parse_pcms_common(&data);
        assert_eq!(c, PcmSynthCommon::default());
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

        let pmt = parse_pcms_pmt(&data);
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

        let p = parse_pcms_partial(&data);
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

        let p = parse_pcms_partial(&data);
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

        let c2 = parse_pcms_common2(&data);
        assert_eq!(c2.tone_category, 42);
        assert_eq!(c2.tone_number, 0xAB);
        assert_eq!(c2.phrase_octave_shift, 63);
        assert_eq!(c2.tfx_switch, 1);
        assert_eq!(c2.phrase_number, 0x1234);
    }
}
