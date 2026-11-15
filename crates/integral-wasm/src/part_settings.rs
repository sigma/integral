//! WASM bindings for the Part settings (`integral_core::part_settings`): the
//! registry the Part settings drawer draws from, and the state's typed
//! getter and setter.

use integral_core::part_settings;
use wasm_bindgen::prelude::*;

use crate::{WasmDeviceState, WasmFieldSpec, check_part, wrap_field_spec};

/// A Part setting, named without its Part (mirrors
/// [`part_settings::PartSetting`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartSetting {
    /// See [`part_settings::PartSetting::KeyRangeLower`].
    KeyRangeLower,
    /// See [`part_settings::PartSetting::KeyRangeUpper`].
    KeyRangeUpper,
    /// See [`part_settings::PartSetting::KeyFadeLower`].
    KeyFadeLower,
    /// See [`part_settings::PartSetting::KeyFadeUpper`].
    KeyFadeUpper,
    /// See [`part_settings::PartSetting::VelocityRangeLower`].
    VelocityRangeLower,
    /// See [`part_settings::PartSetting::VelocityRangeUpper`].
    VelocityRangeUpper,
    /// See [`part_settings::PartSetting::VelocityFadeLower`].
    VelocityFadeLower,
    /// See [`part_settings::PartSetting::VelocityFadeUpper`].
    VelocityFadeUpper,
    /// See [`part_settings::PartSetting::VelocitySensOffset`].
    VelocitySensOffset,
    /// See [`part_settings::PartSetting::MonoPoly`].
    MonoPoly,
    /// See [`part_settings::PartSetting::Legato`].
    Legato,
    /// See [`part_settings::PartSetting::VelocityCurve`].
    VelocityCurve,
    /// See [`part_settings::PartSetting::OctaveShift`].
    OctaveShift,
    /// See [`part_settings::PartSetting::CoarseTune`].
    CoarseTune,
    /// See [`part_settings::PartSetting::FineTune`].
    FineTune,
    /// See [`part_settings::PartSetting::BendRange`].
    BendRange,
    /// See [`part_settings::PartSetting::PortamentoSwitch`].
    PortamentoSwitch,
    /// See [`part_settings::PartSetting::PortamentoTime`].
    PortamentoTime,
    /// See [`part_settings::PartSetting::CutoffOffset`].
    CutoffOffset,
    /// See [`part_settings::PartSetting::ResonanceOffset`].
    ResonanceOffset,
    /// See [`part_settings::PartSetting::AttackOffset`].
    AttackOffset,
    /// See [`part_settings::PartSetting::DecayOffset`].
    DecayOffset,
    /// See [`part_settings::PartSetting::ReleaseOffset`].
    ReleaseOffset,
    /// See [`part_settings::PartSetting::VibratoRate`].
    VibratoRate,
    /// See [`part_settings::PartSetting::VibratoDepth`].
    VibratoDepth,
    /// See [`part_settings::PartSetting::VibratoDelay`].
    VibratoDelay,
    /// See [`part_settings::PartSetting::RxProgramChange`].
    RxProgramChange,
    /// See [`part_settings::PartSetting::RxBankSelect`].
    RxBankSelect,
    /// See [`part_settings::PartSetting::RxPitchBend`].
    RxPitchBend,
    /// See [`part_settings::PartSetting::RxPolyPressure`].
    RxPolyPressure,
    /// See [`part_settings::PartSetting::RxChannelPressure`].
    RxChannelPressure,
    /// See [`part_settings::PartSetting::RxModulation`].
    RxModulation,
    /// See [`part_settings::PartSetting::RxVolume`].
    RxVolume,
    /// See [`part_settings::PartSetting::RxPan`].
    RxPan,
    /// See [`part_settings::PartSetting::RxExpression`].
    RxExpression,
    /// See [`part_settings::PartSetting::RxHold1`].
    RxHold1,
    /// See [`part_settings::PartSetting::VoiceReserve`].
    VoiceReserve,
    /// See [`part_settings::PartSetting::ScaleTuneType`].
    ScaleTuneType,
    /// See [`part_settings::PartSetting::ScaleTuneKey`].
    ScaleTuneKey,
    /// See [`part_settings::PartSetting::ScaleTuneC`].
    ScaleTuneC,
    /// See [`part_settings::PartSetting::ScaleTuneCSharp`].
    ScaleTuneCSharp,
    /// See [`part_settings::PartSetting::ScaleTuneD`].
    ScaleTuneD,
    /// See [`part_settings::PartSetting::ScaleTuneDSharp`].
    ScaleTuneDSharp,
    /// See [`part_settings::PartSetting::ScaleTuneE`].
    ScaleTuneE,
    /// See [`part_settings::PartSetting::ScaleTuneF`].
    ScaleTuneF,
    /// See [`part_settings::PartSetting::ScaleTuneFSharp`].
    ScaleTuneFSharp,
    /// See [`part_settings::PartSetting::ScaleTuneG`].
    ScaleTuneG,
    /// See [`part_settings::PartSetting::ScaleTuneGSharp`].
    ScaleTuneGSharp,
    /// See [`part_settings::PartSetting::ScaleTuneA`].
    ScaleTuneA,
    /// See [`part_settings::PartSetting::ScaleTuneASharp`].
    ScaleTuneASharp,
    /// See [`part_settings::PartSetting::ScaleTuneB`].
    ScaleTuneB,
}

mirror_core_enum!(PartSetting <=> part_settings::PartSetting {
    KeyRangeLower,
    KeyRangeUpper,
    KeyFadeLower,
    KeyFadeUpper,
    VelocityRangeLower,
    VelocityRangeUpper,
    VelocityFadeLower,
    VelocityFadeUpper,
    VelocitySensOffset,
    MonoPoly,
    Legato,
    VelocityCurve,
    OctaveShift,
    CoarseTune,
    FineTune,
    BendRange,
    PortamentoSwitch,
    PortamentoTime,
    CutoffOffset,
    ResonanceOffset,
    AttackOffset,
    DecayOffset,
    ReleaseOffset,
    VibratoRate,
    VibratoDepth,
    VibratoDelay,
    RxProgramChange,
    RxBankSelect,
    RxPitchBend,
    RxPolyPressure,
    RxChannelPressure,
    RxModulation,
    RxVolume,
    RxPan,
    RxExpression,
    RxHold1,
    VoiceReserve,
    ScaleTuneType,
    ScaleTuneKey,
    ScaleTuneC,
    ScaleTuneCSharp,
    ScaleTuneD,
    ScaleTuneDSharp,
    ScaleTuneE,
    ScaleTuneF,
    ScaleTuneFSharp,
    ScaleTuneG,
    ScaleTuneGSharp,
    ScaleTuneA,
    ScaleTuneASharp,
    ScaleTuneB,
});

/// A group of Part settings (mirrors [`part_settings::PartSettingGroup`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartSettingGroup {
    /// See [`part_settings::PartSettingGroup::Kbd`].
    Kbd,
    /// See [`part_settings::PartSettingGroup::Pitch`].
    Pitch,
    /// See [`part_settings::PartSettingGroup::Offset`].
    Offset,
    /// See [`part_settings::PartSettingGroup::Midi`].
    Midi,
    /// See [`part_settings::PartSettingGroup::Voice`].
    Voice,
    /// See [`part_settings::PartSettingGroup::Scale`].
    Scale,
}

mirror_core_enum!(PartSettingGroup <=> part_settings::PartSettingGroup {
    Kbd,
    Pitch,
    Offset,
    Midi,
    Voice,
    Scale,
});

/// Every Part setting, in registry order: by group, as the drawer stacks
/// them.
#[wasm_bindgen]
pub fn part_setting_fields() -> Vec<PartSetting> {
    part_settings::PartSetting::ALL.map(Into::into).to_vec()
}

/// Range, default, display format and family of a Part setting.
#[wasm_bindgen]
pub fn part_setting_spec(field: PartSetting) -> WasmFieldSpec {
    wrap_field_spec(part_settings::PartSetting::from(field).def().spec)
}

/// The CLI's name for a Part setting (`key-lower`).
#[wasm_bindgen]
pub fn part_setting_name(field: PartSetting) -> String {
    part_settings::PartSetting::from(field).name().to_string()
}

/// The label a control draws for a Part setting (`Key Lower`).
#[wasm_bindgen]
pub fn part_setting_label(field: PartSetting) -> String {
    part_settings::PartSetting::from(field)
        .def()
        .label
        .to_string()
}

/// The group a Part setting belongs to.
#[wasm_bindgen]
pub fn part_setting_group(field: PartSetting) -> PartSettingGroup {
    part_settings::PartSetting::from(field).def().group.into()
}

/// A group's name, as the manual's tab spells it (`KBD`).
#[wasm_bindgen]
pub fn part_setting_group_label(group: PartSettingGroup) -> String {
    part_settings::PartSettingGroup::from(group)
        .label()
        .to_string()
}

#[wasm_bindgen]
impl WasmDeviceState {
    /// The raw value of `part`'s (0-based) `field`.
    #[wasm_bindgen(js_name = partSetting)]
    pub fn part_setting(&self, part: u8, field: PartSetting) -> Result<u8, JsError> {
        check_part(part)?;
        Ok(self.inner.part_setting(part, field.into()))
    }

    /// Set `part`'s (0-based) `field` and send it. Returns the value stored:
    /// a key or velocity Lower is held at or under its Upper, an Upper at or
    /// over its Lower.
    #[wasm_bindgen(js_name = setPartSetting)]
    pub fn set_part_setting(
        &mut self,
        part: u8,
        field: PartSetting,
        value: u8,
    ) -> Result<u8, JsError> {
        self.inner
            .set_part_setting(part, field.into(), value)
            .map_err(|e| JsError::new(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_reaches_the_web() {
        let fields = part_setting_fields();
        assert_eq!(fields.len(), part_settings::PartSetting::ALL.len());
        assert_eq!(fields[0], PartSetting::KeyRangeLower);
        assert_eq!(part_setting_name(PartSetting::KeyRangeLower), "key-lower");
        assert_eq!(
            part_setting_group(PartSetting::RxHold1),
            PartSettingGroup::Midi
        );
        assert_eq!(part_setting_group_label(PartSettingGroup::Kbd), "KBD");
        let spec = part_setting_spec(PartSetting::BendRange);
        assert_eq!((spec.min, spec.max, spec.default_value), (0, 25, 25));
        assert_eq!(spec.format_value(25), "TONE");
    }

    #[test]
    fn the_setter_writes_the_state() {
        let mut dev = WasmDeviceState::new(0x10);
        assert_eq!(
            dev.set_part_setting(2, PartSetting::CutoffOffset, 90).ok(),
            Some(90)
        );
        assert_eq!(
            dev.part_setting(2, PartSetting::CutoffOffset).ok(),
            Some(90)
        );
    }

    #[test]
    fn a_part_state_serializes_its_settings_by_cli_name() {
        let mut part = integral_core::state::PartState::default();
        part.settings
            .set(part_settings::PartSetting::KeyRangeLower, 36);
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["settings"]["key-lower"], 36);
        assert_eq!(json["settings"]["scale-type"], 1);
        let back: integral_core::state::PartState = serde_json::from_value(json).unwrap();
        assert_eq!(back, part);
        // The web's own PartState type has no settings: they default.
        let mut json = serde_json::to_value(&part).unwrap();
        json.as_object_mut().unwrap().remove("settings");
        let back: integral_core::state::PartState = serde_json::from_value(json).unwrap();
        assert_eq!(back.settings, part_settings::PartSettings::default());
    }
}
