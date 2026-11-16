//! Studio Set settings registry: the Studio Set Common and Studio Set MIDI
//! fields the Mixer doesn't edit (PRD #606).
//!
//! - Tone Control 1–4 Source (Studio Set Common `00 39`–`00 3C`).
//! - Studio Set Tempo (Studio Set Common `00 3D`, two nibblized bytes).
//! - Phase Lock, one per MIDI channel (Studio Set MIDI `00 10 00`–`00 1F 00`).
//!
//! A [`StudioSetSetting`] names a setting without its index; a
//! [`StudioSetField`] is one instance of it (Tone Control 3 Source, MIDI
//! channel 10's Phase Lock), with its CLI [`name`](StudioSetField::name),
//! address, range and display ([`StudioSetField::spec`]). Ranges and
//! displays follow `docs/midi/05-studio-set.md`.
//!
//! A mixer load reads these fields into
//! [`MixerState::studio_set_settings`](crate::state::MixerState::studio_set_settings);
//! [`DeviceState::set_studio_set_field`](crate::device::DeviceState::set_studio_set_field)
//! writes one.

use thiserror::Error;

use crate::address::Address;
use crate::device_spec::{MIDI_CHANNELS, TONE_CONTROLS};
use crate::family::ParameterFamily;
use crate::field_spec::{FieldSpec, fill_name_template};
use crate::mfx_params::ControlSourceKind;
use crate::param_registry::ParamFormat;
use crate::params;
use crate::parse_helpers::{encode_nibble2, nibble2};
use crate::state::StudioSetSettings;

/// The manual's caveat on Phase Lock (`docs/manual/README.md`, item 4).
pub const PHASE_LOCK_NOTICE: &str = "Phase Lock adds audible latency, and it doesn't work for \
     SN-A organ instruments.";

/// A Studio Set setting, named without its index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StudioSetSetting {
    /// Tone Control 1–4 Source: the controller that drives a Tone Control
    /// in this Studio Set (0–97: OFF, CC01–CC31, CC33–CC95, BEND, AFT).
    ToneControlSource,
    /// Studio Set Tempo (20–250 BPM).
    Tempo,
    /// Phase Lock of a MIDI channel (0 = OFF, 1 = ON).
    PhaseLock,
}

impl StudioSetSetting {
    /// Every setting, in registry order.
    pub const ALL: [Self; 3] = [Self::ToneControlSource, Self::Tempo, Self::PhaseLock];

    /// How many instances the setting has: one per Tone Control, one
    /// tempo, one Phase Lock per MIDI channel.
    pub const fn count(self) -> u8 {
        match self {
            Self::ToneControlSource => TONE_CONTROLS,
            Self::Tempo => 1,
            Self::PhaseLock => MIDI_CHANNELS,
        }
    }

    /// Range, default and display of the setting, shared by its instances.
    /// The host name is a template: `{}` is the instance counted from 1
    /// ([`FieldSpec::element_name`]).
    pub fn spec(self) -> FieldSpec {
        let (min, max, default, format, family, host_name) = match self {
            Self::ToneControlSource => (
                0,
                ControlSourceKind::SystemControl.max().into(),
                0,
                ParamFormat::ControlSource(ControlSourceKind::SystemControl),
                ParameterFamily::Modulation,
                "Tone Control {} Source",
            ),
            Self::Tempo => (
                20,
                250,
                120,
                ParamFormat::Raw,
                ParameterFamily::Time,
                "Studio Set Tempo",
            ),
            Self::PhaseLock => (
                0,
                1,
                0,
                ParamFormat::OnOff,
                ParameterFamily::Common,
                "Channel {} Phase Lock",
            ),
        };
        FieldSpec {
            min,
            max,
            default,
            format,
            family,
            host_name,
        }
    }

    /// The CLI name template: `{}` is the instance counted from 1.
    fn name_template(self) -> &'static str {
        match self {
            Self::ToneControlSource => "tone-control-{}-source",
            Self::Tempo => "tempo",
            Self::PhaseLock => "phase-lock-{}",
        }
    }
}

/// Why a Studio Set field can't be named or written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StudioSetFieldError {
    /// The index is past the setting's last instance.
    #[error("{setting:?} has no instance {index}")]
    IndexOutOfRange {
        /// The setting.
        setting: StudioSetSetting,
        /// The 0-based index asked for.
        index: u8,
    },
    /// The value is outside the field's range.
    #[error("value {value} is out of range {min}–{max}")]
    ValueOutOfRange {
        /// The value asked for.
        value: i32,
        /// Lowest valid value.
        min: i32,
        /// Highest valid value.
        max: i32,
    },
}

/// One instance of a [`StudioSetSetting`]: Tone Control 3 Source, the
/// tempo, MIDI channel 10's Phase Lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StudioSetField {
    setting: StudioSetSetting,
    index: u8,
}

impl StudioSetField {
    /// Instance `index` (0-based: Tone Control 1, MIDI channel 1; 0 for
    /// the tempo) of `setting`.
    ///
    /// # Errors
    ///
    /// [`StudioSetFieldError::IndexOutOfRange`] past the setting's
    /// [`count`](StudioSetSetting::count).
    pub fn new(setting: StudioSetSetting, index: u8) -> Result<Self, StudioSetFieldError> {
        if index < setting.count() {
            Ok(Self { setting, index })
        } else {
            Err(StudioSetFieldError::IndexOutOfRange { setting, index })
        }
    }

    /// Every field, in registry order: Tone Control 1–4 Source, the tempo,
    /// then Phase Lock of MIDI channels 1–16.
    pub fn all() -> impl Iterator<Item = Self> {
        StudioSetSetting::ALL
            .into_iter()
            .flat_map(|setting| (0..setting.count()).map(move |index| Self { setting, index }))
    }

    /// The field whose [`name`](Self::name) is `name`.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::all().find(|field| field.name() == name)
    }

    /// The setting this field is an instance of.
    pub fn setting(self) -> StudioSetSetting {
        self.setting
    }

    /// The instance, 0-based.
    pub fn index(self) -> u8 {
        self.index
    }

    /// The field's name on the command line: `tone-control-1-source`,
    /// `tempo`, `phase-lock-16`.
    pub fn name(self) -> String {
        fill_name_template(self.setting.name_template(), usize::from(self.index))
    }

    /// The field's name as the MIDI Implementation words it: `Tone Control
    /// 1 Source`, `Studio Set Tempo`, `Channel 16 Phase Lock`.
    pub fn label(self) -> String {
        self.spec().element_name(usize::from(self.index))
    }

    /// Range, default and display of the field.
    pub fn spec(self) -> FieldSpec {
        self.setting.spec()
    }

    /// The field's address.
    pub fn address(self) -> Address {
        match self.setting {
            StudioSetSetting::ToneControlSource => {
                params::TONE_CONTROL_1_SOURCE.offset([0, 0, 0, self.index])
            }
            StudioSetSetting::Tempo => params::STUDIO_SET_TEMPO,
            StudioSetSetting::PhaseLock => {
                params::studio_set_midi_address(self.index, params::studio_set_midi::PHASE_LOCK)
            }
        }
    }

    /// The bytes a DT1 writes at [`address`](Self::address) to set the
    /// field to `value`: one byte, or the tempo's two nibbles.
    ///
    /// # Errors
    ///
    /// [`StudioSetFieldError::ValueOutOfRange`] outside the field's range.
    pub fn encode(self, value: i32) -> Result<Vec<u8>, StudioSetFieldError> {
        let spec = self.spec();
        if !(spec.min..=spec.max).contains(&value) {
            return Err(StudioSetFieldError::ValueOutOfRange {
                value,
                min: spec.min,
                max: spec.max,
            });
        }
        // In range, so it fits a byte (or the tempo's u16).
        Ok(match self.setting {
            StudioSetSetting::Tempo => encode_nibble2(value as u16).to_vec(),
            _ => vec![value as u8],
        })
    }

    /// The field's raw value in `settings` (a switch reads 0 or 1).
    pub fn value(self, settings: &StudioSetSettings) -> i32 {
        let i = usize::from(self.index);
        match self.setting {
            StudioSetSetting::ToneControlSource => settings.tone_control_sources[i].into(),
            StudioSetSetting::Tempo => settings.tempo.into(),
            StudioSetSetting::PhaseLock => settings.phase_lock[i].into(),
        }
    }

    /// Set the field to `value`, already in range, in `settings`.
    pub(crate) fn store(self, settings: &mut StudioSetSettings, value: i32) {
        let i = usize::from(self.index);
        match self.setting {
            StudioSetSetting::ToneControlSource => settings.tone_control_sources[i] = value as u8,
            StudioSetSetting::Tempo => settings.tempo = value as u16,
            StudioSetSetting::PhaseLock => settings.phase_lock[i] = value != 0,
        }
    }
}

/// Update `settings` from the block read at
/// [`params::TONE_CONTROL_1_SOURCE`], of [`params::STUDIO_SET_CONTROL_SIZE`]
/// bytes: Tone Control 1–4 Source, then the tempo's two nibbles.
pub(crate) fn apply_control_block(settings: &mut StudioSetSettings, data: &[u8]) {
    let sources = usize::from(TONE_CONTROLS);
    settings
        .tone_control_sources
        .copy_from_slice(&data[..sources]);
    settings.tempo = nibble2(data, sources);
}

#[cfg(test)]
mod tests {
    use super::*;
    use StudioSetSetting::*;

    fn field(setting: StudioSetSetting, index: u8) -> StudioSetField {
        StudioSetField::new(setting, index).unwrap()
    }

    /// Each field's address, range and display, as `docs/midi/05-studio-set.md`
    /// gives them.
    #[test]
    fn fields_match_the_midi_implementation() {
        let doc = [
            // (setting, index, address, min, max, label)
            (
                ToneControlSource,
                0,
                [0x18, 0, 0x00, 0x39],
                0,
                97,
                "Tone Control 1 Source",
            ),
            (
                ToneControlSource,
                1,
                [0x18, 0, 0x00, 0x3A],
                0,
                97,
                "Tone Control 2 Source",
            ),
            (
                ToneControlSource,
                2,
                [0x18, 0, 0x00, 0x3B],
                0,
                97,
                "Tone Control 3 Source",
            ),
            (
                ToneControlSource,
                3,
                [0x18, 0, 0x00, 0x3C],
                0,
                97,
                "Tone Control 4 Source",
            ),
            (Tempo, 0, [0x18, 0, 0x00, 0x3D], 20, 250, "Studio Set Tempo"),
            (
                PhaseLock,
                0,
                [0x18, 0, 0x10, 0x00],
                0,
                1,
                "Channel 1 Phase Lock",
            ),
            (
                PhaseLock,
                9,
                [0x18, 0, 0x19, 0x00],
                0,
                1,
                "Channel 10 Phase Lock",
            ),
            (
                PhaseLock,
                15,
                [0x18, 0, 0x1F, 0x00],
                0,
                1,
                "Channel 16 Phase Lock",
            ),
        ];
        for (setting, index, [a, b, c, d], min, max, label) in doc {
            let f = field(setting, index);
            assert_eq!(f.address(), Address::new(a, b, c, d), "{label}");
            let spec = f.spec();
            assert_eq!((spec.min, spec.max), (min, max), "{label}");
            assert!((min..=max).contains(&spec.default), "{label} default");
            assert_eq!(f.label(), label);
        }
    }

    #[test]
    fn tone_control_sources_show_the_documented_names() {
        let spec = ToneControlSource.spec();
        let shown = |v| spec.format.format(v);
        assert_eq!(shown(0), "OFF");
        assert_eq!(shown(1), "CC01");
        assert_eq!(shown(31), "CC31");
        assert_eq!(shown(33), "CC33");
        assert_eq!(shown(95), "CC95");
        assert_eq!(shown(96), "BEND");
        assert_eq!(shown(97), "AFT");
        assert_eq!(shown(32), "32", "CC32 is not offered");
    }

    #[test]
    fn tempo_and_phase_lock_show_as_documented() {
        assert_eq!(Tempo.spec().format.format(20), "20");
        assert_eq!(Tempo.spec().format.format(250), "250");
        assert_eq!(PhaseLock.spec().format.format(0), "OFF");
        assert_eq!(PhaseLock.spec().format.format(1), "ON");
    }

    #[test]
    fn the_registry_has_every_field_once_by_name() {
        let names: Vec<String> = StudioSetField::all().map(StudioSetField::name).collect();
        assert_eq!(names.len(), 4 + 1 + 16);
        assert_eq!(names[0], "tone-control-1-source");
        assert_eq!(names[3], "tone-control-4-source");
        assert_eq!(names[4], "tempo");
        assert_eq!(names[5], "phase-lock-1");
        assert_eq!(names[20], "phase-lock-16");
        for f in StudioSetField::all() {
            assert_eq!(StudioSetField::from_name(&f.name()), Some(f));
        }
        assert_eq!(StudioSetField::from_name("phase-lock-17"), None);
    }

    #[test]
    fn an_index_past_the_last_instance_is_refused() {
        assert_eq!(
            StudioSetField::new(PhaseLock, 16),
            Err(StudioSetFieldError::IndexOutOfRange {
                setting: PhaseLock,
                index: 16
            })
        );
        assert!(StudioSetField::new(Tempo, 1).is_err());
        assert!(StudioSetField::new(ToneControlSource, 4).is_err());
    }

    #[test]
    fn encode_checks_the_range_and_nibblizes_the_tempo() {
        assert_eq!(field(Tempo, 0).encode(120), Ok(vec![0x07, 0x08]));
        assert_eq!(field(Tempo, 0).encode(250), Ok(vec![0x0F, 0x0A]));
        assert_eq!(field(ToneControlSource, 2).encode(96), Ok(vec![96]));
        assert_eq!(field(PhaseLock, 0).encode(1), Ok(vec![1]));
        assert_eq!(
            field(Tempo, 0).encode(19),
            Err(StudioSetFieldError::ValueOutOfRange {
                value: 19,
                min: 20,
                max: 250
            })
        );
        assert!(field(PhaseLock, 0).encode(2).is_err());
        assert!(field(ToneControlSource, 0).encode(98).is_err());
    }

    #[test]
    fn values_round_trip_through_the_settings() {
        let mut s = StudioSetSettings::default();
        for f in StudioSetField::all() {
            assert_eq!(f.value(&s), f.spec().default, "{} default", f.name());
        }
        field(ToneControlSource, 3).store(&mut s, 97);
        field(Tempo, 0).store(&mut s, 96);
        field(PhaseLock, 15).store(&mut s, 1);
        assert_eq!(s.tone_control_sources, [0, 0, 0, 97]);
        assert_eq!(s.tempo, 96);
        assert!(s.phase_lock[15] && !s.phase_lock[14]);
        assert_eq!(field(PhaseLock, 15).value(&s), 1);
    }

    #[test]
    fn the_control_block_holds_the_sources_and_the_tempo() {
        let mut s = StudioSetSettings::default();
        apply_control_block(&mut s, &[1, 74, 96, 97, 0x06, 0x00]);
        assert_eq!(s.tone_control_sources, [1, 74, 96, 97]);
        assert_eq!(s.tempo, 96);
    }
}
