//! Range, default and display format of each tone block field.
//!
//! Each engine's spec function matches its field enum exhaustively, so a
//! field added to the enum fails the build until it is specified. Ranges
//! and formats follow the MIDI Implementation (`docs/midi/`); defaults are
//! the field's value in the engine's `Default` block, which the codebase
//! treats as INIT (not yet checked against the device: #250).
//!
//! # Host-facing names
//!
//! Every spec also carries a `host_name`, the name its parameter has in a
//! DAW's list. The same word means the same thing on [`FieldSpec`],
//! [`ParamDef`](crate::param_registry::ParamDef) and
//! [`FxParamDef`](crate::fx_params::FxParamDef): the name *within its
//! block*, as the MIDI Implementation spells it (`docs/midi/`). Two things
//! can still be missing from it, and a caller adds them with
//! [`qualified_name`]:
//!
//! - **The instance prefix** of a block that repeats: [`partial_prefix`]
//!   (`Partial 2`), [`KEY_PREFIX`], and the Studio Set scopes'
//!   [`host_prefix`](crate::param_registry::MixerScope::host_prefix)
//!   (`Part 7`, `Ext Part`, `Master`, `Drum Comp/EQ 3`).
//! - **The effect type**, for a MFX, Chorus or Reverb parameter, which only
//!   does anything while its type is active
//!   ([`FxParamDef::qualified_name`](crate::fx_params::FxParamDef::qualified_name)).
//!
//! The block *word* the doc itself writes (`PMT`, `WMT`, `TVF`, `Tone`,
//! `Kit`) stays in the name, never in a prefix. A field holding several
//! values has a template: `{}` is the value's number counted from 1 and
//! `{n0}` the number the doc counts from 0 ([`FieldSpec::element_name`]).

use std::sync::OnceLock;

use crate::block_field::BlockField;
use crate::family::ParameterFamily;
use crate::fx_params::{NOTE_NAMES, SYNC_CLOCK, SYNC_HZ};
use crate::mfx::{MfxCtrlSlot, MfxHeaderField, MfxState};
use crate::mfx_params::{ControlSourceKind, MFX_TYPE_NAMES};
use crate::param_registry::{FieldCatalog, NOTE_FROM, ParamFormat};
use crate::pcm_drum::{self, PcmdCommon2Field, PcmdCommonField, PcmdPartialField, PcmdWmtField};
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
    /// The sound dimension the field changes. A field's family starts from
    /// its section in the editors; the exceptions are noted beside them.
    pub family: ParameterFamily,
    /// The field's name in a host's parameter list, within its block (see
    /// the module docs). A field holding several values has a template:
    /// see [`FieldSpec::element_name`].
    pub host_name: &'static str,
}

impl FieldSpec {
    /// The host name of value `index` (0-based) of the field: the name
    /// itself, or its template with `{}` filled by `index + 1` and `{n0}`
    /// by `index`.
    pub fn element_name(&self, index: usize) -> String {
        fill_name_template(self.host_name, index)
    }
}

/// A name template with `{}` filled by `index + 1` and `{n0}` by `index`.
pub(crate) fn fill_name_template(template: &str, index: usize) -> String {
    template
        .replace("{n0}", &index.to_string())
        .replace("{}", &(index + 1).to_string())
}

/// Label of a repeated Partial block, which [`partial_prefix`] numbers.
const PARTIAL_LABEL: &str = "Partial";

/// Prefix of a drum key's own fields: the key a Kit plugin binds to.
pub const KEY_PREFIX: &str = "Key";

/// The prefix of Partial `index` (0-based) of a tone: `Partial 2` for the
/// second. The caller takes the Partial count from the device spec.
pub fn partial_prefix(index: usize) -> String {
    format!("{PARTIAL_LABEL} {}", index + 1)
}

/// A host parameter name in full: `prefix` and `name` (a `host_name`)
/// joined by a space, or `name` alone for a block with no prefix.
pub fn qualified_name(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix} {name}")
    }
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

use ParamFormat::{Named, Note, OnOff, Pan, PcmLfoRate, Raw, SignedCenter, SnLfoRate, TimeOrNote};

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

fn spec<F: BlockField>(
    field: F,
    block: &[u8],
    Shape { min, max, format }: Shape,
    family: ParameterFamily,
    host_name: &'static str,
) -> FieldSpec {
    FieldSpec {
        min,
        max,
        default: value_in(field, block),
        format,
        family,
        host_name,
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
    spec(
        field,
        init,
        shape,
        pcms_common_family(field),
        pcms_common_host_name(field),
    )
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
    spec(
        field,
        init,
        shape,
        pcms_pmt_family(field),
        pcms_pmt_host_name(field),
    )
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
        // Rate 0-127 shows Hz (docs/research/lfo-rates.md); from 128 on, the
        // tempo-synced note values.
        F::Lfo1Rate | F::Lfo2Rate => shape(0, 149, PcmLfoRate),
        F::DelayTime => shape(0, 149, TimeOrNote),
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
    spec(
        field,
        init,
        shape,
        pcms_partial_family(field),
        pcms_partial_host_name(field),
    )
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
        family: pcms_common2_family(field),
        host_name: pcms_common2_host_name(field),
    }
}

/// The spec of a PCM Drum Kit Common field.
pub fn pcmd_common_spec(field: PcmdCommonField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| pcm_drum::encode_pcmd_common(&Default::default()));
    let shape = match field {
        PcmdCommonField::KitLevel => RAW_127,
    };
    spec(
        field,
        init,
        shape,
        pcmd_common_family(field),
        pcmd_common_host_name(field),
    )
}

/// The spec of a PCM Drum Kit Common 2 field.
pub fn pcmd_common2_spec(field: PcmdCommon2Field) -> FieldSpec {
    // Common 2 has no block encoder: read the default off the struct.
    let init = pcm_drum::PcmDrumCommon2::default();
    let (Shape { min, max, format }, default) = match field {
        PcmdCommon2Field::PhraseNumber => (shape(0, 255, Raw), init.phrase_number.into()),
        PcmdCommon2Field::TfxSwitch => (ON_OFF, init.tfx_switch.into()),
    };
    FieldSpec {
        min,
        max,
        default,
        format,
        family: ParameterFamily::Common,
        host_name: pcmd_common2_host_name(field),
    }
}

/// The spec of an MFX header field, which every Tone plugin and every
/// kit-bound Kit plugin lists ahead of the MFX types' parameters.
pub fn mfx_header_spec(field: MfxHeaderField) -> FieldSpec {
    let init = MfxState::default();
    let control = MfxCtrlSlot::default();
    use MfxHeaderField as F;
    let (Shape { min, max, format }, default) = match field {
        F::Type => (
            shape(0, MFX_TYPE_NAMES.len() as i32 - 1, Named(MFX_TYPE_NAMES)),
            init.mfx_type,
        ),
        F::ChorusSend => (RAW_127, init.chorus_send),
        F::ReverbSend => (RAW_127, init.reverb_send),
        // shortcut: Source and Assign show raw indices; name them when an editor shows them (`control_source_name`, `control_targets`).
        F::ControlSource => (
            shape(0, ControlSourceKind::Mfx.max().into(), Raw),
            control.source,
        ),
        F::ControlSens => (SIGNED_63, control.sens),
        // The doc's 0-16 (OFF, 1-16), though no type lists more than eight destinations.
        F::ControlAssign => (shape(0, 16, Raw), control.assign),
    };
    FieldSpec {
        min,
        max,
        default: default.into(),
        format,
        family: field.family(),
        host_name: field.host_name(),
    }
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
    spec(
        field,
        pcmd_partial_init(),
        shape,
        pcmd_wmt_family(field),
        pcmd_wmt_host_name(field),
    )
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
    spec(
        field,
        pcmd_partial_init(),
        shape,
        pcmd_partial_family(field),
        pcmd_partial_host_name(field),
    )
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
    spec(
        field,
        init,
        shape,
        sns_common_family(field),
        sns_common_host_name(field),
    )
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
        | F::LfoFadeTime
        | F::PwShift
        | F::HpfCutoff
        | F::SuperSawDetune => RAW_127,
        // Hz per docs/research/lfo-rates.md; tempo sync lives in separate
        // fields.
        F::LfoRate | F::ModLfoRate => shape(0, 127, SnLfoRate),
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
        F::WaveNumber => shape(
            0,
            WAVE_NUMBER.max,
            ParamFormat::Catalog(FieldCatalog::SnSynthWave),
        ),
    };
    spec(
        field,
        init,
        shape,
        sns_partial_family(field),
        sns_partial_host_name(field),
    )
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
    spec(
        field,
        init,
        shape,
        sna_common_family(field),
        sna_common_host_name(field),
    )
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
    spec(
        field,
        init,
        shape,
        snd_common_family(field),
        snd_common_host_name(field),
    )
}

/// The spec of an SN Drum Kit Note field.
pub fn snd_note_spec(field: SndNoteField) -> FieldSpec {
    static INIT: OnceLock<Vec<u8>> = OnceLock::new();
    let init = INIT.get_or_init(|| sn_drum::encode_snd_note(&Default::default()));
    use SndNoteField as F;
    let shape = match field {
        F::InstNumber => shape(
            0,
            SND_INST_NUMBER_MAX.into(),
            ParamFormat::Catalog(FieldCatalog::SnDrumInst),
        ),
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
    spec(
        field,
        init,
        shape,
        snd_note_family(field),
        snd_note_host_name(field),
    )
}

// ---------------------------------------------------------------------------
// Host-facing names
// ---------------------------------------------------------------------------
//
// Each `*_host_name` function below gives a field's name as the MIDI
// Implementation spells it (`docs/midi/`), within its block: see the
// module docs for what a caller adds.

/// The host name of a PCM Synth Tone Common field.
fn pcms_common_host_name(field: PcmsCommonField) -> &'static str {
    use PcmsCommonField as F;
    match field {
        F::ToneLevel => "Tone Level",
        F::TonePan => "Tone Pan",
        F::TonePriority => "Tone Priority",
        F::CoarseTune => "Tone Coarse Tune",
        F::FineTune => "Tone Fine Tune",
        F::OctaveShift => "Octave Shift",
        F::StretchTuneDepth => "Stretch Tune Depth",
        F::AnalogFeel => "Analog Feel",
        F::MonoPoly => "Mono/Poly",
        F::LegatoSwitch => "Legato Switch",
        F::LegatoRetrigger => "Legato Retrigger",
        F::PortamentoSwitch => "Portamento Switch",
        F::PortamentoMode => "Portamento Mode",
        F::PortamentoType => "Portamento Type",
        F::PortamentoStart => "Portamento Start",
        F::PortamentoTime => "Portamento Time",
        F::CutoffOffset => "Cutoff Offset",
        F::ResonanceOffset => "Resonance Offset",
        F::AttackTimeOffset => "Attack Time Offset",
        F::ReleaseTimeOffset => "Release Time Offset",
        F::VelocitySensOffset => "Velocity Sens Offset",
        F::PmtControlSwitch => "PMT Control Switch",
        F::PitchBendRangeUp => "Pitch Bend Range Up",
        F::PitchBendRangeDown => "Pitch Bend Range Down",
        F::MatrixControlSource => "Matrix Control {} Source",
        F::MatrixControlDestination1 => "Matrix Control {} Destination 1",
        F::MatrixControlSens1 => "Matrix Control {} Sens 1",
        F::MatrixControlDestination2 => "Matrix Control {} Destination 2",
        F::MatrixControlSens2 => "Matrix Control {} Sens 2",
        F::MatrixControlDestination3 => "Matrix Control {} Destination 3",
        F::MatrixControlSens3 => "Matrix Control {} Sens 3",
        F::MatrixControlDestination4 => "Matrix Control {} Destination 4",
        F::MatrixControlSens4 => "Matrix Control {} Sens 4",
    }
}

/// The host name of a PCM Synth Tone PMT field.
fn pcms_pmt_host_name(field: PcmsPmtField) -> &'static str {
    use PcmsPmtField as F;
    match field {
        F::StructureType12 => "PMT Structure Type 1 & 2",
        F::Booster12 => "PMT Booster 1 & 2",
        F::StructureType34 => "PMT Structure Type 3 & 4",
        F::Booster34 => "PMT Booster 3 & 4",
        F::PmtVelocityControl => "PMT Velocity Control",
        F::PartialSwitch => "PMT{} Partial Switch",
        F::KeyRangeLower => "PMT{} Keyboard Range Lower",
        F::KeyRangeUpper => "PMT{} Keyboard Range Upper",
        F::KeyFadeLower => "PMT{} Keyboard Fade Width Lower",
        F::KeyFadeUpper => "PMT{} Keyboard Fade Width Upper",
        F::VelocityRangeLower => "PMT{} Velocity Range Lower",
        F::VelocityRangeUpper => "PMT{} Velocity Range Upper",
        F::VelocityFadeLower => "PMT{} Velocity Fade Width Lower",
        F::VelocityFadeUpper => "PMT{} Velocity Fade Width Upper",
    }
}

/// The host name of a PCM Synth Tone Partial field.
fn pcms_partial_host_name(field: PcmsPartialField) -> &'static str {
    use PcmsPartialField as F;
    match field {
        F::Level => "Level",
        F::CoarseTune => "Coarse Tune",
        F::FineTune => "Fine Tune",
        F::RandomPitchDepth => "Random Pitch Depth",
        F::Pan => "Pan",
        F::PanKeyfollow => "Pan Keyfollow",
        F::RandomPanDepth => "Random Pan Depth",
        F::AlternatePanDepth => "Alternate Pan Depth",
        F::EnvMode => "Env Mode",
        F::DelayMode => "Delay Mode",
        F::DelayTime => "Delay Time",
        F::OutputLevel => "Output Level",
        F::ChorusSend => "Chorus Send Level",
        F::ReverbSend => "Reverb Send Level",
        F::ReceiveBender => "Receive Bender",
        F::ReceiveExpression => "Receive Expression",
        F::ReceiveHold1 => "Receive Hold-1",
        F::RedamperSwitch => "Redamper Switch",
        F::Control1Switch => "Control 1 Switch {}",
        F::Control2Switch => "Control 2 Switch {}",
        F::Control3Switch => "Control 3 Switch {}",
        F::Control4Switch => "Control 4 Switch {}",
        F::WaveGroupType => "Wave Group Type",
        F::WaveGroupId => "Wave Group ID",
        F::WaveNumberL => "Wave Number L (Mono)",
        F::WaveNumberR => "Wave Number R",
        F::WaveGain => "Wave Gain",
        F::WaveFxmSwitch => "Wave FXM Switch",
        F::WaveFxmColor => "Wave FXM Color",
        F::WaveFxmDepth => "Wave FXM Depth",
        F::WaveTempoSync => "Wave Tempo Sync",
        F::WavePitchKeyfollow => "Wave Pitch Keyfollow",
        F::PitchEnvDepth => "Pitch Env Depth",
        F::PitchEnvVelocitySens => "Pitch Env Velocity Sens",
        F::PitchEnvT1VelocitySens => "Pitch Env Time 1 Velocity Sens",
        F::PitchEnvT4VelocitySens => "Pitch Env Time 4 Velocity Sens",
        F::PitchEnvTimeKeyfollow => "Pitch Env Time Keyfollow",
        F::PitchEnvTime => "Pitch Env Time {}",
        F::PitchEnvLevel => "Pitch Env Level {n0}",
        F::TvfFilterType => "TVF Filter Type",
        F::TvfCutoffFrequency => "TVF Cutoff Frequency",
        F::TvfCutoffKeyfollow => "TVF Cutoff Keyfollow",
        F::TvfCutoffVelocityCurve => "TVF Cutoff Velocity Curve",
        F::TvfCutoffVelocitySens => "TVF Cutoff Velocity Sens",
        F::TvfResonance => "TVF Resonance",
        F::TvfResonanceVelocitySens => "TVF Resonance Velocity Sens",
        F::TvfEnvDepth => "TVF Env Depth",
        F::TvfEnvVelocityCurve => "TVF Env Velocity Curve",
        F::TvfEnvVelocitySens => "TVF Env Velocity Sens",
        F::TvfEnvT1VelocitySens => "TVF Env Time 1 Velocity Sens",
        F::TvfEnvT4VelocitySens => "TVF Env Time 4 Velocity Sens",
        F::TvfEnvTimeKeyfollow => "TVF Env Time Keyfollow",
        F::TvfEnvTime => "TVF Env Time {}",
        F::TvfEnvLevel => "TVF Env Level {n0}",
        F::TvaBiasLevel => "Bias Level",
        F::TvaBiasPosition => "Bias Position",
        F::TvaBiasDirection => "Bias Direction",
        F::TvaLevelVelocityCurve => "TVA Level Velocity Curve",
        F::TvaLevelVelocitySens => "TVA Level Velocity Sens",
        F::TvaEnvT1VelocitySens => "TVA Env Time 1 Velocity Sens",
        F::TvaEnvT4VelocitySens => "TVA Env Time 4 Velocity Sens",
        F::TvaEnvTimeKeyfollow => "TVA Env Time Keyfollow",
        F::TvaEnvTime => "TVA Env Time {}",
        F::TvaEnvLevel => "TVA Env Level {}",
        F::Lfo1Waveform => "LFO1 Waveform",
        F::Lfo1Rate => "LFO1 Rate",
        F::Lfo1Offset => "LFO1 Offset",
        F::Lfo1RateDetune => "LFO1 Rate Detune",
        F::Lfo1DelayTime => "LFO1 Delay Time",
        F::Lfo1DelayTimeKeyfollow => "LFO1 Delay Time Keyfollow",
        F::Lfo1FadeMode => "LFO1 Fade Mode",
        F::Lfo1FadeTime => "LFO1 Fade Time",
        F::Lfo1KeyTrigger => "LFO1 Key Trigger",
        F::Lfo1PitchDepth => "LFO1 Pitch Depth",
        F::Lfo1TvfDepth => "LFO1 TVF Depth",
        F::Lfo1TvaDepth => "LFO1 TVA Depth",
        F::Lfo1PanDepth => "LFO1 Pan Depth",
        F::Lfo2Waveform => "LFO2 Waveform",
        F::Lfo2Rate => "LFO2 Rate",
        F::Lfo2Offset => "LFO2 Offset",
        F::Lfo2RateDetune => "LFO2 Rate Detune",
        F::Lfo2DelayTime => "LFO2 Delay Time",
        F::Lfo2DelayTimeKeyfollow => "LFO2 Delay Time Keyfollow",
        F::Lfo2FadeMode => "LFO2 Fade Mode",
        F::Lfo2FadeTime => "LFO2 Fade Time",
        F::Lfo2KeyTrigger => "LFO2 Key Trigger",
        F::Lfo2PitchDepth => "LFO2 Pitch Depth",
        F::Lfo2TvfDepth => "LFO2 TVF Depth",
        F::Lfo2TvaDepth => "LFO2 TVA Depth",
        F::Lfo2PanDepth => "LFO2 Pan Depth",
        F::LfoStepType => "LFO Step Type",
        F::LfoStep => "LFO Step {}",
    }
}

/// The host name of a PCM Synth Tone Common 2 field.
fn pcms_common2_host_name(field: PcmsCommon2Field) -> &'static str {
    use PcmsCommon2Field as F;
    match field {
        F::ToneCategory => "Tone Category",
        F::ToneNumber => "Tone Number",
        F::PhraseOctaveShift => "Phrase Octave Shift",
        F::TfxSwitch => "TFX Switch",
        F::PhraseNumber => "Phrase Number",
    }
}

/// The host name of a PCM Drum Kit Common 2 field.
fn pcmd_common2_host_name(field: PcmdCommon2Field) -> &'static str {
    match field {
        PcmdCommon2Field::PhraseNumber => "Phrase Number",
        PcmdCommon2Field::TfxSwitch => "TFX Switch",
    }
}

/// The host name of a PCM Drum Kit Common field.
fn pcmd_common_host_name(field: PcmdCommonField) -> &'static str {
    match field {
        PcmdCommonField::KitLevel => "Kit Level",
    }
}

/// The host name of a PCM Drum Kit WMT field. Each holds one value per WMT
/// layer, so the doc's `WMT1`..`WMT4` is the template's number.
fn pcmd_wmt_host_name(field: PcmdWmtField) -> &'static str {
    use PcmdWmtField as F;
    match field {
        F::Switch => "WMT{} Wave Switch",
        F::WaveGroupType => "WMT{} Wave Group Type",
        F::WaveGroupId => "WMT{} Wave Group ID",
        F::WaveNumberL => "WMT{} Wave Number L (Mono)",
        F::WaveNumberR => "WMT{} Wave Number R",
        F::WaveGain => "WMT{} Wave Gain",
        F::WaveFxmSwitch => "WMT{} Wave FXM Switch",
        F::WaveFxmColor => "WMT{} Wave FXM Color",
        F::WaveFxmDepth => "WMT{} Wave FXM Depth",
        F::WaveTempoSync => "WMT{} Wave Tempo Sync",
        F::CoarseTune => "WMT{} Wave Coarse Tune",
        F::FineTune => "WMT{} Wave Fine Tune",
        F::Pan => "WMT{} Wave Pan",
        F::RandomPanSwitch => "WMT{} Wave Random Pan Switch",
        F::AlternatePanSwitch => "WMT{} Wave Alternate Pan Switch",
        F::Level => "WMT{} Wave Level",
        F::VelocityRangeLower => "WMT{} Velocity Range Lower",
        F::VelocityRangeUpper => "WMT{} Velocity Range Upper",
        F::VelocityFadeLower => "WMT{} Velocity Fade Width Lower",
        F::VelocityFadeUpper => "WMT{} Velocity Fade Width Upper",
    }
}

/// The host name of a PCM Drum Kit Partial field outside the WMT layers.
fn pcmd_partial_host_name(field: PcmdPartialField) -> &'static str {
    use PcmdPartialField as F;
    match field {
        F::AssignType => "Assign Type",
        F::MuteGroup => "Mute Group",
        F::Level => "Level",
        F::CoarseTune => "Coarse Tune",
        F::FineTune => "Fine Tune",
        F::RandomPitchDepth => "Random Pitch Depth",
        F::Pan => "Pan",
        F::RandomPanDepth => "Random Pan Depth",
        F::AlternatePanDepth => "Alternate Pan Depth",
        F::EnvMode => "Env Mode",
        F::OutputLevel => "Output Level",
        F::ChorusSend => "Chorus Send Level",
        F::ReverbSend => "Reverb Send Level",
        F::OutputAssign => "Output Assign",
        F::PitchBendRange => "Pitch Bend Range",
        F::ReceiveExpression => "Receive Expression",
        F::ReceiveHold1 => "Receive Hold-1",
        F::WmtVelocityControl => "WMT Velocity Control",
        F::PitchEnvDepth => "Pitch Env Depth",
        F::PitchEnvVelocitySens => "Pitch Env Velocity Sens",
        F::PitchEnvT1VelocitySens => "Pitch Env Time 1 Velocity Sens",
        F::PitchEnvT4VelocitySens => "Pitch Env Time 4 Velocity Sens",
        F::PitchEnvTime => "Pitch Env Time {}",
        F::PitchEnvLevel => "Pitch Env Level {n0}",
        F::TvfFilterType => "TVF Filter Type",
        F::TvfCutoffFrequency => "TVF Cutoff Frequency",
        F::TvfCutoffVelocityCurve => "TVF Cutoff Velocity Curve",
        F::TvfCutoffVelocitySens => "TVF Cutoff Velocity Sens",
        F::TvfResonance => "TVF Resonance",
        F::TvfResonanceVelocitySens => "TVF Resonance Velocity Sens",
        F::TvfEnvDepth => "TVF Env Depth",
        F::TvfEnvVelocityCurve => "TVF Env Velocity Curve Type",
        F::TvfEnvVelocitySens => "TVF Env Velocity Sens",
        F::TvfEnvT1VelocitySens => "TVF Env Time 1 Velocity Sens",
        F::TvfEnvT4VelocitySens => "TVF Env Time 4 Velocity Sens",
        F::TvfEnvTime => "TVF Env Time {}",
        F::TvfEnvLevel => "TVF Env Level {n0}",
        F::TvaLevelVelocityCurve => "TVA Level Velocity Curve",
        F::TvaLevelVelocitySens => "TVA Level Velocity Sens",
        F::TvaEnvT1VelocitySens => "TVA Env Time 1 Velocity Sens",
        F::TvaEnvT4VelocitySens => "TVA Env Time 4 Velocity Sens",
        F::TvaEnvTime => "TVA Env Time {}",
        F::TvaEnvLevel => "TVA Env Level {}",
        F::OneShotMode => "One Shot Mode",
    }
}

/// The host name of an SN Synth Tone Common field.
fn sns_common_host_name(field: SnsCommonField) -> &'static str {
    use SnsCommonField as F;
    match field {
        F::ToneLevel => "Tone Level",
        F::PortamentoSwitch => "Portamento Switch",
        F::PortamentoTime => "Portamento Time",
        F::MonoSwitch => "Mono Switch",
        F::OctaveShift => "Octave Shift",
        F::PitchBendRangeUp => "Pitch Bend Range Up",
        F::PitchBendRangeDown => "Pitch Bend Range Down",
        F::Partial1Switch => "Partial 1 Switch",
        F::Partial1Select => "Partial 1 Select",
        F::Partial2Switch => "Partial 2 Switch",
        F::Partial2Select => "Partial 2 Select",
        F::Partial3Switch => "Partial 3 Switch",
        F::Partial3Select => "Partial 3 Select",
        F::RingSwitch => "Ring Switch",
        F::TfxSwitch => "TFX Switch",
        F::UnisonSwitch => "Unison Switch",
        F::PortamentoMode => "Portamento Mode",
        F::LegatoSwitch => "Legato Switch",
        F::AnalogFeel => "Analog Feel",
        F::WaveShape => "Wave Shape",
        F::ToneCategory => "Tone Category",
        F::PhraseNumber => "Phrase Number",
        F::PhraseOctaveShift => "Phrase Octave Shift",
        F::UnisonSize => "Unison Size",
    }
}

/// The host name of an SN Synth Tone Partial field.
fn sns_partial_host_name(field: SnsPartialField) -> &'static str {
    use SnsPartialField as F;
    match field {
        F::OscWave => "OSC Wave",
        F::OscWaveVariation => "OSC Wave Variation",
        F::OscPitch => "OSC Pitch",
        F::OscDetune => "OSC Detune",
        F::OscPwModDepth => "OSC Pulse Width Mod Depth",
        F::OscPulseWidth => "OSC Pulse Width",
        F::OscPitchEnvAttack => "OSC Pitch Env Attack Time",
        F::OscPitchEnvDecay => "OSC Pitch Env Decay",
        F::OscPitchEnvDepth => "OSC Pitch Env Depth",
        F::FilterMode => "Filter Mode",
        F::FilterSlope => "Filter Slope",
        F::FilterCutoff => "Filter Cutoff",
        F::FilterKeyfollow => "Filter Cutoff Keyfollow",
        F::FilterEnvVelSens => "Filter Env Velocity Sens",
        F::FilterResonance => "Filter Resonance",
        F::FilterEnvAttack => "Filter Env Attack Time",
        F::FilterEnvDecay => "Filter Env Decay Time",
        F::FilterEnvSustain => "Filter Env Sustain Level",
        F::FilterEnvRelease => "Filter Env Release Time",
        F::FilterEnvDepth => "Filter Env Depth",
        F::AmpLevel => "Amp Level",
        F::AmpVelSens => "Amp Level Velocity Sens",
        F::AmpEnvAttack => "Amp Env Attack Time",
        F::AmpEnvDecay => "Amp Env Decay Time",
        F::AmpEnvSustain => "Amp Env Sustain Level",
        F::AmpEnvRelease => "Amp Env Release Time",
        F::AmpPan => "Amp Pan",
        F::LfoShape => "LFO Shape",
        F::LfoRate => "LFO Rate",
        F::LfoTempoSync => "LFO Tempo Sync Switch",
        F::LfoTempoSyncNote => "LFO Tempo Sync Note",
        F::LfoFadeTime => "LFO Fade Time",
        F::LfoKeyTrigger => "LFO Key Trigger",
        F::LfoPitchDepth => "LFO Pitch Depth",
        F::LfoFilterDepth => "LFO Filter Depth",
        F::LfoAmpDepth => "LFO Amp Depth",
        F::LfoPanDepth => "LFO Pan Depth",
        F::ModLfoShape => "Modulation LFO Shape",
        F::ModLfoRate => "Modulation LFO Rate",
        F::ModLfoTempoSync => "Modulation LFO Tempo Sync Switch",
        F::ModLfoTempoSyncNote => "Modulation LFO Tempo Sync Note",
        F::PwShift => "OSC Pulse Width Shift",
        F::ModLfoPitchDepth => "Modulation LFO Pitch Depth",
        F::ModLfoFilterDepth => "Modulation LFO Filter Depth",
        F::ModLfoAmpDepth => "Modulation LFO Amp Depth",
        F::ModLfoPanDepth => "Modulation LFO Pan Depth",
        F::AftertouchCutoff => "Cutoff Aftertouch Sens",
        F::AftertouchLevel => "Level Aftertouch Sens",
        F::WaveGain => "Wave Gain",
        F::WaveNumber => "Wave Number",
        F::HpfCutoff => "HPF Cutoff",
        F::SuperSawDetune => "Super Saw Detune",
        F::ModLfoRateControl => "Modulation LFO Rate Control",
        F::AmpLevelKeyfollow => "Amp Level Keyfollow",
    }
}

/// The host name of an SN Acoustic Tone Common field.
fn sna_common_host_name(field: SnaCommonField) -> &'static str {
    use SnaCommonField as F;
    match field {
        F::ToneLevel => "Tone Level",
        F::MonoPoly => "Mono/Poly",
        F::PortamentoTimeOffset => "Portamento Time Offset",
        F::CutoffOffset => "Cutoff Offset",
        F::ResonanceOffset => "Resonance Offset",
        F::AttackTimeOffset => "Attack Time Offset",
        F::ReleaseTimeOffset => "Release Time Offset",
        F::VibratoRate => "Vibrato Rate",
        F::VibratoDepth => "Vibrato Depth",
        F::VibratoDelay => "Vibrato Delay",
        F::OctaveShift => "Octave Shift",
        F::Category => "Category",
        F::PhraseNumber => "Phrase Number",
        F::PhraseOctaveShift => "Phrase Octave Shift",
        F::TfxSwitch => "TFX Switch",
        F::InstVariation => "Inst Variation",
        F::InstNumber => "Inst Number",
        F::ModifyParameter => "Modify Parameter {}",
    }
}

/// The host name of an SN Drum Kit Common field.
fn snd_common_host_name(field: SndCommonField) -> &'static str {
    use SndCommonField as F;
    match field {
        F::KitLevel => "Kit Level",
        F::AmbienceLevel => "Ambience Level",
        F::PhraseNumber => "Phrase Number",
        F::TfxSwitch => "TFX Switch",
    }
}

/// The host name of an SN Drum Kit Note field.
fn snd_note_host_name(field: SndNoteField) -> &'static str {
    use SndNoteField as F;
    match field {
        F::InstNumber => "Inst Number",
        F::Level => "Level",
        F::Pan => "Pan",
        F::ChorusSend => "Chorus Send Level",
        F::ReverbSend => "Reverb Send Level",
        F::Tune => "Tune",
        F::Attack => "Attack",
        F::Decay => "Decay",
        F::Brilliance => "Brilliance",
        F::Variation => "Variation",
        F::DynamicRange => "Dynamic Range",
        F::StereoWidth => "Stereo Width",
        F::OutputAssign => "Output Assign",
    }
}

// Each `*_family` function below starts every field from its section in the
// web editor and notes the fields whose family differs (`CONTEXT.md`,
// **Parameter family**): Pan is Time, LFO fields are Modulation, envelope
// times and velocity sensitivities follow what they shape, on/off switches
// and structure selectors are Common.

/// The family of a PCM Synth Tone Common field (section COMMON).
fn pcms_common_family(field: PcmsCommonField) -> ParameterFamily {
    use ParameterFamily::*;
    use PcmsCommonField as F;
    match field {
        F::TonePriority
        | F::MonoPoly
        | F::LegatoSwitch
        | F::LegatoRetrigger
        | F::PortamentoSwitch
        | F::PortamentoMode
        | F::PortamentoType
        | F::PortamentoStart
        | F::PmtControlSwitch => Common,
        // Modulation routing, like the MFX Control Source/Sens/Assign.
        F::MatrixControlSource
        | F::MatrixControlDestination1
        | F::MatrixControlDestination2
        | F::MatrixControlDestination3
        | F::MatrixControlDestination4
        | F::MatrixControlSens1
        | F::MatrixControlSens2
        | F::MatrixControlSens3
        | F::MatrixControlSens4 => Modulation,
        // Exceptions.
        F::CoarseTune
        | F::FineTune
        | F::OctaveShift
        | F::StretchTuneDepth
        | F::PitchBendRangeUp
        | F::PitchBendRangeDown
        | F::PortamentoTime => Pitch,
        F::CutoffOffset | F::ResonanceOffset => Filter,
        F::ToneLevel | F::AttackTimeOffset | F::ReleaseTimeOffset | F::VelocitySensOffset => Amp,
        F::AnalogFeel => Modulation,
        F::TonePan => Time,
    }
}

/// The family of a PCM Synth Tone PMT field (section COMMON).
fn pcms_pmt_family(field: PcmsPmtField) -> ParameterFamily {
    use ParameterFamily::*;
    use PcmsPmtField as F;
    match field {
        F::StructureType12
        | F::StructureType34
        | F::PmtVelocityControl
        | F::PartialSwitch
        | F::KeyRangeLower
        | F::KeyRangeUpper
        | F::KeyFadeLower
        | F::KeyFadeUpper
        | F::VelocityRangeLower
        | F::VelocityRangeUpper
        | F::VelocityFadeLower
        | F::VelocityFadeUpper => Common,
        // Exception: the Booster drives the pair into distortion, like an
        // effect's Drive.
        F::Booster12 | F::Booster34 => Filter,
    }
}

/// The family of a PCM Synth Tone Partial field.
fn pcms_partial_family(field: PcmsPartialField) -> ParameterFamily {
    use ParameterFamily::*;
    use PcmsPartialField as F;
    match field {
        // WAVE and PITCH.
        F::WaveGroupType
        | F::WaveGroupId
        | F::WaveNumberL
        | F::WaveNumberR
        | F::WaveFxmColor
        | F::WaveFxmDepth
        | F::CoarseTune
        | F::FineTune
        | F::RandomPitchDepth
        | F::WavePitchKeyfollow
        | F::PitchEnvDepth
        | F::PitchEnvVelocitySens
        | F::PitchEnvTimeKeyfollow
        | F::PitchEnvT1VelocitySens
        | F::PitchEnvT4VelocitySens
        | F::PitchEnvTime
        | F::PitchEnvLevel => Pitch,
        // WAVE exceptions.
        F::WaveGain => Amp,
        F::WaveFxmSwitch | F::WaveTempoSync => Common,
        // FILTER.
        F::TvfFilterType
        | F::TvfCutoffKeyfollow
        | F::TvfCutoffFrequency
        | F::TvfResonance
        | F::TvfCutoffVelocityCurve
        | F::TvfEnvVelocityCurve
        | F::TvfCutoffVelocitySens
        | F::TvfResonanceVelocitySens
        | F::TvfEnvDepth
        | F::TvfEnvVelocitySens
        | F::TvfEnvTimeKeyfollow
        | F::TvfEnvT1VelocitySens
        | F::TvfEnvT4VelocitySens
        | F::TvfEnvTime
        | F::TvfEnvLevel => Filter,
        // AMP.
        F::Level
        | F::TvaLevelVelocityCurve
        | F::TvaLevelVelocitySens
        | F::TvaBiasLevel
        | F::TvaBiasPosition
        | F::TvaBiasDirection
        | F::TvaEnvTimeKeyfollow
        | F::TvaEnvT1VelocitySens
        | F::TvaEnvT4VelocitySens
        | F::TvaEnvTime
        | F::TvaEnvLevel => Amp,
        // AMP exceptions.
        F::Pan | F::PanKeyfollow | F::RandomPanDepth | F::AlternatePanDepth => Time,
        // LFO1, LFO2 and STEP LFO.
        F::Lfo1Waveform
        | F::Lfo1Rate
        | F::Lfo1RateDetune
        | F::Lfo1Offset
        | F::Lfo1DelayTime
        | F::Lfo1DelayTimeKeyfollow
        | F::Lfo1FadeMode
        | F::Lfo1FadeTime
        | F::Lfo1KeyTrigger
        | F::Lfo1PitchDepth
        | F::Lfo1TvfDepth
        | F::Lfo1TvaDepth
        | F::Lfo1PanDepth
        | F::Lfo2Waveform
        | F::Lfo2Rate
        | F::Lfo2RateDetune
        | F::Lfo2Offset
        | F::Lfo2DelayTime
        | F::Lfo2DelayTimeKeyfollow
        | F::Lfo2FadeMode
        | F::Lfo2FadeTime
        | F::Lfo2KeyTrigger
        | F::Lfo2PitchDepth
        | F::Lfo2TvfDepth
        | F::Lfo2TvaDepth
        | F::Lfo2PanDepth
        | F::LfoStepType
        | F::LfoStep => Modulation,
        // CTRL, and the switches no editor shows yet.
        F::EnvMode
        | F::ReceiveBender
        | F::ReceiveExpression
        | F::ReceiveHold1
        | F::RedamperSwitch
        | F::Control1Switch
        | F::Control2Switch
        | F::Control3Switch
        | F::Control4Switch => Common,
        // OUTPUT.
        F::ChorusSend | F::ReverbSend | F::OutputLevel => Amp,
        // OUTPUT exceptions: the partial's delay before it sounds.
        F::DelayMode | F::DelayTime => Time,
    }
}

/// The family of a PCM Synth Tone Common 2 field: the category and the
/// preview phrase, none of which changes the sound.
fn pcms_common2_family(field: PcmsCommon2Field) -> ParameterFamily {
    use PcmsCommon2Field as F;
    match field {
        F::ToneCategory | F::ToneNumber | F::PhraseOctaveShift | F::TfxSwitch | F::PhraseNumber => {
            ParameterFamily::Common
        }
    }
}

/// The family of a PCM Drum Kit Common field (section DRUM KEY).
fn pcmd_common_family(field: PcmdCommonField) -> ParameterFamily {
    match field {
        // Exception.
        PcmdCommonField::KitLevel => ParameterFamily::Amp,
    }
}

/// The family of a PCM Drum Kit WMT field (section WAVE).
fn pcmd_wmt_family(field: PcmdWmtField) -> ParameterFamily {
    use ParameterFamily::*;
    use PcmdWmtField as F;
    match field {
        F::WaveGroupType
        | F::WaveGroupId
        | F::WaveNumberL
        | F::WaveNumberR
        | F::WaveFxmColor
        | F::WaveFxmDepth
        | F::CoarseTune
        | F::FineTune => Pitch,
        // Exceptions.
        F::WaveGain | F::Level => Amp,
        F::Pan | F::AlternatePanSwitch => Time,
        F::Switch
        | F::WaveFxmSwitch
        | F::WaveTempoSync
        | F::RandomPanSwitch
        | F::VelocityRangeLower
        | F::VelocityRangeUpper
        | F::VelocityFadeLower
        | F::VelocityFadeUpper => Common,
    }
}

/// The family of a PCM Drum Kit Partial field outside the WMT layers.
fn pcmd_partial_family(field: PcmdPartialField) -> ParameterFamily {
    use ParameterFamily::*;
    use PcmdPartialField as F;
    match field {
        // LEVEL / PAN.
        F::ChorusSend | F::ReverbSend | F::OutputLevel => Amp,
        // LEVEL / PAN exception, and the pan depths no editor shows yet.
        F::Pan | F::RandomPanDepth | F::AlternatePanDepth => Time,
        // PITCH ENV.
        F::PitchEnvDepth
        | F::PitchEnvVelocitySens
        | F::PitchEnvT1VelocitySens
        | F::PitchEnvT4VelocitySens
        | F::PitchEnvTime
        | F::PitchEnvLevel => Pitch,
        // FILTER.
        F::TvfFilterType
        | F::TvfCutoffVelocityCurve
        | F::TvfEnvVelocityCurve
        | F::TvfCutoffFrequency
        | F::TvfCutoffVelocitySens
        | F::TvfResonance
        | F::TvfResonanceVelocitySens
        | F::TvfEnvDepth
        | F::TvfEnvVelocitySens
        | F::TvfEnvT1VelocitySens
        | F::TvfEnvT4VelocitySens
        | F::TvfEnvTime
        | F::TvfEnvLevel => Filter,
        // AMP.
        F::TvaLevelVelocityCurve
        | F::TvaLevelVelocitySens
        | F::TvaEnvT1VelocitySens
        | F::TvaEnvT4VelocitySens
        | F::TvaEnvTime
        | F::TvaEnvLevel => Amp,
        // MISC; WMT Velocity Control sits in WAVE.
        F::AssignType
        | F::MuteGroup
        | F::EnvMode
        | F::OutputAssign
        | F::OneShotMode
        | F::ReceiveExpression
        | F::ReceiveHold1
        | F::WmtVelocityControl => Common,
        // MISC exceptions.
        F::PitchBendRange | F::CoarseTune | F::FineTune | F::RandomPitchDepth => Pitch,
        F::Level => Amp,
    }
}

/// The family of an SN Synth Tone Common field (section COMMON).
fn sns_common_family(field: SnsCommonField) -> ParameterFamily {
    use ParameterFamily::*;
    use SnsCommonField as F;
    match field {
        F::ToneCategory
        | F::PortamentoSwitch
        | F::PortamentoMode
        | F::MonoSwitch
        | F::LegatoSwitch
        | F::UnisonSwitch
        | F::UnisonSize
        | F::RingSwitch
        | F::Partial1Switch
        | F::Partial1Select
        | F::Partial2Switch
        | F::Partial2Select
        | F::Partial3Switch
        | F::Partial3Select
        | F::TfxSwitch
        | F::PhraseNumber
        | F::PhraseOctaveShift => Common,
        // Exceptions. Wave Shape is Partial 1 modulating Partial 2's pitch
        // at audio rate: a source, not movement.
        F::PortamentoTime
        | F::OctaveShift
        | F::PitchBendRangeUp
        | F::PitchBendRangeDown
        | F::WaveShape => Pitch,
        F::ToneLevel => Amp,
        F::AnalogFeel => Modulation,
    }
}

/// The family of an SN Synth Tone Partial field.
fn sns_partial_family(field: SnsPartialField) -> ParameterFamily {
    use ParameterFamily::*;
    use SnsPartialField as F;
    match field {
        // OSC.
        F::OscWave
        | F::OscWaveVariation
        | F::WaveNumber
        | F::OscPitch
        | F::OscDetune
        | F::OscPulseWidth
        | F::PwShift
        | F::SuperSawDetune
        | F::OscPitchEnvAttack
        | F::OscPitchEnvDecay
        | F::OscPitchEnvDepth => Pitch,
        // OSC exceptions: PW Mod Depth is the LFO's depth on pulse width.
        F::HpfCutoff => Filter,
        F::OscPwModDepth => Modulation,
        // FILTER.
        F::FilterMode
        | F::FilterSlope
        | F::FilterCutoff
        | F::FilterResonance
        | F::FilterKeyfollow
        | F::FilterEnvVelSens
        | F::AftertouchCutoff
        | F::FilterEnvAttack
        | F::FilterEnvDecay
        | F::FilterEnvSustain
        | F::FilterEnvRelease
        | F::FilterEnvDepth => Filter,
        // AMP.
        F::WaveGain
        | F::AmpLevel
        | F::AmpVelSens
        | F::AftertouchLevel
        | F::AmpLevelKeyfollow
        | F::AmpEnvAttack
        | F::AmpEnvDecay
        | F::AmpEnvSustain
        | F::AmpEnvRelease => Amp,
        // AMP exception.
        F::AmpPan => Time,
        // LFO and MOD LFO.
        F::LfoShape
        | F::LfoRate
        | F::LfoTempoSync
        | F::LfoTempoSyncNote
        | F::LfoFadeTime
        | F::LfoKeyTrigger
        | F::LfoPitchDepth
        | F::LfoFilterDepth
        | F::LfoAmpDepth
        | F::LfoPanDepth
        | F::ModLfoShape
        | F::ModLfoRate
        | F::ModLfoTempoSync
        | F::ModLfoTempoSyncNote
        | F::ModLfoRateControl
        | F::ModLfoPitchDepth
        | F::ModLfoFilterDepth
        | F::ModLfoAmpDepth
        | F::ModLfoPanDepth => Modulation,
    }
}

/// The family of an SN Acoustic Tone Common field.
fn sna_common_family(field: SnaCommonField) -> ParameterFamily {
    use ParameterFamily::*;
    use SnaCommonField as F;
    match field {
        // INSTRUMENT.
        F::InstNumber | F::InstVariation | F::ModifyParameter => Pitch,
        // COMMON, and the category and phrase no editor shows.
        F::MonoPoly | F::Category | F::PhraseNumber | F::PhraseOctaveShift | F::TfxSwitch => Common,
        // COMMON exceptions.
        F::OctaveShift | F::PortamentoTimeOffset => Pitch,
        // VIB.
        F::VibratoRate | F::VibratoDepth | F::VibratoDelay => Modulation,
        // FILTER.
        F::CutoffOffset | F::ResonanceOffset => Filter,
        // AMP.
        F::AttackTimeOffset | F::ReleaseTimeOffset | F::ToneLevel => Amp,
    }
}

/// The family of an SN Drum Kit Common field (section DRUM INST).
fn snd_common_family(field: SndCommonField) -> ParameterFamily {
    use ParameterFamily::*;
    use SndCommonField as F;
    match field {
        F::PhraseNumber | F::TfxSwitch => Common,
        // Exceptions.
        F::KitLevel | F::AmbienceLevel => Amp,
    }
}

/// The family of an SN Drum Kit Note field.
fn snd_note_family(field: SndNoteField) -> ParameterFamily {
    use ParameterFamily::*;
    use SndNoteField as F;
    match field {
        // TONE.
        F::InstNumber | F::Tune | F::Variation => Pitch,
        // TONE exceptions.
        F::Attack | F::Decay | F::DynamicRange => Amp,
        F::Brilliance => Filter,
        F::StereoWidth => Time,
        F::OutputAssign => Common,
        // LEVEL / PAN.
        F::Level | F::ChorusSend | F::ReverbSend => Amp,
        // LEVEL / PAN exception.
        F::Pan => Time,
    }
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
    if !matches!(
        pcms_partial_spec(field).format,
        ParamFormat::TimeOrNote | ParamFormat::PcmLfoRate
    ) {
        return None;
    }
    let quarter = NOTE_NAMES
        .iter()
        .position(|n| *n == "1/4")
        .expect("1/4 is a note value") as i32;
    Some(InRangeSync {
        // The delay time's free value is a bare 0-127 step, with no unit:
        // its toggle reads as a clock.
        labels: if field == PcmsPartialField::DelayTime {
            SYNC_CLOCK
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
    use crate::block_field::test_support::doc_rows;
    use crate::mfx::{MfxHeaderField, mfx_header_offset};

    const PCMS_DOC: &str = include_str!("../../../docs/midi/06-pcm-synth-tone.md");
    const PCMD_DOC: &str = include_str!("../../../docs/midi/07-pcm-drum-kit.md");
    const SNS_DOC: &str = include_str!("../../../docs/midi/08-supernatural-synth-tone.md");
    const SNA_DOC: &str = include_str!("../../../docs/midi/09-supernatural-acoustic-tone.md");
    const SND_DOC: &str = include_str!("../../../docs/midi/10-supernatural-drum-kit.md");

    /// A name compared without case or spaces: the doc writes `Partial1
    /// Switch` and `FILTER Mode`, and marks a nibblized row with a `#`.
    fn squash(name: &str) -> String {
        name.chars()
            .filter(|c| !c.is_whitespace() && *c != '#')
            .flat_map(char::to_lowercase)
            .collect()
    }

    /// How a section's host names differ from its doc names, besides case
    /// and spacing: the doc may begin every name with a word the prefix
    /// stands for (`Partial Level` is the host's `Level`), and a host name
    /// may begin with a block word the doc leaves out (`PMT Booster 1 & 2`
    /// is the doc's `Booster 1 & 2`).
    #[derive(Clone, Copy)]
    struct Affixes {
        doc_prefix: &'static str,
        host_word: &'static str,
    }

    const PLAIN: Affixes = Affixes {
        doc_prefix: "",
        host_word: "",
    };

    /// Check every documented row against the field covering its position.
    fn assert_matches_doc<F: BlockField>(
        doc: &str,
        heading: &str,
        keep: impl Fn(&str) -> bool,
        fields: &[F],
        affixes: Affixes,
        spec_of: impl Fn(F) -> FieldSpec,
    ) {
        let rows = doc_rows(doc, heading, keep);
        assert!(!rows.is_empty(), "{heading}: no rows");
        for row in rows {
            let (field, index) = fields
                .iter()
                .copied()
                .find_map(|f| {
                    (0..f.count())
                        .find(|i| f.position() + i * f.stride() == row.position)
                        .map(|i| (f, i))
                })
                .unwrap_or_else(|| {
                    panic!("{heading}: no field at {} ({})", row.position, row.name)
                });
            let spec = spec_of(field);
            assert_eq!(
                (spec.min, spec.max),
                row.range.unwrap(),
                "{} ({field:?})",
                row.name
            );
            let (doc_name, host) = (squash(&row.name), squash(&spec.element_name(index)));
            assert!(
                doc_name == host
                    || doc_name == format!("{}{host}", squash(affixes.doc_prefix))
                    || host == format!("{}{doc_name}", squash(affixes.host_word)),
                "{field:?}[{index}]: doc says {:?}, host name is {:?}",
                row.name,
                spec.element_name(index)
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
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone Common",
            |name| !name.starts_with("PCM Synth Tone Name"),
            PcmsCommonField::ALL,
            Affixes {
                doc_prefix: "PCM Synth",
                host_word: "",
            },
            pcms_common_spec,
        );
    }

    #[test]
    fn pmt_specs_match_the_doc() {
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone PMT (Partial Mix Table)",
            |_| true,
            PcmsPmtField::ALL,
            Affixes {
                doc_prefix: "",
                host_word: "PMT",
            },
            pcms_pmt_spec,
        );
    }

    #[test]
    fn partial_specs_match_the_doc() {
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone Partial",
            |_| true,
            PcmsPartialField::ALL,
            Affixes {
                doc_prefix: "Partial",
                host_word: "",
            },
            pcms_partial_spec,
        );
    }

    #[test]
    fn common2_specs_match_the_doc() {
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone Common 2",
            |_| true,
            PcmsCommon2Field::ALL,
            PLAIN,
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
        assert_eq!(fmt(PcmsPartialField::Lfo1Rate, 0), "0.05 Hz");
        assert_eq!(fmt(PcmsPartialField::Lfo1Rate, 127), "27.1 Hz");
        assert_eq!(fmt(PcmsPartialField::Lfo1Rate, 128), "1/64T");
        assert_eq!(fmt(PcmsPartialField::Lfo1Rate, 149), "2/1");
        assert_eq!(fmt(PcmsPartialField::LfoStep, 28), "-36");
        let center = |f| pcms_partial_spec(f).format.center();
        assert_eq!(center(PcmsPartialField::PitchEnvTimeKeyfollow), Some(64));
        assert_eq!(center(PcmsPartialField::Pan), Some(64));
        assert_eq!(center(PcmsPartialField::Level), None);
    }

    /// An SN Synth PCM Wave and an SN Drum Inst read as their catalog names,
    /// the labels a picker lists.
    #[test]
    fn catalog_fields_show_names() {
        let wave = sns_partial_spec(SnsPartialField::WaveNumber).format;
        let inst = snd_note_spec(SndNoteField::InstNumber).format;
        assert_eq!(wave.format(0), "OFF");
        assert_eq!(inst.format(0), "OFF");
        assert_eq!(inst.format(1), "Studio Kick");
        assert_eq!(inst.format(294), "Telephone 3");
        // A number the catalog doesn't name shows as itself.
        assert_eq!(inst.format(295), "295");
        for (format, count) in [
            (
                &wave,
                crate::waves::waves(crate::waves::WaveGroup::SnSynth).len(),
            ),
            (&inst, sn_drum::sn_drum_insts().len()),
        ] {
            let ParamFormat::Catalog(catalog) = format else {
                panic!("{format:?} is not a catalog format");
            };
            let options = catalog.options();
            assert_eq!(options[0].label, "OFF");
            assert_eq!(options.len(), count + 1);
            for option in &options {
                assert_eq!(format.format(option.value), option.label);
            }
        }
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
            ["⏱", "♪"]
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

    /// A synced field's free value reads as a number with no unit or its
    /// toggle's free label as its unit, and its note with no unit: the
    /// readout draws the label as the unit.
    #[test]
    fn synced_fields_split_their_unit_off_as_the_toggle_label() {
        use PcmsPartialField as P;
        use SnsPartialField as S;
        // The delay time's free value is a bare 0-127, under its clock label.
        for (field, free_unit) in [(P::Lfo1Rate, "Hz"), (P::Lfo2Rate, "Hz"), (P::DelayTime, "")] {
            let sync = pcms_partial_sync(field).unwrap();
            let spec = pcms_partial_spec(field);
            for v in spec.min..=spec.max {
                let expected = if v < sync.note_from { free_unit } else { "" };
                assert_eq!(spec.format.parts(v).1, expected, "{field:?} {v}");
            }
            assert!(
                free_unit.is_empty() || free_unit == sync.labels[0],
                "{field:?}"
            );
        }
        for field in [S::LfoRate, S::ModLfoRate] {
            let sync = sns_partial_sync(field).unwrap();
            let spec = sns_partial_spec(field);
            for v in spec.min..=spec.max {
                assert_eq!(spec.format.parts(v).1, sync.labels[0], "{field:?} {v}");
            }
            let note = sns_partial_spec(sync.note);
            for v in note.min..=note.max {
                assert_eq!(note.format.parts(v).1, "", "{:?} {v}", sync.note);
            }
        }
        let (number, unit) = sns_partial_spec(S::LfoRate).format.parts(0);
        assert_eq!((number.as_str(), unit), ("0.03", "Hz"));
    }

    #[test]
    fn sn_lfo_rates_show_hz() {
        use SnsPartialField as F;
        let fmt = |f: SnsPartialField, v| sns_partial_spec(f).format.format(v);
        assert_eq!(fmt(F::LfoRate, 0), "0.03 Hz");
        assert_eq!(fmt(F::LfoRate, 64), "1.9 Hz");
        assert_eq!(fmt(F::LfoRate, 127), "26.9 Hz");
        assert_eq!(fmt(F::ModLfoRate, 126), "25.6 Hz");
    }

    #[test]
    fn every_engine_classifies_one_field_per_family() {
        use ParameterFamily::*;
        let pcms = [
            pcms_partial_spec(PcmsPartialField::CoarseTune).family,
            pcms_partial_spec(PcmsPartialField::TvfCutoffFrequency).family,
            pcms_partial_spec(PcmsPartialField::Level).family,
            pcms_partial_spec(PcmsPartialField::Lfo1Rate).family,
            pcms_partial_spec(PcmsPartialField::DelayTime).family,
            pcms_pmt_spec(PcmsPmtField::StructureType12).family,
        ];
        assert_eq!(pcms, ParameterFamily::ALL);
        let pcmd = [
            pcmd_wmt_spec(PcmdWmtField::WaveNumberL).family,
            pcmd_partial_spec(PcmdPartialField::TvfResonance).family,
            pcmd_common_spec(PcmdCommonField::KitLevel).family,
            // PCM Drum Kits have no LFO.
            pcmd_partial_spec(PcmdPartialField::AlternatePanDepth).family,
            pcmd_partial_spec(PcmdPartialField::MuteGroup).family,
        ];
        assert_eq!(pcmd, [Pitch, Filter, Amp, Time, Common]);
        let sns = [
            sns_partial_spec(SnsPartialField::OscPitch).family,
            sns_partial_spec(SnsPartialField::FilterCutoff).family,
            sns_partial_spec(SnsPartialField::AmpLevel).family,
            sns_partial_spec(SnsPartialField::ModLfoRate).family,
            sns_partial_spec(SnsPartialField::AmpPan).family,
            sns_common_spec(SnsCommonField::MonoSwitch).family,
        ];
        assert_eq!(sns, ParameterFamily::ALL);
        let sna = [
            sna_common_spec(SnaCommonField::InstNumber).family,
            sna_common_spec(SnaCommonField::CutoffOffset).family,
            sna_common_spec(SnaCommonField::ToneLevel).family,
            sna_common_spec(SnaCommonField::VibratoDepth).family,
            // SN Acoustic Tones have no pan: no Time / Space field.
            sna_common_spec(SnaCommonField::MonoPoly).family,
        ];
        assert_eq!(sna, [Pitch, Filter, Amp, Modulation, Common]);
        let snd = [
            snd_note_spec(SndNoteField::Tune).family,
            snd_note_spec(SndNoteField::Brilliance).family,
            snd_note_spec(SndNoteField::Level).family,
            // SN Drum Kits have no LFO.
            snd_note_spec(SndNoteField::StereoWidth).family,
            snd_common_spec(SndCommonField::TfxSwitch).family,
        ];
        assert_eq!(snd, [Pitch, Filter, Amp, Time, Common]);
    }

    #[test]
    fn pan_is_time_and_lfo_depth_is_modulation() {
        assert_eq!(
            pcms_partial_spec(PcmsPartialField::Pan).family,
            ParameterFamily::Time
        );
        assert_eq!(
            pcms_partial_spec(PcmsPartialField::Lfo1PitchDepth).family,
            ParameterFamily::Modulation
        );
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
            PcmdCommonField::ALL,
            PLAIN,
            pcmd_common_spec,
        );
    }

    #[test]
    fn pcmd_common2_specs_match_the_doc() {
        assert_matches_doc(
            PCMD_DOC,
            "PCM Drum Kit Common 2",
            |_| true,
            PcmdCommon2Field::ALL,
            PLAIN,
            pcmd_common2_spec,
        );
    }

    #[test]
    fn mfx_header_specs_match_the_doc() {
        assert_matches_doc(
            PCMS_DOC,
            "PCM Synth Tone Common MFX",
            |name| name.starts_with("MFX"),
            MfxHeaderField::ALL,
            PLAIN,
            mfx_header_spec,
        );
    }

    #[test]
    fn pcmd_wmt_specs_match_the_doc() {
        assert_matches_doc(
            PCMD_DOC,
            "PCM Drum Kit Partial",
            is_wmt_layer_row,
            PcmdWmtField::ALL,
            PLAIN,
            pcmd_wmt_spec,
        );
    }

    #[test]
    fn pcmd_partial_specs_match_the_doc() {
        assert_matches_doc(
            PCMD_DOC,
            "PCM Drum Kit Partial",
            |name| !name.starts_with("Partial Name") && !is_wmt_layer_row(name),
            PcmdPartialField::ALL,
            Affixes {
                doc_prefix: "Partial",
                host_word: "",
            },
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
        assert_matches_doc(
            SNS_DOC,
            "SN Synth Tone Common",
            |name| !name.starts_with("Tone Name"),
            SnsCommonField::ALL,
            PLAIN,
            sns_common_spec,
        );
    }

    #[test]
    fn sns_partial_specs_match_the_doc() {
        assert_matches_doc(
            SNS_DOC,
            "SN Synth Tone Partial",
            |_| true,
            SnsPartialField::ALL,
            PLAIN,
            sns_partial_spec,
        );
    }

    #[test]
    fn sna_common_specs_match_the_doc() {
        assert_matches_doc(
            SNA_DOC,
            "SN Acoustic Tone Common",
            |name| !name.starts_with("Tone Name"),
            SnaCommonField::ALL,
            PLAIN,
            sna_common_spec,
        );
    }

    #[test]
    fn snd_common_specs_match_the_doc() {
        assert_matches_doc(
            SND_DOC,
            "SN Drum Kit Common",
            |name| !name.starts_with("Kit Name"),
            SndCommonField::ALL,
            PLAIN,
            snd_common_spec,
        );
    }

    #[test]
    fn snd_note_specs_match_the_doc() {
        use SndNoteField as F;
        assert_matches_doc(
            SND_DOC,
            "SN Drum Kit Note",
            |_| true,
            SndNoteField::ALL,
            PLAIN,
            snd_note_spec,
        );
        let fmt = |f: SndNoteField, v| snd_note_spec(f).format.format(v);
        assert_eq!(fmt(F::Tune, 8), "-1200");
        assert_eq!(fmt(F::Tune, 248), "+1200");
        assert_eq!(fmt(F::Attack, 100), "100%");
        assert_eq!(fmt(F::Decay, 1), "-63");
        assert_eq!(fmt(F::Brilliance, 49), "-15");
    }

    /// Every host name of a block: one per value of every field.
    fn host_names<F: BlockField>(
        fields: &[F],
        prefix: &str,
        spec_of: impl Fn(F) -> FieldSpec,
    ) -> Vec<String> {
        fields
            .iter()
            .flat_map(|&field| {
                let spec = spec_of(field);
                (0..field.count())
                    .map(|i| qualified_name(prefix, &spec.element_name(i)))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Fail on an empty name or a name two values of `names` share.
    fn assert_distinct(block: &str, names: &[String]) {
        let mut seen = std::collections::HashSet::new();
        for name in names {
            assert!(!name.trim().is_empty(), "{block}: empty name");
            assert!(seen.insert(name), "{block}: {name:?} named twice");
        }
    }

    /// A field's name is a template exactly when it holds several values.
    fn assert_templates<F: BlockField>(fields: &[F], spec_of: impl Fn(F) -> FieldSpec) {
        for &field in fields {
            let name = spec_of(field).host_name;
            assert!(!name.is_empty(), "{field:?} has no name");
            let placeholders = name.matches("{}").count() + name.matches("{n0}").count();
            assert_eq!(
                placeholders,
                usize::from(field.count() > 1),
                "{field:?}: {name:?} vs {} values",
                field.count()
            );
            let filled = fill_name_template(name, field.count() - 1);
            assert!(!filled.contains('{'), "{field:?}: {filled:?}");
        }
    }

    /// Each `ALL` lists its fields in address order.
    fn assert_ascending<F: BlockField>(block: &str, fields: &[F]) {
        for pair in fields.windows(2) {
            assert!(
                pair[0].position() < pair[1].position(),
                "{block}: {:?} is not before {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn all_lists_are_in_address_order() {
        assert_ascending("PCM Synth Common", PcmsCommonField::ALL);
        assert_ascending("PCM Synth PMT", PcmsPmtField::ALL);
        assert_ascending("PCM Synth Partial", PcmsPartialField::ALL);
        assert_ascending("PCM Synth Common 2", PcmsCommon2Field::ALL);
        assert_ascending("PCM Drum Common", PcmdCommonField::ALL);
        assert_ascending("PCM Drum Common 2", PcmdCommon2Field::ALL);
        assert_ascending("PCM Drum WMT", PcmdWmtField::ALL);
        assert_ascending("PCM Drum Partial", PcmdPartialField::ALL);
        assert_ascending("SN Synth Common", SnsCommonField::ALL);
        assert_ascending("SN Synth Partial", SnsPartialField::ALL);
        assert_ascending("SN Acoustic Common", SnaCommonField::ALL);
        assert_ascending("SN Drum Common", SndCommonField::ALL);
        assert_ascending("SN Drum Note", SndNoteField::ALL);
        let header: Vec<usize> = MfxHeaderField::ALL
            .iter()
            .map(|f| usize::from(mfx_header_offset(*f, 0).unwrap()))
            .collect();
        assert!(header.windows(2).all(|w| w[0] < w[1]), "{header:?}");
    }

    #[test]
    fn every_field_has_a_host_name_unique_in_its_block() {
        fn check<F: BlockField>(
            block: &str,
            fields: &[F],
            spec_of: impl Fn(F) -> FieldSpec + Copy,
        ) {
            assert_templates(fields, spec_of);
            assert_distinct(block, &host_names(fields, "", spec_of));
        }
        check("PCM Synth Common", PcmsCommonField::ALL, pcms_common_spec);
        check("PCM Synth PMT", PcmsPmtField::ALL, pcms_pmt_spec);
        check(
            "PCM Synth Partial",
            PcmsPartialField::ALL,
            pcms_partial_spec,
        );
        check(
            "PCM Synth Common 2",
            PcmsCommon2Field::ALL,
            pcms_common2_spec,
        );
        check("PCM Drum Common", PcmdCommonField::ALL, pcmd_common_spec);
        check(
            "PCM Drum Common 2",
            PcmdCommon2Field::ALL,
            pcmd_common2_spec,
        );
        check("PCM Drum WMT", PcmdWmtField::ALL, pcmd_wmt_spec);
        check("PCM Drum Partial", PcmdPartialField::ALL, pcmd_partial_spec);
        check("SN Synth Common", SnsCommonField::ALL, sns_common_spec);
        check("SN Synth Partial", SnsPartialField::ALL, sns_partial_spec);
        check("SN Acoustic Common", SnaCommonField::ALL, sna_common_spec);
        check("SN Drum Common", SndCommonField::ALL, snd_common_spec);
        check("SN Drum Note", SndNoteField::ALL, snd_note_spec);
    }

    /// The MFX names of a Tone plugin or a kit-bound Kit plugin: the header
    /// fields, and every type's parameters (all types are listed, only the
    /// active one does anything).
    fn mfx_names() -> Vec<String> {
        MfxHeaderField::ALL
            .iter()
            .flat_map(|f| (0..f.count()).map(|i| fill_name_template(f.host_name(), i)))
            .chain(
                crate::mfx_params::effect_types()
                    .into_iter()
                    .filter(|(block, ..)| *block == crate::fx_params::MFX_LABEL)
                    .flat_map(|(block, type_name, params)| {
                        params
                            .iter()
                            .map(move |p| p.qualified_name(block, type_name))
                    }),
            )
            .collect()
    }

    /// The Drum Comp/EQ units' own settings, which a Kit plugin bound to
    /// the whole kit lists (a unit's Output Assign is the Studio Set's).
    fn comp_eq_unit_names() -> Vec<String> {
        use crate::param_registry::{
            CompEqKit, MixerField, MixerScope, MixerScopeKind, mixer_param,
        };
        let kit = CompEqKit {
            part: 0,
            tone_type: crate::params::ToneType::PcmDrum,
        };
        (0..crate::device_spec::INTEGRA7.comp_eq_unit_count)
            .flat_map(|unit| {
                let scope = MixerScope::CompEqUnit { kit, unit };
                MixerField::ALL
                    .into_iter()
                    .filter(|f| f.in_scope(MixerScopeKind::CompEqUnit))
                    .filter(|f| *f != MixerField::CompOutputAssign)
                    .map(move |f| qualified_name(&scope.host_prefix(), mixer_param(f).host_name))
            })
            .collect()
    }

    /// The names a Tone or Kit plugin lists side by side share none: its
    /// blocks each with their prefix, its MFX, and for a Kit plugin the
    /// kit-wide parameters together with one key's (the Binding makes one
    /// half inert, but both are listed).
    #[test]
    fn host_names_are_distinct_across_the_blocks_of_a_plugin() {
        let spec = &crate::device_spec::INTEGRA7;
        let partials = |count: u8, partial: &dyn Fn(&str) -> Vec<String>| -> Vec<String> {
            (0..usize::from(count))
                .flat_map(|i| partial(&partial_prefix(i)))
                .collect()
        };
        let sns: Vec<String> = [
            host_names(SnsCommonField::ALL, "", sns_common_spec),
            partials(spec.sn_synth_partial_count, &|p| {
                host_names(SnsPartialField::ALL, p, sns_partial_spec)
            }),
            mfx_names(),
        ]
        .concat();
        assert_distinct("SN Synth", &sns);
        let sna: Vec<String> = [
            host_names(SnaCommonField::ALL, "", sna_common_spec),
            mfx_names(),
        ]
        .concat();
        assert_distinct("SN Acoustic", &sna);
        let pcms: Vec<String> = [
            host_names(PcmsCommonField::ALL, "", pcms_common_spec),
            host_names(PcmsPmtField::ALL, "", pcms_pmt_spec),
            partials(spec.pcm_synth_partial_count, &|p| {
                host_names(PcmsPartialField::ALL, p, pcms_partial_spec)
            }),
            host_names(PcmsCommon2Field::ALL, "", pcms_common2_spec),
            mfx_names(),
        ]
        .concat();
        assert_distinct("PCM Synth", &pcms);
        let pcmd: Vec<String> = [
            host_names(PcmdCommonField::ALL, "", pcmd_common_spec),
            host_names(PcmdCommon2Field::ALL, "", pcmd_common2_spec),
            host_names(PcmdPartialField::ALL, KEY_PREFIX, pcmd_partial_spec),
            host_names(PcmdWmtField::ALL, KEY_PREFIX, pcmd_wmt_spec),
            mfx_names(),
            comp_eq_unit_names(),
        ]
        .concat();
        assert_distinct("PCM Drum", &pcmd);
        let snd: Vec<String> = [
            host_names(SndCommonField::ALL, "", snd_common_spec),
            host_names(SndNoteField::ALL, KEY_PREFIX, snd_note_spec),
            mfx_names(),
            comp_eq_unit_names(),
        ]
        .concat();
        assert_distinct("SN Drum", &snd);
    }

    #[test]
    fn element_names_fill_the_template() {
        let spec = pcms_partial_spec(PcmsPartialField::PitchEnvLevel);
        assert_eq!(spec.element_name(0), "Pitch Env Level 0");
        assert_eq!(spec.element_name(4), "Pitch Env Level 4");
        let spec = pcms_partial_spec(PcmsPartialField::TvaEnvLevel);
        assert_eq!(spec.element_name(0), "TVA Env Level 1");
        let spec = pcms_partial_spec(PcmsPartialField::Level);
        assert_eq!(spec.element_name(0), "Level");
        assert_eq!(
            qualified_name(
                &partial_prefix(1),
                pcms_partial_spec(PcmsPartialField::TvfCutoffFrequency).host_name
            ),
            "Partial 2 TVF Cutoff Frequency"
        );
    }
}
