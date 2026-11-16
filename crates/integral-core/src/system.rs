//! System Common (`02 00 00 00`, 47 bytes): the registry of its fields,
//! the block read and its parse.
//!
//! Every field the System page and `integral system` show is a
//! [`SystemField`]; [`system_spec`] gives its name, address, range and
//! display, from `docs/midi/04-address-map.md` and, for `00 2E` (USB Audio
//! Sampling Rate), `docs/midi/99-undocumented.md` §8. The fields are
//! grouped as the System page shows them ([`SystemGroup`], #284).
//!
//! Two System Common parameters are not fields here (#284): Master Level
//! stays in the Mixer ([`MixerField::MasterLevel`]), and Ext Part Source
//! Select is a Studio Set byte a panel edit doesn't reach (#283).
//!
//! System edits last until power-off: no SysEx saves them, and the User
//! write request doesn't commit them (`docs/midi/99-undocumented.md` §8).
//!
//! [`MixerField::MasterLevel`]: crate::param_registry::MixerField::MasterLevel

use thiserror::Error;

use crate::address::{Address, DataSize, SYSTEM_COMMON};
use crate::block_field::BlockField;
use crate::fx_params::KHZ;
use crate::host_params::HostFormat;
use crate::mfx_params::ControlSourceKind;
use crate::param_registry::ParamFormat;
use crate::studio_set_settings::StudioSetSetting;
use crate::sysex;

/// Bytes in the System Common block: what one RQ1 of it returns.
pub const SYSTEM_COMMON_LEN: usize = 0x2F;

/// The size of the System Common read: [`SYSTEM_COMMON_LEN`] bytes.
pub const SYSTEM_COMMON_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, SYSTEM_COMMON_LEN as u8);

/// The raw Master Tune that tunes A4 to [`MASTER_TUNE_A4_HZ`] (0 cent).
pub const MASTER_TUNE_CENTER: i32 = 1024;

/// Master Tune steps either side of [`MASTER_TUNE_CENTER`]: ±100.0 cent.
const MASTER_TUNE_SPAN: i32 = 1000;

/// A4 in Hz at Master Tune 0 cent.
pub const MASTER_TUNE_A4_HZ: f64 = 440.0;

/// Master Tune steps per cent.
const MASTER_TUNE_STEPS_PER_CENT: f64 = 10.0;

/// Cents per octave.
const CENTS_PER_OCTAVE: f64 = 1200.0;

/// The A4 frequency in Hz of raw Master Tune `value` (0.1 cent a step,
/// 0 cent at [`MASTER_TUNE_CENTER`]).
pub fn master_tune_hz(value: i32) -> f64 {
    let cents = f64::from(value - MASTER_TUNE_CENTER) / MASTER_TUNE_STEPS_PER_CENT;
    MASTER_TUNE_A4_HZ * (cents / CENTS_PER_OCTAVE).exp2()
}

/// The raw Master Tune nearest to A4 at `hz`; it may be out of range.
pub fn master_tune_from_hz(hz: f64) -> i32 {
    let cents = CENTS_PER_OCTAVE * (hz / MASTER_TUNE_A4_HZ).log2();
    MASTER_TUNE_CENTER + (cents * MASTER_TUNE_STEPS_PER_CENT).round() as i32
}

/// What the panel and the System page say of System edits, which no SysEx
/// saves.
pub const VOLATILE_NOTICE: &str = "These settings last until power-off. To keep them, save on \
     the INTEGRA-7's panel: [SHIFT] + [EDIT] in the SYSTEM screen.";

/// A System Common field. The discriminant is its byte position in the
/// block; [`BlockField`] gives the nibblized fields' width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum SystemField {
    /// Master Tune (nibblized, 4 bytes).
    MasterTune = 0x00,
    /// Master Key Shift.
    MasterKeyShift = 0x04,
    /// Scale Tune Switch.
    ScaleTuneSwitch = 0x06,
    /// Studio Set Control Channel.
    StudioSetControlChannel = 0x11,
    /// System Control 1 Source.
    SystemControl1Source = 0x20,
    /// System Control 2 Source.
    SystemControl2Source = 0x21,
    /// System Control 3 Source.
    SystemControl3Source = 0x22,
    /// System Control 4 Source.
    SystemControl4Source = 0x23,
    /// Control Source: whether System Control 1–4 or the Studio Set's Tone
    /// Control 1–4 are in force.
    ControlSource = 0x24,
    /// System Clock Source.
    ClockSource = 0x25,
    /// System Tempo (nibblized, 2 bytes).
    SystemTempo = 0x26,
    /// Tempo Assign Source: whether the System Tempo or the Studio Set's
    /// is in force.
    TempoAssignSource = 0x28,
    /// Receive Program Change.
    ReceiveProgramChange = 0x29,
    /// Receive Bank Select.
    ReceiveBankSelect = 0x2A,
    /// 5.1CH Center Speaker Switch.
    CenterSpeakerSwitch = 0x2B,
    /// 5.1CH Sub Woofer Switch.
    SubWooferSwitch = 0x2C,
    /// 2CH Output Mode.
    TwoChOutputMode = 0x2D,
    /// USB Audio Sampling Rate (a reserve in the MIDI Implementation).
    UsbAudioSamplingRate = 0x2E,
}

impl SystemField {
    /// Every field, in block (device address) order, which is also the
    /// System page's group order.
    pub const ALL: &[Self] = &[
        Self::MasterTune,
        Self::MasterKeyShift,
        Self::ScaleTuneSwitch,
        Self::StudioSetControlChannel,
        Self::SystemControl1Source,
        Self::SystemControl2Source,
        Self::SystemControl3Source,
        Self::SystemControl4Source,
        Self::ControlSource,
        Self::ClockSource,
        Self::SystemTempo,
        Self::TempoAssignSource,
        Self::ReceiveProgramChange,
        Self::ReceiveBankSelect,
        Self::CenterSpeakerSwitch,
        Self::SubWooferSwitch,
        Self::TwoChOutputMode,
        Self::UsbAudioSamplingRate,
    ];

    /// The field whose [`SystemFieldSpec::name`] is `name`.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|&f| system_spec(f).name == name)
    }

    /// The field's address on the device.
    pub const fn address(self) -> Address {
        SYSTEM_COMMON.offset([0, 0, 0, self as u8])
    }

    /// System Control `index + 1` Source (`index` 0-based), the System side
    /// of the Studio Set's Tone Control of the same number; `None` past
    /// [`TONE_CONTROLS`](crate::device_spec::TONE_CONTROLS).
    pub fn system_control_source(index: u8) -> Option<Self> {
        SystemOrStudioSet::Control
            .system_side()
            .get(usize::from(index))
            .copied()
    }
}

/// A SYSTEM-or-STUDIO SET pair: a System Common selector decides whether
/// System's setting or the current Studio Set's is in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemOrStudioSet {
    /// System Control 1–4 Source or the Studio Set's Tone Control 1–4
    /// Source, by [`SystemField::ControlSource`].
    Control,
    /// System Tempo or the Studio Set Tempo, by
    /// [`SystemField::TempoAssignSource`].
    Tempo,
}

impl SystemOrStudioSet {
    /// Every pair, in System page order.
    pub const ALL: [Self; 2] = [Self::Control, Self::Tempo];

    /// System's side: the fields the Studio Set's
    /// [`studio_set_side`](Self::studio_set_side) stands in for, instance
    /// `i` of one against field `i` of the other (System Control 1 against
    /// Tone Control 1).
    pub const fn system_side(self) -> &'static [SystemField] {
        match self {
            Self::Control => &[
                SystemField::SystemControl1Source,
                SystemField::SystemControl2Source,
                SystemField::SystemControl3Source,
                SystemField::SystemControl4Source,
            ],
            Self::Tempo => &[SystemField::SystemTempo],
        }
    }

    /// The Studio Set's side: the setting whose instances stand in for
    /// [`system_side`](Self::system_side).
    pub const fn studio_set_side(self) -> StudioSetSetting {
        match self {
            Self::Control => StudioSetSetting::ToneControlSource,
            Self::Tempo => StudioSetSetting::Tempo,
        }
    }

    /// The System field that picks the side in force.
    pub const fn selector(self) -> SystemField {
        match self {
            Self::Control => SystemField::ControlSource,
            Self::Tempo => SystemField::TempoAssignSource,
        }
    }

    /// Whether `selector_value`, the raw value of
    /// [`selector`](Self::selector), puts the Studio Set's side in force
    /// (System's is otherwise).
    pub const fn studio_set_in_force(self, selector_value: u16) -> bool {
        selector_value == STUDIO_SET_SIDE
    }
}

impl BlockField for SystemField {
    fn position(self) -> usize {
        self as usize
    }

    fn width(self) -> usize {
        match self {
            Self::MasterTune => 4,
            Self::SystemTempo => 2,
            _ => 1,
        }
    }
}

/// A group of the System page (#284).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SystemGroup {
    /// Master Tune, Master Key Shift, Scale Tune Switch.
    Tuning,
    /// Studio Set Control Channel, System Control 1–4 Source, Control Source.
    Control,
    /// Clock Source, System Tempo, Tempo Assign Source.
    SyncTempo,
    /// Receive Program Change and Bank Select.
    MidiRx,
    /// The 5.1ch switches and the 2CH Output Mode.
    Output,
    /// USB Audio Sampling Rate.
    Usb,
}

impl SystemGroup {
    /// Every group, in page order.
    pub const ALL: &[Self] = &[
        Self::Tuning,
        Self::Control,
        Self::SyncTempo,
        Self::MidiRx,
        Self::Output,
        Self::Usb,
    ];

    /// The group's heading.
    pub fn label(self) -> &'static str {
        match self {
            Self::Tuning => "Tuning",
            Self::Control => "Control",
            Self::SyncTempo => "Sync/Tempo",
            Self::MidiRx => "MIDI Rx",
            Self::Output => "Output",
            Self::Usb => "USB",
        }
    }

    /// What the group says of a System setting it leaves to the panel, if
    /// any: Output points to Ext Part Source Select (#284), whose byte a
    /// panel edit doesn't reach (#283).
    pub fn panel_note(self) -> Option<&'static str> {
        match self {
            Self::Output => Some(EXT_PART_SOURCE_NOTE),
            _ => None,
        }
    }
}

/// Where Ext Part Source Select is set: on the panel, not from here (#284).
pub const EXT_PART_SOURCE_NOTE: &str =
    "Ext Part Source Select is set on the panel, in SYSTEM > SOUND.";

/// Name, group, range and display of a [`SystemField`]; its address is
/// [`SystemField::address`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemFieldSpec {
    /// The field.
    pub field: SystemField,
    /// The name `integral system` takes and shows (`master-tune`).
    pub name: &'static str,
    /// The field's label, in the MIDI Implementation's wording.
    pub label: &'static str,
    /// The System page group it sits in.
    pub group: SystemGroup,
    /// Minimum raw value (inclusive).
    pub min: i32,
    /// Maximum raw value (inclusive).
    pub max: i32,
    /// Display format.
    pub format: ParamFormat,
}

impl SystemFieldSpec {
    /// Whether `value` is in the field's range.
    pub fn accepts(&self, value: i32) -> bool {
        (self.min..=self.max).contains(&value)
    }

    /// The raw value a display string stands for: a value in range whose
    /// [`format`](ParamFormat::format) is `text`, ignoring case (`ON`,
    /// `studio set`, `-3`, `CC01`), with its unit and a `+` optional (`3`,
    /// `44.1`), by [`HostFormat::parse`]: a value a Control Source list
    /// doesn't name (the unused 32) doesn't parse. Master Tune takes Hz
    /// (`442`, `442.0 Hz`) and gives the nearest step. `None` when nothing
    /// in range shows as `text`.
    pub fn parse(&self, text: &str) -> Option<i32> {
        if self.format == ParamFormat::MasterTune {
            let text = text.trim().strip_prefix('+').unwrap_or(text.trim());
            let cut = text.len().saturating_sub(2);
            let hz = match text.get(cut..) {
                Some(unit) if unit.eq_ignore_ascii_case("hz") => &text[..cut],
                _ => text,
            }
            .trim();
            let value = master_tune_from_hz(hz.parse().ok()?);
            return self.accepts(value).then_some(value);
        }
        HostFormat::Field(self.format.clone()).parse(text, self.min, self.max)
    }
}

/// Studio Set Control Channel: 0–15 are channels 1–16, 16 is OFF.
const CONTROL_CHANNELS: &[&str] = &[
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "OFF",
];
/// Control Source and Tempo Assign Source.
const SYSTEM_OR_STUDIO_SET: &[&str] = &["SYSTEM", "STUDIO SET"];
/// The raw value of a [`SystemOrStudioSet`] selector that puts the Studio
/// Set's side in force: its place in [`SYSTEM_OR_STUDIO_SET`].
const STUDIO_SET_SIDE: u16 = 1;
/// System Clock Source.
const CLOCK_SOURCES: &[&str] = &["MIDI", "USB"];
/// 2CH Output Mode.
const OUTPUT_MODES: &[&str] = &["SPEAKER", "PHONES"];
/// USB Audio Sampling Rate, per Roland's editor (`00`, `01` checked on the
/// device; `02` never written: `docs/midi/99-undocumented.md` §8).
const SAMPLING_RATES: ParamFormat = ParamFormat::Table {
    values: &["44.1", "48", "96"],
    unit: KHZ,
};

/// The highest raw value of a [`ParamFormat::Named`] list.
const fn last(names: &[&str]) -> i32 {
    names.len() as i32 - 1
}

/// The spec of a System Common field, from `docs/midi/04-address-map.md`
/// ("System Common Parameters") and `docs/midi/99-undocumented.md` §8.
pub fn system_spec(field: SystemField) -> SystemFieldSpec {
    use ParamFormat::{MasterTune, Named, OnOff, Raw, SignedCenter};
    use SystemField as F;
    use SystemGroup as G;
    let control = |name, label| {
        (
            name,
            label,
            G::Control,
            0,
            i32::from(ControlSourceKind::SystemControl.max()),
            ParamFormat::ControlSource(ControlSourceKind::SystemControl),
        )
    };
    let (name, label, group, min, max, format) = match field {
        F::MasterTune => (
            "master-tune",
            "Master Tune",
            G::Tuning,
            MASTER_TUNE_CENTER - MASTER_TUNE_SPAN,
            MASTER_TUNE_CENTER + MASTER_TUNE_SPAN,
            MasterTune,
        ),
        F::MasterKeyShift => (
            "master-key-shift",
            "Master Key Shift",
            G::Tuning,
            40,
            88,
            SignedCenter(64),
        ),
        F::ScaleTuneSwitch => (
            "scale-tune-switch",
            "Scale Tune Switch",
            G::Tuning,
            0,
            1,
            OnOff,
        ),
        F::StudioSetControlChannel => (
            "studio-set-control-channel",
            "Studio Set Control Channel",
            G::Control,
            0,
            last(CONTROL_CHANNELS),
            Named(CONTROL_CHANNELS),
        ),
        F::SystemControl1Source => control("system-control-1-source", "System Control 1 Source"),
        F::SystemControl2Source => control("system-control-2-source", "System Control 2 Source"),
        F::SystemControl3Source => control("system-control-3-source", "System Control 3 Source"),
        F::SystemControl4Source => control("system-control-4-source", "System Control 4 Source"),
        F::ControlSource => (
            "control-source",
            "Control Source",
            G::Control,
            0,
            last(SYSTEM_OR_STUDIO_SET),
            Named(SYSTEM_OR_STUDIO_SET),
        ),
        F::ClockSource => (
            "clock-source",
            "System Clock Source",
            G::SyncTempo,
            0,
            last(CLOCK_SOURCES),
            Named(CLOCK_SOURCES),
        ),
        F::SystemTempo => ("system-tempo", "System Tempo", G::SyncTempo, 20, 250, Raw),
        F::TempoAssignSource => (
            "tempo-assign-source",
            "Tempo Assign Source",
            G::SyncTempo,
            0,
            last(SYSTEM_OR_STUDIO_SET),
            Named(SYSTEM_OR_STUDIO_SET),
        ),
        F::ReceiveProgramChange => (
            "receive-program-change",
            "Receive Program Change",
            G::MidiRx,
            0,
            1,
            OnOff,
        ),
        F::ReceiveBankSelect => (
            "receive-bank-select",
            "Receive Bank Select",
            G::MidiRx,
            0,
            1,
            OnOff,
        ),
        F::CenterSpeakerSwitch => (
            "center-speaker-switch",
            "5.1CH Center Speaker Switch",
            G::Output,
            0,
            1,
            OnOff,
        ),
        F::SubWooferSwitch => (
            "sub-woofer-switch",
            "5.1CH Sub Woofer Switch",
            G::Output,
            0,
            1,
            OnOff,
        ),
        F::TwoChOutputMode => (
            "2ch-output-mode",
            "2CH Output Mode",
            G::Output,
            0,
            last(OUTPUT_MODES),
            Named(OUTPUT_MODES),
        ),
        F::UsbAudioSamplingRate => (
            "usb-audio-sampling-rate",
            "USB Audio Sampling Rate",
            G::Usb,
            0,
            // A read may show 96 kHz, but `02` isn't written until a device
            // check shows what it does (#283).
            1,
            SAMPLING_RATES,
        ),
    };
    SystemFieldSpec {
        field,
        name,
        label,
        group,
        min,
        max,
        format,
    }
}

/// The RQ1 that reads all of System Common.
pub fn build_read_request(device_id: u8) -> Vec<u8> {
    sysex::build_rq1(device_id, &SYSTEM_COMMON, &SYSTEM_COMMON_SIZE)
}

/// A System Common dump shorter than the block.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("System Common dump too short: expected {SYSTEM_COMMON_LEN} bytes, got {got}")]
pub struct SystemCommonTooShort {
    /// Bytes received.
    pub got: usize,
}

/// A System Common dump: every [`SystemField`]'s value, and the block's
/// reserves as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemCommon {
    data: [u8; SYSTEM_COMMON_LEN],
}

impl SystemCommon {
    /// The raw value of `field`, nibbles joined.
    pub fn get(&self, field: SystemField) -> u16 {
        field.value(&self.data)
    }

    /// The display of `field`'s value, per its [`system_spec`].
    pub fn display(&self, field: SystemField) -> String {
        system_spec(field).format.format(self.get(field).into())
    }

    /// The block's bytes, as read.
    pub fn bytes(&self) -> &[u8; SYSTEM_COMMON_LEN] {
        &self.data
    }

    /// Whether the Studio Set's side of `pair` is in force (System's is
    /// otherwise).
    pub fn studio_set_in_force(&self, pair: SystemOrStudioSet) -> bool {
        pair.studio_set_in_force(self.get(pair.selector()))
    }
}

/// Parse a System Common dump (the data of the reply to
/// [`build_read_request`]). Bytes past the block are ignored.
///
/// # Errors
/// `data` is shorter than [`SYSTEM_COMMON_LEN`].
pub fn parse_system_common(data: &[u8]) -> Result<SystemCommon, SystemCommonTooShort> {
    let block = data
        .get(..SYSTEM_COMMON_LEN)
        .ok_or(SystemCommonTooShort { got: data.len() })?;
    let mut parsed = SystemCommon {
        data: [0; SYSTEM_COMMON_LEN],
    };
    parsed.data.copy_from_slice(block);
    Ok(parsed)
}

/// A value outside a System field's range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{field:?} takes {min}–{max}, not {value}")]
pub struct SystemValueOutOfRange {
    /// The field written.
    pub field: SystemField,
    /// The value refused.
    pub value: i32,
    /// The field's minimum.
    pub min: i32,
    /// The field's maximum.
    pub max: i32,
}

/// The DT1 data that writes `value` to `field`, nibblized when the field
/// is.
///
/// # Errors
/// `value` is outside the field's range.
pub fn encode_value(field: SystemField, value: i32) -> Result<Vec<u8>, SystemValueOutOfRange> {
    let spec = system_spec(field);
    let raw = u16::try_from(value)
        .ok()
        .filter(|_| spec.accepts(value))
        .ok_or(SystemValueOutOfRange {
            field,
            value,
            min: spec.min,
            max: spec.max,
        })?;
    Ok(field.encode(raw))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mfx_params::control_source_choices;

    /// Each field's offset, range and the display of its ends, from the
    /// MIDI Implementation (`04-address-map.md`, `99-undocumented.md` §8
    /// for `00 2E`). Master Tune's ±100.0 cent shows as A4 in Hz, and the
    /// USB rate stops at the 48 kHz checked on the device.
    #[rustfmt::skip]
    const DOC: &[(SystemField, u8, i32, i32, &str, &str)] = &[
        (SystemField::MasterTune, 0x00, 24, 2024, "415.30 Hz", "466.16 Hz"),
        (SystemField::MasterKeyShift, 0x04, 40, 88, "-24", "+24"),
        (SystemField::ScaleTuneSwitch, 0x06, 0, 1, "OFF", "ON"),
        (SystemField::StudioSetControlChannel, 0x11, 0, 16, "1", "OFF"),
        (SystemField::SystemControl1Source, 0x20, 0, 97, "OFF", "AFT"),
        (SystemField::SystemControl2Source, 0x21, 0, 97, "OFF", "AFT"),
        (SystemField::SystemControl3Source, 0x22, 0, 97, "OFF", "AFT"),
        (SystemField::SystemControl4Source, 0x23, 0, 97, "OFF", "AFT"),
        (SystemField::ControlSource, 0x24, 0, 1, "SYSTEM", "STUDIO SET"),
        (SystemField::ClockSource, 0x25, 0, 1, "MIDI", "USB"),
        (SystemField::SystemTempo, 0x26, 20, 250, "20", "250"),
        (SystemField::TempoAssignSource, 0x28, 0, 1, "SYSTEM", "STUDIO SET"),
        (SystemField::ReceiveProgramChange, 0x29, 0, 1, "OFF", "ON"),
        (SystemField::ReceiveBankSelect, 0x2A, 0, 1, "OFF", "ON"),
        (SystemField::CenterSpeakerSwitch, 0x2B, 0, 1, "OFF", "ON"),
        (SystemField::SubWooferSwitch, 0x2C, 0, 1, "OFF", "ON"),
        (SystemField::TwoChOutputMode, 0x2D, 0, 1, "SPEAKER", "PHONES"),
        (SystemField::UsbAudioSamplingRate, 0x2E, 0, 1, "44.1 kHz", "48 kHz"),
    ];

    #[test]
    fn every_field_matches_the_docs() {
        assert_eq!(DOC.len(), SystemField::ALL.len());
        for (&(field, offset, min, max, low, high), &listed) in DOC.iter().zip(SystemField::ALL) {
            assert_eq!(field, listed, "ALL is in address order");
            let spec = system_spec(field);
            assert_eq!(
                field.address(),
                Address::new(0x02, 0x00, 0x00, offset),
                "{field:?}"
            );
            assert_eq!((spec.min, spec.max), (min, max), "{field:?} range");
            assert_eq!(spec.format.format(min), low, "{field:?} at {min}");
            assert_eq!(spec.format.format(max), high, "{field:?} at {max}");
        }
    }

    #[test]
    fn master_tune_is_440_hz_at_its_center_in_tenths_of_a_cent() {
        let tune = system_spec(SystemField::MasterTune).format;
        assert_eq!(tune.format(1024), "440.00 Hz");
        assert_eq!(tune.center(), Some(1024));
        // Each 0.1-cent step reads apart.
        assert_eq!(tune.format(1102), "441.99 Hz");
        assert_eq!(tune.format(1103), "442.01 Hz");
        let shown: std::collections::HashSet<_> = (24..=2024).map(|v| tune.format(v)).collect();
        assert_eq!(shown.len(), 2001);
    }

    #[test]
    fn display_strings_parse_back_to_raw_values() {
        let parse = |field, text| system_spec(field).parse(text);
        // Master Tune takes Hz and gives the nearest 0.1 cent (+7.9).
        assert_eq!(parse(SystemField::MasterTune, "442.0"), Some(1103));
        assert_eq!(parse(SystemField::MasterTune, "440 Hz"), Some(1024));
        assert_eq!(parse(SystemField::MasterTune, "400"), None);
        assert_eq!(parse(SystemField::MasterKeyShift, "-3"), Some(61));
        assert_eq!(parse(SystemField::MasterKeyShift, "+3"), Some(67));
        assert_eq!(parse(SystemField::MasterKeyShift, "3"), Some(67));
        assert_eq!(parse(SystemField::ScaleTuneSwitch, "on"), Some(1));
        assert_eq!(parse(SystemField::StudioSetControlChannel, "OFF"), Some(16));
        assert_eq!(parse(SystemField::StudioSetControlChannel, "1"), Some(0));
        assert_eq!(parse(SystemField::SystemControl1Source, "cc33"), Some(33));
        assert_eq!(parse(SystemField::ControlSource, "Studio Set"), Some(1));
        assert_eq!(parse(SystemField::SystemTempo, "121"), Some(121));
        assert_eq!(parse(SystemField::SystemTempo, "251"), None);
        assert_eq!(parse(SystemField::UsbAudioSamplingRate, "48"), Some(1));
        assert_eq!(parse(SystemField::UsbAudioSamplingRate, "96 kHz"), None);
        assert_eq!(parse(SystemField::ReceiveBankSelect, "maybe"), None);
    }

    #[test]
    fn master_tune_unit_ignores_case() {
        let parse = |text| system_spec(SystemField::MasterTune).parse(text);
        assert_eq!(parse("442 hz"), Some(1103));
        assert_eq!(parse("442 HZ"), Some(1103));
        assert_eq!(parse("442hZ"), Some(1103));
        assert_eq!(parse("hz"), None);
    }

    #[test]
    fn every_display_parses_back() {
        for &field in SystemField::ALL {
            let spec = system_spec(field);
            // A Control Source list skips a value (the unused 32), which
            // shows as its bare number: only the values it names parse.
            let named: std::collections::HashSet<i32> = match spec.format {
                ParamFormat::ControlSource(kind) => control_source_choices(kind)
                    .map(|(value, _)| i32::from(value))
                    .collect(),
                _ => (spec.min..=spec.max).collect(),
            };
            for value in spec.min..=spec.max {
                let shown = spec.format.format(value);
                assert_eq!(
                    spec.parse(&shown),
                    named.contains(&value).then_some(value),
                    "{field:?} {shown}"
                );
            }
        }
    }

    #[test]
    fn studio_set_control_channel_counts_channels_from_1() {
        let channel = system_spec(SystemField::StudioSetControlChannel).format;
        assert_eq!(channel.format(15), "16");
    }

    #[test]
    fn system_control_sources_name_cc_numbers_and_skip_32() {
        let source = system_spec(SystemField::SystemControl1Source).format;
        assert_eq!(source.format(1), "CC01");
        assert_eq!(source.format(31), "CC31");
        assert_eq!(source.format(32), "32");
        assert_eq!(source.format(33), "CC33");
        assert_eq!(source.format(96), "BEND");
    }

    #[test]
    fn system_control_sources_parse_only_as_a_source_the_list_names() {
        for field in [
            SystemField::SystemControl1Source,
            SystemField::SystemControl2Source,
            SystemField::SystemControl3Source,
            SystemField::SystemControl4Source,
        ] {
            let spec = system_spec(field);
            // The unused 32 shows as the bare number, which names no source.
            assert_eq!(spec.parse("32"), None, "{field:?}");
            assert_eq!(spec.parse("CC01"), Some(1), "{field:?}");
            for (value, name) in control_source_choices(ControlSourceKind::SystemControl) {
                assert_eq!(spec.parse(name), Some(i32::from(value)), "{field:?} {name}");
            }
        }
    }

    #[test]
    fn master_level_and_ext_part_source_are_not_fields() {
        // Master Level (`00 05`) stays in the Mixer; Ext Part Source Select
        // is in the Studio Set (#284).
        assert!(
            SystemField::ALL
                .iter()
                .all(|f| f.address() != crate::params::SYSTEM_MASTER_LEVEL)
        );
    }

    #[test]
    fn names_are_unique_and_found() {
        for &field in SystemField::ALL {
            assert_eq!(SystemField::from_name(system_spec(field).name), Some(field));
        }
        assert_eq!(SystemField::from_name("master-level"), None);
    }

    #[test]
    fn groups_follow_page_order() {
        let groups: Vec<_> = SystemField::ALL
            .iter()
            .map(|&f| system_spec(f).group)
            .collect();
        let mut deduped = groups.clone();
        deduped.dedup();
        assert_eq!(deduped, SystemGroup::ALL);
    }

    #[test]
    fn only_output_points_to_the_panel() {
        let noted: Vec<_> = SystemGroup::ALL
            .iter()
            .filter(|g| g.panel_note().is_some())
            .collect();
        assert_eq!(noted, [&SystemGroup::Output]);
    }

    #[test]
    fn read_request_is_one_rq1_of_the_block() {
        assert_eq!(
            build_read_request(0x10),
            sysex::build_rq1(
                0x10,
                &Address::new(0x02, 0, 0, 0),
                &DataSize::new(0, 0, 0, 0x2F)
            )
        );
    }

    /// System Common as the device answered a 47-byte RQ1 on 2026-11-15:
    /// the factory values (`docs/svd/10-system.md`) but for Clock Source
    /// USB.
    #[rustfmt::skip]
    const DUMP: [u8; SYSTEM_COMMON_LEN] = [
        0x00, 0x04, 0x00, 0x00, 0x40, 0x7F, 0x01, 0x00, // 00-07
        0x01, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, // 08-0F
        0x07, 0x0F, 0x01, 0x00, 0x40, 0x40, 0x40, 0x40, // 10-17
        0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, // 18-1F
        0x01, 0x61, 0x02, 0x04, 0x01, 0x01, 0x07, 0x08, // 20-27
        0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,       // 28-2E
    ];

    #[test]
    fn a_dump_parses_into_every_field() {
        let system = parse_system_common(&DUMP).unwrap();
        let shown: Vec<_> = SystemField::ALL
            .iter()
            .map(|&f| (system_spec(f).name, system.display(f)))
            .collect();
        assert_eq!(
            shown,
            [
                ("master-tune", "440.00 Hz"),
                ("master-key-shift", "0"),
                ("scale-tune-switch", "ON"),
                ("studio-set-control-channel", "16"),
                ("system-control-1-source", "CC01"),
                ("system-control-2-source", "AFT"),
                ("system-control-3-source", "CC02"),
                ("system-control-4-source", "CC04"),
                ("control-source", "STUDIO SET"),
                ("clock-source", "USB"),
                ("system-tempo", "120"),
                ("tempo-assign-source", "SYSTEM"),
                ("receive-program-change", "ON"),
                ("receive-bank-select", "ON"),
                ("center-speaker-switch", "OFF"),
                ("sub-woofer-switch", "OFF"),
                ("2ch-output-mode", "SPEAKER"),
                ("usb-audio-sampling-rate", "44.1 kHz"),
            ]
            .map(|(n, v)| (n, v.to_string()))
        );
        assert_eq!(system.get(SystemField::MasterTune), 1024);
        assert_eq!(system.get(SystemField::SystemTempo), 120);
        assert_eq!(system.bytes(), &DUMP);
    }

    #[test]
    fn each_pair_follows_its_selector() {
        // The dump's Control Source is STUDIO SET, its Tempo Assign SYSTEM.
        let system = parse_system_common(&DUMP).unwrap();
        assert!(system.studio_set_in_force(SystemOrStudioSet::Control));
        assert!(!system.studio_set_in_force(SystemOrStudioSet::Tempo));
        for pair in [SystemOrStudioSet::Control, SystemOrStudioSet::Tempo] {
            let format = system_spec(pair.selector()).format;
            assert_eq!(format.format(STUDIO_SET_SIDE.into()), "STUDIO SET");
        }
    }

    #[test]
    fn each_pair_sets_its_sides_one_for_one_in_the_selector_group() {
        for pair in SystemOrStudioSet::ALL {
            let system = pair.system_side();
            assert_eq!(system.len(), usize::from(pair.studio_set_side().count()));
            let group = system_spec(pair.selector()).group;
            for &field in system {
                assert_eq!(system_spec(field).group, group, "{pair:?}: {field:?}");
            }
        }
    }

    #[test]
    fn system_control_sources_pair_with_tone_controls_by_number() {
        let paired: Vec<_> = (0..=crate::device_spec::TONE_CONTROLS)
            .map(SystemField::system_control_source)
            .collect();
        assert_eq!(
            paired,
            [
                Some(SystemField::SystemControl1Source),
                Some(SystemField::SystemControl2Source),
                Some(SystemField::SystemControl3Source),
                Some(SystemField::SystemControl4Source),
                None,
            ]
        );
    }

    #[test]
    fn a_short_dump_is_refused() {
        assert_eq!(
            parse_system_common(&DUMP[..46]),
            Err(SystemCommonTooShort { got: 46 })
        );
    }

    #[test]
    fn values_encode_nibblized_and_in_range() {
        assert_eq!(
            encode_value(SystemField::MasterTune, 1024),
            Ok(vec![0, 4, 0, 0])
        );
        assert_eq!(encode_value(SystemField::SystemTempo, 121), Ok(vec![7, 9]));
        assert_eq!(encode_value(SystemField::ClockSource, 1), Ok(vec![1]));
        assert_eq!(
            encode_value(SystemField::SystemTempo, 251),
            Err(SystemValueOutOfRange {
                field: SystemField::SystemTempo,
                value: 251,
                min: 20,
                max: 250,
            })
        );
        assert!(encode_value(SystemField::MasterKeyShift, 39).is_err());
        assert!(encode_value(SystemField::MasterTune, -1).is_err());
    }
}
