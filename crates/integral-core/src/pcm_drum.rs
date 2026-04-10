//! PCM Drum Kit address map, state types, and parse functions.
//!
//! The PCM Drum Kit tone occupies offset `10 00 00` within the temporary tone
//! block. It contains: Common parameters, MFX (handled by `mfx.rs`), Comp+EQ
//! (handled by `params.rs`), 88 per-key Partial blocks, and Common2.

use crate::address::{Address, DataSize};
use crate::mfx::MfxState;
use crate::params;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// PCM Drum Kit tone type offset within the temporary tone block.
const PCMD_TONE_OFFSET: [u8; 4] = [0x00, 0x10, 0x00, 0x00];

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

/// Number of PCM Drum Kit key slots.
pub const PCMD_KEY_COUNT: usize = (PCMD_LAST_KEY - PCMD_FIRST_KEY + 1) as usize; // 88

/// Number of bytes per WMT layer in a PCM Drum Kit partial.
const WMT_LAYER_SIZE: usize = 29;

// ---------------------------------------------------------------------------
// Address functions
// ---------------------------------------------------------------------------

/// Compute the absolute PCM Drum Kit base address for a part.
pub const fn pcmd_base(part_index: u8) -> Address {
    params::temporary_tone_base(part_index).offset(PCMD_TONE_OFFSET)
}

/// Compute the absolute PCM Drum Kit Common block address for a part.
pub const fn pcmd_common_address(part_index: u8) -> Address {
    pcmd_base(part_index)
}

/// Compute the absolute address for a single PCM Drum Kit Common parameter.
pub const fn pcmd_common_param_address(part_index: u8, offset: u8) -> Address {
    pcmd_common_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

/// Compute the absolute PCM Drum Kit Partial block address for a part and
/// key number.
///
/// Key 21 maps to offset `00 10 00`, key 22 to `00 12 00`, etc.
/// Each key is spaced `00 02 00` apart.
pub const fn pcmd_partial_address(part_index: u8, key: u8) -> Address {
    let key_offset = (key - PCMD_FIRST_KEY) as u16;
    // Each key occupies 0x02 00 in the address space (i.e., byte2 increments by 2)
    // Starting at byte1=0x00, byte2=0x10
    let byte2_total = 0x10 + key_offset * 2;
    // Handle 7-bit carry: byte2 wraps at 128
    let byte1_add = (byte2_total / 128) as u8;
    let byte2 = (byte2_total % 128) as u8;
    pcmd_base(part_index).offset([0x00, byte1_add, byte2, 0x00])
}

/// Compute the absolute address for a single PCM Drum Kit Partial parameter.
///
/// The offset is `u16` because partial blocks are 195 bytes (offsets go up
/// to `0x0142`). The offset is split into two address bytes.
pub const fn pcmd_partial_param_address(part_index: u8, key: u8, offset: u16) -> Address {
    let byte2 = (offset >> 8) as u8;
    let byte3 = (offset & 0xFF) as u8;
    pcmd_partial_address(part_index, key).offset([0x00, 0x00, byte2, byte3])
}

/// Compute the absolute PCM Drum Kit Common2 block address for a part.
pub const fn pcmd_common2_address(part_index: u8) -> Address {
    pcmd_base(part_index).offset([0x00, 0x02, 0x00, 0x00])
}

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
            wave_group_id: 0,
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
            wmt: vec![
                PcmDrumWmt::default(),
                PcmDrumWmt::default(),
                PcmDrumWmt::default(),
                PcmDrumWmt::default(),
            ],

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

/// Helper: decode a nibblized 2-byte value from a data slice.
fn nibble2(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16 & 0x0F) << 4) | (data[offset + 1] as u16 & 0x0F)
}

/// Helper: decode a nibblized 4-byte value from a data slice.
fn nibble4(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16 & 0x0F) << 12)
        | ((data[offset + 1] as u16 & 0x0F) << 8)
        | ((data[offset + 2] as u16 & 0x0F) << 4)
        | (data[offset + 3] as u16 & 0x0F)
}

/// Helper: parse a name from 12 ASCII bytes, trimming trailing spaces.
fn parse_name(data: &[u8]) -> String {
    data[..12]
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

/// Parse a single WMT layer from 29 contiguous bytes.
fn parse_pcmd_wmt(data: &[u8]) -> PcmDrumWmt {
    let mut w = PcmDrumWmt::default();
    if data.len() < WMT_LAYER_SIZE {
        return w;
    }
    w.switch = data[0];
    w.wave_group_type = data[1];
    w.wave_group_id = nibble4(data, 2);
    w.wave_number_l = nibble4(data, 6);
    w.wave_number_r = nibble4(data, 10);
    w.wave_gain = data[14];
    w.wave_fxm_switch = data[15];
    w.wave_fxm_color = data[16];
    w.wave_fxm_depth = data[17];
    w.wave_tempo_sync = data[18];
    w.coarse_tune = data[19];
    w.fine_tune = data[20];
    w.pan = data[21];
    w.random_pan_switch = data[22];
    w.alternate_pan_switch = data[23];
    w.level = data[24];
    w.velocity_range_lower = data[25];
    w.velocity_range_upper = data[26];
    w.velocity_fade_lower = data[27];
    w.velocity_fade_upper = data[28];
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
pub fn parse_pcmd_common(data: &[u8]) -> PcmDrumCommon {
    let mut c = PcmDrumCommon::default();
    if data.len() < PCMD_COMMON_SIZE {
        return c;
    }
    c.kit_name = parse_name(data);
    c.kit_level = data[0x0C];
    c
}

/// Parse a PCM Drum Kit per-key Partial dump (195 bytes).
///
/// The data array is a contiguous byte stream from the DT1 response.
/// Linear indices map to SysEx offsets: indices 0-127 = offsets `00 00`-`00 7F`,
/// index 128 = offset `01 00`, etc.
pub fn parse_pcmd_partial(data: &[u8]) -> PcmDrumPartial {
    let mut p = PcmDrumPartial::default();
    if data.len() < PCMD_PARTIAL_SIZE {
        return p;
    }

    // -- Common per-key (linear indices 0-30) --
    p.partial_name = parse_name(data);
    p.assign_type = data[0x0C];
    p.mute_group = data[0x0D];
    p.level = data[0x0E];
    p.coarse_tune = data[0x0F];
    p.fine_tune = data[0x10];
    p.random_pitch_depth = data[0x11];
    p.pan = data[0x12];
    p.random_pan_depth = data[0x13];
    p.alternate_pan_depth = data[0x14];
    p.env_mode = data[0x15];
    p.output_level = data[0x16];
    // 0x17, 0x18: reserved
    p.chorus_send = data[0x19];
    p.reverb_send = data[0x1A];
    p.output_assign = data[0x1B];
    p.pitch_bend_range = data[0x1C];
    p.receive_expression = data[0x1D];
    p.receive_hold1 = data[0x1E];
    // 0x1F: reserved

    // -- WMT --
    p.wmt_velocity_control = data[0x20];

    // WMT1 at index 0x21, WMT2 at 0x3E, WMT3 at 0x5B, WMT4 at 0x78
    let wmt_starts: [usize; 4] = [0x21, 0x3E, 0x5B, 0x78];
    for (i, &start) in wmt_starts.iter().enumerate() {
        p.wmt[i] = parse_pcmd_wmt(&data[start..start + WMT_LAYER_SIZE]);
    }

    // -- Pitch Envelope (offsets 01 15 - 01 21, linear 149-161) --
    p.pitch_env_depth = data[149];
    p.pitch_env_velocity_sens = data[150];
    p.pitch_env_t1_velocity_sens = data[151];
    p.pitch_env_t4_velocity_sens = data[152];
    p.pitch_env_time.copy_from_slice(&data[153..157]);
    p.pitch_env_level.copy_from_slice(&data[157..162]);

    // -- TVF (offsets 01 22 - 01 27, linear 162-167) --
    p.tvf_filter_type = data[162];
    p.tvf_cutoff_frequency = data[163];
    p.tvf_cutoff_velocity_curve = data[164];
    p.tvf_cutoff_velocity_sens = data[165];
    p.tvf_resonance = data[166];
    p.tvf_resonance_velocity_sens = data[167];

    // -- TVF Envelope (offsets 01 28 - 01 35, linear 168-181) --
    p.tvf_env_depth = data[168];
    p.tvf_env_velocity_curve = data[169];
    p.tvf_env_velocity_sens = data[170];
    p.tvf_env_t1_velocity_sens = data[171];
    p.tvf_env_t4_velocity_sens = data[172];
    p.tvf_env_time.copy_from_slice(&data[173..177]);
    p.tvf_env_level.copy_from_slice(&data[177..182]);

    // -- TVA (offsets 01 36 - 01 40, linear 182-192) --
    p.tva_level_velocity_curve = data[182];
    p.tva_level_velocity_sens = data[183];
    p.tva_env_t1_velocity_sens = data[184];
    p.tva_env_t4_velocity_sens = data[185];
    p.tva_env_time.copy_from_slice(&data[186..190]);
    p.tva_env_level.copy_from_slice(&data[190..193]);

    // -- One Shot Mode (offset 01 41, linear 193) --
    p.one_shot_mode = data[193];
    // data[194] = reserved

    p
}

/// Parse a PCM Drum Kit Common2 dump (50 bytes).
///
/// Offsets:
/// - `00 00`-`00 0F`: reserved
/// - `00 10`-`00 11`: Phrase Number (nibblized 2 bytes)
/// - `00 12`-`00 30`: reserved
/// - `00 31`: TFX Switch
pub fn parse_pcmd_common2(data: &[u8]) -> PcmDrumCommon2 {
    let mut c2 = PcmDrumCommon2::default();
    if data.len() < PCMD_COMMON2_SIZE {
        return c2;
    }
    c2.phrase_number = nibble2(data, 0x10);
    c2.tfx_switch = data[0x31];
    c2
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let addr = pcmd_common_param_address(0, 0x0C);
        assert_eq!(addr, Address::new(0x19, 0x10, 0x00, 0x0C));
    }

    #[test]
    fn pcmd_partial_address_first_key() {
        let addr = pcmd_partial_address(0, 21);
        // Key 21 → offset 0x10 in byte 2
        assert_eq!(addr, Address::new(0x19, 0x10, 0x10, 0x00));
    }

    #[test]
    fn pcmd_partial_address_second_key() {
        let addr = pcmd_partial_address(0, 22);
        // Key 22 → offset 0x12 in byte 2
        assert_eq!(addr, Address::new(0x19, 0x10, 0x12, 0x00));
    }

    #[test]
    fn pcmd_partial_address_last_key() {
        let addr = pcmd_partial_address(0, 108);
        // Key 108: offset = 0x10 + (108-21)*2 = 0x10 + 174 = 16 + 174 = 190
        // 190 in 7-bit: 190/128=1 remainder 62, so byte1+=1, byte2=62=0x3E
        // Final: 19 11 3E 00
        assert_eq!(addr, Address::new(0x19, 0x11, 0x3E, 0x00));
    }

    #[test]
    fn pcmd_partial_param_address_wmt1_switch() {
        let addr = pcmd_partial_param_address(0, 21, 0x0021);
        assert_eq!(addr, Address::new(0x19, 0x10, 0x10, 0x21));
    }

    #[test]
    fn pcmd_partial_param_address_pitch_env_depth() {
        // Pitch Env Depth at offset 01 15
        let addr = pcmd_partial_param_address(0, 21, 0x0115);
        assert_eq!(addr, Address::new(0x19, 0x10, 0x11, 0x15));
    }

    #[test]
    fn pcmd_common2_address_part1() {
        let addr = pcmd_common2_address(0);
        assert_eq!(addr, Address::new(0x19, 0x12, 0x00, 0x00));
    }

    #[test]
    fn pcmd_common2_param_address_tfx_switch() {
        let addr = pcmd_common2_param_address(0, 0x31);
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

        let c = parse_pcmd_common(&data);
        assert_eq!(c.kit_name, "Rock Kit");
        assert_eq!(c.kit_level, 100);
    }

    #[test]
    fn parse_common_short_data_returns_default() {
        let data = [0u8; 10];
        let c = parse_pcmd_common(&data);
        assert_eq!(c, PcmDrumCommon::default());
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

        let p = parse_pcmd_partial(&data);
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
    fn parse_partial_short_data_returns_default() {
        let data = [0u8; 50];
        let p = parse_pcmd_partial(&data);
        assert_eq!(p, PcmDrumPartial::default());
    }

    #[test]
    fn parse_common2_basic() {
        let mut data = [0u8; PCMD_COMMON2_SIZE];
        // Phrase Number = 42 → nibbles: 2, A
        data[0x10] = 0x02;
        data[0x11] = 0x0A;
        data[0x31] = 1; // TFX on

        let c2 = parse_pcmd_common2(&data);
        assert_eq!(c2.phrase_number, 42);
        assert_eq!(c2.tfx_switch, 1);
    }

    #[test]
    fn parse_common2_short_data_returns_default() {
        let data = [0u8; 10];
        let c2 = parse_pcmd_common2(&data);
        assert_eq!(c2, PcmDrumCommon2::default());
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

        let p = parse_pcmd_partial(&data);
        assert_eq!(p.wmt[3].switch, 1);
        assert_eq!(p.wmt[3].wave_number_l, 1000);
    }
}
