//! Range, default and display format of each tone block field.
//!
//! Each engine's spec function matches its field enum exhaustively, so a
//! field added to the enum fails the build until it is specified. Ranges
//! and formats follow the MIDI Implementation (`docs/midi/`); defaults are
//! the field's value in the engine's `Default` block, which the codebase
//! treats as INIT (not yet checked against the device: #250).

use std::sync::OnceLock;

use crate::block_field::BlockField;
use crate::fx_params::{NOTE_NAMES, SYNC_HZ, SYNC_MS};
use crate::param_registry::{NOTE_FROM, ParamFormat};
use crate::pcm_drum::{self, PcmdCommonField, PcmdPartialField, PcmdWmtField};
use crate::pcm_synth::{self, PcmsCommon2Field, PcmsCommonField, PcmsPartialField, PcmsPmtField};
use crate::sn_acoustic::{self, SnaCommonField};
use crate::sn_drum::{self, SND_INST_NUMBER_MAX, SndCommonField, SndNoteField};
use crate::sn_synth::{self, SnsCommonField, SnsPartialField};

/// Range, default and display format of a tone block field, looked up by
/// the field itself rather than by a string id. A field holding several
/// values (an envelope's times) has one spec for all of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
    /// Minimum raw value (inclusive).
    pub min: i32,
    /// Maximum raw value (inclusive).
    pub max: i32,
    /// Default raw value.
    pub default: i32,
    /// Display format.
    pub format: ParamFormat,
}

/// Range and format of a field; [`FieldSpec`] adds its default.
struct Shape {
    min: i32,
    max: i32,
    format: ParamFormat,
}

/// A [`Shape`] from `min..=max` shown as `format`.
const fn shape(min: i32, max: i32, format: ParamFormat) -> Shape {
    Shape { min, max, format }
}

use ParamFormat::{Named, Note, OnOff, Pan, Raw, SignedCenter, TimeOrNote};

const RAW_127: Shape = shape(0, 127, Raw);
const SIGNED_63: Shape = shape(1, 127, SignedCenter(64));
/// Keyfollow, -100 to +100 in steps of 10.
const KEYFOLLOW_100: Shape = shape(
    54,
    74,
    ParamFormat::SignedStep {
        center: 64,
        step: 10,
    },
);
/// Pitch and cutoff keyfollow, -200 to +200 in steps of 10.
const KEYFOLLOW_200: Shape = shape(
    44,
    84,
    ParamFormat::SignedStep {
        center: 64,
        step: 10,
    },
);
const ON_OFF: Shape = shape(0, 1, OnOff);
const COARSE_TUNE: Shape = shape(16, 112, SignedCenter(64));
const FINE_TUNE: Shape = shape(14, 114, SignedCenter(64));
const OCTAVE_SHIFT: Shape = shape(61, 67, SignedCenter(64));
const VELOCITY_CURVE: Shape = shape(0, 7, Named(&["FIXED", "1", "2", "3", "4", "5", "6", "7"]));
const FADE_MODE: Shape = shape(0, 3, Named(&["ON-IN", "ON-OUT", "OFF-IN", "OFF-OUT"]));
const LFO_WAVEFORM: Shape = shape(
    0,
    12,
    Named(&[
        "SIN", "TRI", "SAW-UP", "SAW-DW", "SQR", "RND", "BEND-UP", "BEND-DW", "TRP", "S&H", "CHS",
        "VSIN", "STEP",
    ]),
);
const LFO_OFFSET: Shape = shape(0, 4, Named(&["-100", "-50", "0", "+50", "+100"]));
const PAN: Shape = shape(0, 127, Pan);
/// Alternate Pan Depth, L63 to 63R.
const ALTERNATE_PAN_DEPTH: Shape = shape(1, 127, Pan);
const VELOCITY_RANGE: Shape = shape(1, 127, Raw);
const OFF_ON_REVERSE: Shape = shape(0, 2, Named(&["OFF", "ON", "REVERSE"]));
const MONO_POLY: Shape = shape(0, 1, Named(&["MONO", "POLY"]));
/// A drum key's Output Assign; the doc spells them COMP+EQ1 to COMP+EQ6.
const DRUM_OUTPUT_ASSIGN: Shape = shape(
    0,
    6,
    Named(&["PART", "C+EQ1", "C+EQ2", "C+EQ3", "C+EQ4", "C+EQ5", "C+EQ6"]),
);
const PORTAMENTO_MODE: Shape = shape(0, 1, Named(&["NORMAL", "LEGATO"]));
const PHRASE_NUMBER: Shape = shape(0, 65535, Raw);
const ENV_MODE: Shape = shape(0, 1, Named(&["NO-SUS", "SUSTAIN"]));
const WAVE_GROUP_TYPE: Shape = shape(0, 3, Named(&["INT", "SRX", "---", "---"]));
const WAVE_NUMBER: Shape = shape(0, 16384, Raw);
const WAVE_GAIN: Shape = shape(0, 3, Named(&["-6dB", "0dB", "+6dB", "+12dB"]));
const WAVE_FXM_COLOR: Shape = shape(0, 3, Named(&["1", "2", "3", "4"]));
const WAVE_FXM_DEPTH: Shape = shape(0, 16, Raw);
const FILTER_TYPE: Shape = shape(
    0,
    6,
    Named(&["OFF", "LPF", "BPF", "HPF", "PKG", "LPF2", "LPF3"]),
);

/// Cents for each Random Pitch Depth value (0-30), shared by PCM Synth
/// Tone and PCM Drum Kit partials.
const RANDOM_PITCH_DEPTH: Shape = shape(
    0,
    30,
    Named(&[
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "20", "30", "40", "50", "60", "70",
        "80", "90", "100", "200", "300", "400", "500", "600", "700", "800", "900", "1000", "1100",
        "1200",
    ]),
);
/// Pitch Env Depth, -12 to +12 semitones, shared by PCM Synth Tone and PCM
/// Drum Kit partials.
const PITCH_ENV_DEPTH: Shape = shape(52, 76, SignedCenter(64));

/// First value of `field` in a block dump, nibblized or not.
fn value_in<F: BlockField>(field: F, block: &[u8]) -> i32 {
    if field.is_nibblized() {
        i32::from(field.nibbles(block))
    } else {
        i32::from(field.byte(block))
    }
}

fn spec<F: BlockField>(field: F, block: &[u8], Shape { min, max, format }: Shape) -> FieldSpec {
    FieldSpec {
        min,
        max,
        default: value_in(field, block),
        format,
    }
}

/// The spec of a PCM Synth Tone Common field.
pub fn pcms_common_spec(field: PcmsCommonField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| pcm_synth::encode_pcms_common(&Default::default()));
    use PcmsCommonField as F;
    let shape = match field {
        F::ToneLevel | F::AnalogFeel | F::PortamentoTime => RAW_127,
        F::TonePan => PAN,
        F::TonePriority => shape(0, 1, Named(&["LAST", "LOUDEST"])),
        F::CoarseTune => COARSE_TUNE,
        F::FineTune => FINE_TUNE,
        F::OctaveShift => OCTAVE_SHIFT,
        F::StretchTuneDepth => shape(0, 3, Named(&["OFF", "1", "2", "3"])),
        F::MonoPoly => MONO_POLY,
        F::LegatoSwitch | F::LegatoRetrigger | F::PortamentoSwitch | F::PmtControlSwitch => ON_OFF,
        F::PortamentoMode => PORTAMENTO_MODE,
        F::PortamentoType => shape(0, 1, Named(&["RATE", "TIME"])),
        F::PortamentoStart => shape(0, 1, Named(&["PITCH", "NOTE"])),
        F::CutoffOffset
        | F::ResonanceOffset
        | F::AttackTimeOffset
        | F::ReleaseTimeOffset
        | F::VelocitySensOffset => SIGNED_63,
        F::PitchBendRangeUp | F::PitchBendRangeDown => shape(0, 48, Raw),
        // shortcut: Source and Destination show raw indices; name them when an editor shows them (the doc's Destination list is short of its 0-33 range).
        F::MatrixControlSource => shape(0, 109, Raw),
        F::MatrixControlDestination1
        | F::MatrixControlDestination2
        | F::MatrixControlDestination3
        | F::MatrixControlDestination4 => shape(0, 33, Raw),
        F::MatrixControlSens1
        | F::MatrixControlSens2
        | F::MatrixControlSens3
        | F::MatrixControlSens4 => SIGNED_63,
    };
    spec(field, init, shape)
}

/// The spec of a PCM Synth Tone PMT field.
pub fn pcms_pmt_spec(field: PcmsPmtField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| pcm_synth::encode_pcms_pmt(&Default::default()));
    use PcmsPmtField as F;
    let shape = match field {
        F::StructureType12 | F::StructureType34 => shape(
            0,
            9,
            Named(&["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]),
        ),
        F::Booster12 | F::Booster34 => shape(0, 3, Named(&["0dB", "+6dB", "+12dB", "+18dB"])),
        F::PmtVelocityControl => shape(0, 3, Named(&["OFF", "ON", "RANDOM", "CYCLE"])),
        F::PartialSwitch => ON_OFF,
        F::KeyRangeLower | F::KeyRangeUpper => shape(0, 127, Note),
        F::KeyFadeLower | F::KeyFadeUpper | F::VelocityFadeLower | F::VelocityFadeUpper => RAW_127,
        F::VelocityRangeLower | F::VelocityRangeUpper => VELOCITY_RANGE,
    };
    spec(field, init, shape)
}

/// The spec of a PCM Synth Tone Partial field.
pub fn pcms_partial_spec(field: PcmsPartialField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| pcm_synth::encode_pcms_partial(&Default::default()));
    use PcmsPartialField as F;
    let shape = match field {
        F::Level
        | F::OutputLevel
        | F::ChorusSend
        | F::ReverbSend
        | F::TvfCutoffFrequency
        | F::TvfResonance
        | F::PitchEnvTime
        | F::TvfEnvTime
        | F::TvfEnvLevel
        | F::TvaEnvTime
        | F::TvaEnvLevel
        | F::Lfo1RateDetune
        | F::Lfo1DelayTime
        | F::Lfo1FadeTime
        | F::Lfo2RateDetune
        | F::Lfo2DelayTime
        | F::Lfo2FadeTime => RAW_127,
        F::CoarseTune => COARSE_TUNE,
        F::FineTune => FINE_TUNE,
        F::RandomPitchDepth => RANDOM_PITCH_DEPTH,
        F::Pan => PAN,
        F::AlternatePanDepth => ALTERNATE_PAN_DEPTH,
        F::RandomPanDepth => shape(0, 63, Raw),
        F::PanKeyfollow
        | F::PitchEnvTimeKeyfollow
        | F::TvfEnvTimeKeyfollow
        | F::TvaBiasLevel
        | F::TvaEnvTimeKeyfollow
        | F::Lfo1DelayTimeKeyfollow
        | F::Lfo2DelayTimeKeyfollow => KEYFOLLOW_100,
        F::WavePitchKeyfollow | F::TvfCutoffKeyfollow => KEYFOLLOW_200,
        F::EnvMode => ENV_MODE,
        F::DelayMode => shape(
            0,
            3,
            Named(&["NORMAL", "HOLD", "KEY-OFF-NORMAL", "KEY-OFF-DECAY"]),
        ),
        F::DelayTime | F::Lfo1Rate | F::Lfo2Rate => shape(0, 149, TimeOrNote),
        F::ReceiveBender
        | F::ReceiveExpression
        | F::ReceiveHold1
        | F::RedamperSwitch
        | F::WaveFxmSwitch
        | F::WaveTempoSync
        | F::Lfo1KeyTrigger
        | F::Lfo2KeyTrigger => ON_OFF,
        F::Control1Switch | F::Control2Switch | F::Control3Switch | F::Control4Switch => {
            OFF_ON_REVERSE
        }
        F::WaveGroupType => WAVE_GROUP_TYPE,
        F::WaveGroupId | F::WaveNumberL | F::WaveNumberR => WAVE_NUMBER,
        F::WaveGain => WAVE_GAIN,
        F::WaveFxmColor => WAVE_FXM_COLOR,
        F::WaveFxmDepth => WAVE_FXM_DEPTH,
        F::PitchEnvDepth => PITCH_ENV_DEPTH,
        F::PitchEnvVelocitySens
        | F::PitchEnvT1VelocitySens
        | F::PitchEnvT4VelocitySens
        | F::PitchEnvLevel
        | F::TvfCutoffVelocitySens
        | F::TvfResonanceVelocitySens
        | F::TvfEnvDepth
        | F::TvfEnvVelocitySens
        | F::TvfEnvT1VelocitySens
        | F::TvfEnvT4VelocitySens
        | F::TvaLevelVelocitySens
        | F::TvaEnvT1VelocitySens
        | F::TvaEnvT4VelocitySens
        | F::Lfo1PitchDepth
        | F::Lfo1TvfDepth
        | F::Lfo1TvaDepth
        | F::Lfo1PanDepth
        | F::Lfo2PitchDepth
        | F::Lfo2TvfDepth
        | F::Lfo2TvaDepth
        | F::Lfo2PanDepth => SIGNED_63,
        F::TvfFilterType => FILTER_TYPE,
        F::TvfCutoffVelocityCurve | F::TvfEnvVelocityCurve | F::TvaLevelVelocityCurve => {
            VELOCITY_CURVE
        }
        F::TvaBiasPosition => shape(0, 127, Note),
        // The doc spells them LOWER, UPPER, LOWER&UPPER.
        F::TvaBiasDirection => shape(0, 3, Named(&["LO", "UP", "LO&UP", "ALL"])),
        F::Lfo1Waveform | F::Lfo2Waveform => LFO_WAVEFORM,
        F::Lfo1Offset | F::Lfo2Offset => LFO_OFFSET,
        F::Lfo1FadeMode | F::Lfo2FadeMode => FADE_MODE,
        // The doc shows only 0-1; the names are the editor's.
        F::LfoStepType => shape(0, 1, Named(&["STEP", "SMOOTH"])),
        F::LfoStep => shape(28, 100, SignedCenter(64)),
    };
    spec(field, init, shape)
}

/// The spec of a PCM Synth Tone Common 2 field.
pub fn pcms_common2_spec(field: PcmsCommon2Field) -> FieldSpec {
    // Common 2 has no block encoder: read the default off the struct.
    let init = pcm_synth::PcmSynthCommon2::default();
    use PcmsCommon2Field as F;
    let (Shape { min, max, format }, default) = match field {
        // shortcut: Tone Category shows its raw index; name it when an editor shows it.
        F::ToneCategory => (RAW_127, init.tone_category.into()),
        F::ToneNumber => (shape(0, 255, Raw), init.tone_number.into()),
        F::PhraseOctaveShift => (OCTAVE_SHIFT, init.phrase_octave_shift.into()),
        F::TfxSwitch => (ON_OFF, init.tfx_switch.into()),
        F::PhraseNumber => (PHRASE_NUMBER, init.phrase_number.into()),
    };
    FieldSpec {
        min,
        max,
        default,
        format,
    }
}

/// The spec of a PCM Drum Kit Common field.
pub fn pcmd_common_spec(field: PcmdCommonField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| pcm_drum::encode_pcmd_common(&Default::default()));
    let shape = match field {
        PcmdCommonField::KitLevel => RAW_127,
    };
    spec(field, init, shape)
}

/// The INIT PCM Drum Kit Partial block, which holds both the WMT and the
/// other Partial fields.
fn pcmd_partial_init() -> &'static [u8] {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    INIT.get_or_init(|| pcm_drum::encode_pcmd_partial(&Default::default()))
}

/// The spec of a PCM Drum Kit WMT field.
pub fn pcmd_wmt_spec(field: PcmdWmtField) -> FieldSpec {
    use PcmdWmtField as F;
    let shape = match field {
        F::Switch | F::WaveFxmSwitch | F::WaveTempoSync | F::RandomPanSwitch => ON_OFF,
        F::WaveGroupType => WAVE_GROUP_TYPE,
        F::WaveGroupId | F::WaveNumberL | F::WaveNumberR => WAVE_NUMBER,
        F::WaveGain => WAVE_GAIN,
        F::WaveFxmColor => WAVE_FXM_COLOR,
        F::WaveFxmDepth => WAVE_FXM_DEPTH,
        F::CoarseTune => COARSE_TUNE,
        F::FineTune => FINE_TUNE,
        F::Pan => PAN,
        F::AlternatePanSwitch => OFF_ON_REVERSE,
        F::Level | F::VelocityFadeLower | F::VelocityFadeUpper => RAW_127,
        F::VelocityRangeLower | F::VelocityRangeUpper => VELOCITY_RANGE,
    };
    spec(field, pcmd_partial_init(), shape)
}

/// The spec of a PCM Drum Kit Partial field outside the WMT layers.
pub fn pcmd_partial_spec(field: PcmdPartialField) -> FieldSpec {
    use PcmdPartialField as F;
    let shape = match field {
        F::AssignType => shape(0, 1, Named(&["MULTI", "SINGLE"])),
        F::MuteGroup => shape(
            0,
            31,
            Named(&[
                "OFF", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
                "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28",
                "29", "30", "31",
            ]),
        ),
        F::Level
        | F::OutputLevel
        | F::ChorusSend
        | F::ReverbSend
        | F::TvfCutoffFrequency
        | F::TvfResonance
        | F::PitchEnvTime
        | F::TvfEnvTime
        | F::TvfEnvLevel
        | F::TvaEnvTime
        | F::TvaEnvLevel => RAW_127,
        F::CoarseTune => shape(0, 127, Note),
        F::FineTune => FINE_TUNE,
        F::RandomPitchDepth => RANDOM_PITCH_DEPTH,
        F::Pan => PAN,
        F::RandomPanDepth => shape(0, 63, Raw),
        F::AlternatePanDepth => ALTERNATE_PAN_DEPTH,
        F::EnvMode => ENV_MODE,
        F::OutputAssign => DRUM_OUTPUT_ASSIGN,
        F::PitchBendRange => shape(0, 48, Raw),
        F::ReceiveExpression | F::ReceiveHold1 | F::OneShotMode => ON_OFF,
        F::WmtVelocityControl => shape(0, 2, Named(&["OFF", "ON", "RANDOM"])),
        F::PitchEnvDepth => PITCH_ENV_DEPTH,
        F::PitchEnvVelocitySens
        | F::PitchEnvT1VelocitySens
        | F::PitchEnvT4VelocitySens
        | F::PitchEnvLevel
        | F::TvfCutoffVelocitySens
        | F::TvfResonanceVelocitySens
        | F::TvfEnvDepth
        | F::TvfEnvVelocitySens
        | F::TvfEnvT1VelocitySens
        | F::TvfEnvT4VelocitySens
        | F::TvaLevelVelocitySens
        | F::TvaEnvT1VelocitySens
        | F::TvaEnvT4VelocitySens => SIGNED_63,
        F::TvfFilterType => FILTER_TYPE,
        F::TvfCutoffVelocityCurve | F::TvfEnvVelocityCurve | F::TvaLevelVelocityCurve => {
            VELOCITY_CURVE
        }
    };
    spec(field, pcmd_partial_init(), shape)
}

/// The spec of an SN Synth Tone Common field.
pub fn sns_common_spec(field: SnsCommonField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| sn_synth::encode_sns_common(&Default::default()));
    use SnsCommonField as F;
    let shape = match field {
        F::ToneLevel | F::PortamentoTime | F::AnalogFeel | F::WaveShape => RAW_127,
        // shortcut: Tone Category shows its raw index; name it when an editor shows it.
        F::ToneCategory => RAW_127,
        F::PortamentoSwitch
        | F::MonoSwitch
        | F::Partial1Switch
        | F::Partial1Select
        | F::Partial2Switch
        | F::Partial2Select
        | F::Partial3Switch
        | F::Partial3Select
        | F::TfxSwitch
        | F::UnisonSwitch
        | F::LegatoSwitch => ON_OFF,
        F::OctaveShift | F::PhraseOctaveShift => OCTAVE_SHIFT,
        F::PitchBendRangeUp | F::PitchBendRangeDown => shape(0, 24, Raw),
        F::RingSwitch => shape(0, 2, Named(&["OFF", "---", "ON"])),
        F::PortamentoMode => PORTAMENTO_MODE,
        F::PhraseNumber => PHRASE_NUMBER,
        F::UnisonSize => shape(0, 3, Named(&["2", "4", "6", "8"])),
    };
    spec(field, init, shape)
}

/// SN Synth Tone LFO shapes.
const SNS_LFO_SHAPE: Shape = shape(0, 5, Named(&["TRI", "SIN", "SAW", "SQR", "S&H", "RND"]));
/// SN Synth Tone LFO Tempo Sync notes.
const SNS_TEMPO_SYNC_NOTE: Shape = shape(
    0,
    19,
    Named(&[
        "16", "12", "8", "4", "2", "1", "3/4", "2/3", "1/2", "3/8", "1/3", "1/4", "3/16", "1/6",
        "1/8", "3/32", "1/12", "1/16", "1/24", "1/32",
    ]),
);

/// The spec of an SN Synth Tone Partial field.
pub fn sns_partial_spec(field: SnsPartialField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| sn_synth::encode_sns_partial(&Default::default()));
    use SnsPartialField as F;
    let shape = match field {
        // The doc spells SP-SAW as SUPER-SAW; the OSC scope draws by these names.
        F::OscWave => shape(
            0,
            7,
            Named(&[
                "SAW", "SQR", "PW-SQR", "TRI", "SINE", "NOISE", "SP-SAW", "PCM",
            ]),
        ),
        F::OscWaveVariation => shape(0, 2, Named(&["A", "B", "C"])),
        F::OscPitch => shape(40, 88, SignedCenter(64)),
        F::OscDetune => FINE_TUNE,
        F::OscPwModDepth
        | F::OscPulseWidth
        | F::OscPitchEnvAttack
        | F::OscPitchEnvDecay
        | F::FilterCutoff
        | F::FilterResonance
        | F::FilterEnvAttack
        | F::FilterEnvDecay
        | F::FilterEnvSustain
        | F::FilterEnvRelease
        | F::AmpLevel
        | F::AmpEnvAttack
        | F::AmpEnvDecay
        | F::AmpEnvSustain
        | F::AmpEnvRelease
        | F::LfoRate
        | F::LfoFadeTime
        | F::ModLfoRate
        | F::PwShift
        | F::HpfCutoff
        | F::SuperSawDetune => RAW_127,
        F::OscPitchEnvDepth
        | F::FilterEnvVelSens
        | F::FilterEnvDepth
        | F::AmpVelSens
        | F::LfoPitchDepth
        | F::LfoFilterDepth
        | F::LfoAmpDepth
        | F::LfoPanDepth
        | F::ModLfoPitchDepth
        | F::ModLfoFilterDepth
        | F::ModLfoAmpDepth
        | F::ModLfoPanDepth
        | F::AftertouchCutoff
        | F::AftertouchLevel
        | F::ModLfoRateControl => SIGNED_63,
        // The doc spells BYP as BYPASS.
        F::FilterMode => shape(
            0,
            7,
            Named(&["BYP", "LPF", "HPF", "BPF", "PKG", "LPF2", "LPF3", "LPF4"]),
        ),
        F::FilterSlope => shape(0, 1, Named(&["-12dB", "-24dB"])),
        F::FilterKeyfollow | F::AmpLevelKeyfollow => KEYFOLLOW_100,
        F::AmpPan => PAN,
        F::LfoShape | F::ModLfoShape => SNS_LFO_SHAPE,
        F::LfoTempoSync | F::LfoKeyTrigger | F::ModLfoTempoSync => ON_OFF,
        F::LfoTempoSyncNote | F::ModLfoTempoSyncNote => SNS_TEMPO_SYNC_NOTE,
        F::WaveGain => WAVE_GAIN,
        F::WaveNumber => WAVE_NUMBER,
    };
    spec(field, init, shape)
}

/// The spec of an SN Acoustic Tone Common field.
pub fn sna_common_spec(field: SnaCommonField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| sn_acoustic::encode_sna_common(&Default::default()));
    use SnaCommonField as F;
    let shape = match field {
        F::ToneLevel => RAW_127,
        F::MonoPoly => MONO_POLY,
        F::PortamentoTimeOffset
        | F::CutoffOffset
        | F::ResonanceOffset
        | F::AttackTimeOffset
        | F::ReleaseTimeOffset
        | F::VibratoRate
        | F::VibratoDepth
        | F::VibratoDelay => shape(0, 127, SignedCenter(64)),
        F::OctaveShift | F::PhraseOctaveShift => OCTAVE_SHIFT,
        // shortcut: Category shows its raw index; name it when an editor shows it.
        F::Category => RAW_127,
        // Two nibbles here, not the four of PHRASE_NUMBER.
        F::PhraseNumber => shape(0, 255, Raw),
        F::TfxSwitch => ON_OFF,
        F::InstVariation | F::InstNumber => RAW_127,
        // Each instrument gives its Modify Parameters their own ranges (`sna_inst_params`).
        F::ModifyParameter => RAW_127,
    };
    spec(field, init, shape)
}

/// The spec of an SN Drum Kit Common field.
pub fn snd_common_spec(field: SndCommonField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| sn_drum::encode_snd_common(&Default::default()));
    use SndCommonField as F;
    let shape = match field {
        F::KitLevel | F::AmbienceLevel | F::PhraseNumber => RAW_127,
        F::TfxSwitch => ON_OFF,
    };
    spec(field, init, shape)
}

/// The spec of an SN Drum Kit Note field.
pub fn snd_note_spec(field: SndNoteField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| sn_drum::encode_snd_note(&Default::default()));
    use SndNoteField as F;
    let shape = match field {
        F::InstNumber => shape(0, SND_INST_NUMBER_MAX.into(), Raw),
        F::Level | F::ChorusSend | F::ReverbSend | F::StereoWidth => RAW_127,
        F::Pan => PAN,
        // -1200 to +1200 cents.
        F::Tune => shape(
            8,
            248,
            ParamFormat::SignedStep {
                center: 128,
                step: 10,
            },
        ),
        F::Attack => shape(0, 100, ParamFormat::Percent),
        // -63 to 0.
        F::Decay => shape(1, 64, SignedCenter(64)),
        F::Brilliance => shape(49, 76, SignedCenter(64)),
        F::Variation => shape(
            0,
            7,
            Named(&[
                "OFF", "FLAM1", "FLAM2", "FLAM3", "BUZZ1", "BUZZ2", "BUZZ3", "ROLL",
            ]),
        ),
        F::DynamicRange => shape(0, 63, Raw),
        F::OutputAssign => DRUM_OUTPUT_ASSIGN,
    };
    spec(field, init, shape)
}

/// A field whose low values are a free rate or time and whose values from
/// `note_from` up are tempo-synced notes ([`ParamFormat::TimeOrNote`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InRangeSync {
    /// Labels of the free/note toggle, e.g. `["Hz", "♪"]`.
    pub labels: [&'static str; 2],
    /// First note value.
    pub note_from: i32,
    /// Value written when switching to note mode (1/4).
    pub note_default: i32,
}

/// A rate field whose tempo-synced note lives in a separate field,
/// selected by an on/off switch field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldSync<F> {
    /// Labels of the free/note toggle, e.g. `["Hz", "♪"]`.
    pub labels: [&'static str; 2],
    /// The switch: 1 = note mode.
    pub switch: F,
    /// The note field.
    pub note: F,
}

/// How a PCM Synth Tone Partial rate or time field follows the tempo, if
/// it can.
pub fn pcms_partial_sync(field: PcmsPartialField) -> Option<InRangeSync> {
    if pcms_partial_spec(field).format != ParamFormat::TimeOrNote {
        return None;
    }
    let quarter = NOTE_NAMES
        .iter()
        .position(|n| *n == "1/4")
        .expect("1/4 is a note value") as i32;
    Some(InRangeSync {
        labels: if field == PcmsPartialField::DelayTime {
            SYNC_MS
        } else {
            SYNC_HZ
        },
        note_from: NOTE_FROM,
        note_default: NOTE_FROM + quarter,
    })
}

/// How an SN Synth Tone Partial rate field follows the tempo, if it can.
pub fn sns_partial_sync(field: SnsPartialField) -> Option<FieldSync<SnsPartialField>> {
    use SnsPartialField as F;
    let (switch, note) = match field {
        F::LfoRate => (F::LfoTempoSync, F::LfoTempoSyncNote),
        F::ModLfoRate => (F::ModLfoTempoSync, F::ModLfoTempoSyncNote),
        _ => return None,
    };
    Some(FieldSync {
        labels: SYNC_HZ,
        switch,
        note,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PCMS_DOC: &str = include_str!("../../../docs/midi/06-pcm-synth-tone.md");
    const PCMD_DOC: &str = include_str!("../../../docs/midi/07-pcm-drum-kit.md");
    const SNS_DOC: &str = include_str!("../../../docs/midi/08-supernatural-synth-tone.md");
    const SNA_DOC: &str = include_str!("../../../docs/midi/09-supernatural-acoustic-tone.md");
    const SND_DOC: &str = include_str!("../../../docs/midi/10-supernatural-drum-kit.md");

    /// A documented parameter: its byte position in the block and its range.
    struct DocRow {
        name: String,
        position: usize,
        min: i32,
        max: i32,
    }

    /// The parameter rows of the `## <heading>` section of `doc`, whose
    /// heading may go on with an `(offset ...)`. Cells are split on `|`;
    /// reserves, unnamed rows, `...` rows and rows `keep` rejects are left
    /// out.
    fn doc_rows(doc: &str, heading: &str, keep: impl Fn(&str) -> bool) -> Vec<DocRow> {
        let section = doc
            .split("\n## ")
            .find(|s| {
                s.lines().next().is_some_and(|l| {
                    l.strip_prefix(heading)
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with(" ("))
                })
            })
            .unwrap_or_else(|| panic!("no section {heading}"));
        section
            .lines()
            .filter(|l| l.starts_with("| `"))
            .filter_map(|line| {
                let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
                let name = cells[2];
                if name.starts_with('(') || !keep(name) {
                    return None;
                }
                let offset = cells[0].split('`').nth(1).unwrap();
                let [hi, lo] = [0, 1]
                    .map(|i| usize::from_str_radix(offset.split(' ').nth(i).unwrap(), 16).unwrap());
                // The PCM Synth doc separates the range with " - ", the PCM Drum doc with " -- ".
                let (min, max) = (cells[3].split_once(" -- "))
                    .or_else(|| cells[3].split_once(" - "))
                    .unwrap();
                Some(DocRow {
                    name: name.to_string(),
                    position: hi * 128 + lo,
                    min: min.parse().unwrap(),
                    max: max.parse().unwrap(),
                })
            })
            .collect()
    }

    /// Check every documented row against the field covering its position.
    fn assert_matches_doc<F: BlockField>(
        doc: &str,
        heading: &str,
        keep: impl Fn(&str) -> bool,
        fields: &[F],
        spec_of: impl Fn(F) -> FieldSpec,
    ) {
        let rows = doc_rows(doc, heading, keep);
        assert!(!rows.is_empty(), "{heading}: no rows");
        for row in rows {
            let field = fields
                .iter()
                .copied()
                .find(|f| (0..f.count()).any(|i| f.position() + i * f.stride() == row.position))
                .unwrap_or_else(|| {
                    panic!("{heading}: no field at {} ({})", row.position, row.name)
                });
            let spec = spec_of(field);
            assert_eq!(
                (spec.min, spec.max),
                (row.min, row.max),
                "{} ({field:?})",
                row.name
            );
        }
        for &field in fields {
            let spec = spec_of(field);
            assert!(
                (spec.min..=spec.max).contains(&spec.default),
                "{field:?}: default {} outside {}..={}",
                spec.default,
                spec.min,
                spec.max
            );
            if let Named(names) = spec.format {
                assert_eq!(
                    names.len() as i32,
                    spec.max + 1,
                    "{field:?}: one name per value"
                );
            }
        }
    }

    #[test]
    fn common_specs_match_the_doc() {
        use PcmsCommonField as F;
        let fields = [
            F::ToneLevel,
            F::TonePan,
            F::TonePriority,
            F::CoarseTune,
            F::FineTune,
            F::OctaveShift,
            F::StretchTuneDepth,
            F::AnalogFeel,
            F::MonoPoly,
            F::LegatoSwitch,
            F::LegatoRetrigger,
            F::PortamentoSwitch,
            F::PortamentoMode,
            F::PortamentoType,
            F::PortamentoStart,
            F::PortamentoTime,
            F::CutoffOffset,
            F::ResonanceOffset,
            F::AttackTimeOffset,
            F::ReleaseTimeOffset,
            F::VelocitySensOffset,
            F::PmtControlSwitch,
            F::PitchBendRangeUp,
            F::PitchBendRangeDown,
            F::MatrixControlSource,
            F::MatrixControlDestination1,
            F::MatrixControlSens1,
            F::MatrixControlDestination2,
            F::MatrixControlSens2,
            F::MatrixControlDestination3,
            F::MatrixControlSens3,
            F::MatrixControlDestination4,
            F::MatrixControlSens4,
        ];
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone Common",
            |name| !name.starts_with("PCM Synth Tone Name"),
            &fields,
            pcms_common_spec,
        );
    }

    #[test]
    fn pmt_specs_match_the_doc() {
        use PcmsPmtField as F;
        let fields = [
            F::StructureType12,
            F::Booster12,
            F::StructureType34,
            F::Booster34,
            F::PmtVelocityControl,
            F::PartialSwitch,
            F::KeyRangeLower,
            F::KeyRangeUpper,
            F::KeyFadeLower,
            F::KeyFadeUpper,
            F::VelocityRangeLower,
            F::VelocityRangeUpper,
            F::VelocityFadeLower,
            F::VelocityFadeUpper,
        ];
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone PMT (Partial Mix Table)",
            |_| true,
            &fields,
            pcms_pmt_spec,
        );
    }

    #[test]
    fn partial_specs_match_the_doc() {
        use PcmsPartialField as F;
        let fields = [
            F::Level,
            F::CoarseTune,
            F::FineTune,
            F::RandomPitchDepth,
            F::Pan,
            F::PanKeyfollow,
            F::RandomPanDepth,
            F::AlternatePanDepth,
            F::EnvMode,
            F::DelayMode,
            F::DelayTime,
            F::OutputLevel,
            F::ChorusSend,
            F::ReverbSend,
            F::ReceiveBender,
            F::ReceiveExpression,
            F::ReceiveHold1,
            F::RedamperSwitch,
            F::Control1Switch,
            F::Control2Switch,
            F::Control3Switch,
            F::Control4Switch,
            F::WaveGroupType,
            F::WaveGroupId,
            F::WaveNumberL,
            F::WaveNumberR,
            F::WaveGain,
            F::WaveFxmSwitch,
            F::WaveFxmColor,
            F::WaveFxmDepth,
            F::WaveTempoSync,
            F::WavePitchKeyfollow,
            F::PitchEnvDepth,
            F::PitchEnvVelocitySens,
            F::PitchEnvT1VelocitySens,
            F::PitchEnvT4VelocitySens,
            F::PitchEnvTimeKeyfollow,
            F::PitchEnvTime,
            F::PitchEnvLevel,
            F::TvfFilterType,
            F::TvfCutoffFrequency,
            F::TvfCutoffKeyfollow,
            F::TvfCutoffVelocityCurve,
            F::TvfCutoffVelocitySens,
            F::TvfResonance,
            F::TvfResonanceVelocitySens,
            F::TvfEnvDepth,
            F::TvfEnvVelocityCurve,
            F::TvfEnvVelocitySens,
            F::TvfEnvT1VelocitySens,
            F::TvfEnvT4VelocitySens,
            F::TvfEnvTimeKeyfollow,
            F::TvfEnvTime,
            F::TvfEnvLevel,
            F::TvaBiasLevel,
            F::TvaBiasPosition,
            F::TvaBiasDirection,
            F::TvaLevelVelocityCurve,
            F::TvaLevelVelocitySens,
            F::TvaEnvT1VelocitySens,
            F::TvaEnvT4VelocitySens,
            F::TvaEnvTimeKeyfollow,
            F::TvaEnvTime,
            F::TvaEnvLevel,
            F::Lfo1Waveform,
            F::Lfo1Rate,
            F::Lfo1Offset,
            F::Lfo1RateDetune,
            F::Lfo1DelayTime,
            F::Lfo1DelayTimeKeyfollow,
            F::Lfo1FadeMode,
            F::Lfo1FadeTime,
            F::Lfo1KeyTrigger,
            F::Lfo1PitchDepth,
            F::Lfo1TvfDepth,
            F::Lfo1TvaDepth,
            F::Lfo1PanDepth,
            F::Lfo2Waveform,
            F::Lfo2Rate,
            F::Lfo2Offset,
            F::Lfo2RateDetune,
            F::Lfo2DelayTime,
            F::Lfo2DelayTimeKeyfollow,
            F::Lfo2FadeMode,
            F::Lfo2FadeTime,
            F::Lfo2KeyTrigger,
            F::Lfo2PitchDepth,
            F::Lfo2TvfDepth,
            F::Lfo2TvaDepth,
            F::Lfo2PanDepth,
            F::LfoStepType,
            F::LfoStep,
        ];
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone Partial",
            |_| true,
            &fields,
            pcms_partial_spec,
        );
    }

    #[test]
    fn common2_specs_match_the_doc() {
        use PcmsCommon2Field as F;
        let fields = [
            F::ToneCategory,
            F::ToneNumber,
            F::PhraseOctaveShift,
            F::TfxSwitch,
            F::PhraseNumber,
        ];
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone Common 2",
            |_| true,
            &fields,
            pcms_common2_spec,
        );
    }

    #[test]
    fn formats_follow_the_doc_display() {
        let fmt = |f: PcmsPartialField, v| pcms_partial_spec(f).format.format(v);
        assert_eq!(fmt(PcmsPartialField::RandomPitchDepth, 11), "20");
        assert_eq!(fmt(PcmsPartialField::RandomPitchDepth, 30), "1200");
        assert_eq!(fmt(PcmsPartialField::PitchEnvDepth, 52), "-12");
        assert_eq!(fmt(PcmsPartialField::PitchEnvTimeKeyfollow, 54), "-100");
        assert_eq!(fmt(PcmsPartialField::WavePitchKeyfollow, 84), "+200");
        assert_eq!(fmt(PcmsPartialField::Pan, 0), "L64");
        assert_eq!(fmt(PcmsPartialField::Pan, 127), "63R");
        assert_eq!(fmt(PcmsPartialField::TvaBiasPosition, 0), "C-1");
        assert_eq!(fmt(PcmsPartialField::TvaBiasPosition, 127), "G9");
        assert_eq!(fmt(PcmsPartialField::Lfo1Rate, 127), "127");
        assert_eq!(fmt(PcmsPartialField::Lfo1Rate, 128), "1/64T");
        assert_eq!(fmt(PcmsPartialField::Lfo1Rate, 149), "2/1");
        assert_eq!(fmt(PcmsPartialField::LfoStep, 28), "-36");
        let center = |f| pcms_partial_spec(f).format.center();
        assert_eq!(center(PcmsPartialField::PitchEnvTimeKeyfollow), Some(64));
        assert_eq!(center(PcmsPartialField::Pan), Some(64));
        assert_eq!(center(PcmsPartialField::Level), None);
    }

    #[test]
    fn pcms_rates_and_delay_sync_in_range() {
        let lfo = pcms_partial_sync(PcmsPartialField::Lfo1Rate).unwrap();
        assert_eq!(lfo.labels, SYNC_HZ);
        assert_eq!(lfo.note_from, 128);
        let quarter = pcms_partial_spec(PcmsPartialField::Lfo1Rate)
            .format
            .format(lfo.note_default);
        assert_eq!(quarter, "1/4");
        assert_eq!(pcms_partial_sync(PcmsPartialField::Lfo2Rate), Some(lfo));
        assert_eq!(
            pcms_partial_sync(PcmsPartialField::DelayTime)
                .unwrap()
                .labels,
            SYNC_MS
        );
        assert_eq!(pcms_partial_sync(PcmsPartialField::Lfo1DelayTime), None);
    }

    #[test]
    fn sns_lfo_rates_sync_through_their_fields() {
        use SnsPartialField as F;
        let sync = sns_partial_sync(F::LfoRate).unwrap();
        assert_eq!(
            (sync.switch, sync.note),
            (F::LfoTempoSync, F::LfoTempoSyncNote)
        );
        let sync = sns_partial_sync(F::ModLfoRate).unwrap();
        assert_eq!(
            (sync.switch, sync.note),
            (F::ModLfoTempoSync, F::ModLfoTempoSyncNote)
        );
        assert_eq!(sns_partial_sync(F::LfoPitchDepth), None);
    }

    /// Whether a PCM Drum Kit Partial row is a WMT layer's (`WMT1 Wave
    /// Switch`), not `WMT Velocity Control`.
    fn is_wmt_layer_row(name: &str) -> bool {
        name.strip_prefix("WMT")
            .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
    }

    #[test]
    fn pcmd_common_specs_match_the_doc() {
        assert_matches_doc(
            PCMD_DOC,
            "PCM Drum Kit Common",
            |name| !name.starts_with("Kit Name"),
            &[PcmdCommonField::KitLevel],
            pcmd_common_spec,
        );
    }

    #[test]
    fn pcmd_wmt_specs_match_the_doc() {
        use PcmdWmtField as F;
        let fields = [
            F::Switch,
            F::WaveGroupType,
            F::WaveGroupId,
            F::WaveNumberL,
            F::WaveNumberR,
            F::WaveGain,
            F::WaveFxmSwitch,
            F::WaveFxmColor,
            F::WaveFxmDepth,
            F::WaveTempoSync,
            F::CoarseTune,
            F::FineTune,
            F::Pan,
            F::RandomPanSwitch,
            F::AlternatePanSwitch,
            F::Level,
            F::VelocityRangeLower,
            F::VelocityRangeUpper,
            F::VelocityFadeLower,
            F::VelocityFadeUpper,
        ];
        assert_matches_doc(
            PCMD_DOC,
            "PCM Drum Kit Partial",
            is_wmt_layer_row,
            &fields,
            pcmd_wmt_spec,
        );
    }

    #[test]
    fn pcmd_partial_specs_match_the_doc() {
        use PcmdPartialField as F;
        let fields = [
            F::AssignType,
            F::MuteGroup,
            F::Level,
            F::CoarseTune,
            F::FineTune,
            F::RandomPitchDepth,
            F::Pan,
            F::RandomPanDepth,
            F::AlternatePanDepth,
            F::EnvMode,
            F::OutputLevel,
            F::ChorusSend,
            F::ReverbSend,
            F::OutputAssign,
            F::PitchBendRange,
            F::ReceiveExpression,
            F::ReceiveHold1,
            F::WmtVelocityControl,
            F::PitchEnvDepth,
            F::PitchEnvVelocitySens,
            F::PitchEnvT1VelocitySens,
            F::PitchEnvT4VelocitySens,
            F::PitchEnvTime,
            F::PitchEnvLevel,
            F::TvfFilterType,
            F::TvfCutoffFrequency,
            F::TvfCutoffVelocityCurve,
            F::TvfCutoffVelocitySens,
            F::TvfResonance,
            F::TvfResonanceVelocitySens,
            F::TvfEnvDepth,
            F::TvfEnvVelocityCurve,
            F::TvfEnvVelocitySens,
            F::TvfEnvT1VelocitySens,
            F::TvfEnvT4VelocitySens,
            F::TvfEnvTime,
            F::TvfEnvLevel,
            F::TvaLevelVelocityCurve,
            F::TvaLevelVelocitySens,
            F::TvaEnvT1VelocitySens,
            F::TvaEnvT4VelocitySens,
            F::TvaEnvTime,
            F::TvaEnvLevel,
            F::OneShotMode,
        ];
        assert_matches_doc(
            PCMD_DOC,
            "PCM Drum Kit Partial",
            |name| !name.starts_with("Partial Name") && !is_wmt_layer_row(name),
            &fields,
            pcmd_partial_spec,
        );
    }

    #[test]
    fn pcm_drum_shares_the_pitch_specs() {
        for (d, s) in [
            (
                PcmdPartialField::RandomPitchDepth,
                PcmsPartialField::RandomPitchDepth,
            ),
            (
                PcmdPartialField::PitchEnvDepth,
                PcmsPartialField::PitchEnvDepth,
            ),
        ] {
            assert_eq!(pcmd_partial_spec(d), pcms_partial_spec(s));
        }
    }

    #[test]
    fn sns_common_specs_match_the_doc() {
        use SnsCommonField as F;
        let fields = [
            F::ToneLevel,
            F::PortamentoSwitch,
            F::PortamentoTime,
            F::MonoSwitch,
            F::OctaveShift,
            F::PitchBendRangeUp,
            F::PitchBendRangeDown,
            F::Partial1Switch,
            F::Partial1Select,
            F::Partial2Switch,
            F::Partial2Select,
            F::Partial3Switch,
            F::Partial3Select,
            F::RingSwitch,
            F::TfxSwitch,
            F::UnisonSwitch,
            F::PortamentoMode,
            F::LegatoSwitch,
            F::AnalogFeel,
            F::WaveShape,
            F::ToneCategory,
            F::PhraseNumber,
            F::PhraseOctaveShift,
            F::UnisonSize,
        ];
        assert_matches_doc(
            SNS_DOC,
            "SN Synth Tone Common",
            |name| !name.starts_with("Tone Name"),
            &fields,
            sns_common_spec,
        );
    }

    #[test]
    fn sns_partial_specs_match_the_doc() {
        use SnsPartialField as F;
        let fields = [
            F::OscWave,
            F::OscWaveVariation,
            F::OscPitch,
            F::OscDetune,
            F::OscPwModDepth,
            F::OscPulseWidth,
            F::OscPitchEnvAttack,
            F::OscPitchEnvDecay,
            F::OscPitchEnvDepth,
            F::FilterMode,
            F::FilterSlope,
            F::FilterCutoff,
            F::FilterKeyfollow,
            F::FilterEnvVelSens,
            F::FilterResonance,
            F::FilterEnvAttack,
            F::FilterEnvDecay,
            F::FilterEnvSustain,
            F::FilterEnvRelease,
            F::FilterEnvDepth,
            F::AmpLevel,
            F::AmpVelSens,
            F::AmpEnvAttack,
            F::AmpEnvDecay,
            F::AmpEnvSustain,
            F::AmpEnvRelease,
            F::AmpPan,
            F::LfoShape,
            F::LfoRate,
            F::LfoTempoSync,
            F::LfoTempoSyncNote,
            F::LfoFadeTime,
            F::LfoKeyTrigger,
            F::LfoPitchDepth,
            F::LfoFilterDepth,
            F::LfoAmpDepth,
            F::LfoPanDepth,
            F::ModLfoShape,
            F::ModLfoRate,
            F::ModLfoTempoSync,
            F::ModLfoTempoSyncNote,
            F::PwShift,
            F::ModLfoPitchDepth,
            F::ModLfoFilterDepth,
            F::ModLfoAmpDepth,
            F::ModLfoPanDepth,
            F::AftertouchCutoff,
            F::AftertouchLevel,
            F::WaveGain,
            F::WaveNumber,
            F::HpfCutoff,
            F::SuperSawDetune,
            F::ModLfoRateControl,
            F::AmpLevelKeyfollow,
        ];
        assert_matches_doc(
            SNS_DOC,
            "SN Synth Tone Partial",
            |_| true,
            &fields,
            sns_partial_spec,
        );
    }

    #[test]
    fn sna_common_specs_match_the_doc() {
        use SnaCommonField as F;
        let fields = [
            F::ToneLevel,
            F::MonoPoly,
            F::PortamentoTimeOffset,
            F::CutoffOffset,
            F::ResonanceOffset,
            F::AttackTimeOffset,
            F::ReleaseTimeOffset,
            F::VibratoRate,
            F::VibratoDepth,
            F::VibratoDelay,
            F::OctaveShift,
            F::Category,
            F::PhraseNumber,
            F::PhraseOctaveShift,
            F::TfxSwitch,
            F::InstVariation,
            F::InstNumber,
            F::ModifyParameter,
        ];
        assert_matches_doc(
            SNA_DOC,
            "SN Acoustic Tone Common",
            |name| !name.starts_with("Tone Name"),
            &fields,
            sna_common_spec,
        );
    }

    #[test]
    fn snd_common_specs_match_the_doc() {
        use SndCommonField as F;
        assert_matches_doc(
            SND_DOC,
            "SN Drum Kit Common",
            |name| !name.starts_with("Kit Name"),
            &[F::KitLevel, F::AmbienceLevel, F::PhraseNumber, F::TfxSwitch],
            snd_common_spec,
        );
    }

    #[test]
    fn snd_note_specs_match_the_doc() {
        use SndNoteField as F;
        let fields = [
            F::InstNumber,
            F::Level,
            F::Pan,
            F::ChorusSend,
            F::ReverbSend,
            F::Tune,
            F::Attack,
            F::Decay,
            F::Brilliance,
            F::Variation,
            F::DynamicRange,
            F::StereoWidth,
            F::OutputAssign,
        ];
        assert_matches_doc(
            SND_DOC,
            "SN Drum Kit Note",
            |_| true,
            &fields,
            snd_note_spec,
        );
        let fmt = |f: SndNoteField, v| snd_note_spec(f).format.format(v);
        assert_eq!(fmt(F::Tune, 8), "-1200");
        assert_eq!(fmt(F::Tune, 248), "+1200");
        assert_eq!(fmt(F::Attack, 100), "100%");
        assert_eq!(fmt(F::Decay, 1), "-63");
        assert_eq!(fmt(F::Brilliance, 49), "-15");
    }
}
