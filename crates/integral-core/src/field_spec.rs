//! Range, default and display format of each tone block field.
//!
//! Each engine's spec function matches its field enum exhaustively, so a
//! field added to the enum fails the build until it is specified. Ranges
//! and formats follow the MIDI Implementation (`docs/midi/`); defaults are
//! the field's value in the engine's `Default` block, which the codebase
//! treats as INIT (not yet checked against the device: #250).

use std::sync::OnceLock;

use crate::block_field::BlockField;
use crate::param_registry::ParamFormat;
use crate::pcm_drum::PcmdPartialField;
use crate::pcm_synth::{self, PcmsCommon2Field, PcmsCommonField, PcmsPartialField, PcmsPmtField};

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
        F::TonePan => shape(0, 127, Pan),
        F::TonePriority => shape(0, 1, Named(&["LAST", "LOUDEST"])),
        F::CoarseTune => COARSE_TUNE,
        F::FineTune => FINE_TUNE,
        F::OctaveShift => OCTAVE_SHIFT,
        F::StretchTuneDepth => shape(0, 3, Named(&["OFF", "1", "2", "3"])),
        F::MonoPoly => shape(0, 1, Named(&["MONO", "POLY"])),
        F::LegatoSwitch | F::LegatoRetrigger | F::PortamentoSwitch | F::PmtControlSwitch => ON_OFF,
        F::PortamentoMode => shape(0, 1, Named(&["NORMAL", "LEGATO"])),
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
        F::Booster12 | F::Booster34 => shape(0, 3, Named(&["0", "+6", "+12", "+18"])),
        F::PmtVelocityControl => shape(0, 3, Named(&["OFF", "ON", "RANDOM", "CYCLE"])),
        F::PartialSwitch => ON_OFF,
        F::KeyRangeLower | F::KeyRangeUpper => shape(0, 127, Note),
        F::KeyFadeLower | F::KeyFadeUpper | F::VelocityFadeLower | F::VelocityFadeUpper => RAW_127,
        F::VelocityRangeLower | F::VelocityRangeUpper => shape(1, 127, Raw),
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
        F::Pan => shape(0, 127, Pan),
        F::AlternatePanDepth => shape(1, 127, Pan),
        F::RandomPanDepth => shape(0, 63, Raw),
        F::PanKeyfollow
        | F::PitchEnvTimeKeyfollow
        | F::TvfEnvTimeKeyfollow
        | F::TvaBiasLevel
        | F::TvaEnvTimeKeyfollow
        | F::Lfo1DelayTimeKeyfollow
        | F::Lfo2DelayTimeKeyfollow => KEYFOLLOW_100,
        F::WavePitchKeyfollow | F::TvfCutoffKeyfollow => KEYFOLLOW_200,
        F::EnvMode => shape(0, 1, Named(&["NO-SUS", "SUSTAIN"])),
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
            shape(0, 2, Named(&["OFF", "ON", "REVERSE"]))
        }
        F::WaveGroupType => shape(0, 3, Named(&["INT", "SRX", "---", "---"])),
        F::WaveGroupId | F::WaveNumberL | F::WaveNumberR => shape(0, 16384, Raw),
        F::WaveGain => shape(0, 3, Named(&["-6dB", "0dB", "+6dB", "+12dB"])),
        F::WaveFxmColor => shape(0, 3, Named(&["1", "2", "3", "4"])),
        F::WaveFxmDepth => shape(0, 16, Raw),
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
        F::TvfFilterType => shape(
            0,
            6,
            Named(&["OFF", "LPF", "BPF", "HPF", "PKG", "LPF2", "LPF3"]),
        ),
        F::TvfCutoffVelocityCurve | F::TvfEnvVelocityCurve | F::TvaLevelVelocityCurve => {
            VELOCITY_CURVE
        }
        F::TvaBiasPosition => shape(0, 127, Note),
        F::TvaBiasDirection => shape(0, 3, Named(&["LOWER", "UPPER", "LOWER&UPPER", "ALL"])),
        F::Lfo1Waveform | F::Lfo2Waveform => LFO_WAVEFORM,
        F::Lfo1Offset | F::Lfo2Offset => LFO_OFFSET,
        F::Lfo1FadeMode | F::Lfo2FadeMode => FADE_MODE,
        F::LfoStepType => shape(0, 1, Raw),
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
        F::PhraseNumber => (shape(0, 65535, Raw), init.phrase_number.into()),
    };
    FieldSpec {
        min,
        max,
        default,
        format,
    }
}

/// The spec of a PCM Drum Kit Partial field, for the fields specified so
/// far; `None` for the rest.
pub fn pcmd_partial_spec(field: PcmdPartialField) -> Option<FieldSpec> {
    let Shape { min, max, format } = match field {
        PcmdPartialField::RandomPitchDepth => RANDOM_PITCH_DEPTH,
        PcmdPartialField::PitchEnvDepth => PITCH_ENV_DEPTH,
        _ => return None,
    };
    Some(FieldSpec {
        min,
        max,
        // shortcut: the PCM-S INIT values, until PCM Drum Kit fields get their own specs (#245).
        default: match field {
            PcmdPartialField::PitchEnvDepth => 64,
            _ => 0,
        },
        format,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = include_str!("../../../docs/midi/06-pcm-synth-tone.md");

    /// A documented parameter: its byte position in the block and its range.
    struct DocRow {
        name: String,
        position: usize,
        min: i32,
        max: i32,
    }

    /// The parameter rows of the `## <heading>` section of the PCM Synth
    /// Tone doc. Cells are split on `|`; reserves, `...` rows and rows
    /// matching `skip` are left out.
    fn doc_rows(heading: &str, skip: &str) -> Vec<DocRow> {
        let section = DOC
            .split("\n## ")
            .find(|s| s.lines().next() == Some(heading))
            .unwrap_or_else(|| panic!("no section {heading}"));
        section
            .lines()
            .filter(|l| l.starts_with("| `"))
            .filter_map(|line| {
                let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
                let name = cells[2];
                if name.starts_with("(reserve)") || (!skip.is_empty() && name.starts_with(skip)) {
                    return None;
                }
                let offset = cells[0].split('`').nth(1).unwrap();
                let [hi, lo] = [0, 1]
                    .map(|i| usize::from_str_radix(offset.split(' ').nth(i).unwrap(), 16).unwrap());
                let (min, max) = cells[3].split_once(" - ").unwrap();
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
        heading: &str,
        skip: &str,
        fields: &[F],
        spec_of: impl Fn(F) -> FieldSpec,
    ) {
        let rows = doc_rows(heading, skip);
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
            "PCM Synth Tone Common",
            "PCM Synth Tone Name",
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
            "PCM Synth Tone PMT (Partial Mix Table)",
            "",
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
        assert_matches_doc("PCM Synth Tone Partial", "", &fields, pcms_partial_spec);
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
        assert_matches_doc("PCM Synth Tone Common 2", "", &fields, pcms_common2_spec);
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
            assert_eq!(pcmd_partial_spec(d), Some(pcms_partial_spec(s)));
        }
    }
}
