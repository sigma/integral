//! SuperNATURAL Synth Tone address map, state types, and parse functions.
//!
//! The SN-S tone occupies offset `01 00 00` within the temporary tone block.
//! It contains: Common parameters, MFX (handled by `mfx.rs`), and up to
//! three Partial blocks.

use crate::address::{Address, DataSize};
use crate::mfx::MfxState;
use crate::params;

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

/// SN-S tone type offset within the temporary tone block.
const SNS_TONE_OFFSET: [u8; 4] = [0x00, 0x01, 0x00, 0x00];

/// SN-S Common block offset (relative to SN-S base).
const SNS_COMMON_OFFSET: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// SN-S Common block size.
pub const SNS_COMMON_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x40);

/// SN-S Common parse size in bytes.
pub const SNS_COMMON_SIZE: usize = 0x40;

/// SN-S Partial block size.
pub const SNS_PARTIAL_BLOCK_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x3D);

/// SN-S Partial parse size in bytes.
pub const SNS_PARTIAL_SIZE: usize = 0x3D;

/// Partial block offsets (relative to SN-S base), indexed 0–2.
const SNS_PARTIAL_OFFSETS: [[u8; 4]; 3] = [
    [0x00, 0x00, 0x20, 0x00],
    [0x00, 0x00, 0x21, 0x00],
    [0x00, 0x00, 0x22, 0x00],
];

/// Compute the absolute SN-S base address for a part.
const fn sns_base(part_index: u8) -> Address {
    params::temporary_tone_base(part_index).offset(SNS_TONE_OFFSET)
}

/// Compute the absolute SN-S Common block address for a part.
pub const fn sns_common_address(part_index: u8) -> Address {
    sns_base(part_index).offset(SNS_COMMON_OFFSET)
}

/// Compute the absolute address for an SN-S Partial block (partial 0–2).
pub const fn sns_partial_address(part_index: u8, partial_index: u8) -> Address {
    sns_base(part_index).offset(SNS_PARTIAL_OFFSETS[partial_index as usize])
}

/// Compute the absolute address for a single SN-S Common parameter.
pub const fn sns_common_param_address(part_index: u8, offset: u8) -> Address {
    sns_common_address(part_index).offset([0x00, 0x00, 0x00, offset])
}

/// Compute the absolute address for a single SN-S Partial parameter.
pub const fn sns_partial_param_address(part_index: u8, partial_index: u8, offset: u8) -> Address {
    sns_partial_address(part_index, partial_index).offset([0x00, 0x00, 0x00, offset])
}

// ---------------------------------------------------------------------------
// State types
// ---------------------------------------------------------------------------

/// SN Synth Tone Common parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnSynthCommon {
    /// Tone name (up to 12 ASCII characters).
    pub tone_name: String,
    /// Tone Level (0–127).
    pub tone_level: u8,
    /// Portamento Switch (0=OFF, 1=ON).
    pub portamento_switch: u8,
    /// Portamento Time (0–127).
    pub portamento_time: u8,
    /// Mono Switch (0=OFF, 1=ON).
    pub mono_switch: u8,
    /// Octave Shift (raw 61–67, display −3 to +3).
    pub octave_shift: u8,
    /// Pitch Bend Range Up (0–24).
    pub pitch_bend_range_up: u8,
    /// Pitch Bend Range Down (0–24).
    pub pitch_bend_range_down: u8,
    /// Partial 1 Switch (0=OFF, 1=ON).
    pub partial1_switch: u8,
    /// Partial 1 Select (0=OFF, 1=ON).
    pub partial1_select: u8,
    /// Partial 2 Switch (0=OFF, 1=ON).
    pub partial2_switch: u8,
    /// Partial 2 Select (0=OFF, 1=ON).
    pub partial2_select: u8,
    /// Partial 3 Switch (0=OFF, 1=ON).
    pub partial3_switch: u8,
    /// Partial 3 Select (0=OFF, 1=ON).
    pub partial3_select: u8,
    /// RING Switch (0=OFF, 2=ON).
    pub ring_switch: u8,
    /// TFX Switch (0=OFF, 1=ON).
    pub tfx_switch: u8,
    /// Unison Switch (0=OFF, 1=ON).
    pub unison_switch: u8,
    /// Portamento Mode (0=NORMAL, 1=LEGATO).
    pub portamento_mode: u8,
    /// Legato Switch (0=OFF, 1=ON).
    pub legato_switch: u8,
    /// Analog Feel (0–127).
    pub analog_feel: u8,
    /// Wave Shape (0–127).
    pub wave_shape: u8,
    /// Tone Category (0–127).
    pub tone_category: u8,
    /// Phrase Number (0–65535, nibblized).
    pub phrase_number: u16,
    /// Phrase Octave Shift (raw 61–67, display −3 to +3).
    pub phrase_octave_shift: u8,
    /// Unison Size (0–3, display 2/4/6/8).
    pub unison_size: u8,
}

impl Default for SnSynthCommon {
    fn default() -> Self {
        Self {
            tone_name: String::new(),
            tone_level: 127,
            portamento_switch: 0,
            portamento_time: 0,
            mono_switch: 0,
            octave_shift: 64, // 0
            pitch_bend_range_up: 2,
            pitch_bend_range_down: 2,
            partial1_switch: 1,
            partial1_select: 1,
            partial2_switch: 0,
            partial2_select: 0,
            partial3_switch: 0,
            partial3_select: 0,
            ring_switch: 0,
            tfx_switch: 0,
            unison_switch: 0,
            portamento_mode: 0,
            legato_switch: 0,
            analog_feel: 0,
            wave_shape: 0,
            tone_category: 0,
            phrase_number: 0,
            phrase_octave_shift: 64, // 0
            unison_size: 0,
        }
    }
}

/// SN Synth Tone Partial parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnSynthPartial {
    /// OSC Wave (0–7: SAW, SQR, PW-SQR, TRI, SINE, NOISE, SUPER-SAW, PCM).
    pub osc_wave: u8,
    /// OSC Wave Variation (0–2: A, B, C).
    pub osc_wave_variation: u8,
    /// OSC Pitch (raw 40–88, display −24 to +24).
    pub osc_pitch: u8,
    /// OSC Detune (raw 14–114, display −50 to +50).
    pub osc_detune: u8,
    /// OSC Pulse Width Mod Depth (0–127).
    pub osc_pw_mod_depth: u8,
    /// OSC Pulse Width (0–127).
    pub osc_pulse_width: u8,
    /// OSC Pitch Env Attack Time (0–127).
    pub osc_pitch_env_attack: u8,
    /// OSC Pitch Env Decay (0–127).
    pub osc_pitch_env_decay: u8,
    /// OSC Pitch Env Depth (raw 1–127, display −63 to +63).
    pub osc_pitch_env_depth: u8,
    /// FILTER Mode (0–7: BYPASS, LPF, HPF, BPF, PKG, LPF2, LPF3, LPF4).
    pub filter_mode: u8,
    /// FILTER Slope (0=−12dB, 1=−24dB).
    pub filter_slope: u8,
    /// FILTER Cutoff (0–127).
    pub filter_cutoff: u8,
    /// FILTER Cutoff Keyfollow (raw 54–74, display −100 to +100).
    pub filter_keyfollow: u8,
    /// FILTER Env Velocity Sens (raw 1–127, display −63 to +63).
    pub filter_env_vel_sens: u8,
    /// FILTER Resonance (0–127).
    pub filter_resonance: u8,
    /// FILTER Env Attack Time (0–127).
    pub filter_env_attack: u8,
    /// FILTER Env Decay Time (0–127).
    pub filter_env_decay: u8,
    /// FILTER Env Sustain Level (0–127).
    pub filter_env_sustain: u8,
    /// FILTER Env Release Time (0–127).
    pub filter_env_release: u8,
    /// FILTER Env Depth (raw 1–127, display −63 to +63).
    pub filter_env_depth: u8,
    /// AMP Level (0–127).
    pub amp_level: u8,
    /// AMP Level Velocity Sens (raw 1–127, display −63 to +63).
    pub amp_vel_sens: u8,
    /// AMP Env Attack Time (0–127).
    pub amp_env_attack: u8,
    /// AMP Env Decay Time (0–127).
    pub amp_env_decay: u8,
    /// AMP Env Sustain Level (0–127).
    pub amp_env_sustain: u8,
    /// AMP Env Release Time (0–127).
    pub amp_env_release: u8,
    /// AMP Pan (0–127, display L64–63R).
    pub amp_pan: u8,
    /// LFO Shape (0–5: TRI, SIN, SAW, SQR, S&H, RND).
    pub lfo_shape: u8,
    /// LFO Rate (0–127).
    pub lfo_rate: u8,
    /// LFO Tempo Sync Switch (0=OFF, 1=ON).
    pub lfo_tempo_sync: u8,
    /// LFO Tempo Sync Note (0–19).
    pub lfo_tempo_sync_note: u8,
    /// LFO Fade Time (0–127).
    pub lfo_fade_time: u8,
    /// LFO Key Trigger (0=OFF, 1=ON).
    pub lfo_key_trigger: u8,
    /// LFO Pitch Depth (raw 1–127, display −63 to +63).
    pub lfo_pitch_depth: u8,
    /// LFO Filter Depth (raw 1–127, display −63 to +63).
    pub lfo_filter_depth: u8,
    /// LFO Amp Depth (raw 1–127, display −63 to +63).
    pub lfo_amp_depth: u8,
    /// LFO Pan Depth (raw 1–127, display −63 to +63).
    pub lfo_pan_depth: u8,
    /// Modulation LFO Shape (0–5: TRI, SIN, SAW, SQR, S&H, RND).
    pub mod_lfo_shape: u8,
    /// Modulation LFO Rate (0–127).
    pub mod_lfo_rate: u8,
    /// Modulation LFO Tempo Sync Switch (0=OFF, 1=ON).
    pub mod_lfo_tempo_sync: u8,
    /// Modulation LFO Tempo Sync Note (0–19).
    pub mod_lfo_tempo_sync_note: u8,
    /// OSC Pulse Width Shift (0–127).
    pub pw_shift: u8,
    /// Modulation LFO Pitch Depth (raw 1–127, display −63 to +63).
    pub mod_lfo_pitch_depth: u8,
    /// Modulation LFO Filter Depth (raw 1–127, display −63 to +63).
    pub mod_lfo_filter_depth: u8,
    /// Modulation LFO Amp Depth (raw 1–127, display −63 to +63).
    pub mod_lfo_amp_depth: u8,
    /// Modulation LFO Pan Depth (raw 1–127, display −63 to +63).
    pub mod_lfo_pan_depth: u8,
    /// Cutoff Aftertouch Sens (raw 1–127, display −63 to +63).
    pub aftertouch_cutoff: u8,
    /// Level Aftertouch Sens (raw 1–127, display −63 to +63).
    pub aftertouch_level: u8,
    /// Wave Gain (0–3: −6, 0, +6, +12 dB).
    pub wave_gain: u8,
    /// Wave Number (0–16384, nibblized; 0=OFF).
    pub wave_number: u16,
    /// HPF Cutoff (0–127).
    pub hpf_cutoff: u8,
    /// Super Saw Detune (0–127).
    pub super_saw_detune: u8,
    /// Modulation LFO Rate Control (raw 1–127, display −63 to +63).
    pub mod_lfo_rate_control: u8,
    /// AMP Level Keyfollow (raw 54–74, display −100 to +100).
    pub amp_level_keyfollow: u8,
}

impl Default for SnSynthPartial {
    fn default() -> Self {
        Self {
            osc_wave: 0,
            osc_wave_variation: 0,
            osc_pitch: 64,  // 0
            osc_detune: 64, // 0
            osc_pw_mod_depth: 0,
            osc_pulse_width: 0,
            osc_pitch_env_attack: 0,
            osc_pitch_env_decay: 0,
            osc_pitch_env_depth: 64, // 0
            filter_mode: 1,          // LPF
            filter_slope: 0,
            filter_cutoff: 127,
            filter_keyfollow: 64, // 0
            filter_env_vel_sens: 64,
            filter_resonance: 0,
            filter_env_attack: 0,
            filter_env_decay: 0,
            filter_env_sustain: 0,
            filter_env_release: 0,
            filter_env_depth: 64,
            amp_level: 127,
            amp_vel_sens: 64,
            amp_env_attack: 0,
            amp_env_decay: 0,
            amp_env_sustain: 127,
            amp_env_release: 0,
            amp_pan: 64,
            lfo_shape: 0,
            lfo_rate: 0,
            lfo_tempo_sync: 0,
            lfo_tempo_sync_note: 0,
            lfo_fade_time: 0,
            lfo_key_trigger: 0,
            lfo_pitch_depth: 64,
            lfo_filter_depth: 64,
            lfo_amp_depth: 64,
            lfo_pan_depth: 64,
            mod_lfo_shape: 0,
            mod_lfo_rate: 0,
            mod_lfo_tempo_sync: 0,
            mod_lfo_tempo_sync_note: 0,
            pw_shift: 0,
            mod_lfo_pitch_depth: 64,
            mod_lfo_filter_depth: 64,
            mod_lfo_amp_depth: 64,
            mod_lfo_pan_depth: 64,
            aftertouch_cutoff: 64,
            aftertouch_level: 64,
            wave_gain: 1, // 0 dB
            wave_number: 0,
            hpf_cutoff: 0,
            super_saw_detune: 0,
            mod_lfo_rate_control: 64,
            amp_level_keyfollow: 64,
        }
    }
}

/// Full SN Synth Tone state for one part.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnSynthState {
    /// Common parameters.
    pub common: SnSynthCommon,
    /// Per-partial parameters (3 partials).
    pub partials: [SnSynthPartial; 3],
    /// MFX parameters.
    pub mfx: MfxState,
}

// ---------------------------------------------------------------------------
// Parse functions
// ---------------------------------------------------------------------------

/// Parse an SN Synth Tone Common dump (0x40 bytes).
///
/// Offsets follow the MIDI implementation doc:
/// - `00 00`–`00 0B`: Tone Name (12 ASCII chars)
/// - `00 0C`: Tone Level
/// - `00 0D`–`00 0F`: (reserve, nibblized)
/// - `00 10`–`00 11`: (reserve)
/// - `00 12`: Portamento Switch
/// - ...through `00 3F`.
pub fn parse_sns_common(data: &[u8]) -> SnSynthCommon {
    let mut c = SnSynthCommon::default();
    if data.len() < SNS_COMMON_SIZE {
        return c;
    }

    // Tone Name: bytes 0x00–0x0B
    c.tone_name = data[0x00..0x0C]
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
        .to_string();

    c.tone_level = data[0x0C];
    // 0x0D–0x0F: reserve (nibblized, 3 bytes) — skip
    // 0x10–0x11: reserve — skip
    c.portamento_switch = data[0x12];
    c.portamento_time = data[0x13];
    c.mono_switch = data[0x14];
    c.octave_shift = data[0x15];
    c.pitch_bend_range_up = data[0x16];
    c.pitch_bend_range_down = data[0x17];
    // 0x18: reserve — skip
    c.partial1_switch = data[0x19];
    c.partial1_select = data[0x1A];
    c.partial2_switch = data[0x1B];
    c.partial2_select = data[0x1C];
    c.partial3_switch = data[0x1D];
    c.partial3_select = data[0x1E];
    c.ring_switch = data[0x1F];
    c.tfx_switch = data[0x20];
    // 0x21–0x2D: reserve — skip
    c.unison_switch = data[0x2E];
    // 0x2F–0x30: reserve — skip
    c.portamento_mode = data[0x31];
    c.legato_switch = data[0x32];
    // 0x33: reserve — skip
    c.analog_feel = data[0x34];
    c.wave_shape = data[0x35];
    c.tone_category = data[0x36];

    // Phrase Number: nibblized 4 bytes at 0x37–0x3A
    c.phrase_number = ((data[0x37] as u16 & 0x0F) << 12)
        | ((data[0x38] as u16 & 0x0F) << 8)
        | ((data[0x39] as u16 & 0x0F) << 4)
        | (data[0x3A] as u16 & 0x0F);

    c.phrase_octave_shift = data[0x3B];
    c.unison_size = data[0x3C];
    // 0x3D–0x3F: reserve — skip

    c
}

/// Parse an SN Synth Tone Partial dump (0x3D bytes).
///
/// Offsets follow the MIDI implementation doc for each partial block.
pub fn parse_sns_partial(data: &[u8]) -> SnSynthPartial {
    let mut p = SnSynthPartial::default();
    if data.len() < SNS_PARTIAL_SIZE {
        return p;
    }

    p.osc_wave = data[0x00];
    p.osc_wave_variation = data[0x01];
    // 0x02: reserve — skip
    p.osc_pitch = data[0x03];
    p.osc_detune = data[0x04];
    p.osc_pw_mod_depth = data[0x05];
    p.osc_pulse_width = data[0x06];
    p.osc_pitch_env_attack = data[0x07];
    p.osc_pitch_env_decay = data[0x08];
    p.osc_pitch_env_depth = data[0x09];
    p.filter_mode = data[0x0A];
    p.filter_slope = data[0x0B];
    p.filter_cutoff = data[0x0C];
    p.filter_keyfollow = data[0x0D];
    p.filter_env_vel_sens = data[0x0E];
    p.filter_resonance = data[0x0F];
    p.filter_env_attack = data[0x10];
    p.filter_env_decay = data[0x11];
    p.filter_env_sustain = data[0x12];
    p.filter_env_release = data[0x13];
    p.filter_env_depth = data[0x14];
    p.amp_level = data[0x15];
    p.amp_vel_sens = data[0x16];
    p.amp_env_attack = data[0x17];
    p.amp_env_decay = data[0x18];
    p.amp_env_sustain = data[0x19];
    p.amp_env_release = data[0x1A];
    p.amp_pan = data[0x1B];
    p.lfo_shape = data[0x1C];
    p.lfo_rate = data[0x1D];
    p.lfo_tempo_sync = data[0x1E];
    p.lfo_tempo_sync_note = data[0x1F];
    p.lfo_fade_time = data[0x20];
    p.lfo_key_trigger = data[0x21];
    p.lfo_pitch_depth = data[0x22];
    p.lfo_filter_depth = data[0x23];
    p.lfo_amp_depth = data[0x24];
    p.lfo_pan_depth = data[0x25];
    p.mod_lfo_shape = data[0x26];
    p.mod_lfo_rate = data[0x27];
    p.mod_lfo_tempo_sync = data[0x28];
    p.mod_lfo_tempo_sync_note = data[0x29];
    p.pw_shift = data[0x2A];
    // 0x2B: reserve — skip
    p.mod_lfo_pitch_depth = data[0x2C];
    p.mod_lfo_filter_depth = data[0x2D];
    p.mod_lfo_amp_depth = data[0x2E];
    p.mod_lfo_pan_depth = data[0x2F];
    p.aftertouch_cutoff = data[0x30];
    p.aftertouch_level = data[0x31];
    // 0x32–0x33: reserve — skip
    p.wave_gain = data[0x34];

    // Wave Number: nibblized 4 bytes at 0x35–0x38
    p.wave_number = ((data[0x35] as u16 & 0x0F) << 12)
        | ((data[0x36] as u16 & 0x0F) << 8)
        | ((data[0x37] as u16 & 0x0F) << 4)
        | (data[0x38] as u16 & 0x0F);

    p.hpf_cutoff = data[0x39];
    p.super_saw_detune = data[0x3A];
    p.mod_lfo_rate_control = data[0x3B];
    p.amp_level_keyfollow = data[0x3C];

    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sns_common_address_part1() {
        let addr = sns_common_address(0);
        // Part 1 tone base = 19 00 00 00, + 01 00 00 = 19 01 00 00
        assert_eq!(addr, Address::new(0x19, 0x01, 0x00, 0x00));
    }

    #[test]
    fn sns_partial_address_part1() {
        let addr = sns_partial_address(0, 0);
        // 19 01 00 00 + 00 00 20 00 = 19 01 20 00
        assert_eq!(addr, Address::new(0x19, 0x01, 0x20, 0x00));
    }

    #[test]
    fn sns_partial_address_part1_partial3() {
        let addr = sns_partial_address(0, 2);
        // 19 01 00 00 + 00 00 22 00 = 19 01 22 00
        assert_eq!(addr, Address::new(0x19, 0x01, 0x22, 0x00));
    }

    #[test]
    fn sns_common_param_address_tone_level() {
        let addr = sns_common_param_address(0, 0x0C);
        assert_eq!(addr, Address::new(0x19, 0x01, 0x00, 0x0C));
    }

    #[test]
    fn sns_partial_param_address_filter_cutoff() {
        let addr = sns_partial_param_address(0, 1, 0x0C);
        // partial 1 = 19 01 21 00, + 0C = 19 01 21 0C
        assert_eq!(addr, Address::new(0x19, 0x01, 0x21, 0x0C));
    }

    #[test]
    fn parse_common_basic() {
        let mut data = [0u8; SNS_COMMON_SIZE];
        // Tone name "Test        "
        data[0x00] = b'T';
        data[0x01] = b'e';
        data[0x02] = b's';
        data[0x03] = b't';
        data[4..12].fill(b' ');
        data[0x0C] = 100; // tone level
        data[0x12] = 1; // portamento on
        data[0x15] = 65; // octave shift +1
        data[0x19] = 1; // partial1 switch on
        data[0x1F] = 2; // ring on
        data[0x36] = 42; // tone category
        // Phrase number = 1234 → nibbles: 0, 4, 13, 2
        data[0x37] = 0x00;
        data[0x38] = 0x04;
        data[0x39] = 0x0D;
        data[0x3A] = 0x02;
        data[0x3C] = 2; // unison size = 6

        let c = parse_sns_common(&data);
        assert_eq!(c.tone_name, "Test");
        assert_eq!(c.tone_level, 100);
        assert_eq!(c.portamento_switch, 1);
        assert_eq!(c.octave_shift, 65);
        assert_eq!(c.partial1_switch, 1);
        assert_eq!(c.ring_switch, 2);
        assert_eq!(c.tone_category, 42);
        assert_eq!(c.phrase_number, 0x04D2); // 1234
        assert_eq!(c.unison_size, 2);
    }

    #[test]
    fn parse_partial_basic() {
        let mut data = [0u8; SNS_PARTIAL_SIZE];
        data[0x00] = 6; // SUPER-SAW
        data[0x01] = 1; // variation B
        data[0x03] = 64; // pitch = 0
        data[0x0C] = 100; // filter cutoff
        data[0x15] = 80; // amp level
        data[0x1B] = 64; // pan center
        data[0x3A] = 50; // super saw detune

        // Wave Number = 256 → nibbles: 0, 1, 0, 0
        data[0x35] = 0x00;
        data[0x36] = 0x01;
        data[0x37] = 0x00;
        data[0x38] = 0x00;

        let p = parse_sns_partial(&data);
        assert_eq!(p.osc_wave, 6);
        assert_eq!(p.osc_wave_variation, 1);
        assert_eq!(p.osc_pitch, 64);
        assert_eq!(p.filter_cutoff, 100);
        assert_eq!(p.amp_level, 80);
        assert_eq!(p.amp_pan, 64);
        assert_eq!(p.wave_number, 256);
        assert_eq!(p.super_saw_detune, 50);
    }
}
