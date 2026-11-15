//! WASM bindings for System Common: the field registry, the block read and
//! the setters (see [`integral_core::system`]).

use integral_core::param_registry::ParamFormat;
use integral_core::system;
use wasm_bindgen::prelude::*;

use crate::{ControlSourceKind, StudioSetSetting, WasmDeviceState};

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

/// What `group` says of a System setting it leaves to the panel, or
/// `undefined` (see [`system::SystemGroup::panel_note`]).
#[wasm_bindgen]
pub fn system_group_note(group: SystemGroup) -> Option<String> {
    system::SystemGroup::from(group)
        .panel_note()
        .map(str::to_string)
}

/// A SYSTEM-or-STUDIO SET pair (mirrors [`system::SystemOrStudioSet`]).
#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemOrStudioSet {
    /// See [`system::SystemOrStudioSet::Control`].
    Control,
    /// See [`system::SystemOrStudioSet::Tempo`].
    Tempo,
}

mirror_core_enum!(SystemOrStudioSet <=> system::SystemOrStudioSet { Control, Tempo });

/// The System field that picks the side of `pair` in force.
#[wasm_bindgen]
pub fn system_pair_selector(pair: SystemOrStudioSet) -> SystemField {
    system::SystemOrStudioSet::from(pair).selector().into()
}

/// System Control `index + 1` Source (`index` 0-based), the System side of
/// the Studio Set's Tone Control of the same number; `undefined` past the
/// last.
#[wasm_bindgen]
pub fn system_control_source_field(index: u8) -> Option<SystemField> {
    system::SystemField::system_control_source(index).map(Into::into)
}

/// Every SYSTEM-or-STUDIO SET pair, in System page order.
#[wasm_bindgen]
pub fn system_pairs() -> Vec<SystemOrStudioSet> {
    system::SystemOrStudioSet::ALL
        .iter()
        .map(|&p| p.into())
        .collect()
}

/// System's side of `pair`: field `i` stands against instance `i` of
/// [`system_pair_studio_set_side`].
#[wasm_bindgen]
pub fn system_pair_system_side(pair: SystemOrStudioSet) -> Vec<SystemField> {
    system::SystemOrStudioSet::from(pair)
        .system_side()
        .iter()
        .map(|&f| f.into())
        .collect()
}

/// The Studio Set's side of `pair`: the setting edited in the Studio Set
/// settings drawer.
#[wasm_bindgen]
pub fn system_pair_studio_set_side(pair: SystemOrStudioSet) -> StudioSetSetting {
    system::SystemOrStudioSet::from(pair)
        .studio_set_side()
        .into()
}

/// Whether `selector_value`, the raw value of `pair`'s
/// [`system_pair_selector`], puts the Studio Set's side in force.
#[wasm_bindgen]
pub fn system_pair_studio_set_in_force(pair: SystemOrStudioSet, selector_value: u16) -> bool {
    system::SystemOrStudioSet::from(pair).studio_set_in_force(selector_value)
}

/// The address of the System Common block: what `requestData` reads.
#[wasm_bindgen]
pub fn system_common_address() -> Vec<u8> {
    integral_core::address::SYSTEM_COMMON.as_bytes().to_vec()
}

/// The size of the System Common read: the whole block.
#[wasm_bindgen]
pub fn system_common_size() -> Vec<u8> {
    system::SYSTEM_COMMON_SIZE.as_bytes().to_vec()
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

    /// Whether the field names its values from a list (`SYSTEM`,
    /// `STUDIO SET`): a picker lists them by
    /// [`format_value`](Self::format_value).
    #[wasm_bindgen(getter, js_name = namedList)]
    pub fn named_list(&self) -> bool {
        matches!(self.0.format, ParamFormat::Named(_))
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

    /// Whether the Studio Set's side of `pair` is in force.
    #[wasm_bindgen(js_name = studioSetInForce)]
    pub fn studio_set_in_force(&self, pair: SystemOrStudioSet) -> bool {
        self.0.studio_set_in_force(pair.into())
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
    fn read_covers_the_block_the_core_requests() {
        let rq1 = system::build_read_request(0x10);
        assert_eq!(rq1[7..11], system_common_address()[..]);
        assert_eq!(rq1[11..15], system_common_size()[..]);
    }

    #[test]
    fn named_lists_are_flagged() {
        assert!(system_field_spec(SystemField::StudioSetControlChannel).named_list());
        assert!(!system_field_spec(SystemField::SystemTempo).named_list());
        assert!(!system_field_spec(SystemField::SystemControl1Source).named_list());
    }

    #[test]
    fn output_carries_the_ext_part_source_note() {
        assert_eq!(
            system_group_note(SystemGroup::Output).as_deref(),
            Some(system::EXT_PART_SOURCE_NOTE)
        );
        assert_eq!(system_group_note(SystemGroup::Tuning), None);
    }

    #[test]
    fn the_pairs_reach_js() {
        assert_eq!(
            system_pair_selector(SystemOrStudioSet::Tempo),
            SystemField::TempoAssignSource
        );
        assert_eq!(
            system_control_source_field(0),
            Some(SystemField::SystemControl1Source)
        );
        assert_eq!(system_control_source_field(4), None);
        assert!(!system_pair_studio_set_in_force(
            SystemOrStudioSet::Control,
            0
        ));
        assert!(system_pair_studio_set_in_force(
            SystemOrStudioSet::Control,
            1
        ));
        assert_eq!(
            system_pairs(),
            [SystemOrStudioSet::Control, SystemOrStudioSet::Tempo]
        );
        assert_eq!(
            system_pair_system_side(SystemOrStudioSet::Tempo),
            [SystemField::SystemTempo]
        );
        assert_eq!(
            system_pair_studio_set_side(SystemOrStudioSet::Control),
            StudioSetSetting::ToneControlSource
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
