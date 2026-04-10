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
            tone_pan: 64,       // center
            tone_priority: 0,   // LAST
            coarse_tune: 64,    // 0
            fine_tune: 64,      // 0
            octave_shift: 64,   // 0
            stretch_tune_depth: 0,
            analog_feel: 0,
            mono_poly: 1,       // POLY
            legato_switch: 0,
            legato_retrigger: 0,
            portamento_switch: 0,
            portamento_mode: 0,
            portamento_type: 0,
            portamento_start: 0,
            portamento_time: 0,
            cutoff_offset: 64,          // 0
            resonance_offset: 64,       // 0
            attack_time_offset: 64,     // 0
            release_time_offset: 64,    // 0
            velocity_sens_offset: 64,   // 0
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
            coarse_tune: 64,        // 0
            fine_tune: 64,           // 0
            random_pitch_depth: 0,
            pan: 64,                 // center
            pan_keyfollow: 64,       // 0
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
            pitch_env_depth: 64,              // 0
            pitch_env_velocity_sens: 64,      // 0
            pitch_env_t1_velocity_sens: 64,   // 0
            pitch_env_t4_velocity_sens: 64,   // 0
            pitch_env_time_keyfollow: 64,     // 0
            pitch_env_time: [0; 4],
            pitch_env_level: [64; 5],         // all center

            // TVF
            tvf_filter_type: 1,       // LPF
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
}
