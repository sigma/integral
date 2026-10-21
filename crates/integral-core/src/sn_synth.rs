//! SuperNATURAL Synth Tone address map, state types, and parse functions.
//!
//! The SN-S tone occupies offset `01 00 00` within the temporary tone block.
//! It contains: Common parameters, MFX (handled by `mfx.rs`), and up to
//! three Partial blocks.

use crate::address::{Address, DataSize};
use crate::block_field::BlockField;
use crate::params;
use crate::parse_helpers::{encode_ascii_name, parse_ascii_name};

// ---------------------------------------------------------------------------
// Address constants
// ---------------------------------------------------------------------------

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

/// Number of Partials in an SN-S Tone.
pub const SNS_PARTIAL_COUNT: usize = SNS_PARTIAL_OFFSETS.len();

/// [`SnSynthPartial::osc_wave`] value selecting a PCM Wave
/// ([`SnSynthPartial::wave_number`] in the SN Synth Wave list).
pub const OSC_WAVE_PCM: u8 = 7;

/// [`SnSynthPartial::osc_wave`] value selecting the Super Saw
/// ([`SnSynthPartial::super_saw_detune`] sets its detune).
pub const OSC_WAVE_SUPER_SAW: u8 = 6;

/// Compute the absolute SN-S base address for a part.
const fn sns_base(part_index: u8) -> Address {
    params::ToneType::SnSynth.base_address(part_index)
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

/// A field of an SN Synth Tone Common block. The discriminant is the
/// field's first byte position within the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SnsCommonField {
    /// Tone Level.
    ToneLevel = 0x0C,
    /// Portamento Switch.
    PortamentoSwitch = 0x12,
    /// Portamento Time.
    PortamentoTime = 0x13,
    /// Mono Switch.
    MonoSwitch = 0x14,
    /// Octave Shift.
    OctaveShift = 0x15,
    /// Pitch Bend Range Up.
    PitchBendRangeUp = 0x16,
    /// Pitch Bend Range Down.
    PitchBendRangeDown = 0x17,
    /// Partial1 Switch.
    Partial1Switch = 0x19,
    /// Partial1 Select.
    Partial1Select = 0x1A,
    /// Partial2 Switch.
    Partial2Switch = 0x1B,
    /// Partial2 Select.
    Partial2Select = 0x1C,
    /// Partial3 Switch.
    Partial3Switch = 0x1D,
    /// Partial3 Select.
    Partial3Select = 0x1E,
    /// RING Switch.
    RingSwitch = 0x1F,
    /// TFX Switch.
    TfxSwitch = 0x20,
    /// Unison Switch.
    UnisonSwitch = 0x2E,
    /// Portamento Mode.
    PortamentoMode = 0x31,
    /// Legato Switch.
    LegatoSwitch = 0x32,
    /// Analog Feel.
    AnalogFeel = 0x34,
    /// Wave Shape.
    WaveShape = 0x35,
    /// Tone Category.
    ToneCategory = 0x36,
    /// Phrase Number (nibblized, 4 bytes).
    PhraseNumber = 0x37,
    /// Phrase Octave Shift.
    PhraseOctaveShift = 0x3B,
    /// Unison Size.
    UnisonSize = 0x3C,
}

impl BlockField for SnsCommonField {
    fn position(self) -> usize {
        self as usize
    }

    fn width(self) -> usize {
        match self {
            Self::PhraseNumber => 4,
            _ => 1,
        }
    }
}

impl SnsCommonField {
    /// The Partial1–3 Switch fields, by Partial index.
    pub const PARTIAL_SWITCHES: [Self; SNS_PARTIAL_COUNT] = [
        Self::Partial1Switch,
        Self::Partial2Switch,
        Self::Partial3Switch,
    ];
}

/// A field of an SN Synth Tone Partial block. The discriminant is the
/// field's first byte position within the block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SnsPartialField {
    /// OSC Wave.
    OscWave = 0x00,
    /// OSC Wave Variation.
    OscWaveVariation = 0x01,
    /// OSC Pitch.
    OscPitch = 0x03,
    /// OSC Detune.
    OscDetune = 0x04,
    /// OSC Pulse Width Mod Depth.
    OscPwModDepth = 0x05,
    /// OSC Pulse Width.
    OscPulseWidth = 0x06,
    /// OSC Pitch Env Attack Time.
    OscPitchEnvAttack = 0x07,
    /// OSC Pitch Env Decay.
    OscPitchEnvDecay = 0x08,
    /// OSC Pitch Env Depth.
    OscPitchEnvDepth = 0x09,
    /// FILTER Mode.
    FilterMode = 0x0A,
    /// FILTER Slope.
    FilterSlope = 0x0B,
    /// FILTER Cutoff.
    FilterCutoff = 0x0C,
    /// FILTER Cutoff Keyfollow.
    FilterKeyfollow = 0x0D,
    /// FILTER Env Velocity Sens.
    FilterEnvVelSens = 0x0E,
    /// FILTER Resonance.
    FilterResonance = 0x0F,
    /// FILTER Env Attack Time.
    FilterEnvAttack = 0x10,
    /// FILTER Env Decay Time.
    FilterEnvDecay = 0x11,
    /// FILTER Env Sustain Level.
    FilterEnvSustain = 0x12,
    /// FILTER Env Release Time.
    FilterEnvRelease = 0x13,
    /// FILTER Env Depth.
    FilterEnvDepth = 0x14,
    /// AMP Level.
    AmpLevel = 0x15,
    /// AMP Level Velocity Sens.
    AmpVelSens = 0x16,
    /// AMP Env Attack Time.
    AmpEnvAttack = 0x17,
    /// AMP Env Decay Time.
    AmpEnvDecay = 0x18,
    /// AMP Env Sustain Level.
    AmpEnvSustain = 0x19,
    /// AMP Env Release Time.
    AmpEnvRelease = 0x1A,
    /// AMP Pan.
    AmpPan = 0x1B,
    /// LFO Shape.
    LfoShape = 0x1C,
    /// LFO Rate.
    LfoRate = 0x1D,
    /// LFO Tempo Sync Switch.
    LfoTempoSync = 0x1E,
    /// LFO Tempo Sync Note.
    LfoTempoSyncNote = 0x1F,
    /// LFO Fade Time.
    LfoFadeTime = 0x20,
    /// LFO Key Trigger.
    LfoKeyTrigger = 0x21,
    /// LFO Pitch Depth.
    LfoPitchDepth = 0x22,
    /// LFO Filter Depth.
    LfoFilterDepth = 0x23,
    /// LFO Amp Depth.
    LfoAmpDepth = 0x24,
    /// LFO Pan Depth.
    LfoPanDepth = 0x25,
    /// Modulation LFO Shape.
    ModLfoShape = 0x26,
    /// Modulation LFO Rate.
    ModLfoRate = 0x27,
    /// Modulation LFO Tempo Sync Switch.
    ModLfoTempoSync = 0x28,
    /// Modulation LFO Tempo Sync Note.
    ModLfoTempoSyncNote = 0x29,
    /// OSC Pulse Width Shift.
    PwShift = 0x2A,
    /// Modulation LFO Pitch Depth.
    ModLfoPitchDepth = 0x2C,
    /// Modulation LFO Filter Depth.
    ModLfoFilterDepth = 0x2D,
    /// Modulation LFO Amp Depth.
    ModLfoAmpDepth = 0x2E,
    /// Modulation LFO Pan Depth.
    ModLfoPanDepth = 0x2F,
    /// Cutoff Aftertouch Sens.
    AftertouchCutoff = 0x30,
    /// Level Aftertouch Sens.
    AftertouchLevel = 0x31,
    /// Wave Gain.
    WaveGain = 0x34,
    /// Wave Number (nibblized, 4 bytes).
    WaveNumber = 0x35,
    /// HPF Cutoff.
    HpfCutoff = 0x39,
    /// Super Saw Detune.
    SuperSawDetune = 0x3A,
    /// Modulation LFO Rate Control.
    ModLfoRateControl = 0x3B,
    /// AMP Level Keyfollow.
    AmpLevelKeyfollow = 0x3C,
}

impl BlockField for SnsPartialField {
    fn position(self) -> usize {
        self as usize
    }

    fn width(self) -> usize {
        match self {
            Self::WaveNumber => 4,
            _ => 1,
        }
    }
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
            tone_level: 100,
            portamento_switch: 0,
            portamento_time: 20,
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
            tfx_switch: 1,
            unison_switch: 0,
            portamento_mode: 1,
            legato_switch: 1,
            analog_feel: 0,
            wave_shape: 0,
            tone_category: 0,
            phrase_number: 0,
            phrase_octave_shift: 64, // 0
            unison_size: 3,
        }
    }
}

/// SN Synth Tone Partial parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnSynthPartial {
    /// OSC Wave (0–7: SAW, SQR, PW-SQR, TRI, SINE, NOISE, SUPER-SAW,
    /// [`OSC_WAVE_PCM`]).
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
            filter_slope: 1,
            filter_cutoff: 127,
            filter_keyfollow: 64, // 0
            filter_env_vel_sens: 64,
            filter_resonance: 0,
            filter_env_attack: 0,
            filter_env_decay: 36,
            filter_env_sustain: 0,
            filter_env_release: 0,
            filter_env_depth: 64,
            amp_level: 100,
            amp_vel_sens: 83,
            amp_env_attack: 0,
            amp_env_decay: 0,
            amp_env_sustain: 127,
            amp_env_release: 0,
            amp_pan: 64,
            lfo_shape: 0,
            lfo_rate: 81,
            lfo_tempo_sync: 0,
            lfo_tempo_sync_note: 17,
            lfo_fade_time: 0,
            lfo_key_trigger: 0,
            lfo_pitch_depth: 64,
            lfo_filter_depth: 64,
            lfo_amp_depth: 64,
            lfo_pan_depth: 64,
            mod_lfo_shape: 0,
            mod_lfo_rate: 88,
            mod_lfo_tempo_sync: 0,
            mod_lfo_tempo_sync_note: 17,
            pw_shift: 127, // the full PW range; 0 holds PW-SQR at a square
            mod_lfo_pitch_depth: 80,
            mod_lfo_filter_depth: 64,
            mod_lfo_amp_depth: 64,
            mod_lfo_pan_depth: 64,
            aftertouch_cutoff: 73,
            aftertouch_level: 74,
            wave_gain: 1, // 0 dB
            wave_number: 45,
            hpf_cutoff: 0,
            super_saw_detune: 0,
            mod_lfo_rate_control: 82,
            amp_level_keyfollow: 64,
        }
    }
}

/// An SN Synth Tone's Common and Partial parameters (its MFX is read apart).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SnSynthTone {
    /// Common parameters.
    pub common: SnSynthCommon,
    /// Per-partial parameters.
    pub partials: [SnSynthPartial; SNS_PARTIAL_COUNT],
}

impl Default for SnSynthTone {
    /// INIT TONE: Partials 1–3 play SAW, SQR and PCM.
    fn default() -> Self {
        Self {
            common: SnSynthCommon::default(),
            partials: [0, 1, 7].map(|osc_wave| SnSynthPartial {
                osc_wave,
                ..SnSynthPartial::default()
            }),
        }
    }
}

/// A block of an SN Synth Tone read with one RQ1: its Common or a Partial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnsBlock {
    /// The Common block.
    Common,
    /// The Partial block with this index (0–2).
    Partial(u8),
}

impl SnsBlock {
    /// Every block of a tone, Common first.
    pub const ALL: [SnsBlock; 1 + SNS_PARTIAL_COUNT] = [
        SnsBlock::Common,
        SnsBlock::Partial(0),
        SnsBlock::Partial(1),
        SnsBlock::Partial(2),
    ];

    /// Position in [`ALL`](Self::ALL).
    pub(crate) const fn index(self) -> usize {
        match self {
            SnsBlock::Common => 0,
            SnsBlock::Partial(i) => 1 + i as usize,
        }
    }

    /// The block's absolute address for a part.
    pub const fn address(self, part_index: u8) -> Address {
        match self {
            SnsBlock::Common => sns_common_address(part_index),
            SnsBlock::Partial(i) => sns_partial_address(part_index, i),
        }
    }

    /// The RQ1 size that reads the whole block.
    pub const fn size(self) -> DataSize {
        match self {
            SnsBlock::Common => SNS_COMMON_BLOCK_SIZE,
            SnsBlock::Partial(_) => SNS_PARTIAL_BLOCK_SIZE,
        }
    }

    /// The block's length in bytes.
    const fn len(self) -> usize {
        match self {
            SnsBlock::Common => SNS_COMMON_SIZE,
            SnsBlock::Partial(_) => SNS_PARTIAL_SIZE,
        }
    }
}

/// One part's SN Synth Tone blocks, as read from the device and patched by
/// edits.
#[derive(Debug, Clone, Default)]
pub struct SnSynthToneBlocks {
    blocks: [Option<Vec<u8>>; 1 + SNS_PARTIAL_COUNT],
}

impl SnSynthToneBlocks {
    /// Store `data` as `block` if it is the whole block; whether it was.
    pub fn store(&mut self, block: SnsBlock, data: &[u8]) -> bool {
        if data.len() != block.len() {
            return false;
        }
        self.blocks[block.index()] = Some(data.to_vec());
        true
    }

    /// Whether `block` has arrived.
    pub fn has(&self, block: SnsBlock) -> bool {
        self.blocks[block.index()].is_some()
    }

    /// Overwrite `bytes` at `offset` in `block`, if it has arrived.
    pub fn patch(&mut self, block: SnsBlock, offset: u8, bytes: &[u8]) {
        let offset = offset as usize;
        if let Some(dst) = self.blocks[block.index()]
            .as_mut()
            .and_then(|b| b.get_mut(offset..offset + bytes.len()))
        {
            dst.copy_from_slice(bytes);
        }
    }

    /// The parsed tone, once every block has arrived.
    pub fn tone(&self) -> Option<SnSynthTone> {
        let [common, partials @ ..] = &self.blocks;
        let mut tone = SnSynthTone {
            common: parse_sns_common(common.as_deref()?).ok()?,
            ..SnSynthTone::default()
        };
        for (parsed, block) in tone.partials.iter_mut().zip(partials) {
            *parsed = parse_sns_partial(block.as_deref()?).ok()?;
        }
        Some(tone)
    }
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
#[allow(clippy::field_reassign_with_default)]
pub fn parse_sns_common(data: &[u8]) -> Result<SnSynthCommon, crate::ToneParseError> {
    if data.len() < SNS_COMMON_SIZE {
        return Err(crate::ToneParseError {
            expected: SNS_COMMON_SIZE,
            got: data.len(),
        });
    }
    let mut c = SnSynthCommon::default();

    use SnsCommonField as F;
    c.tone_name = parse_ascii_name(data);
    c.tone_level = F::ToneLevel.byte(data);
    c.portamento_switch = F::PortamentoSwitch.byte(data);
    c.portamento_time = F::PortamentoTime.byte(data);
    c.mono_switch = F::MonoSwitch.byte(data);
    c.octave_shift = F::OctaveShift.byte(data);
    c.pitch_bend_range_up = F::PitchBendRangeUp.byte(data);
    c.pitch_bend_range_down = F::PitchBendRangeDown.byte(data);
    c.partial1_switch = F::Partial1Switch.byte(data);
    c.partial1_select = F::Partial1Select.byte(data);
    c.partial2_switch = F::Partial2Switch.byte(data);
    c.partial2_select = F::Partial2Select.byte(data);
    c.partial3_switch = F::Partial3Switch.byte(data);
    c.partial3_select = F::Partial3Select.byte(data);
    c.ring_switch = F::RingSwitch.byte(data);
    c.tfx_switch = F::TfxSwitch.byte(data);
    c.unison_switch = F::UnisonSwitch.byte(data);
    c.portamento_mode = F::PortamentoMode.byte(data);
    c.legato_switch = F::LegatoSwitch.byte(data);
    c.analog_feel = F::AnalogFeel.byte(data);
    c.wave_shape = F::WaveShape.byte(data);
    c.tone_category = F::ToneCategory.byte(data);
    c.phrase_number = F::PhraseNumber.nibbles(data);
    c.phrase_octave_shift = F::PhraseOctaveShift.byte(data);
    c.unison_size = F::UnisonSize.byte(data);

    Ok(c)
}

/// Parse an SN Synth Tone Partial dump (0x3D bytes).
///
/// Offsets follow the MIDI implementation doc for each partial block.
#[allow(clippy::field_reassign_with_default)]
pub fn parse_sns_partial(data: &[u8]) -> Result<SnSynthPartial, crate::ToneParseError> {
    if data.len() < SNS_PARTIAL_SIZE {
        return Err(crate::ToneParseError {
            expected: SNS_PARTIAL_SIZE,
            got: data.len(),
        });
    }
    let mut p = SnSynthPartial::default();

    use SnsPartialField as F;
    p.osc_wave = F::OscWave.byte(data);
    p.osc_wave_variation = F::OscWaveVariation.byte(data);
    p.osc_pitch = F::OscPitch.byte(data);
    p.osc_detune = F::OscDetune.byte(data);
    p.osc_pw_mod_depth = F::OscPwModDepth.byte(data);
    p.osc_pulse_width = F::OscPulseWidth.byte(data);
    p.osc_pitch_env_attack = F::OscPitchEnvAttack.byte(data);
    p.osc_pitch_env_decay = F::OscPitchEnvDecay.byte(data);
    p.osc_pitch_env_depth = F::OscPitchEnvDepth.byte(data);
    p.filter_mode = F::FilterMode.byte(data);
    p.filter_slope = F::FilterSlope.byte(data);
    p.filter_cutoff = F::FilterCutoff.byte(data);
    p.filter_keyfollow = F::FilterKeyfollow.byte(data);
    p.filter_env_vel_sens = F::FilterEnvVelSens.byte(data);
    p.filter_resonance = F::FilterResonance.byte(data);
    p.filter_env_attack = F::FilterEnvAttack.byte(data);
    p.filter_env_decay = F::FilterEnvDecay.byte(data);
    p.filter_env_sustain = F::FilterEnvSustain.byte(data);
    p.filter_env_release = F::FilterEnvRelease.byte(data);
    p.filter_env_depth = F::FilterEnvDepth.byte(data);
    p.amp_level = F::AmpLevel.byte(data);
    p.amp_vel_sens = F::AmpVelSens.byte(data);
    p.amp_env_attack = F::AmpEnvAttack.byte(data);
    p.amp_env_decay = F::AmpEnvDecay.byte(data);
    p.amp_env_sustain = F::AmpEnvSustain.byte(data);
    p.amp_env_release = F::AmpEnvRelease.byte(data);
    p.amp_pan = F::AmpPan.byte(data);
    p.lfo_shape = F::LfoShape.byte(data);
    p.lfo_rate = F::LfoRate.byte(data);
    p.lfo_tempo_sync = F::LfoTempoSync.byte(data);
    p.lfo_tempo_sync_note = F::LfoTempoSyncNote.byte(data);
    p.lfo_fade_time = F::LfoFadeTime.byte(data);
    p.lfo_key_trigger = F::LfoKeyTrigger.byte(data);
    p.lfo_pitch_depth = F::LfoPitchDepth.byte(data);
    p.lfo_filter_depth = F::LfoFilterDepth.byte(data);
    p.lfo_amp_depth = F::LfoAmpDepth.byte(data);
    p.lfo_pan_depth = F::LfoPanDepth.byte(data);
    p.mod_lfo_shape = F::ModLfoShape.byte(data);
    p.mod_lfo_rate = F::ModLfoRate.byte(data);
    p.mod_lfo_tempo_sync = F::ModLfoTempoSync.byte(data);
    p.mod_lfo_tempo_sync_note = F::ModLfoTempoSyncNote.byte(data);
    p.pw_shift = F::PwShift.byte(data);
    p.mod_lfo_pitch_depth = F::ModLfoPitchDepth.byte(data);
    p.mod_lfo_filter_depth = F::ModLfoFilterDepth.byte(data);
    p.mod_lfo_amp_depth = F::ModLfoAmpDepth.byte(data);
    p.mod_lfo_pan_depth = F::ModLfoPanDepth.byte(data);
    p.aftertouch_cutoff = F::AftertouchCutoff.byte(data);
    p.aftertouch_level = F::AftertouchLevel.byte(data);
    p.wave_gain = F::WaveGain.byte(data);
    p.wave_number = F::WaveNumber.nibbles(data);
    p.hpf_cutoff = F::HpfCutoff.byte(data);
    p.super_saw_detune = F::SuperSawDetune.byte(data);
    p.mod_lfo_rate_control = F::ModLfoRateControl.byte(data);
    p.amp_level_keyfollow = F::AmpLevelKeyfollow.byte(data);

    Ok(p)
}

/// Encode an SN Synth Tone Common block (0x40 bytes), the inverse of
/// [`parse_sns_common`]. The reserves get 0.
pub fn encode_sns_common(c: &SnSynthCommon) -> Vec<u8> {
    let mut data = encode_ascii_name(&c.tone_name, SnsCommonField::ToneLevel.position());
    data.resize(SNS_COMMON_SIZE, 0);
    use SnsCommonField as F;
    F::ToneLevel.set_byte(&mut data, c.tone_level);
    F::PortamentoSwitch.set_byte(&mut data, c.portamento_switch);
    F::PortamentoTime.set_byte(&mut data, c.portamento_time);
    F::MonoSwitch.set_byte(&mut data, c.mono_switch);
    F::OctaveShift.set_byte(&mut data, c.octave_shift);
    F::PitchBendRangeUp.set_byte(&mut data, c.pitch_bend_range_up);
    F::PitchBendRangeDown.set_byte(&mut data, c.pitch_bend_range_down);
    F::Partial1Switch.set_byte(&mut data, c.partial1_switch);
    F::Partial1Select.set_byte(&mut data, c.partial1_select);
    F::Partial2Switch.set_byte(&mut data, c.partial2_switch);
    F::Partial2Select.set_byte(&mut data, c.partial2_select);
    F::Partial3Switch.set_byte(&mut data, c.partial3_switch);
    F::Partial3Select.set_byte(&mut data, c.partial3_select);
    F::RingSwitch.set_byte(&mut data, c.ring_switch);
    F::TfxSwitch.set_byte(&mut data, c.tfx_switch);
    F::UnisonSwitch.set_byte(&mut data, c.unison_switch);
    F::PortamentoMode.set_byte(&mut data, c.portamento_mode);
    F::LegatoSwitch.set_byte(&mut data, c.legato_switch);
    F::AnalogFeel.set_byte(&mut data, c.analog_feel);
    F::WaveShape.set_byte(&mut data, c.wave_shape);
    F::ToneCategory.set_byte(&mut data, c.tone_category);
    F::PhraseNumber.set_nibbles(&mut data, c.phrase_number);
    F::PhraseOctaveShift.set_byte(&mut data, c.phrase_octave_shift);
    F::UnisonSize.set_byte(&mut data, c.unison_size);
    data
}

/// Encode an SN Synth Tone Partial block (0x3D bytes), the inverse of
/// [`parse_sns_partial`]. The reserves get 0.
pub fn encode_sns_partial(p: &SnSynthPartial) -> Vec<u8> {
    let mut data = vec![0; SNS_PARTIAL_SIZE];
    use SnsPartialField as F;
    F::OscWave.set_byte(&mut data, p.osc_wave);
    F::OscWaveVariation.set_byte(&mut data, p.osc_wave_variation);
    F::OscPitch.set_byte(&mut data, p.osc_pitch);
    F::OscDetune.set_byte(&mut data, p.osc_detune);
    F::OscPwModDepth.set_byte(&mut data, p.osc_pw_mod_depth);
    F::OscPulseWidth.set_byte(&mut data, p.osc_pulse_width);
    F::OscPitchEnvAttack.set_byte(&mut data, p.osc_pitch_env_attack);
    F::OscPitchEnvDecay.set_byte(&mut data, p.osc_pitch_env_decay);
    F::OscPitchEnvDepth.set_byte(&mut data, p.osc_pitch_env_depth);
    F::FilterMode.set_byte(&mut data, p.filter_mode);
    F::FilterSlope.set_byte(&mut data, p.filter_slope);
    F::FilterCutoff.set_byte(&mut data, p.filter_cutoff);
    F::FilterKeyfollow.set_byte(&mut data, p.filter_keyfollow);
    F::FilterEnvVelSens.set_byte(&mut data, p.filter_env_vel_sens);
    F::FilterResonance.set_byte(&mut data, p.filter_resonance);
    F::FilterEnvAttack.set_byte(&mut data, p.filter_env_attack);
    F::FilterEnvDecay.set_byte(&mut data, p.filter_env_decay);
    F::FilterEnvSustain.set_byte(&mut data, p.filter_env_sustain);
    F::FilterEnvRelease.set_byte(&mut data, p.filter_env_release);
    F::FilterEnvDepth.set_byte(&mut data, p.filter_env_depth);
    F::AmpLevel.set_byte(&mut data, p.amp_level);
    F::AmpVelSens.set_byte(&mut data, p.amp_vel_sens);
    F::AmpEnvAttack.set_byte(&mut data, p.amp_env_attack);
    F::AmpEnvDecay.set_byte(&mut data, p.amp_env_decay);
    F::AmpEnvSustain.set_byte(&mut data, p.amp_env_sustain);
    F::AmpEnvRelease.set_byte(&mut data, p.amp_env_release);
    F::AmpPan.set_byte(&mut data, p.amp_pan);
    F::LfoShape.set_byte(&mut data, p.lfo_shape);
    F::LfoRate.set_byte(&mut data, p.lfo_rate);
    F::LfoTempoSync.set_byte(&mut data, p.lfo_tempo_sync);
    F::LfoTempoSyncNote.set_byte(&mut data, p.lfo_tempo_sync_note);
    F::LfoFadeTime.set_byte(&mut data, p.lfo_fade_time);
    F::LfoKeyTrigger.set_byte(&mut data, p.lfo_key_trigger);
    F::LfoPitchDepth.set_byte(&mut data, p.lfo_pitch_depth);
    F::LfoFilterDepth.set_byte(&mut data, p.lfo_filter_depth);
    F::LfoAmpDepth.set_byte(&mut data, p.lfo_amp_depth);
    F::LfoPanDepth.set_byte(&mut data, p.lfo_pan_depth);
    F::ModLfoShape.set_byte(&mut data, p.mod_lfo_shape);
    F::ModLfoRate.set_byte(&mut data, p.mod_lfo_rate);
    F::ModLfoTempoSync.set_byte(&mut data, p.mod_lfo_tempo_sync);
    F::ModLfoTempoSyncNote.set_byte(&mut data, p.mod_lfo_tempo_sync_note);
    F::PwShift.set_byte(&mut data, p.pw_shift);
    F::ModLfoPitchDepth.set_byte(&mut data, p.mod_lfo_pitch_depth);
    F::ModLfoFilterDepth.set_byte(&mut data, p.mod_lfo_filter_depth);
    F::ModLfoAmpDepth.set_byte(&mut data, p.mod_lfo_amp_depth);
    F::ModLfoPanDepth.set_byte(&mut data, p.mod_lfo_pan_depth);
    F::AftertouchCutoff.set_byte(&mut data, p.aftertouch_cutoff);
    F::AftertouchLevel.set_byte(&mut data, p.aftertouch_level);
    F::WaveGain.set_byte(&mut data, p.wave_gain);
    F::WaveNumber.set_nibbles(&mut data, p.wave_number);
    F::HpfCutoff.set_byte(&mut data, p.hpf_cutoff);
    F::SuperSawDetune.set_byte(&mut data, p.super_saw_detune);
    F::ModLfoRateControl.set_byte(&mut data, p.mod_lfo_rate_control);
    F::AmpLevelKeyfollow.set_byte(&mut data, p.amp_level_keyfollow);
    data
}

impl SnSynthCommon {
    /// The raw value of `field`, as the device holds it.
    pub fn value(&self, field: SnsCommonField) -> u16 {
        field.value(&encode_sns_common(self))
    }
}

impl SnSynthPartial {
    /// The raw value of `field`, as the device holds it.
    pub fn value(&self, field: SnsPartialField) -> u16 {
        field.value(&encode_sns_partial(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A field's value is what the parsed block holds at its position.
    #[test]
    fn value_reads_the_field_from_the_state() {
        let mut data = [0u8; SNS_PARTIAL_SIZE];
        data[SnsPartialField::FilterCutoff.position()] = 93;
        data[SnsPartialField::WaveNumber.position()..][..4].copy_from_slice(&[0, 1, 2, 3]);
        let partial = parse_sns_partial(&data).unwrap();
        assert_eq!(partial.value(SnsPartialField::FilterCutoff), 93);
        assert_eq!(partial.value(SnsPartialField::WaveNumber), 0x123);
        assert_eq!(partial.value(SnsPartialField::OscWave), 0);

        let common = SnSynthCommon {
            tone_level: 101,
            phrase_number: 0x1234,
            ..SnSynthCommon::default()
        };
        assert_eq!(common.value(SnsCommonField::ToneLevel), 101);
        assert_eq!(common.value(SnsCommonField::PhraseNumber), 0x1234);
    }

    /// The INIT TONE blocks, read from an unwritten User slot (`53 7F xx 00`)
    /// on the device.
    #[test]
    fn default_matches_the_device_init_tone() {
        let partials = [
            include_bytes!("../tests/fixtures/init/sn-synth-partial-1.bin"),
            include_bytes!("../tests/fixtures/init/sn-synth-partial-2.bin"),
            include_bytes!("../tests/fixtures/init/sn-synth-partial-3.bin"),
        ];
        let init = SnSynthTone {
            common: parse_sns_common(include_bytes!("../tests/fixtures/init/sn-synth-common.bin"))
                .unwrap(),
            partials: partials.map(|p| parse_sns_partial(p).unwrap()),
        };
        let mut ours = SnSynthTone::default();
        ours.common.tone_name = "INIT TONE".into();
        assert_eq!(init, ours);
    }

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

        let c = parse_sns_common(&data).unwrap();
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

        let p = parse_sns_partial(&data).unwrap();
        assert_eq!(p.osc_wave, 6);
        assert_eq!(p.osc_wave_variation, 1);
        assert_eq!(p.osc_pitch, 64);
        assert_eq!(p.filter_cutoff, 100);
        assert_eq!(p.amp_level, 80);
        assert_eq!(p.amp_pan, 64);
        assert_eq!(p.wave_number, 256);
        assert_eq!(p.super_saw_detune, 50);
    }

    #[test]
    fn parse_common_reads_every_field_at_its_position() {
        use crate::block_field::test_support::{assert_reads_at_positions, positional_block};
        use SnsCommonField as F;
        let c = parse_sns_common(&positional_block(SNS_COMMON_SIZE)).unwrap();
        assert_reads_at_positions!(c, {
            tone_level: F::ToneLevel,
            portamento_switch: F::PortamentoSwitch,
            portamento_time: F::PortamentoTime,
            mono_switch: F::MonoSwitch,
            octave_shift: F::OctaveShift,
            pitch_bend_range_up: F::PitchBendRangeUp,
            pitch_bend_range_down: F::PitchBendRangeDown,
            partial1_switch: F::Partial1Switch,
            partial1_select: F::Partial1Select,
            partial2_switch: F::Partial2Switch,
            partial2_select: F::Partial2Select,
            partial3_switch: F::Partial3Switch,
            partial3_select: F::Partial3Select,
            ring_switch: F::RingSwitch,
            tfx_switch: F::TfxSwitch,
            unison_switch: F::UnisonSwitch,
            portamento_mode: F::PortamentoMode,
            legato_switch: F::LegatoSwitch,
            analog_feel: F::AnalogFeel,
            wave_shape: F::WaveShape,
            tone_category: F::ToneCategory,
            phrase_number: F::PhraseNumber,
            phrase_octave_shift: F::PhraseOctaveShift,
            unison_size: F::UnisonSize,
        });
    }

    #[test]
    fn parse_partial_reads_every_field_at_its_position() {
        use crate::block_field::test_support::{assert_reads_at_positions, positional_block};
        use SnsPartialField as F;
        let p = parse_sns_partial(&positional_block(SNS_PARTIAL_SIZE)).unwrap();
        assert_reads_at_positions!(p, {
            osc_wave: F::OscWave,
            osc_wave_variation: F::OscWaveVariation,
            osc_pitch: F::OscPitch,
            osc_detune: F::OscDetune,
            osc_pw_mod_depth: F::OscPwModDepth,
            osc_pulse_width: F::OscPulseWidth,
            osc_pitch_env_attack: F::OscPitchEnvAttack,
            osc_pitch_env_decay: F::OscPitchEnvDecay,
            osc_pitch_env_depth: F::OscPitchEnvDepth,
            filter_mode: F::FilterMode,
            filter_slope: F::FilterSlope,
            filter_cutoff: F::FilterCutoff,
            filter_keyfollow: F::FilterKeyfollow,
            filter_env_vel_sens: F::FilterEnvVelSens,
            filter_resonance: F::FilterResonance,
            filter_env_attack: F::FilterEnvAttack,
            filter_env_decay: F::FilterEnvDecay,
            filter_env_sustain: F::FilterEnvSustain,
            filter_env_release: F::FilterEnvRelease,
            filter_env_depth: F::FilterEnvDepth,
            amp_level: F::AmpLevel,
            amp_vel_sens: F::AmpVelSens,
            amp_env_attack: F::AmpEnvAttack,
            amp_env_decay: F::AmpEnvDecay,
            amp_env_sustain: F::AmpEnvSustain,
            amp_env_release: F::AmpEnvRelease,
            amp_pan: F::AmpPan,
            lfo_shape: F::LfoShape,
            lfo_rate: F::LfoRate,
            lfo_tempo_sync: F::LfoTempoSync,
            lfo_tempo_sync_note: F::LfoTempoSyncNote,
            lfo_fade_time: F::LfoFadeTime,
            lfo_key_trigger: F::LfoKeyTrigger,
            lfo_pitch_depth: F::LfoPitchDepth,
            lfo_filter_depth: F::LfoFilterDepth,
            lfo_amp_depth: F::LfoAmpDepth,
            lfo_pan_depth: F::LfoPanDepth,
            mod_lfo_shape: F::ModLfoShape,
            mod_lfo_rate: F::ModLfoRate,
            mod_lfo_tempo_sync: F::ModLfoTempoSync,
            mod_lfo_tempo_sync_note: F::ModLfoTempoSyncNote,
            pw_shift: F::PwShift,
            mod_lfo_pitch_depth: F::ModLfoPitchDepth,
            mod_lfo_filter_depth: F::ModLfoFilterDepth,
            mod_lfo_amp_depth: F::ModLfoAmpDepth,
            mod_lfo_pan_depth: F::ModLfoPanDepth,
            aftertouch_cutoff: F::AftertouchCutoff,
            aftertouch_level: F::AftertouchLevel,
            wave_gain: F::WaveGain,
            wave_number: F::WaveNumber,
            hpf_cutoff: F::HpfCutoff,
            super_saw_detune: F::SuperSawDetune,
            mod_lfo_rate_control: F::ModLfoRateControl,
            amp_level_keyfollow: F::AmpLevelKeyfollow,
        });
    }

    #[test]
    fn encoders_invert_the_parsers() {
        use crate::block_field::test_support::positional_block;
        let mut c = parse_sns_common(&positional_block(SNS_COMMON_SIZE)).unwrap();
        c.tone_name = "INIT TONE".into();
        let block = encode_sns_common(&c);
        assert_eq!(block.len(), SNS_COMMON_SIZE);
        assert_eq!(parse_sns_common(&block).unwrap(), c);

        // Shifted by one, so the field at position 0 (OSC Wave) is not 0.
        let shifted: Vec<u8> = positional_block(SNS_PARTIAL_SIZE)
            .iter()
            .map(|b| b + 1)
            .collect();
        let p = parse_sns_partial(&shifted).unwrap();
        let block = encode_sns_partial(&p);
        assert_eq!(block.len(), SNS_PARTIAL_SIZE);
        assert_eq!(parse_sns_partial(&block).unwrap(), p);
    }
}
