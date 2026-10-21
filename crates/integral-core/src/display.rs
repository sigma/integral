//! Which fields each Display reads.
//!
//! A **Display** (see `CONTEXT.md`, *Layout*) binds several parameters or
//! none: an envelope reads its times and levels, a scope its shape and
//! rate. Which ones is device knowledge, so it lives here, beside
//! [`FieldSpec`](crate::field_spec::FieldSpec). A layout names a Display by
//! its [`DisplayKind`], and its Scope supplies the indices (part, partial,
//! drum key, Comp+EQ unit) the fields are read at. Decided in
//! [#331](https://github.com/sigma/integral/issues/331).
//!
//! [`display_binding`] gives a kind's [`DisplaySlot`]s on a [`DisplayHost`]:
//! the editor of a tone type, or the mixer. Each slot names the Display
//! prop it feeds and where its value comes from; the renderer resolves the
//! props from the slots, and the Displays themselves stay props-driven.

use crate::block_field::BlockField;
use crate::param_registry::MixerField;
use crate::params::ToneType;
use crate::pcm_drum::{PcmdPartialField, PcmdWmtField};
use crate::pcm_synth::{PcmsPartialField, PcmsPmtField};
use crate::sn_acoustic::SnaCommonField;
use crate::sn_drum::SndNoteField;
use crate::sn_synth::SnsPartialField;

/// A Display a layout can place, named independently of the engine: the
/// [`DisplayHost`] passed to [`display_binding`] picks the fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKind {
    /// Pitch envelope curve: SN Synth OSC (attack, decay), PCM Synth and
    /// PCM Drum (breakpoints).
    PitchEnvelope,
    /// Filter envelope curve: SN Synth (ADSR), PCM Synth and PCM Drum TVF
    /// (breakpoints).
    FilterEnvelope,
    /// Amp envelope curve: SN Synth (ADSR), SN Acoustic (attack and
    /// release offsets), PCM Synth and PCM Drum TVA (breakpoints).
    AmpEnvelope,
    /// SN Synth OSC waveform scope.
    OscScope,
    /// Filter response scope: SN Acoustic offsets, PCM Synth and PCM Drum
    /// TVF.
    FilterScope,
    /// SN Synth LFO scope.
    LfoScope,
    /// SN Synth Mod LFO scope.
    ModLfoScope,
    /// PCM Synth LFO1 scope.
    Lfo1Scope,
    /// PCM Synth LFO2 scope.
    Lfo2Scope,
    /// PCM Synth Step LFO pattern.
    StepLfoScope,
    /// PCM Synth Structure diagram of partials 1 & 2.
    Structure12,
    /// PCM Synth Structure diagram of partials 3 & 4.
    Structure34,
    /// Mixer Comp+EQ compressor attack and release curve.
    CompEnvelope,
    /// Mixer Comp+EQ compressor transfer curve.
    CompTransferCurve,
    /// Motional Surround XY pad: every part's position, and the external
    /// input's.
    SurroundPad,
    /// Wave picker, by name: a PCM Drum WMT layer's Wave Group and L/R
    /// Wave Numbers, or an SN Drum key's Inst.
    WavePicker,
}

impl DisplayKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 16] = [
        Self::PitchEnvelope,
        Self::FilterEnvelope,
        Self::AmpEnvelope,
        Self::OscScope,
        Self::FilterScope,
        Self::LfoScope,
        Self::ModLfoScope,
        Self::Lfo1Scope,
        Self::Lfo2Scope,
        Self::StepLfoScope,
        Self::Structure12,
        Self::Structure34,
        Self::CompEnvelope,
        Self::CompTransferCurve,
        Self::SurroundPad,
        Self::WavePicker,
    ];
}

/// The Display prop a slot feeds. An array prop (an envelope's times) gets
/// one slot per element, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayProp {
    /// ADSR envelope attack.
    Attack,
    /// ADSR envelope decay.
    Decay,
    /// ADSR envelope sustain.
    Sustain,
    /// ADSR envelope release.
    Release,
    /// Breakpoint envelope times, one slot per time.
    Times,
    /// Breakpoint envelope levels, one slot per level.
    Levels,
    /// OSC waveform.
    Wave,
    /// OSC pulse width.
    PulseWidth,
    /// OSC pulse width modulation depth.
    PwmDepth,
    /// OSC pulse width shift.
    PwShift,
    /// OSC Super Saw detune.
    SuperSawDetune,
    /// LFO waveform.
    Shape,
    /// LFO rate.
    Rate,
    /// Step LFO values, one slot per step.
    Steps,
    /// Step LFO type; 1 draws the steps smoothed.
    Smooth,
    /// Filter type.
    FilterType,
    /// Filter cutoff.
    Cutoff,
    /// Filter resonance.
    Resonance,
    /// Structure Type.
    Structure,
    /// Number of the first partial a Structure diagram shows.
    First,
    /// Compressor threshold.
    Threshold,
    /// Compressor ratio.
    Ratio,
    /// A part's surround L-R position.
    Lr,
    /// A part's surround F-B position.
    Fb,
    /// The external input's surround L-R position.
    ExtLr,
    /// The external input's surround F-B position.
    ExtFb,
    /// Wave Group Type (INT, SRX, ...).
    WaveGroupType,
    /// Wave Group ID within its type.
    WaveGroupId,
    /// Left (or mono) Wave Number.
    WaveNumberL,
    /// Right Wave Number; 0 for mono.
    WaveNumberR,
    /// SN Drum Inst number.
    Inst,
}

/// A parameter a Display reads, in the block its Scope addresses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayField {
    /// An SN Synth Tone Partial field; the Scope supplies the partial.
    SnsPartial(SnsPartialField),
    /// An SN Acoustic Tone Common field.
    SnaCommon(SnaCommonField),
    /// A PCM Synth Tone Partial field; the Scope supplies the partial.
    PcmsPartial(PcmsPartialField),
    /// A PCM Synth Tone PMT field.
    PcmsPmt(PcmsPmtField),
    /// A PCM Drum Kit Partial field; the Scope supplies the key.
    PcmdPartial(PcmdPartialField),
    /// A PCM Drum Kit WMT layer field; the Scope supplies the key and the
    /// layer.
    PcmdWmt(PcmdWmtField),
    /// An SN Drum Kit Note field; the Scope supplies the key.
    SndNote(SndNoteField),
    /// A mixer field; the Scope supplies its indices (the part or Comp+EQ
    /// unit). An `Ext` prop reads it in the external input's Scope.
    Mixer(MixerField),
}

impl DisplayField {
    /// Number of values the field holds: an envelope's times hold several.
    pub fn count(self) -> usize {
        match self {
            Self::SnsPartial(f) => f.count(),
            Self::SnaCommon(f) => f.count(),
            Self::PcmsPartial(f) => f.count(),
            Self::PcmsPmt(f) => f.count(),
            Self::PcmdPartial(f) => f.count(),
            Self::SndNote(f) => f.count(),
            // The layer is the Scope's, not one of the field's values.
            Self::PcmdWmt(_) | Self::Mixer(_) => 1,
        }
    }
}

/// Where a slot's value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplaySource {
    /// Value `index` of `field`; 0 unless the field holds several.
    Field {
        /// The field read.
        field: DisplayField,
        /// Which of the field's values.
        index: u8,
    },
    /// A raw value the device fixes, such as a TVA envelope's first and
    /// last level.
    Fixed(i32),
}

/// One prop of a Display and where its value comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplaySlot {
    /// The prop fed.
    pub prop: DisplayProp,
    /// Its value's source.
    pub source: DisplaySource,
}

/// A slot reading value 0 of `field`.
fn read(prop: DisplayProp, field: DisplayField) -> DisplaySlot {
    read_at(prop, field, 0)
}

/// A slot reading value `index` of `field`.
fn read_at(prop: DisplayProp, field: DisplayField, index: u8) -> DisplaySlot {
    DisplaySlot {
        prop,
        source: DisplaySource::Field { field, index },
    }
}

/// A slot holding a fixed raw value.
fn fixed(prop: DisplayProp, value: i32) -> DisplaySlot {
    DisplaySlot {
        prop,
        source: DisplaySource::Fixed(value),
    }
}

/// One `prop` slot per value of `field`, in order.
fn each(prop: DisplayProp, field: DisplayField) -> impl Iterator<Item = DisplaySlot> {
    let count = u8::try_from(field.count()).expect("a field holds at most 255 values");
    (0..count).map(move |i| read_at(prop, field, i))
}

/// Slots reading one field each.
fn reads(slots: &[(DisplayProp, DisplayField)]) -> Vec<DisplaySlot> {
    slots
        .iter()
        .map(|&(prop, field)| read(prop, field))
        .collect()
}

/// A breakpoint envelope: every time, then every level.
fn breakpoint(time: DisplayField, level: DisplayField) -> Vec<DisplaySlot> {
    each(DisplayProp::Times, time)
        .chain(each(DisplayProp::Levels, level))
        .collect()
}

/// A TVA breakpoint envelope. It stores only its middle levels: it starts
/// and ends at 0.
fn tva_breakpoint(time: DisplayField, level: DisplayField) -> Vec<DisplaySlot> {
    let silent = fixed(DisplayProp::Levels, 0);
    each(DisplayProp::Times, time)
        .chain([silent])
        .chain(each(DisplayProp::Levels, level))
        .chain([silent])
        .collect()
}

/// A Step LFO's pattern: its 16 steps, then its step type.
fn steps() -> impl Iterator<Item = DisplaySlot> {
    let f = DisplayField::PcmsPartial;
    each(DisplayProp::Steps, f(PcmsPartialField::LfoStep))
        .chain([read(DisplayProp::Smooth, f(PcmsPartialField::LfoStepType))])
}

/// Where a Display is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayHost {
    /// The editor of a tone type.
    Editor(ToneType),
    /// The mixer.
    Mixer,
}

/// The slots of `kind` shown on `host`; `None` when `host` has no such
/// Display.
pub fn display_binding(kind: DisplayKind, host: DisplayHost) -> Option<Vec<DisplaySlot>> {
    match host {
        DisplayHost::Editor(ToneType::SnSynth) => sn_synth(kind),
        DisplayHost::Editor(ToneType::SnAcoustic) => sn_acoustic(kind),
        DisplayHost::Editor(ToneType::PcmSynth) => pcm_synth(kind),
        DisplayHost::Editor(ToneType::PcmDrum) => pcm_drum(kind),
        DisplayHost::Editor(ToneType::SnDrum) => sn_drum(kind),
        DisplayHost::Mixer => mixer(kind),
    }
}

fn sn_synth(kind: DisplayKind) -> Option<Vec<DisplaySlot>> {
    use DisplayProp as P;
    use SnsPartialField as F;
    let f = DisplayField::SnsPartial;
    Some(match kind {
        DisplayKind::PitchEnvelope => reads(&[
            (P::Attack, f(F::OscPitchEnvAttack)),
            (P::Decay, f(F::OscPitchEnvDecay)),
        ]),
        DisplayKind::FilterEnvelope => reads(&[
            (P::Attack, f(F::FilterEnvAttack)),
            (P::Decay, f(F::FilterEnvDecay)),
            (P::Sustain, f(F::FilterEnvSustain)),
            (P::Release, f(F::FilterEnvRelease)),
        ]),
        DisplayKind::AmpEnvelope => reads(&[
            (P::Attack, f(F::AmpEnvAttack)),
            (P::Decay, f(F::AmpEnvDecay)),
            (P::Sustain, f(F::AmpEnvSustain)),
            (P::Release, f(F::AmpEnvRelease)),
        ]),
        DisplayKind::OscScope => reads(&[
            (P::Wave, f(F::OscWave)),
            (P::PulseWidth, f(F::OscPulseWidth)),
            (P::PwmDepth, f(F::OscPwModDepth)),
            (P::PwShift, f(F::PwShift)),
            (P::SuperSawDetune, f(F::SuperSawDetune)),
        ]),
        DisplayKind::LfoScope => reads(&[(P::Shape, f(F::LfoShape)), (P::Rate, f(F::LfoRate))]),
        DisplayKind::ModLfoScope => {
            reads(&[(P::Shape, f(F::ModLfoShape)), (P::Rate, f(F::ModLfoRate))])
        }
        _ => return None,
    })
}

fn sn_acoustic(kind: DisplayKind) -> Option<Vec<DisplaySlot>> {
    use DisplayProp as P;
    use SnaCommonField as F;
    let f = DisplayField::SnaCommon;
    Some(match kind {
        // SN-A exposes no filter type: the scope draws a low-pass.
        DisplayKind::FilterScope => reads(&[
            (P::Cutoff, f(F::CutoffOffset)),
            (P::Resonance, f(F::ResonanceOffset)),
        ]),
        DisplayKind::AmpEnvelope => reads(&[
            (P::Attack, f(F::AttackTimeOffset)),
            (P::Release, f(F::ReleaseTimeOffset)),
        ]),
        _ => return None,
    })
}

fn pcm_synth(kind: DisplayKind) -> Option<Vec<DisplaySlot>> {
    use DisplayProp as P;
    use PcmsPartialField as F;
    let f = DisplayField::PcmsPartial;
    let lfo = |shape, rate| {
        let mut slots = reads(&[(P::Shape, f(shape)), (P::Rate, f(rate))]);
        slots.extend(steps());
        slots
    };
    let structure = |field, first| {
        vec![
            read(P::Structure, DisplayField::PcmsPmt(field)),
            fixed(P::First, first),
        ]
    };
    Some(match kind {
        DisplayKind::PitchEnvelope => breakpoint(f(F::PitchEnvTime), f(F::PitchEnvLevel)),
        DisplayKind::FilterEnvelope => breakpoint(f(F::TvfEnvTime), f(F::TvfEnvLevel)),
        DisplayKind::AmpEnvelope => tva_breakpoint(f(F::TvaEnvTime), f(F::TvaEnvLevel)),
        DisplayKind::FilterScope => reads(&[
            (P::FilterType, f(F::TvfFilterType)),
            (P::Cutoff, f(F::TvfCutoffFrequency)),
            (P::Resonance, f(F::TvfResonance)),
        ]),
        DisplayKind::Lfo1Scope => lfo(F::Lfo1Waveform, F::Lfo1Rate),
        DisplayKind::Lfo2Scope => lfo(F::Lfo2Waveform, F::Lfo2Rate),
        DisplayKind::StepLfoScope => steps().collect(),
        DisplayKind::Structure12 => structure(PcmsPmtField::StructureType12, 1),
        DisplayKind::Structure34 => structure(PcmsPmtField::StructureType34, 3),
        _ => return None,
    })
}

fn pcm_drum(kind: DisplayKind) -> Option<Vec<DisplaySlot>> {
    use DisplayProp as P;
    use PcmdPartialField as F;
    let f = DisplayField::PcmdPartial;
    Some(match kind {
        DisplayKind::PitchEnvelope => breakpoint(f(F::PitchEnvTime), f(F::PitchEnvLevel)),
        DisplayKind::FilterEnvelope => breakpoint(f(F::TvfEnvTime), f(F::TvfEnvLevel)),
        DisplayKind::AmpEnvelope => tva_breakpoint(f(F::TvaEnvTime), f(F::TvaEnvLevel)),
        DisplayKind::FilterScope => reads(&[
            (P::FilterType, f(F::TvfFilterType)),
            (P::Cutoff, f(F::TvfCutoffFrequency)),
            (P::Resonance, f(F::TvfResonance)),
        ]),
        DisplayKind::WavePicker => {
            use PcmdWmtField as W;
            let w = DisplayField::PcmdWmt;
            reads(&[
                (P::WaveGroupType, w(W::WaveGroupType)),
                (P::WaveGroupId, w(W::WaveGroupId)),
                (P::WaveNumberL, w(W::WaveNumberL)),
                (P::WaveNumberR, w(W::WaveNumberR)),
            ])
        }
        _ => return None,
    })
}

fn sn_drum(kind: DisplayKind) -> Option<Vec<DisplaySlot>> {
    match kind {
        DisplayKind::WavePicker => Some(reads(&[(
            DisplayProp::Inst,
            DisplayField::SndNote(SndNoteField::InstNumber),
        )])),
        _ => None,
    }
}

/// Mixer Displays, which read [`MixerField`]s.
fn mixer(kind: DisplayKind) -> Option<Vec<DisplaySlot>> {
    use DisplayProp as P;
    let mixer = DisplayField::Mixer;
    Some(match kind {
        // The curve draws the release as its fall, so it feeds the decay.
        DisplayKind::CompEnvelope => reads(&[
            (P::Attack, mixer(MixerField::CompAttack)),
            (P::Decay, mixer(MixerField::CompRelease)),
        ]),
        DisplayKind::CompTransferCurve => reads(&[
            (P::Threshold, mixer(MixerField::CompThreshold)),
            (P::Ratio, mixer(MixerField::CompRatio)),
        ]),
        DisplayKind::SurroundPad => reads(&[
            (P::Lr, mixer(MixerField::SurroundLr)),
            (P::Fb, mixer(MixerField::SurroundFb)),
            (P::ExtLr, mixer(MixerField::SurroundLr)),
            (P::ExtFb, mixer(MixerField::SurroundFb)),
        ]),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    //! Each binding is checked against the fields today's caller passes
    //! its Display, named in the test's doc comment.

    use super::*;
    use DisplayKind as K;
    use DisplayProp as P;

    /// The fields a binding reads, as (prop, field, index); fixed values
    /// as (prop, `None`, value).
    fn sources(kind: K, host: DisplayHost) -> Vec<(P, Option<DisplayField>, i32)> {
        display_binding(kind, host)
            .unwrap_or_else(|| panic!("{kind:?} has a binding on {host:?}"))
            .into_iter()
            .map(|s| match s.source {
                DisplaySource::Field { field, index } => (s.prop, Some(field), i32::from(index)),
                DisplaySource::Fixed(v) => (s.prop, None, v),
            })
            .collect()
    }

    /// `prop` reading value 0 of `field`.
    fn one<F: Into<DisplayField>>(prop: P, field: F) -> (P, Option<DisplayField>, i32) {
        (prop, Some(field.into()), 0)
    }

    /// `prop` reading values `0..n` of `field`.
    fn many<F: Into<DisplayField> + Copy>(
        prop: P,
        field: F,
        n: i32,
    ) -> Vec<(P, Option<DisplayField>, i32)> {
        (0..n).map(|i| (prop, Some(field.into()), i)).collect()
    }

    impl From<SnsPartialField> for DisplayField {
        fn from(f: SnsPartialField) -> Self {
            Self::SnsPartial(f)
        }
    }
    impl From<SnaCommonField> for DisplayField {
        fn from(f: SnaCommonField) -> Self {
            Self::SnaCommon(f)
        }
    }
    impl From<PcmsPartialField> for DisplayField {
        fn from(f: PcmsPartialField) -> Self {
            Self::PcmsPartial(f)
        }
    }
    impl From<PcmsPmtField> for DisplayField {
        fn from(f: PcmsPmtField) -> Self {
            Self::PcmsPmt(f)
        }
    }
    impl From<PcmdPartialField> for DisplayField {
        fn from(f: PcmdPartialField) -> Self {
            Self::PcmdPartial(f)
        }
    }

    const SNS: DisplayHost = DisplayHost::Editor(ToneType::SnSynth);
    const SNA: DisplayHost = DisplayHost::Editor(ToneType::SnAcoustic);
    const PCMS: DisplayHost = DisplayHost::Editor(ToneType::PcmSynth);
    const PCMD: DisplayHost = DisplayHost::Editor(ToneType::PcmDrum);
    const SND: DisplayHost = DisplayHost::Editor(ToneType::SnDrum);
    const MIXER: DisplayHost = DisplayHost::Mixer;

    /// `sn-synth/PartialRow.tsx` `OscPanel`: `ADSREnvelope` attack, decay.
    #[test]
    fn sns_pitch_envelope() {
        use SnsPartialField as F;
        assert_eq!(
            sources(K::PitchEnvelope, SNS),
            [
                one(P::Attack, F::OscPitchEnvAttack),
                one(P::Decay, F::OscPitchEnvDecay)
            ]
        );
    }

    /// `sn-synth/PartialRow.tsx` `FilterPanel`: `ADSREnvelope`.
    #[test]
    fn sns_filter_envelope() {
        use SnsPartialField as F;
        assert_eq!(
            sources(K::FilterEnvelope, SNS),
            [
                one(P::Attack, F::FilterEnvAttack),
                one(P::Decay, F::FilterEnvDecay),
                one(P::Sustain, F::FilterEnvSustain),
                one(P::Release, F::FilterEnvRelease),
            ]
        );
    }

    /// `sn-synth/PartialRow.tsx` `AmpPanel`: `ADSREnvelope`.
    #[test]
    fn sns_amp_envelope() {
        use SnsPartialField as F;
        assert_eq!(
            sources(K::AmpEnvelope, SNS),
            [
                one(P::Attack, F::AmpEnvAttack),
                one(P::Decay, F::AmpEnvDecay),
                one(P::Sustain, F::AmpEnvSustain),
                one(P::Release, F::AmpEnvRelease),
            ]
        );
    }

    /// `sn-synth/PartialRow.tsx` `OscPanel`: `OscWaveScope`.
    #[test]
    fn sns_osc_scope() {
        use SnsPartialField as F;
        assert_eq!(
            sources(K::OscScope, SNS),
            [
                one(P::Wave, F::OscWave),
                one(P::PulseWidth, F::OscPulseWidth),
                one(P::PwmDepth, F::OscPwModDepth),
                one(P::PwShift, F::PwShift),
                one(P::SuperSawDetune, F::SuperSawDetune),
            ]
        );
    }

    /// `sn-synth/PartialRow.tsx` `LfoPanel`: `LfoScope` shape, rate.
    #[test]
    fn sns_lfo_scope() {
        use SnsPartialField as F;
        assert_eq!(
            sources(K::LfoScope, SNS),
            [one(P::Shape, F::LfoShape), one(P::Rate, F::LfoRate)]
        );
    }

    /// `sn-synth/PartialRow.tsx` `ModLfoPanel`: `LfoScope` shape, rate.
    #[test]
    fn sns_mod_lfo_scope() {
        use SnsPartialField as F;
        assert_eq!(
            sources(K::ModLfoScope, SNS),
            [one(P::Shape, F::ModLfoShape), one(P::Rate, F::ModLfoRate)]
        );
    }

    /// `SnAcousticEditor.tsx` filter: `FilterScope` cutoff, resonance.
    #[test]
    fn sna_filter_scope() {
        use SnaCommonField as F;
        assert_eq!(
            sources(K::FilterScope, SNA),
            [
                one(P::Cutoff, F::CutoffOffset),
                one(P::Resonance, F::ResonanceOffset)
            ]
        );
    }

    /// `SnAcousticEditor.tsx` amp: `ADSREnvelope` attack, release.
    #[test]
    fn sna_amp_envelope() {
        use SnaCommonField as F;
        assert_eq!(
            sources(K::AmpEnvelope, SNA),
            [
                one(P::Attack, F::AttackTimeOffset),
                one(P::Release, F::ReleaseTimeOffset)
            ]
        );
    }

    /// The breakpoint envelope `pcm-synth/PartialRow.tsx` and
    /// `PcmDrumEditor.tsx` (`BreakpointDisplay`) read: T1-T4, then L0-L4.
    fn breakpoints<F: Into<DisplayField> + Copy>(
        time: F,
        level: F,
    ) -> Vec<(P, Option<DisplayField>, i32)> {
        let mut want = many(P::Times, time, 4);
        want.extend(many(P::Levels, level, 5));
        want
    }

    /// As [`breakpoints`], for a TVA: L0 and L4 fixed at 0, and L1-L3 its
    /// three stored levels.
    fn tva_breakpoints<F: Into<DisplayField> + Copy>(
        time: F,
        level: F,
    ) -> Vec<(P, Option<DisplayField>, i32)> {
        let mut want = many(P::Times, time, 4);
        want.push((P::Levels, None, 0));
        want.extend(many(P::Levels, level, 3));
        want.push((P::Levels, None, 0));
        want
    }

    /// `pcm-synth/PartialRow.tsx` pitch section: `BreakpointEnvelope`.
    #[test]
    fn pcms_pitch_envelope() {
        use PcmsPartialField as F;
        assert_eq!(
            sources(K::PitchEnvelope, PCMS),
            breakpoints(F::PitchEnvTime, F::PitchEnvLevel)
        );
    }

    /// `pcm-synth/PartialRow.tsx` `TvfSection`: `BreakpointEnvelope`.
    #[test]
    fn pcms_filter_envelope() {
        use PcmsPartialField as F;
        assert_eq!(
            sources(K::FilterEnvelope, PCMS),
            breakpoints(F::TvfEnvTime, F::TvfEnvLevel)
        );
    }

    /// `pcm-synth/PartialRow.tsx` `TvaSection`: `BreakpointEnvelope`.
    #[test]
    fn pcms_amp_envelope() {
        use PcmsPartialField as F;
        assert_eq!(
            sources(K::AmpEnvelope, PCMS),
            tva_breakpoints(F::TvaEnvTime, F::TvaEnvLevel)
        );
    }

    /// `pcm-synth/PartialRow.tsx` `TvfSection`: `FilterScope`.
    #[test]
    fn pcms_filter_scope() {
        use PcmsPartialField as F;
        assert_eq!(
            sources(K::FilterScope, PCMS),
            [
                one(P::FilterType, F::TvfFilterType),
                one(P::Cutoff, F::TvfCutoffFrequency),
                one(P::Resonance, F::TvfResonance),
            ]
        );
    }

    /// The `LfoScope` `pcm-synth/PartialRow.tsx` passes an LFO: its shape
    /// and rate, the 16 steps and the step type.
    fn pcms_lfo(
        shape: PcmsPartialField,
        rate: PcmsPartialField,
    ) -> Vec<(P, Option<DisplayField>, i32)> {
        use PcmsPartialField as F;
        let mut want = vec![one(P::Shape, shape), one(P::Rate, rate)];
        want.extend(many(P::Steps, F::LfoStep, 16));
        want.push(one(P::Smooth, F::LfoStepType));
        want
    }

    /// `pcm-synth/PartialRow.tsx` `Lfo1Section`: `LfoScope`.
    #[test]
    fn pcms_lfo1_scope() {
        use PcmsPartialField as F;
        assert_eq!(
            sources(K::Lfo1Scope, PCMS),
            pcms_lfo(F::Lfo1Waveform, F::Lfo1Rate)
        );
    }

    /// `pcm-synth/PartialRow.tsx` `Lfo2Section`: `LfoScope`.
    #[test]
    fn pcms_lfo2_scope() {
        use PcmsPartialField as F;
        assert_eq!(
            sources(K::Lfo2Scope, PCMS),
            pcms_lfo(F::Lfo2Waveform, F::Lfo2Rate)
        );
    }

    /// `pcm-synth/PartialRow.tsx` `StepLfoSection`: `LfoScope` steps and
    /// smooth (its shape and rate are the Display's own).
    #[test]
    fn pcms_step_lfo_scope() {
        use PcmsPartialField as F;
        let mut want = many(P::Steps, F::LfoStep, 16);
        want.push(one(P::Smooth, F::LfoStepType));
        assert_eq!(sources(K::StepLfoScope, PCMS), want);
    }

    /// `pcm-synth/PmtStrip.tsx`: `StructureDiagram` structure, first = 1.
    #[test]
    fn pcms_structure_12() {
        assert_eq!(
            sources(K::Structure12, PCMS),
            [
                one(P::Structure, PcmsPmtField::StructureType12),
                (P::First, None, 1)
            ]
        );
    }

    /// `pcm-synth/PmtStrip.tsx`: `StructureDiagram` structure, first = 3.
    #[test]
    fn pcms_structure_34() {
        assert_eq!(
            sources(K::Structure34, PCMS),
            [
                one(P::Structure, PcmsPmtField::StructureType34),
                (P::First, None, 3)
            ]
        );
    }

    /// `layouts/drum/pcmDrum.ts` PITCH ENV: `BreakpointDisplay`.
    #[test]
    fn pcmd_pitch_envelope() {
        use PcmdPartialField as F;
        assert_eq!(
            sources(K::PitchEnvelope, PCMD),
            breakpoints(F::PitchEnvTime, F::PitchEnvLevel)
        );
    }

    /// `layouts/drum/pcmDrum.ts` FILTER ENV: `BreakpointDisplay`.
    #[test]
    fn pcmd_filter_envelope() {
        use PcmdPartialField as F;
        assert_eq!(
            sources(K::FilterEnvelope, PCMD),
            breakpoints(F::TvfEnvTime, F::TvfEnvLevel)
        );
    }

    /// `layouts/drum/pcmDrum.ts` AMP: `BreakpointDisplay`.
    #[test]
    fn pcmd_amp_envelope() {
        use PcmdPartialField as F;
        assert_eq!(
            sources(K::AmpEnvelope, PCMD),
            tva_breakpoints(F::TvaEnvTime, F::TvaEnvLevel)
        );
    }

    /// `layouts/drum/pcmDrum.ts` FILTER: `FilterScope`.
    #[test]
    fn pcmd_filter_scope() {
        use PcmdPartialField as F;
        assert_eq!(
            sources(K::FilterScope, PCMD),
            [
                one(P::FilterType, F::TvfFilterType),
                one(P::Cutoff, F::TvfCutoffFrequency),
                one(P::Resonance, F::TvfResonance),
            ]
        );
    }

    /// `ChannelStrip.tsx` comp box: `ADSREnvelope` attack and, as its
    /// decay, release.
    #[test]
    fn mixer_comp_envelope() {
        assert_eq!(
            sources(K::CompEnvelope, MIXER),
            [
                one(P::Attack, DisplayField::Mixer(MixerField::CompAttack)),
                one(P::Decay, DisplayField::Mixer(MixerField::CompRelease)),
            ]
        );
    }

    /// `ChannelStrip.tsx` comp box: `TransferCurve` threshold, ratio.
    #[test]
    fn mixer_comp_transfer_curve() {
        assert_eq!(
            sources(K::CompTransferCurve, MIXER),
            [
                one(P::Threshold, DisplayField::Mixer(MixerField::CompThreshold)),
                one(P::Ratio, DisplayField::Mixer(MixerField::CompRatio)),
            ]
        );
    }

    /// `SurroundPage.tsx`: `SurroundXYPad` reads each part's L-R and F-B,
    /// and the Ext part's.
    #[test]
    fn mixer_surround_pad() {
        assert_eq!(
            sources(K::SurroundPad, MIXER),
            [
                one(P::Lr, DisplayField::Mixer(MixerField::SurroundLr)),
                one(P::Fb, DisplayField::Mixer(MixerField::SurroundFb)),
                one(P::ExtLr, DisplayField::Mixer(MixerField::SurroundLr)),
                one(P::ExtFb, DisplayField::Mixer(MixerField::SurroundFb)),
            ]
        );
    }

    /// `pcm-drum/WmtWave.tsx`: `WavePicker` group, L, R.
    #[test]
    fn pcmd_wave_picker() {
        use PcmdWmtField as W;
        let w = DisplayField::PcmdWmt;
        assert_eq!(
            sources(K::WavePicker, PCMD),
            [
                (P::WaveGroupType, Some(w(W::WaveGroupType)), 0),
                (P::WaveGroupId, Some(w(W::WaveGroupId)), 0),
                (P::WaveNumberL, Some(w(W::WaveNumberL)), 0),
                (P::WaveNumberR, Some(w(W::WaveNumberR)), 0),
            ]
        );
    }

    /// `SnDrumEditor.tsx` TONE: `WavePicker` Inst.
    #[test]
    fn snd_wave_picker() {
        assert_eq!(
            sources(K::WavePicker, SND),
            [(
                P::Inst,
                Some(DisplayField::SndNote(SndNoteField::InstNumber)),
                0
            )]
        );
    }

    /// [`DisplayKind::ALL`] lists every kind once, in order.
    #[test]
    fn all_lists_every_kind() {
        assert_eq!(K::ALL.len(), K::WavePicker as usize + 1);
        for (i, kind) in K::ALL.into_iter().enumerate() {
            assert_eq!(kind as usize, i, "{kind:?}");
        }
    }

    /// Which editors (or the mixer) have each Display today; every other
    /// combination has no binding.
    #[test]
    fn bindings_exist_where_displays_are_shown() {
        let hosts = [SNS, SNA, PCMS, PCMD, SND, MIXER];
        let shown = |kind: K| -> &[DisplayHost] {
            match kind {
                K::PitchEnvelope | K::FilterEnvelope => &[SNS, PCMS, PCMD],
                K::AmpEnvelope => &[SNS, SNA, PCMS, PCMD],
                K::FilterScope => &[SNA, PCMS, PCMD],
                K::OscScope | K::LfoScope | K::ModLfoScope => &[SNS],
                K::Lfo1Scope | K::Lfo2Scope | K::StepLfoScope | K::Structure12 | K::Structure34 => {
                    &[PCMS]
                }
                K::CompEnvelope | K::CompTransferCurve | K::SurroundPad => &[MIXER],
                K::WavePicker => &[PCMD, SND],
            }
        };
        for kind in K::ALL {
            for host in hosts {
                assert_eq!(
                    display_binding(kind, host).is_some(),
                    shown(kind).contains(&host),
                    "{kind:?} on {host:?}"
                );
            }
        }
    }

    /// Every slot reads a value its field holds.
    #[test]
    fn indices_are_within_their_field() {
        let hosts = [SNS, SNA, PCMS, PCMD, SND, MIXER];
        for kind in K::ALL {
            for host in hosts {
                for slot in display_binding(kind, host).unwrap_or_default() {
                    if let DisplaySource::Field { field, index } = slot.source {
                        assert!(usize::from(index) < field.count(), "{kind:?} {slot:?}");
                    }
                }
            }
        }
    }
}
