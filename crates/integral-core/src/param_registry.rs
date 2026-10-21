//! Parameter registry — definitions, formatting, and lookup for all
//! controllable parameters on the Integra-7.
//!
//! Mixer parameters are named by the index-free [`MixerField`]. A
//! [`MixerScope`] (a part, the Ext input, the master, a Drum Comp+EQ unit
//! or the Studio Set common block) supplies the indices, and
//! [`mixer_field_address`] resolves the write address of a field in a
//! scope.

use std::fmt::Write;
use std::sync::OnceLock;

use thiserror::Error;

use crate::address::Address;
use crate::device_spec::{INTEGRA7, MIDI_CHANNELS};
use crate::family::ParameterFamily;
use crate::params::{self, ToneType};
use crate::state::{CompEqUnit, DrumCompEqState, EqState, SurroundPartState, SurroundState};

/// How a parameter value should be displayed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ParamFormat {
    /// Display value as-is.
    Raw,
    /// Display as value minus center (e.g. pan 64 -> "0", 0 -> "-64", 127 -> "+63").
    SignedCenter(i32),
    /// Index into a static name array.
    Named(&'static [&'static str]),
    /// Boolean: 0=OFF, 1=ON.
    OnOff,
    /// Pan position: `L64`..`C`..`63R`, centered on 64.
    Pan,
    /// Value minus `center`, times `step`, signed (keyfollow 54 -> "-100"
    /// with center 64 and step 10).
    SignedStep {
        /// Raw value shown as 0.
        center: i32,
        /// Display units per raw step.
        step: i32,
    },
    /// MIDI note name, `C-1` (0) to `G9` (127).
    Note,
    /// 0-127 as a number, then the tempo-synced note values from
    /// [`NOTE_FROM`].
    TimeOrNote,
    /// A PCM Synth Tone LFO1/LFO2 rate as Hz
    /// ([`docs/research/lfo-rates.md`], ±3% device scatter), then the
    /// tempo-synced note values from [`NOTE_FROM`].
    ///
    /// [`docs/research/lfo-rates.md`]: crate::lfo_rates
    PcmLfoRate,
    /// An SN Synth Tone LFO / Modulation LFO rate as Hz
    /// ([`docs/research/lfo-rates.md`], ±3% device scatter; rate 0 is an
    /// unresolved estimate). Tempo sync lives in separate fields.
    ///
    /// [`docs/research/lfo-rates.md`]: crate::lfo_rates
    SnLfoRate,
    /// The value as a percentage (`100` -> "100%").
    Percent,
}

/// First raw value of the note range in a [`ParamFormat::TimeOrNote`] field.
pub const NOTE_FROM: i32 = 128;

/// The tempo-synced note name of a value at or above [`NOTE_FROM`].
fn note_name(value: i32) -> Option<&'static str> {
    value
        .checked_sub(NOTE_FROM)
        .and_then(|i| crate::fx_params::NOTE_NAMES.get(i as usize).copied())
}

/// Logical grouping for a parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamGroup {
    /// Per-part parameter; the [`MixerScope::Part`] supplies the index.
    Part,
    /// Master output.
    Master,
    /// External input.
    Ext,
    /// Chorus effect.
    Chorus,
    /// Reverb effect.
    Reverb,
    /// Mixer 3-band EQ, shared by parts, master and Drum Comp+EQ units.
    Eq,
    /// Compressor of a Drum Comp+EQ unit, shared by all units.
    Comp,
    /// Motional Surround: its common block, and each part's (and the Ext
    /// Part's) position.
    Surround,
    /// Global / top-level.
    Global,
}

/// A single mixer parameter definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamDef {
    /// The index-free field this defines.
    pub field: MixerField,
    /// Human-readable display name.
    pub name: &'static str,
    /// Which group this parameter belongs to.
    pub group: ParamGroup,
    /// Minimum raw value (inclusive).
    pub min: i32,
    /// Maximum raw value (inclusive).
    pub max: i32,
    /// Default raw value.
    pub default: i32,
    /// Display format.
    pub format: ParamFormat,
    /// The parameter's role in shaping the sound (ADR 0002).
    pub family: ParameterFamily,
}

/// Format a raw parameter value according to the parameter's [`ParamFormat`].
///
/// Returns the formatted string. Out-of-range named indices fall back to the
/// raw numeric representation.
pub fn format_value(def: &ParamDef, value: i32) -> String {
    def.format.format(value)
}

impl ParamFormat {
    /// The raw value a control centres on, for formats signed around one.
    pub fn center(&self) -> Option<i32> {
        match self {
            ParamFormat::SignedCenter(center) | ParamFormat::SignedStep { center, .. } => {
                Some(*center)
            }
            ParamFormat::Pan => Some(64),
            _ => None,
        }
    }

    /// Format a raw value; out-of-range named indices fall back to the raw
    /// number.
    pub fn format(&self, value: i32) -> String {
        match self {
            ParamFormat::Raw => value.to_string(),
            ParamFormat::SignedCenter(center) => {
                let offset = value - center;
                if offset > 0 {
                    let mut s = String::new();
                    let _ = write!(s, "+{offset}");
                    s
                } else {
                    offset.to_string()
                }
            }
            ParamFormat::Named(names) => {
                let idx = value as usize;
                if idx < names.len() {
                    names[idx].to_string()
                } else {
                    value.to_string()
                }
            }
            ParamFormat::OnOff => {
                if value == 0 {
                    "OFF".to_string()
                } else {
                    "ON".to_string()
                }
            }
            ParamFormat::Pan => match value {
                64 => "C".to_string(),
                v if v < 64 => format!("L{}", 64 - v),
                v => format!("{}R", v - 64),
            },
            ParamFormat::SignedStep { center, step } => {
                ParamFormat::SignedCenter(0).format((value - center) * step)
            }
            ParamFormat::Note => {
                const NAMES: [&str; 12] = [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ];
                format!(
                    "{}{}",
                    NAMES[value.rem_euclid(12) as usize],
                    value.div_euclid(12) - 1
                )
            }
            ParamFormat::TimeOrNote => match note_name(value) {
                Some(note) => note.to_string(),
                None => value.to_string(),
            },
            ParamFormat::PcmLfoRate => match note_name(value) {
                Some(note) => note.to_string(),
                None => crate::lfo_rates::format_hz(crate::lfo_rates::pcm_lfo_hz(value)),
            },
            ParamFormat::SnLfoRate => {
                crate::lfo_rates::format_hz(crate::lfo_rates::sn_lfo_hz(value))
            }
            ParamFormat::Percent => format!("{value}%"),
        }
    }
}

// ---------------------------------------------------------------------------
// Mixer fields and scopes
// ---------------------------------------------------------------------------

/// A mixer parameter, named without indices.
///
/// The [`MixerScope`] a field is used in supplies the part or Comp+EQ unit
/// index, so one layout can name `EqLowGain` for every part, the master
/// and every Comp+EQ unit. [`MixerField::in_scope`] says which scopes a
/// field belongs to; [`mixer_param`] gives its range and format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixerField {
    /// Part level (0–127).
    PartLevel,
    /// Part pan (0–127, 64 = centre).
    PartPan,
    /// Part mute (0 = OFF, 1 = ON).
    PartMute,
    /// Part chorus (FX1) send level (0–127).
    PartChorusSend,
    /// Part reverb (FX2) send level (0–127).
    PartReverbSend,
    /// Part output assign (an index into the device's output assigns).
    PartOutputAssign,
    /// Part receive channel (0–15 = MIDI channel 1–16).
    PartReceiveChannel,
    /// System master level (0–127).
    MasterLevel,
    /// External input level (0–127).
    ExtLevel,
    /// External input mute (0 = OFF, 1 = ON).
    ExtMute,
    /// Chorus switch (0 = OFF, 1 = ON).
    ChorusSwitch,
    /// Chorus type (0 = OFF).
    ChorusType,
    /// Chorus level (0–127).
    ChorusLevel,
    /// Chorus output assign.
    ChorusOutputAssign,
    /// Chorus output select.
    ChorusOutputSelect,
    /// Reverb switch (0 = OFF, 1 = ON).
    ReverbSwitch,
    /// Reverb type (0 = OFF).
    ReverbType,
    /// Reverb level (0–127).
    ReverbLevel,
    /// Reverb output assign.
    ReverbOutputAssign,
    /// Solo part (0 = OFF, 1–16 = that part).
    SoloPart,
    /// Drum Comp+EQ switch (0 = OFF, 1 = ON): whether the assigned part's
    /// kit runs through the Comp+EQ units.
    DrumCompEqSwitch,
    /// Drum Comp+EQ assigned part (0–15 = Part 1–16): the part whose drum
    /// kit the Comp+EQ units process.
    DrumCompEqPart,
    /// 3-band EQ switch (0 = OFF, 1 = ON).
    EqSwitch,
    /// EQ low band frequency (0 = 200 Hz, 1 = 400 Hz).
    EqLowFreq,
    /// EQ low band gain (0–30 = −15 to +15 dB).
    EqLowGain,
    /// EQ mid band frequency (0–16 = 200 Hz to 8 kHz).
    EqMidFreq,
    /// EQ mid band gain (0–30 = −15 to +15 dB).
    EqMidGain,
    /// EQ mid band Q (0–4 = 0.5 to 8.0).
    EqMidQ,
    /// EQ high band frequency (0–2 = 2, 4, 8 kHz).
    EqHighFreq,
    /// EQ high band gain (0–30 = −15 to +15 dB).
    EqHighGain,
    /// Comp+EQ compressor switch (0 = OFF, 1 = ON).
    CompSwitch,
    /// Comp+EQ compressor attack time.
    CompAttack,
    /// Comp+EQ compressor release time.
    CompRelease,
    /// Comp+EQ compressor threshold (0–127).
    CompThreshold,
    /// Comp+EQ compressor ratio.
    CompRatio,
    /// Comp+EQ compressor output gain (0–24 dB).
    CompOutputGain,
    /// Comp+EQ unit output assign (an index into the device's Comp+EQ
    /// output assigns). It sits in the Studio Set common block, one byte
    /// per unit, not in the kit.
    CompOutputAssign,
    /// Motional Surround switch (0 = OFF, 1 = ON).
    SurroundSwitch,
    /// Motional Surround room type (an index into the device's room types).
    SurroundRoomType,
    /// Motional Surround room size (an index into the device's room sizes).
    SurroundRoomSize,
    /// Motional Surround depth (0–100).
    SurroundDepth,
    /// Motional Surround ambience level (0–127).
    SurroundAmbienceLevel,
    /// Motional Surround ambience time (0–100).
    SurroundAmbienceTime,
    /// Motional Surround ambience density (0–100).
    SurroundAmbienceDensity,
    /// Motional Surround ambience HF damp (0–100).
    SurroundAmbienceHfDamp,
    /// Surround L-R position of a part or the Ext Part (0–127, 64 = centre).
    SurroundLr,
    /// Surround F-B position of a part or the Ext Part (0–127, 64 = centre).
    SurroundFb,
    /// Surround width of a part or the Ext Part (0–32).
    SurroundWidth,
    /// Surround ambience send level of a part or the Ext Part (0–127),
    /// drawn in place of a part's FX sends while surround is on.
    SurroundAmbienceSend,
}

impl MixerField {
    /// Every mixer field, in registry order.
    pub const ALL: [MixerField; 49] = [
        Self::PartLevel,
        Self::PartPan,
        Self::PartMute,
        Self::PartChorusSend,
        Self::PartReverbSend,
        Self::PartOutputAssign,
        Self::PartReceiveChannel,
        Self::MasterLevel,
        Self::ExtLevel,
        Self::ExtMute,
        Self::ChorusSwitch,
        Self::ChorusType,
        Self::ChorusLevel,
        Self::ChorusOutputAssign,
        Self::ChorusOutputSelect,
        Self::ReverbSwitch,
        Self::ReverbType,
        Self::ReverbLevel,
        Self::ReverbOutputAssign,
        Self::SoloPart,
        Self::DrumCompEqSwitch,
        Self::DrumCompEqPart,
        Self::EqSwitch,
        Self::EqLowFreq,
        Self::EqLowGain,
        Self::EqMidFreq,
        Self::EqMidGain,
        Self::EqMidQ,
        Self::EqHighFreq,
        Self::EqHighGain,
        Self::CompSwitch,
        Self::CompAttack,
        Self::CompRelease,
        Self::CompThreshold,
        Self::CompRatio,
        Self::CompOutputGain,
        Self::CompOutputAssign,
        Self::SurroundSwitch,
        Self::SurroundRoomType,
        Self::SurroundRoomSize,
        Self::SurroundDepth,
        Self::SurroundAmbienceLevel,
        Self::SurroundAmbienceTime,
        Self::SurroundAmbienceDensity,
        Self::SurroundAmbienceHfDamp,
        Self::SurroundLr,
        Self::SurroundFb,
        Self::SurroundWidth,
        Self::SurroundAmbienceSend,
    ];

    /// Whether the field exists in a scope of this kind.
    pub fn in_scope(self, kind: MixerScopeKind) -> bool {
        match kind {
            MixerScopeKind::Part => {
                part_offset(self).is_some()
                    || part_eq_offset(self).is_some()
                    || part_surround_offset(self).is_some()
            }
            MixerScopeKind::Ext => ext_address(self).is_some(),
            MixerScopeKind::Master => {
                self == Self::MasterLevel || master_eq_address(self).is_some()
            }
            MixerScopeKind::CompEqUnit => {
                self == Self::CompOutputAssign || comp_eq_offset(self).is_some()
            }
            MixerScopeKind::Common => common_address(self).is_some(),
        }
    }
}

/// The kind of a [`MixerScope`], without its indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MixerScopeKind {
    /// A Studio Set part.
    Part,
    /// The external input.
    Ext,
    /// The master output and its EQ.
    Master,
    /// A Drum Comp+EQ unit.
    CompEqUnit,
    /// The Studio Set common block: Chorus, Reverb, Solo Part and Motional
    /// Surround.
    Common,
}

/// The drum kit whose tone holds the Drum Comp+EQ units: the assigned
/// part and its tone type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompEqKit {
    /// The Comp+EQ assigned part (0-based).
    pub part: u8,
    /// The tone type of that part's kit ([`ToneType::PcmDrum`] or
    /// [`ToneType::SnDrum`]).
    pub tone_type: ToneType,
}

/// Where a [`MixerField`] is written: the scope supplies the indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerScope {
    /// A Studio Set part (0-based).
    Part(u8),
    /// The external input.
    Ext,
    /// The master output and its EQ.
    Master,
    /// A Drum Comp+EQ unit (0-based) in the assigned part's kit.
    CompEqUnit {
        /// The kit holding the units.
        kit: CompEqKit,
        /// The unit (0-based).
        unit: u8,
    },
    /// The Studio Set common block.
    Common,
}

impl MixerScope {
    /// The scope's kind.
    pub fn kind(&self) -> MixerScopeKind {
        match self {
            Self::Part(_) => MixerScopeKind::Part,
            Self::Ext => MixerScopeKind::Ext,
            Self::Master => MixerScopeKind::Master,
            Self::CompEqUnit { .. } => MixerScopeKind::CompEqUnit,
            Self::Common => MixerScopeKind::Common,
        }
    }
}

/// Why a [`MixerField`] has no address in a [`MixerScope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MixerFieldError {
    /// The field doesn't exist in this kind of scope.
    #[error("mixer field {field:?} is not in a {scope:?} scope")]
    NotInScope {
        /// The field.
        field: MixerField,
        /// The scope kind it was used in.
        scope: MixerScopeKind,
    },
    /// The part index is past the device's last part.
    #[error("part index {0} out of range")]
    PartOutOfRange(u8),
    /// The Comp+EQ unit index is past the last unit.
    #[error("Comp+EQ unit {0} out of range")]
    UnitOutOfRange(u8),
}

/// The field's offset in a Studio Set Part block.
fn part_offset(field: MixerField) -> Option<[u8; 3]> {
    use MixerField as F;
    use params::part;
    Some(match field {
        F::PartLevel => part::LEVEL,
        F::PartPan => part::PAN,
        F::PartMute => part::MUTE,
        F::PartChorusSend => part::CHORUS_SEND,
        F::PartReverbSend => part::REVERB_SEND,
        F::PartOutputAssign => part::OUTPUT_ASSIGN,
        F::PartReceiveChannel => part::RECEIVE_CHANNEL,
        _ => return None,
    })
}

/// The surround field's offset in a Studio Set Part block.
fn part_surround_offset(field: MixerField) -> Option<[u8; 3]> {
    use MixerField as F;
    use params::part_surround;
    Some(match field {
        F::SurroundLr => part_surround::LR,
        F::SurroundFb => part_surround::FB,
        F::SurroundWidth => part_surround::WIDTH,
        F::SurroundAmbienceSend => part_surround::AMBIENCE_SEND,
        _ => return None,
    })
}

/// The Motional Surround common field a mixer field is, if any. The Ext
/// Part's position lives in that block too, so the Ext scope reads it here.
fn surround_field(field: MixerField, scope: MixerScopeKind) -> Option<params::SurroundField> {
    use MixerField as F;
    use MixerScopeKind as K;
    use params::SurroundField as S;
    Some(match (scope, field) {
        (K::Common, F::SurroundSwitch) => S::Switch,
        (K::Common, F::SurroundRoomType) => S::RoomType,
        (K::Common, F::SurroundRoomSize) => S::RoomSize,
        (K::Common, F::SurroundDepth) => S::Depth,
        (K::Common, F::SurroundAmbienceLevel) => S::AmbienceLevel,
        (K::Common, F::SurroundAmbienceTime) => S::AmbienceTime,
        (K::Common, F::SurroundAmbienceDensity) => S::AmbienceDensity,
        (K::Common, F::SurroundAmbienceHfDamp) => S::AmbienceHfDamp,
        (K::Ext, F::SurroundLr) => S::ExtPartLr,
        (K::Ext, F::SurroundFb) => S::ExtPartFb,
        (K::Ext, F::SurroundWidth) => S::ExtPartWidth,
        (K::Ext, F::SurroundAmbienceSend) => S::ExtPartAmbienceSend,
        _ => return None,
    })
}

/// The address of a Motional Surround common field in `scope`.
fn surround_address(field: MixerField, scope: MixerScopeKind) -> Option<Address> {
    surround_field(field, scope).map(|f| params::surround_address(f as u8))
}

/// The EQ field's offset in a Studio Set Part EQ block.
fn part_eq_offset(field: MixerField) -> Option<u8> {
    use MixerField as F;
    use params::part_eq;
    Some(match field {
        F::EqSwitch => part_eq::SWITCH,
        F::EqLowFreq => part_eq::LOW_FREQ,
        F::EqLowGain => part_eq::LOW_GAIN,
        F::EqMidFreq => part_eq::MID_FREQ,
        F::EqMidGain => part_eq::MID_GAIN,
        F::EqMidQ => part_eq::MID_Q,
        F::EqHighFreq => part_eq::HIGH_FREQ,
        F::EqHighGain => part_eq::HIGH_GAIN,
        _ => return None,
    })
}

/// The address of a Master EQ field. Its switch sits in the Studio Set
/// common block, not in the Master EQ block.
fn master_eq_address(field: MixerField) -> Option<Address> {
    use MixerField as F;
    use params::master_eq;
    let offset = match field {
        F::EqSwitch => return Some(params::MASTER_EQ_SWITCH),
        F::EqLowFreq => master_eq::LOW_FREQ,
        F::EqLowGain => master_eq::LOW_GAIN,
        F::EqMidFreq => master_eq::MID_FREQ,
        F::EqMidGain => master_eq::MID_GAIN,
        F::EqMidQ => master_eq::MID_Q,
        F::EqHighFreq => master_eq::HIGH_FREQ,
        F::EqHighGain => master_eq::HIGH_GAIN,
        _ => return None,
    };
    Some(params::master_eq_address(offset))
}

/// The field's offset in a 14-byte Comp+EQ unit.
fn comp_eq_offset(field: MixerField) -> Option<u8> {
    use MixerField as F;
    use params::{comp, comp_eq};
    Some(match field {
        F::CompSwitch => comp::SWITCH,
        F::CompAttack => comp::ATTACK,
        F::CompRelease => comp::RELEASE,
        F::CompThreshold => comp::THRESHOLD,
        F::CompRatio => comp::RATIO,
        F::CompOutputGain => comp::OUTPUT_GAIN,
        F::EqSwitch => comp_eq::EQ_SWITCH,
        F::EqLowFreq => comp_eq::EQ_LOW_FREQ,
        F::EqLowGain => comp_eq::EQ_LOW_GAIN,
        F::EqMidFreq => comp_eq::EQ_MID_FREQ,
        F::EqMidGain => comp_eq::EQ_MID_GAIN,
        F::EqMidQ => comp_eq::EQ_MID_Q,
        F::EqHighFreq => comp_eq::EQ_HIGH_FREQ,
        F::EqHighGain => comp_eq::EQ_HIGH_GAIN,
        _ => return None,
    })
}

/// The address of an external input field.
fn ext_address(field: MixerField) -> Option<Address> {
    match field {
        MixerField::ExtLevel => Some(params::EXT_PART_LEVEL),
        MixerField::ExtMute => Some(params::EXT_PART_MUTE),
        _ => surround_address(field, MixerScopeKind::Ext),
    }
}

/// The address of a Studio Set common field.
fn common_address(field: MixerField) -> Option<Address> {
    use MixerField as F;
    use params::{chorus, reverb};
    Some(match field {
        F::ChorusSwitch => params::CHORUS_SWITCH,
        F::ChorusType => params::chorus_address(chorus::TYPE),
        F::ChorusLevel => params::chorus_address(chorus::LEVEL),
        F::ChorusOutputAssign => params::chorus_address(chorus::OUTPUT_ASSIGN),
        F::ChorusOutputSelect => params::chorus_address(chorus::OUTPUT_SELECT),
        F::ReverbSwitch => params::REVERB_SWITCH,
        F::ReverbType => params::reverb_address(reverb::TYPE),
        F::ReverbLevel => params::reverb_address(reverb::LEVEL),
        F::ReverbOutputAssign => params::reverb_address(reverb::OUTPUT_ASSIGN),
        F::SoloPart => params::SOLO_PART,
        F::DrumCompEqSwitch => params::DRUM_COMP_EQ_SWITCH,
        F::DrumCompEqPart => params::DRUM_COMP_EQ_PART,
        _ => return surround_address(field, MixerScopeKind::Common),
    })
}

/// The address a value of `field` is written to in `scope`.
///
/// One function serves every scope: an EQ field resolves to the Part EQ,
/// Master EQ or Comp+EQ unit block by the scope it is used in.
///
/// # Errors
///
/// [`MixerFieldError::NotInScope`] if the field doesn't exist in the
/// scope's kind (see [`MixerField::in_scope`]), and
/// [`MixerFieldError::PartOutOfRange`] or
/// [`MixerFieldError::UnitOutOfRange`] for an index past the device's
/// parts or Comp+EQ units (for a Comp+EQ unit, either its kit's part or
/// the unit).
pub fn mixer_field_address(
    field: MixerField,
    scope: MixerScope,
) -> Result<Address, MixerFieldError> {
    let check_part = |part: u8| {
        if part < INTEGRA7.part_count {
            Ok(())
        } else {
            Err(MixerFieldError::PartOutOfRange(part))
        }
    };
    let address = match scope {
        MixerScope::Part(part) => {
            check_part(part)?;
            part_offset(field)
                .map(|offset| params::part_address(part, offset))
                .or_else(|| part_eq_offset(field).map(|o| params::part_eq_address(part, o)))
                .or_else(|| part_surround_offset(field).map(|o| params::part_address(part, o)))
        }
        MixerScope::Ext => ext_address(field),
        MixerScope::Master => match field {
            MixerField::MasterLevel => Some(params::SYSTEM_MASTER_LEVEL),
            _ => master_eq_address(field),
        },
        MixerScope::CompEqUnit { kit, unit } => {
            check_part(kit.part)?;
            if usize::from(unit) >= params::COMP_EQ_UNIT_COUNT {
                return Err(MixerFieldError::UnitOutOfRange(unit));
            }
            match field {
                MixerField::CompOutputAssign => Some(params::drum_comp_eq_output_assign(unit)),
                _ => comp_eq_offset(field).map(|offset| {
                    params::comp_eq_param_address(kit.part, kit.tone_type, unit, offset)
                }),
            }
        }
        MixerScope::Common => common_address(field),
    };
    address.ok_or(MixerFieldError::NotInScope {
        field,
        scope: scope.kind(),
    })
}

// ---------------------------------------------------------------------------
// Mixer parameter definitions
// ---------------------------------------------------------------------------

/// Receive-channel labels `"1"..="16"`, one per [`MIDI_CHANNELS`].
fn channel_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        (1..=MIDI_CHANNELS)
            .map(|n| &*Box::leak(n.to_string().into_boxed_str()))
            .collect()
    })
}

/// Part labels `"1"..`, one per part of the device.
fn part_names() -> &'static [&'static str] {
    static NAMES: OnceLock<Vec<&'static str>> = OnceLock::new();
    NAMES.get_or_init(|| {
        (1..=INTEGRA7.part_count)
            .map(|n| &*Box::leak(n.to_string().into_boxed_str()))
            .collect()
    })
}

/// Mixer EQ frequency and Q labels, shared by parts, master and Drum
/// Comp+EQ units.
static EQ_LOW_FREQ_NAMES: &[&str] = &["200", "400"];
static EQ_MID_FREQ_NAMES: &[&str] = &[
    "200", "250", "315", "400", "500", "630", "800", "1.0k", "1.2k", "1.6k", "2.0k", "2.5k",
    "3.1k", "4.0k", "5.0k", "6.3k", "8.0k",
];
static EQ_HIGH_FREQ_NAMES: &[&str] = &["2.0k", "4.0k", "8.0k"];
static EQ_Q_NAMES: &[&str] = &["0.5", "1.0", "2.0", "4.0", "8.0"];

/// The definition of a mixer field: its name, range, default, format and
/// family.
///
/// The definition is index-free. `PartReceiveChannel` defaults to
/// channel 1, Part 1's; Part *n* starts on channel *n*.
pub fn mixer_param(field: MixerField) -> ParamDef {
    mixer_param_for(field, INTEGRA7.part_count)
}

/// [`mixer_param`] for a device with `part_count` parts.
fn mixer_param_for(field: MixerField, part_count: u8) -> ParamDef {
    use MixerField as F;
    use ParamFormat::{Named, OnOff, Raw, SignedCenter};
    use ParameterFamily::{Amp, Common, Filter, Time};

    let spec = &INTEGRA7;
    let last = |names: &[&str]| names.len() as i32 - 1;
    let eq = EqState::default();
    let unit = CompEqUnit::default();
    let surround = SurroundState::default();
    let position = SurroundPartState::default();
    let def = |name, group, max, default, format, family| ParamDef {
        field,
        name,
        group,
        min: 0,
        max,
        default,
        format,
        family,
    };
    let eq_gain = SignedCenter(15);

    match field {
        F::PartLevel => def("Level", ParamGroup::Part, 127, 100, Raw, Amp),
        F::PartPan => def("Pan", ParamGroup::Part, 127, 64, SignedCenter(64), Time),
        F::PartMute => def("Mute", ParamGroup::Part, 1, 0, OnOff, Common),
        F::PartChorusSend => def("FX1", ParamGroup::Part, 127, 0, Raw, Amp),
        F::PartReverbSend => def("FX2", ParamGroup::Part, 127, 0, Raw, Amp),
        F::PartOutputAssign => def(
            "Output Assign",
            ParamGroup::Part,
            last(spec.output_assigns),
            0,
            Named(spec.output_assigns),
            Common,
        ),
        F::PartReceiveChannel => def(
            "Receive Channel",
            ParamGroup::Part,
            last(channel_names()),
            0,
            Named(channel_names()),
            Common,
        ),
        F::MasterLevel => def("Master Level", ParamGroup::Master, 127, 100, Raw, Amp),
        F::ExtLevel => def("Ext Level", ParamGroup::Ext, 127, 100, Raw, Amp),
        F::ExtMute => def("Ext Mute", ParamGroup::Ext, 1, 0, OnOff, Common),
        F::ChorusSwitch => def("Chorus", ParamGroup::Chorus, 1, 1, OnOff, Common),
        F::ChorusType => def(
            "Chorus Type",
            ParamGroup::Chorus,
            last(spec.chorus_type_names),
            0,
            Named(spec.chorus_type_names),
            Common,
        ),
        F::ChorusLevel => def("Chorus Level", ParamGroup::Chorus, 127, 0, Raw, Amp),
        F::ChorusOutputAssign => def(
            "Chorus Output Assign",
            ParamGroup::Chorus,
            last(spec.output_assign_names),
            0,
            Named(spec.output_assign_names),
            Common,
        ),
        F::ChorusOutputSelect => def(
            "Chorus Output Select",
            ParamGroup::Chorus,
            last(spec.chorus_output_names),
            0,
            Named(spec.chorus_output_names),
            Common,
        ),
        F::ReverbSwitch => def("Reverb", ParamGroup::Reverb, 1, 1, OnOff, Common),
        F::ReverbType => def(
            "Reverb Type",
            ParamGroup::Reverb,
            last(spec.reverb_type_names),
            0,
            Named(spec.reverb_type_names),
            Common,
        ),
        F::ReverbLevel => def("Reverb Level", ParamGroup::Reverb, 127, 0, Raw, Amp),
        F::ReverbOutputAssign => def(
            "Reverb Output Assign",
            ParamGroup::Reverb,
            last(spec.output_assign_names),
            0,
            Named(spec.output_assign_names),
            Common,
        ),
        F::SoloPart => def(
            "Solo Part",
            ParamGroup::Global,
            i32::from(part_count),
            0,
            Raw,
            Common,
        ),
        F::DrumCompEqSwitch => def(
            "Drum Comp+EQ",
            ParamGroup::Global,
            1,
            i32::from(DrumCompEqState::default().enabled),
            OnOff,
            Common,
        ),
        F::DrumCompEqPart => def(
            "Drum Comp+EQ Part",
            ParamGroup::Global,
            i32::from(part_count) - 1,
            i32::from(DrumCompEqState::default().part),
            Named(part_names()),
            Common,
        ),
        // Gains and frequencies all shape the spectrum.
        F::EqSwitch => def(
            "EQ Switch",
            ParamGroup::Eq,
            1,
            i32::from(eq.enabled),
            OnOff,
            Common,
        ),
        F::EqLowFreq => def(
            "Low Freq",
            ParamGroup::Eq,
            last(EQ_LOW_FREQ_NAMES),
            i32::from(eq.low_freq),
            Named(EQ_LOW_FREQ_NAMES),
            Filter,
        ),
        F::EqLowGain => def(
            "Low Gain",
            ParamGroup::Eq,
            30,
            i32::from(eq.low_gain),
            eq_gain,
            Filter,
        ),
        F::EqMidFreq => def(
            "Mid Freq",
            ParamGroup::Eq,
            last(EQ_MID_FREQ_NAMES),
            i32::from(eq.mid_freq),
            Named(EQ_MID_FREQ_NAMES),
            Filter,
        ),
        F::EqMidGain => def(
            "Mid Gain",
            ParamGroup::Eq,
            30,
            i32::from(eq.mid_gain),
            eq_gain,
            Filter,
        ),
        F::EqMidQ => def(
            "Mid Q",
            ParamGroup::Eq,
            last(EQ_Q_NAMES),
            i32::from(eq.mid_q),
            Named(EQ_Q_NAMES),
            Filter,
        ),
        F::EqHighFreq => def(
            "High Freq",
            ParamGroup::Eq,
            last(EQ_HIGH_FREQ_NAMES),
            i32::from(eq.high_freq),
            Named(EQ_HIGH_FREQ_NAMES),
            Filter,
        ),
        F::EqHighGain => def(
            "High Gain",
            ParamGroup::Eq,
            30,
            i32::from(eq.high_gain),
            eq_gain,
            Filter,
        ),
        // Drum Comp+EQ compressor: every unit shares these ranges and defaults.
        F::CompSwitch => def(
            "Comp Switch",
            ParamGroup::Comp,
            1,
            i32::from(unit.comp_switch),
            OnOff,
            Common,
        ),
        F::CompAttack => def(
            "Attack",
            ParamGroup::Comp,
            last(spec.comp_attack_times),
            i32::from(unit.comp_attack),
            Named(spec.comp_attack_times),
            Amp,
        ),
        F::CompRelease => def(
            "Release",
            ParamGroup::Comp,
            last(spec.comp_release_times),
            i32::from(unit.comp_release),
            Named(spec.comp_release_times),
            Amp,
        ),
        F::CompThreshold => def(
            "Threshold",
            ParamGroup::Comp,
            127,
            i32::from(unit.comp_threshold),
            Raw,
            Amp,
        ),
        F::CompRatio => def(
            "Ratio",
            ParamGroup::Comp,
            last(spec.comp_ratios),
            i32::from(unit.comp_ratio),
            Named(spec.comp_ratios),
            Amp,
        ),
        F::CompOutputGain => def(
            "Output Gain",
            ParamGroup::Comp,
            24,
            i32::from(unit.comp_output_gain),
            SignedCenter(0),
            Amp,
        ),
        F::CompOutputAssign => def(
            "Output Assign",
            ParamGroup::Comp,
            last(spec.comp_eq_output_assigns),
            i32::from(DrumCompEqState::default().output_assigns[0]),
            Named(spec.comp_eq_output_assigns),
            Common,
        ),
        F::SurroundSwitch => def(
            "Surround",
            ParamGroup::Surround,
            1,
            i32::from(surround.enabled),
            OnOff,
            Common,
        ),
        F::SurroundRoomType => def(
            "Room Type",
            ParamGroup::Surround,
            last(spec.surround_room_types),
            i32::from(surround.room_type),
            Named(spec.surround_room_types),
            Common,
        ),
        F::SurroundRoomSize => def(
            "Room Size",
            ParamGroup::Surround,
            last(spec.surround_room_sizes),
            i32::from(surround.room_size),
            Named(spec.surround_room_sizes),
            Common,
        ),
        F::SurroundDepth => def(
            "Depth",
            ParamGroup::Surround,
            100,
            i32::from(surround.depth),
            Raw,
            Amp,
        ),
        F::SurroundAmbienceLevel => def(
            "Ambience Level",
            ParamGroup::Surround,
            127,
            i32::from(surround.ambience_level),
            Raw,
            Amp,
        ),
        F::SurroundAmbienceTime => def(
            "Ambience Time",
            ParamGroup::Surround,
            100,
            i32::from(surround.ambience_time),
            Raw,
            Time,
        ),
        F::SurroundAmbienceDensity => def(
            "Ambience Density",
            ParamGroup::Surround,
            100,
            i32::from(surround.ambience_density),
            Raw,
            Time,
        ),
        F::SurroundAmbienceHfDamp => def(
            "Ambience HF Damp",
            ParamGroup::Surround,
            100,
            i32::from(surround.ambience_hf_damp),
            Raw,
            Filter,
        ),
        // Position and width place the part in space, as pan does.
        F::SurroundLr => def(
            "L-R",
            ParamGroup::Surround,
            127,
            i32::from(position.lr),
            SignedCenter(64),
            Time,
        ),
        F::SurroundFb => def(
            "F-B",
            ParamGroup::Surround,
            127,
            i32::from(position.fb),
            SignedCenter(64),
            Time,
        ),
        F::SurroundWidth => def(
            "Width",
            ParamGroup::Surround,
            32,
            i32::from(position.width),
            Raw,
            Time,
        ),
        F::SurroundAmbienceSend => def(
            "Ambience Send",
            ParamGroup::Surround,
            127,
            i32::from(position.ambience_send),
            Raw,
            Amp,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_field_once() {
        let mut seen = std::collections::HashSet::new();
        for field in MixerField::ALL {
            assert!(seen.insert(field), "{field:?} listed twice");
            assert_eq!(mixer_param(field).field, field);
        }
    }

    #[test]
    fn every_field_has_a_scope() {
        use MixerScopeKind as K;
        let kinds = [K::Part, K::Ext, K::Master, K::CompEqUnit, K::Common];
        for field in MixerField::ALL {
            assert!(
                kinds.iter().any(|&k| field.in_scope(k)),
                "{field:?} has no scope"
            );
        }
    }

    /// One scope of each kind, with its indices.
    fn scopes() -> [MixerScope; 5] {
        let kit = CompEqKit {
            part: 9,
            tone_type: ToneType::PcmDrum,
        };
        [
            MixerScope::Part(3),
            MixerScope::Ext,
            MixerScope::Master,
            MixerScope::CompEqUnit { kit, unit: 2 },
            MixerScope::Common,
        ]
    }

    #[test]
    fn in_scope_matches_address_resolution() {
        for field in MixerField::ALL {
            for scope in scopes() {
                assert_eq!(
                    mixer_field_address(field, scope).is_ok(),
                    field.in_scope(scope.kind()),
                    "{field:?} in {scope:?}"
                );
            }
        }
    }

    /// Every EQ field, with its offset in the Part EQ, Master EQ and
    /// Comp+EQ unit blocks as the strips hand-mapped them (part base 1,
    /// master base 0, Comp+EQ `0x07 + offset`, switches apart).
    const EQ_OFFSETS: [(MixerField, u8, u8, u8); 7] = [
        (MixerField::EqLowFreq, 1, 0, 0x07),
        (MixerField::EqLowGain, 2, 1, 0x08),
        (MixerField::EqMidFreq, 3, 2, 0x09),
        (MixerField::EqMidGain, 4, 3, 0x0A),
        (MixerField::EqMidQ, 5, 4, 0x0B),
        (MixerField::EqHighFreq, 6, 5, 0x0C),
        (MixerField::EqHighGain, 7, 6, 0x0D),
    ];

    #[test]
    fn eq_fields_resolve_per_scope() {
        let kit = CompEqKit {
            part: 9,
            tone_type: ToneType::SnDrum,
        };
        for (field, part_off, master_off, unit_off) in EQ_OFFSETS {
            assert_eq!(
                mixer_field_address(field, MixerScope::Part(5)),
                Ok(params::part_eq_address(5, part_off)),
                "{field:?} part"
            );
            assert_eq!(
                mixer_field_address(field, MixerScope::Master),
                Ok(params::master_eq_address(master_off)),
                "{field:?} master"
            );
            assert_eq!(
                mixer_field_address(field, MixerScope::CompEqUnit { kit, unit: 4 }),
                Ok(params::comp_eq_param_address(
                    9,
                    ToneType::SnDrum,
                    4,
                    unit_off
                )),
                "{field:?} Comp+EQ"
            );
        }
    }

    #[test]
    fn eq_switch_resolves_per_scope() {
        let kit = CompEqKit {
            part: 0,
            tone_type: ToneType::PcmDrum,
        };
        let switch = MixerField::EqSwitch;
        assert_eq!(
            mixer_field_address(switch, MixerScope::Part(15)),
            Ok(params::part_eq_address(15, 0x00))
        );
        assert_eq!(
            mixer_field_address(switch, MixerScope::Master),
            Ok(params::MASTER_EQ_SWITCH)
        );
        assert_eq!(
            mixer_field_address(switch, MixerScope::CompEqUnit { kit, unit: 0 }),
            Ok(params::comp_eq_param_address(0, ToneType::PcmDrum, 0, 0x06))
        );
        assert!(!switch.in_scope(MixerScopeKind::Ext));
    }

    #[test]
    fn comp_fields_resolve_in_their_unit() {
        let kit = CompEqKit {
            part: 2,
            tone_type: ToneType::PcmDrum,
        };
        let comp = [
            MixerField::CompSwitch,
            MixerField::CompAttack,
            MixerField::CompRelease,
            MixerField::CompThreshold,
            MixerField::CompRatio,
            MixerField::CompOutputGain,
        ];
        for (offset, field) in (0u8..).zip(comp) {
            assert_eq!(
                mixer_field_address(field, MixerScope::CompEqUnit { kit, unit: 5 }),
                Ok(params::comp_eq_param_address(
                    2,
                    ToneType::PcmDrum,
                    5,
                    offset
                )),
                "{field:?}"
            );
            assert!(!field.in_scope(MixerScopeKind::Part), "{field:?}");
        }
    }

    #[test]
    fn strip_fields_resolve_to_their_blocks() {
        assert_eq!(
            mixer_field_address(MixerField::PartChorusSend, MixerScope::Part(1)),
            Ok(params::part_address(1, params::part::CHORUS_SEND))
        );
        assert_eq!(
            mixer_field_address(MixerField::ExtMute, MixerScope::Ext),
            Ok(params::EXT_PART_MUTE)
        );
        assert_eq!(
            mixer_field_address(MixerField::MasterLevel, MixerScope::Master),
            Ok(params::SYSTEM_MASTER_LEVEL)
        );
        assert_eq!(
            mixer_field_address(MixerField::ReverbOutputAssign, MixerScope::Common),
            Ok(params::reverb_address(params::reverb::OUTPUT_ASSIGN))
        );
    }

    #[test]
    fn drum_comp_eq_part_is_the_studio_set_common_assign() {
        // Studio Set Common 00 44, Drum Comp/EQ Part (docs/midi/05-studio-set.md).
        assert_eq!(
            mixer_field_address(MixerField::DrumCompEqPart, MixerScope::Common),
            Ok(Address::new(0x18, 0x00, 0x00, 0x44))
        );
        assert!(!MixerField::DrumCompEqPart.in_scope(MixerScopeKind::Part));
        let def = mixer_param(MixerField::DrumCompEqPart);
        assert_eq!((def.min, def.max, def.default), (0, 15, 9));
        assert_eq!(format_value(&def, 0), "1");
        assert_eq!(format_value(&def, 15), "16");
    }

    #[test]
    fn drum_comp_eq_switch_is_the_studio_set_common_switch() {
        // Studio Set Common 00 43, Drum Comp/EQ Switch (docs/midi/05-studio-set.md).
        let field = MixerField::DrumCompEqSwitch;
        assert_eq!(
            mixer_field_address(field, MixerScope::Common),
            Ok(Address::new(0x18, 0x00, 0x00, 0x43))
        );
        assert!(!field.in_scope(MixerScopeKind::Part));
        let def = mixer_param(field);
        assert_eq!((def.min, def.max, def.default), (0, 1, 0));
        assert_eq!(format_value(&def, 1), "ON");
    }

    #[test]
    fn comp_output_assign_is_the_units_studio_set_common_assign() {
        // Studio Set Common 00 45 - 00 4A, Drum Comp/EQ 1-6 Output Assign
        // (docs/midi/05-studio-set.md): in the common block, whatever the kit.
        let field = MixerField::CompOutputAssign;
        for tone_type in [ToneType::PcmDrum, ToneType::SnDrum] {
            let kit = CompEqKit { part: 4, tone_type };
            for unit in 0..6 {
                assert_eq!(
                    mixer_field_address(field, MixerScope::CompEqUnit { kit, unit }),
                    Ok(Address::new(0x18, 0x00, 0x00, 0x45 + unit)),
                    "unit {unit}"
                );
            }
            assert_eq!(
                mixer_field_address(field, MixerScope::CompEqUnit { kit, unit: 6 }),
                Err(MixerFieldError::UnitOutOfRange(6))
            );
        }
        assert!(field.in_scope(MixerScopeKind::CompEqUnit));
        assert!(!field.in_scope(MixerScopeKind::Part));
        let def = mixer_param(field);
        assert_eq!((def.min, def.max, def.default), (0, 12, 0));
        assert_eq!(format_value(&def, 0), "PART");
        assert_eq!(format_value(&def, 12), "8");
    }

    #[test]
    fn ambience_send_is_the_motional_surround_send_of_a_part_or_ext() {
        // Studio Set Part 00 49 and Motional Surround 00 0A, Ambience Send
        // Level (docs/midi/05-studio-set.md).
        let field = MixerField::SurroundAmbienceSend;
        assert_eq!(
            mixer_field_address(field, MixerScope::Part(7)),
            Ok(params::part_address(7, [0x00, 0x00, 0x49]))
        );
        assert_eq!(
            mixer_field_address(field, MixerScope::Ext),
            Ok(Address::new(0x18, 0x00, 0x08, 0x0A))
        );
        assert!(!field.in_scope(MixerScopeKind::Master));
        let def = mixer_param(field);
        assert_eq!((def.min, def.max, def.default), (0, 127, 0));
        assert_eq!(def.group, ParamGroup::Surround);
    }

    #[test]
    fn out_of_scope_and_range_are_errors() {
        assert_eq!(
            mixer_field_address(MixerField::PartLevel, MixerScope::Master),
            Err(MixerFieldError::NotInScope {
                field: MixerField::PartLevel,
                scope: MixerScopeKind::Master,
            })
        );
        assert_eq!(
            mixer_field_address(MixerField::EqLowGain, MixerScope::Part(16)),
            Err(MixerFieldError::PartOutOfRange(16))
        );
        let kit = CompEqKit {
            part: 0,
            tone_type: ToneType::PcmDrum,
        };
        assert_eq!(
            mixer_field_address(
                MixerField::CompRatio,
                MixerScope::CompEqUnit { kit, unit: 6 }
            ),
            Err(MixerFieldError::UnitOutOfRange(6))
        );
    }

    #[test]
    fn part_count_drives_solo_part() {
        assert_eq!(mixer_param_for(MixerField::SoloPart, 4).max, 4);
        let rx = mixer_param(MixerField::PartReceiveChannel);
        // Receive channels follow MIDI, not the part count.
        assert_eq!(rx.max, i32::from(MIDI_CHANNELS) - 1);
        assert_eq!(format_value(&rx, 15), "16");
    }

    #[test]
    fn part_level_def() {
        let def = mixer_param(MixerField::PartLevel);
        assert_eq!((def.min, def.max, def.default), (0, 127, 100));
        assert_eq!(def.group, ParamGroup::Part);
    }

    #[test]
    fn format_pan_center() {
        assert_eq!(format_value(&mixer_param(MixerField::PartPan), 64), "0");
    }

    #[test]
    fn format_on_off() {
        let def = mixer_param(MixerField::PartMute);
        assert_eq!(format_value(&def, 0), "OFF");
        assert_eq!(format_value(&def, 1), "ON");
    }

    #[test]
    fn format_named() {
        let def = mixer_param(MixerField::PartOutputAssign);
        assert_eq!(format_value(&def, 0), "A");
        assert_eq!(format_value(&def, 3), "D");
    }

    #[test]
    fn format_chorus_output_assign() {
        let def = mixer_param(MixerField::ChorusOutputAssign);
        assert_eq!(format_value(&def, 0), "A");
        assert_eq!(format_value(&def, 3), "D");
    }

    #[test]
    fn fx_type_zero_is_off() {
        for field in [MixerField::ChorusType, MixerField::ReverbType] {
            assert_eq!(format_value(&mixer_param(field), 0), "OFF", "{field:?}");
        }
    }

    #[test]
    fn eq_named_tables_cover_their_range() {
        for field in [
            MixerField::EqLowFreq,
            MixerField::EqMidFreq,
            MixerField::EqMidQ,
            MixerField::EqHighFreq,
        ] {
            let def = mixer_param(field);
            let ParamFormat::Named(names) = def.format else {
                panic!("{field:?} should be named");
            };
            assert_eq!(names.len() as i32, def.max - def.min + 1, "{field:?}");
        }
        let gain = mixer_param(MixerField::EqMidGain);
        assert_eq!(format_value(&gain, 0), "-15");
        assert_eq!(format_value(&gain, 30), "+15");
    }

    #[test]
    fn eq_defaults_match_eq_state() {
        let eq = EqState::default();
        assert_eq!(
            mixer_param(MixerField::EqSwitch).default,
            i32::from(eq.enabled)
        );
        assert_eq!(mixer_param(MixerField::EqMidFreq).default, 7);
        assert_eq!(mixer_param(MixerField::EqLowFreq).default, 1);
    }

    #[test]
    fn eq_gains_are_filter() {
        for field in [
            MixerField::EqLowGain,
            MixerField::EqMidGain,
            MixerField::EqHighGain,
        ] {
            assert_eq!(
                mixer_param(field).family,
                ParameterFamily::Filter,
                "{field:?}"
            );
        }
    }

    #[test]
    fn comp_threshold_is_amp() {
        assert_eq!(
            mixer_param(MixerField::CompThreshold).family,
            ParameterFamily::Amp
        );
        assert_eq!(
            mixer_param(MixerField::CompSwitch).family,
            ParameterFamily::Common
        );
    }

    #[test]
    fn comp_named_tables_cover_their_range() {
        for field in [
            MixerField::CompAttack,
            MixerField::CompRelease,
            MixerField::CompRatio,
        ] {
            let def = mixer_param(field);
            let ParamFormat::Named(names) = def.format else {
                panic!("{field:?} should be named");
            };
            assert_eq!(names.len() as i32, def.max - def.min + 1, "{field:?}");
        }
        let ratio = mixer_param(MixerField::CompRatio);
        assert_eq!(format_value(&ratio, ratio.max), "inf:1");
    }

    /// The per-part surround fields, with their Part block offset and the
    /// Ext Part field they resolve to in the Ext scope.
    const SURROUND_POSITION: [(MixerField, [u8; 3], params::SurroundField); 4] = [
        (
            MixerField::SurroundLr,
            params::part_surround::LR,
            params::SurroundField::ExtPartLr,
        ),
        (
            MixerField::SurroundFb,
            params::part_surround::FB,
            params::SurroundField::ExtPartFb,
        ),
        (
            MixerField::SurroundWidth,
            params::part_surround::WIDTH,
            params::SurroundField::ExtPartWidth,
        ),
        (
            MixerField::SurroundAmbienceSend,
            params::part_surround::AMBIENCE_SEND,
            params::SurroundField::ExtPartAmbienceSend,
        ),
    ];

    #[test]
    fn surround_position_fields_resolve_per_part_and_ext() {
        for (field, offset, ext) in SURROUND_POSITION {
            assert_eq!(
                mixer_field_address(field, MixerScope::Part(7)),
                Ok(params::part_address(7, offset)),
                "{field:?} part"
            );
            assert_eq!(
                mixer_field_address(field, MixerScope::Ext),
                Ok(params::surround_address(ext as u8)),
                "{field:?} ext"
            );
            assert!(!field.in_scope(MixerScopeKind::Common), "{field:?}");
        }
    }

    #[test]
    fn surround_common_fields_resolve_in_the_surround_block() {
        use params::SurroundField as S;
        let common = [
            (MixerField::SurroundSwitch, S::Switch),
            (MixerField::SurroundRoomType, S::RoomType),
            (MixerField::SurroundAmbienceLevel, S::AmbienceLevel),
            (MixerField::SurroundRoomSize, S::RoomSize),
            (MixerField::SurroundAmbienceTime, S::AmbienceTime),
            (MixerField::SurroundAmbienceDensity, S::AmbienceDensity),
            (MixerField::SurroundAmbienceHfDamp, S::AmbienceHfDamp),
            (MixerField::SurroundDepth, S::Depth),
        ];
        for (field, surround) in common {
            assert_eq!(
                mixer_field_address(field, MixerScope::Common),
                Ok(params::surround_address(surround as u8)),
                "{field:?}"
            );
            assert!(!field.in_scope(MixerScopeKind::Part), "{field:?}");
        }
    }

    #[test]
    fn surround_defs_follow_the_device_and_state() {
        let state = SurroundState::default();
        let part = SurroundPartState::default();
        let room = mixer_param(MixerField::SurroundRoomType);
        assert_eq!(room.max as usize, INTEGRA7.surround_room_types.len() - 1);
        assert_eq!(format_value(&room, 3), "Hall 2");
        let size = mixer_param(MixerField::SurroundRoomSize);
        assert_eq!(size.default, i32::from(state.room_size));
        assert_eq!(format_value(&size, 0), "Small");
        let width = mixer_param(MixerField::SurroundWidth);
        assert_eq!((width.max, width.default), (32, i32::from(part.width)));
        let lr = mixer_param(MixerField::SurroundLr);
        assert_eq!(format_value(&lr, 64), "0");
        assert_eq!(mixer_param(MixerField::SurroundDepth).max, 100);
        assert_eq!(
            mixer_param(MixerField::SurroundSwitch).format,
            ParamFormat::OnOff
        );
    }
}
