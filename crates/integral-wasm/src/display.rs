//! Display bindings (mirrors [`integral_core::display`]).

use integral_core::display::{self as core, DisplayField, DisplaySource};
use wasm_bindgen::prelude::*;

use crate::{
    MixerField, PcmdPartialField, PcmdWmtField, PcmsPartialField, PcmsPmtField, SnaCommonField,
    SnsPartialField, ToneType,
};

/// A Display a layout can place (mirrors [`core::DisplayKind`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayKind {
    /// See [`core::DisplayKind::PitchEnvelope`].
    PitchEnvelope,
    /// See [`core::DisplayKind::FilterEnvelope`].
    FilterEnvelope,
    /// See [`core::DisplayKind::AmpEnvelope`].
    AmpEnvelope,
    /// See [`core::DisplayKind::OscScope`].
    OscScope,
    /// See [`core::DisplayKind::FilterScope`].
    FilterScope,
    /// See [`core::DisplayKind::LfoScope`].
    LfoScope,
    /// See [`core::DisplayKind::ModLfoScope`].
    ModLfoScope,
    /// See [`core::DisplayKind::Lfo1Scope`].
    Lfo1Scope,
    /// See [`core::DisplayKind::Lfo2Scope`].
    Lfo2Scope,
    /// See [`core::DisplayKind::StepLfoScope`].
    StepLfoScope,
    /// See [`core::DisplayKind::Structure12`].
    Structure12,
    /// See [`core::DisplayKind::Structure34`].
    Structure34,
    /// See [`core::DisplayKind::CompEnvelope`].
    CompEnvelope,
    /// See [`core::DisplayKind::CompTransferCurve`].
    CompTransferCurve,
    /// See [`core::DisplayKind::SurroundPad`].
    SurroundPad,
    /// See [`core::DisplayKind::WavePicker`].
    WavePicker,
    /// See [`core::DisplayKind::StripName`].
    StripName,
}

mirror_core_enum!(DisplayKind <=> core::DisplayKind {
    PitchEnvelope,
    FilterEnvelope,
    AmpEnvelope,
    OscScope,
    FilterScope,
    LfoScope,
    ModLfoScope,
    Lfo1Scope,
    Lfo2Scope,
    StepLfoScope,
    Structure12,
    Structure34,
    CompEnvelope,
    CompTransferCurve,
    SurroundPad,
    WavePicker,
    StripName,
});

/// The Display prop a slot feeds (mirrors [`core::DisplayProp`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayProp {
    /// See [`core::DisplayProp::Attack`].
    Attack,
    /// See [`core::DisplayProp::Decay`].
    Decay,
    /// See [`core::DisplayProp::Sustain`].
    Sustain,
    /// See [`core::DisplayProp::Release`].
    Release,
    /// See [`core::DisplayProp::Times`].
    Times,
    /// See [`core::DisplayProp::Levels`].
    Levels,
    /// See [`core::DisplayProp::Wave`].
    Wave,
    /// See [`core::DisplayProp::PulseWidth`].
    PulseWidth,
    /// See [`core::DisplayProp::PwmDepth`].
    PwmDepth,
    /// See [`core::DisplayProp::PwShift`].
    PwShift,
    /// See [`core::DisplayProp::SuperSawDetune`].
    SuperSawDetune,
    /// See [`core::DisplayProp::Shape`].
    Shape,
    /// See [`core::DisplayProp::Rate`].
    Rate,
    /// See [`core::DisplayProp::Steps`].
    Steps,
    /// See [`core::DisplayProp::Smooth`].
    Smooth,
    /// See [`core::DisplayProp::FilterType`].
    FilterType,
    /// See [`core::DisplayProp::Cutoff`].
    Cutoff,
    /// See [`core::DisplayProp::Resonance`].
    Resonance,
    /// See [`core::DisplayProp::Slope`].
    Slope,
    /// See [`core::DisplayProp::HpfCutoff`].
    HpfCutoff,
    /// See [`core::DisplayProp::EnvDepth`].
    EnvDepth,
    /// See [`core::DisplayProp::EnvVelSens`].
    EnvVelSens,
    /// See [`core::DisplayProp::AftertouchSens`].
    AftertouchSens,
    /// See [`core::DisplayProp::LfoDepth`].
    LfoDepth,
    /// See [`core::DisplayProp::ModLfoDepth`].
    ModLfoDepth,
    /// See [`core::DisplayProp::Keyfollow`].
    Keyfollow,
    /// See [`core::DisplayProp::Structure`].
    Structure,
    /// See [`core::DisplayProp::First`].
    First,
    /// See [`core::DisplayProp::Threshold`].
    Threshold,
    /// See [`core::DisplayProp::Ratio`].
    Ratio,
    /// See [`core::DisplayProp::Lr`].
    Lr,
    /// See [`core::DisplayProp::Fb`].
    Fb,
    /// See [`core::DisplayProp::ExtLr`].
    ExtLr,
    /// See [`core::DisplayProp::ExtFb`].
    ExtFb,
    /// See [`core::DisplayProp::WaveGroupType`].
    WaveGroupType,
    /// See [`core::DisplayProp::WaveGroupId`].
    WaveGroupId,
    /// See [`core::DisplayProp::WaveNumberL`].
    WaveNumberL,
    /// See [`core::DisplayProp::WaveNumberR`].
    WaveNumberR,
}

mirror_core_enum!(DisplayProp <=> core::DisplayProp {
    Attack,
    Decay,
    Sustain,
    Release,
    Times,
    Levels,
    Wave,
    PulseWidth,
    PwmDepth,
    PwShift,
    SuperSawDetune,
    Shape,
    Rate,
    Steps,
    Smooth,
    FilterType,
    Cutoff,
    Resonance,
    Slope,
    HpfCutoff,
    EnvDepth,
    EnvVelSens,
    AftertouchSens,
    LfoDepth,
    ModLfoDepth,
    Keyfollow,
    Structure,
    First,
    Threshold,
    Ratio,
    Lr,
    Fb,
    ExtLr,
    ExtFb,
    WaveGroupType,
    WaveGroupId,
    WaveNumberL,
    WaveNumberR,
});

/// One prop of a Display and where its value comes from (mirrors
/// [`core::DisplaySlot`]). Exactly one of the field getters, or
/// [`fixed`](Self::fixed), is set.
#[wasm_bindgen]
pub struct WasmDisplaySlot(core::DisplaySlot);

/// A slot's field, as the one getter that exposes its block.
#[derive(Default)]
struct FieldGetters {
    sns_partial: Option<SnsPartialField>,
    sna_common: Option<SnaCommonField>,
    pcms_partial: Option<PcmsPartialField>,
    pcms_pmt: Option<PcmsPmtField>,
    pcmd_partial: Option<PcmdPartialField>,
    pcmd_wmt: Option<PcmdWmtField>,
    mixer: Option<MixerField>,
}

impl WasmDisplaySlot {
    /// The slot's field under its getter. The `match` is exhaustive, so a
    /// field block added in core fails the build until it has a getter.
    fn getters(&self) -> FieldGetters {
        let DisplaySource::Field { field, .. } = self.0.source else {
            return FieldGetters::default();
        };
        let none = FieldGetters::default();
        match field {
            DisplayField::SnsPartial(f) => FieldGetters {
                sns_partial: Some(f.into()),
                ..none
            },
            DisplayField::SnaCommon(f) => FieldGetters {
                sna_common: Some(f.into()),
                ..none
            },
            DisplayField::PcmsPartial(f) => FieldGetters {
                pcms_partial: Some(f.into()),
                ..none
            },
            DisplayField::PcmsPmt(f) => FieldGetters {
                pcms_pmt: Some(f.into()),
                ..none
            },
            DisplayField::PcmdPartial(f) => FieldGetters {
                pcmd_partial: Some(f.into()),
                ..none
            },
            DisplayField::PcmdWmt(f) => FieldGetters {
                pcmd_wmt: Some(f.into()),
                ..none
            },
            DisplayField::Mixer(f) => FieldGetters {
                mixer: Some(f.into()),
                ..none
            },
        }
    }
}

#[wasm_bindgen]
impl WasmDisplaySlot {
    /// The prop fed.
    #[wasm_bindgen(getter)]
    pub fn prop(&self) -> DisplayProp {
        self.0.prop.into()
    }

    /// Which of the field's values is read; 0 for a fixed value.
    #[wasm_bindgen(getter)]
    pub fn index(&self) -> u8 {
        match self.0.source {
            DisplaySource::Field { index, .. } => index,
            DisplaySource::Fixed(_) => 0,
        }
    }

    /// The raw value the device fixes, if the slot reads no field.
    #[wasm_bindgen(getter)]
    pub fn fixed(&self) -> Option<i32> {
        match self.0.source {
            DisplaySource::Fixed(value) => Some(value),
            DisplaySource::Field { .. } => None,
        }
    }

    /// The SN Synth Tone Partial field read.
    #[wasm_bindgen(getter, js_name = snsPartial)]
    pub fn sns_partial(&self) -> Option<SnsPartialField> {
        self.getters().sns_partial
    }

    /// The SN Acoustic Tone Common field read.
    #[wasm_bindgen(getter, js_name = snaCommon)]
    pub fn sna_common(&self) -> Option<SnaCommonField> {
        self.getters().sna_common
    }

    /// The PCM Synth Tone Partial field read.
    #[wasm_bindgen(getter, js_name = pcmsPartial)]
    pub fn pcms_partial(&self) -> Option<PcmsPartialField> {
        self.getters().pcms_partial
    }

    /// The PCM Synth Tone PMT field read.
    #[wasm_bindgen(getter, js_name = pcmsPmt)]
    pub fn pcms_pmt(&self) -> Option<PcmsPmtField> {
        self.getters().pcms_pmt
    }

    /// The PCM Drum Kit Partial field read.
    #[wasm_bindgen(getter, js_name = pcmdPartial)]
    pub fn pcmd_partial(&self) -> Option<PcmdPartialField> {
        self.getters().pcmd_partial
    }

    /// The PCM Drum Kit WMT layer field read.
    #[wasm_bindgen(getter, js_name = pcmdWmt)]
    pub fn pcmd_wmt(&self) -> Option<PcmdWmtField> {
        self.getters().pcmd_wmt
    }

    /// The mixer field read; the Scope supplies its indices.
    #[wasm_bindgen(getter)]
    pub fn mixer(&self) -> Option<MixerField> {
        self.getters().mixer
    }
}

/// The slots of `kind`, shown in an editor of `tone_type` or, without one,
/// on the mixer; see [`core::display_binding`]. `undefined` when that
/// editor (or the mixer) has no such Display.
#[wasm_bindgen]
pub fn display_binding(
    kind: DisplayKind,
    tone_type: Option<ToneType>,
) -> Option<Vec<WasmDisplaySlot>> {
    let host = tone_type.map_or(core::DisplayHost::Mixer, |t| {
        core::DisplayHost::Editor(t.into())
    });
    core::display_binding(kind.into(), host)
        .map(|slots| slots.into_iter().map(WasmDisplaySlot).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_exposes_its_field_through_one_getter() {
        let slots = display_binding(DisplayKind::AmpEnvelope, Some(ToneType::PcmSynth)).unwrap();
        let time = &slots[0];
        assert_eq!(time.prop(), DisplayProp::Times);
        assert_eq!(time.pcms_partial(), Some(PcmsPartialField::TvaEnvTime));
        assert_eq!(time.index(), 0);
        assert_eq!(time.fixed(), None);
        assert_eq!(time.sns_partial(), None);

        let l0 = &slots[4];
        assert_eq!(l0.prop(), DisplayProp::Levels);
        assert_eq!(l0.fixed(), Some(0));
        assert_eq!(l0.pcms_partial(), None);
    }

    #[test]
    fn drum_wave_picker_reads_its_block() {
        let pcmd = display_binding(DisplayKind::WavePicker, Some(ToneType::PcmDrum)).unwrap();
        assert_eq!(pcmd[2].prop(), DisplayProp::WaveNumberL);
        assert_eq!(pcmd[2].pcmd_wmt(), Some(PcmdWmtField::WaveNumberL));
        assert_eq!(pcmd[2].sns_partial(), None);
        assert!(display_binding(DisplayKind::WavePicker, Some(ToneType::SnDrum)).is_none());
    }

    #[test]
    fn mixer_slots_carry_mixer_fields() {
        let pad = display_binding(DisplayKind::SurroundPad, None).unwrap();
        assert_eq!(pad[0].mixer(), Some(MixerField::SurroundLr));
        assert_eq!(pad[2].mixer(), Some(MixerField::SurroundLr));
        let curve = display_binding(DisplayKind::CompTransferCurve, None).unwrap();
        assert_eq!(curve[0].mixer(), Some(MixerField::CompThreshold));
    }
}
