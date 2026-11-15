//! WASM bindings for System Common: the field registry, the block read and
//! the setters (see [`integral_core::system`]).

use integral_core::param_registry::ParamFormat;
use integral_core::system;
use wasm_bindgen::prelude::*;

use crate::{ControlSourceKind, WasmDeviceState};

/// A System Common field (mirrors [`system::SystemField`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemField {
    /// See [`system::SystemField::MasterTune`].
    MasterTune,
    /// See [`system::SystemField::MasterKeyShift`].
    MasterKeyShift,
    /// See [`system::SystemField::ScaleTuneSwitch`].
    ScaleTuneSwitch,
    /// See [`system::SystemField::StudioSetControlChannel`].
    StudioSetControlChannel,
    /// See [`system::SystemField::SystemControl1Source`].
    SystemControl1Source,
    /// See [`system::SystemField::SystemControl2Source`].
    SystemControl2Source,
    /// See [`system::SystemField::SystemControl3Source`].
    SystemControl3Source,
    /// See [`system::SystemField::SystemControl4Source`].
    SystemControl4Source,
    /// See [`system::SystemField::ControlSource`].
    ControlSource,
    /// See [`system::SystemField::ClockSource`].
    ClockSource,
    /// See [`system::SystemField::SystemTempo`].
    SystemTempo,
    /// See [`system::SystemField::TempoAssignSource`].
    TempoAssignSource,
    /// See [`system::SystemField::ReceiveProgramChange`].
    ReceiveProgramChange,
    /// See [`system::SystemField::ReceiveBankSelect`].
    ReceiveBankSelect,
    /// See [`system::SystemField::CenterSpeakerSwitch`].
    CenterSpeakerSwitch,
    /// See [`system::SystemField::SubWooferSwitch`].
    SubWooferSwitch,
    /// See [`system::SystemField::TwoChOutputMode`].
    TwoChOutputMode,
    /// See [`system::SystemField::UsbAudioSamplingRate`].
    UsbAudioSamplingRate,
}

mirror_core_enum!(SystemField <=> system::SystemField {
    MasterTune,
    MasterKeyShift,
    ScaleTuneSwitch,
    StudioSetControlChannel,
    SystemControl1Source,
    SystemControl2Source,
    SystemControl3Source,
    SystemControl4Source,
    ControlSource,
    ClockSource,
    SystemTempo,
    TempoAssignSource,
    ReceiveProgramChange,
    ReceiveBankSelect,
    CenterSpeakerSwitch,
    SubWooferSwitch,
    TwoChOutputMode,
    UsbAudioSamplingRate,
});

/// A group of the System page (mirrors [`system::SystemGroup`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemGroup {
    /// See [`system::SystemGroup::Tuning`].
    Tuning,
    /// See [`system::SystemGroup::Control`].
    Control,
    /// See [`system::SystemGroup::SyncTempo`].
    SyncTempo,
    /// See [`system::SystemGroup::MidiRx`].
    MidiRx,
    /// See [`system::SystemGroup::Output`].
    Output,
    /// See [`system::SystemGroup::Usb`].
    Usb,
}

mirror_core_enum!(SystemGroup <=> system::SystemGroup {
    Tuning,
    Control,
    SyncTempo,
    MidiRx,
    Output,
    Usb,
});

/// Every System field, in page order.
#[wasm_bindgen]
pub fn system_fields() -> Vec<SystemField> {
    system::SystemField::ALL.iter().map(|&f| f.into()).collect()
}

/// Every System page group, in page order.
#[wasm_bindgen]
pub fn system_groups() -> Vec<SystemGroup> {
    system::SystemGroup::ALL.iter().map(|&g| g.into()).collect()
}

/// The heading of a System page group.
#[wasm_bindgen]
pub fn system_group_label(group: SystemGroup) -> String {
    system::SystemGroup::from(group).label().to_string()
}

/// The line the System page shows: System edits last until power-off.
#[wasm_bindgen]
pub fn system_volatile_notice() -> String {
    system::VOLATILE_NOTICE.to_string()
}

/// Name, group, range and display of a System field (mirrors
/// [`system::SystemFieldSpec`]).
#[wasm_bindgen]
pub struct WasmSystemFieldSpec(system::SystemFieldSpec);

#[wasm_bindgen]
impl WasmSystemFieldSpec {
    /// The name `integral system` takes (`master-tune`).
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.0.name.to_string()
    }

    /// The field's label.
    #[wasm_bindgen(getter)]
    pub fn label(&self) -> String {
        self.0.label.to_string()
    }

    /// The System page group it sits in.
    #[wasm_bindgen(getter)]
    pub fn group(&self) -> SystemGroup {
        self.0.group.into()
    }

    /// Minimum raw value.
    #[wasm_bindgen(getter)]
    pub fn min(&self) -> i32 {
        self.0.min
    }

    /// Maximum raw value.
    #[wasm_bindgen(getter)]
    pub fn max(&self) -> i32 {
        self.0.max
    }

    /// The raw value a control centres on, for signed formats.
    #[wasm_bindgen(getter)]
    pub fn center(&self) -> Option<i32> {
        self.0.format.center()
    }

    /// The Control Source list the field names its values from, or
    /// `undefined`: a picker lists [`crate::control_source_choices`].
    #[wasm_bindgen(getter, js_name = controlSource)]
    pub fn control_source(&self) -> Option<ControlSourceKind> {
        match self.0.format {
            ParamFormat::ControlSource(kind) => Some(kind.into()),
            _ => None,
        }
    }

    /// Format a raw value for display.
    #[wasm_bindgen(js_name = formatValue)]
    pub fn format_value(&self, value: i32) -> String {
        self.0.format.format(value)
    }

    /// A raw value's readout without its unit (`440.00` of `440.00 Hz`).
    #[wasm_bindgen(js_name = formatNumber)]
    pub fn format_number(&self, value: i32) -> String {
        self.0.format.parts(value).0
    }

    /// The raw value a display string stands for, or `undefined` (see
    /// [`system::SystemFieldSpec::parse`]).
    #[wasm_bindgen(js_name = parseValue)]
    pub fn parse_value(&self, text: &str) -> Option<i32> {
        self.0.parse(text)
    }
}

/// The spec of a System field.
#[wasm_bindgen]
pub fn system_field_spec(field: SystemField) -> WasmSystemFieldSpec {
    WasmSystemFieldSpec(system::system_spec(field.into()))
}

/// A parsed System Common dump (mirrors [`system::SystemCommon`]).
#[wasm_bindgen]
pub struct WasmSystemCommon(system::SystemCommon);

#[wasm_bindgen]
impl WasmSystemCommon {
    /// The raw value of `field`.
    pub fn get(&self, field: SystemField) -> u16 {
        self.0.get(field.into())
    }

    /// The display of `field`'s value.
    pub fn display(&self, field: SystemField) -> String {
        self.0.display(field.into())
    }
}

/// Parse a System Common dump: the data of the reply to
/// `buildSystemCommonRequest`.
///
/// # Errors
/// The dump is shorter than the 47-byte block.
#[wasm_bindgen(js_name = parseSystemCommon)]
pub fn parse_system_common(data: &[u8]) -> Result<WasmSystemCommon, JsError> {
    Ok(WasmSystemCommon(system::parse_system_common(data)?))
}

#[wasm_bindgen]
impl WasmDeviceState {
    /// Build the RQ1 that reads all of System Common.
    #[wasm_bindgen(js_name = buildSystemCommonRequest)]
    pub fn build_system_common_request(&self) -> Vec<u8> {
        self.inner.build_system_common_request()
    }

    /// Set a System field (nibblized when the field is). The edit lasts
    /// until power-off.
    ///
    /// # Errors
    /// `value` is outside the field's range; nothing is sent.
    #[wasm_bindgen(js_name = setSystemField)]
    pub fn set_system_field(&mut self, field: SystemField, value: i32) -> Result<(), JsError> {
        Ok(self.inner.set_system_field(field.into(), value)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core_field_is_mirrored_in_order() {
        let mirrored: Vec<system::SystemField> =
            system_fields().into_iter().map(Into::into).collect();
        assert_eq!(mirrored, system::SystemField::ALL);
    }

    #[test]
    fn system_control_sources_name_their_list() {
        assert_eq!(
            system_field_spec(SystemField::SystemControl2Source).control_source(),
            Some(ControlSourceKind::SystemControl)
        );
        assert_eq!(
            system_field_spec(SystemField::SystemTempo).control_source(),
            None
        );
    }

    #[test]
    fn setter_queues_the_core_dt1() {
        let mut dev = WasmDeviceState::new(0x10);
        let mut core = integral_core::device::DeviceState::new(0x10);
        dev.set_system_field(SystemField::SystemTempo, 121).unwrap();
        core.set_system_field(system::SystemField::SystemTempo, 121)
            .unwrap();
        assert_eq!(dev.inner.drain(0.0), core.drain(0.0));
    }
}
