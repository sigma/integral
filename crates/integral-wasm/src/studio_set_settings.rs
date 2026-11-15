//! WASM bindings for the Studio Set settings registry
//! (`integral_core::studio_set_settings`): Tone Control 1–4 Source, the
//! Studio Set Tempo and each MIDI channel's Phase Lock. A mixer load reads
//! their values into `readState().studioSetSettings`.

use integral_core::studio_set_settings::{self as settings, StudioSetField};
use wasm_bindgen::prelude::*;

use crate::{WasmDeviceState, WasmFieldSpec, wrap_field_spec};

/// A Studio Set setting, named without its index (mirrors
/// [`settings::StudioSetSetting`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioSetSetting {
    /// See [`settings::StudioSetSetting::ToneControlSource`].
    ToneControlSource,
    /// See [`settings::StudioSetSetting::Tempo`].
    Tempo,
    /// See [`settings::StudioSetSetting::PhaseLock`].
    PhaseLock,
}

mirror_core_enum!(StudioSetSetting <=> settings::StudioSetSetting {
    ToneControlSource,
    Tempo,
    PhaseLock,
});

/// Instance `index` (0-based) of `setting`, or a JS error past its last.
fn field(setting: StudioSetSetting, index: u8) -> Result<StudioSetField, JsError> {
    StudioSetField::new(setting.into(), index).map_err(|e| JsError::new(&e.to_string()))
}

/// Range, default and display of a Studio Set setting, shared by its
/// instances. A Tone Control source's picker lists the
/// `control_source_choices` of its spec's `controlSource`.
#[wasm_bindgen]
pub fn studio_set_setting_spec(setting: StudioSetSetting) -> WasmFieldSpec {
    wrap_field_spec(settings::StudioSetSetting::from(setting).spec())
}

/// How many instances a Studio Set setting has: 4 Tone Controls, 1 tempo,
/// one Phase Lock per MIDI channel.
#[wasm_bindgen]
pub fn studio_set_setting_count(setting: StudioSetSetting) -> u8 {
    settings::StudioSetSetting::from(setting).count()
}

/// The CLI name of instance `index` of `setting` (`tone-control-1-source`,
/// `tempo`, `phase-lock-16`).
#[wasm_bindgen]
pub fn studio_set_field_name(setting: StudioSetSetting, index: u8) -> Result<String, JsError> {
    Ok(field(setting, index)?.name())
}

/// The MIDI Implementation's name of instance `index` of `setting` (`Tone
/// Control 1 Source`, `Studio Set Tempo`, `Channel 16 Phase Lock`).
#[wasm_bindgen]
pub fn studio_set_field_label(setting: StudioSetSetting, index: u8) -> Result<String, JsError> {
    Ok(field(setting, index)?.label())
}

#[wasm_bindgen]
impl WasmDeviceState {
    /// Set instance `index` (0-based) of a Studio Set setting to `value`:
    /// the state takes it and its DT1 is queued. A Phase Lock is 0 or 1.
    #[wasm_bindgen(js_name = setStudioSetField)]
    pub fn set_studio_set_field(
        &mut self,
        setting: StudioSetSetting,
        index: u8,
        value: i32,
    ) -> Result<(), JsError> {
        self.inner
            .set_studio_set_field(field(setting, index)?, value)
            .map_err(|e| JsError::new(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_reaches_js() {
        let tempo = studio_set_setting_spec(StudioSetSetting::Tempo);
        assert_eq!((tempo.min, tempo.max), (20, 250));
        let source = studio_set_setting_spec(StudioSetSetting::ToneControlSource);
        assert_eq!(source.format_value(96), "BEND");
        assert_eq!(
            source.control_source(),
            Some(crate::ControlSourceKind::SystemControl)
        );
        assert_eq!(tempo.control_source(), None);
        assert_eq!(studio_set_setting_count(StudioSetSetting::PhaseLock), 16);
        assert_eq!(
            studio_set_field_name(StudioSetSetting::ToneControlSource, 3).unwrap(),
            "tone-control-4-source"
        );
        assert_eq!(
            studio_set_field_label(StudioSetSetting::PhaseLock, 0).unwrap(),
            "Channel 1 Phase Lock"
        );
    }

    #[test]
    fn set_studio_set_field_updates_the_state() {
        let mut dev = WasmDeviceState::new(0x10);
        dev.set_studio_set_field(StudioSetSetting::Tempo, 0, 96)
            .unwrap();
        assert_eq!(dev.inner.state().studio_set_settings.tempo, 96);
    }
}
