//! Part settings: the Studio Set Part parameters past the mixer's, in the
//! groups the manual's Part tabs name (KBD, PITCH, OFFSET, MIDI, VOICE,
//! SCALE).
//!
//! [`PartSetting`] names a field without its Part; [`PartSetting::def`]
//! gives its CLI name, label, group, range, default, display and Parameter
//! family, and [`PartSetting::address`] where a Part's value is written.
//! Addresses and ranges follow `docs/midi/05-studio-set.md` (Studio Set
//! Part, and Studio Set Common for Voice Reserve); defaults are the Roland
//! editor's `init` values (`docs/research/studio-set-part-gaps.md`).
//!
//! A mixer load reads them: the Part dump (`00 00`-`00 29`) holds the KBD,
//! PITCH and OFFSET fields, [`SETTINGS_READ_FROM`] the SCALE and MIDI
//! fields and the Velocity Curve, and Studio Set Common the Voice Reserves
//! (see [`MixerBlock`](crate::mixer_load::MixerBlock)).

use thiserror::Error;

use crate::address::{self, Address, DataSize};
use crate::device_spec::INTEGRA7;
use crate::family::ParameterFamily;
use crate::field_spec::FieldSpec;
use crate::param_registry::ParamFormat;

/// A group of Part settings, as the manual's Part tabs name them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartSettingGroup {
    /// Key and velocity ranges and fades, velocity sens, Mono/Poly, Legato
    /// and the velocity curve.
    Kbd,
    /// Octave, coarse and fine tune, bend range and portamento.
    Pitch,
    /// The Sound Ctrl offsets: cutoff, resonance, envelope and vibrato.
    Offset,
    /// The receive filters.
    Midi,
    /// Voice Reserve.
    Voice,
    /// Scale Tune Type, Key and the 12 per-note offsets.
    Scale,
}

impl PartSettingGroup {
    /// Every group, in the drawer's order.
    pub const ALL: [Self; 6] = [
        Self::Kbd,
        Self::Pitch,
        Self::Offset,
        Self::Midi,
        Self::Voice,
        Self::Scale,
    ];

    /// The group's name, as the manual's tab spells it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Kbd => "KBD",
            Self::Pitch => "PITCH",
            Self::Offset => "OFFSET",
            Self::Midi => "MIDI",
            Self::Voice => "VOICE",
            Self::Scale => "SCALE",
        }
    }
}

/// A Part setting, named without its Part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartSetting {
    /// Keyboard Range Lower (`00 1D`).
    KeyRangeLower,
    /// Keyboard Range Upper (`00 1E`).
    KeyRangeUpper,
    /// Keyboard Fade Width Lower (`00 1F`).
    KeyFadeLower,
    /// Keyboard Fade Width Upper (`00 20`).
    KeyFadeUpper,
    /// Velocity Range Lower (`00 21`).
    VelocityRangeLower,
    /// Velocity Range Upper (`00 22`).
    VelocityRangeUpper,
    /// Velocity Fade Width Lower (`00 23`).
    VelocityFadeLower,
    /// Velocity Fade Width Upper (`00 24`).
    VelocityFadeUpper,
    /// Part Velocity Sens Offset (`00 1C`).
    VelocitySensOffset,
    /// Part Mono/Poly (`00 0D`).
    MonoPoly,
    /// Part Legato Switch (`00 0E`).
    Legato,
    /// Velocity Curve Type (`00 43`).
    VelocityCurve,
    /// Part Octave Shift (`00 1B`).
    OctaveShift,
    /// Part Coarse Tune (`00 0B`).
    CoarseTune,
    /// Part Fine Tune (`00 0C`).
    FineTune,
    /// Part Pitch Bend Range (`00 0F`).
    BendRange,
    /// Part Portamento Switch (`00 10`).
    PortamentoSwitch,
    /// Part Portamento Time (`00 11`-`00 12`, nibblized).
    PortamentoTime,
    /// Part Cutoff Offset (`00 13`).
    CutoffOffset,
    /// Part Resonance Offset (`00 14`).
    ResonanceOffset,
    /// Part Attack Time Offset (`00 15`).
    AttackOffset,
    /// Part Decay Time Offset (`00 16`).
    DecayOffset,
    /// Part Release Time Offset (`00 17`).
    ReleaseOffset,
    /// Part Vibrato Rate (`00 18`).
    VibratoRate,
    /// Part Vibrato Depth (`00 19`).
    VibratoDepth,
    /// Part Vibrato Delay (`00 1A`).
    VibratoDelay,
    /// Receive Program Change (`00 39`).
    RxProgramChange,
    /// Receive Bank Select (`00 3A`).
    RxBankSelect,
    /// Receive Pitch Bend (`00 3B`).
    RxPitchBend,
    /// Receive Polyphonic Key Pressure (`00 3C`).
    RxPolyPressure,
    /// Receive Channel Pressure (`00 3D`).
    RxChannelPressure,
    /// Receive Modulation (`00 3E`).
    RxModulation,
    /// Receive Volume (`00 3F`).
    RxVolume,
    /// Receive Pan (`00 40`).
    RxPan,
    /// Receive Expression (`00 41`).
    RxExpression,
    /// Receive Hold-1 (`00 42`).
    RxHold1,
    /// The Part's Voice Reserve (Studio Set Common `00 18` + Part).
    VoiceReserve,
    /// Part Scale Tune Type (`00 2B`).
    ScaleTuneType,
    /// Part Scale Tune Key (`00 2C`).
    ScaleTuneKey,
    /// Part Scale Tune for C (`00 2D`).
    ScaleTuneC,
    /// Part Scale Tune for C# (`00 2E`).
    ScaleTuneCSharp,
    /// Part Scale Tune for D (`00 2F`).
    ScaleTuneD,
    /// Part Scale Tune for D# (`00 30`).
    ScaleTuneDSharp,
    /// Part Scale Tune for E (`00 31`).
    ScaleTuneE,
    /// Part Scale Tune for F (`00 32`).
    ScaleTuneF,
    /// Part Scale Tune for F# (`00 33`).
    ScaleTuneFSharp,
    /// Part Scale Tune for G (`00 34`).
    ScaleTuneG,
    /// Part Scale Tune for G# (`00 35`).
    ScaleTuneGSharp,
    /// Part Scale Tune for A (`00 36`).
    ScaleTuneA,
    /// Part Scale Tune for A# (`00 37`).
    ScaleTuneASharp,
    /// Part Scale Tune for B (`00 38`).
    ScaleTuneB,
}

/// The block a [`PartSetting`] lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartSettingBlock {
    /// The Part's own Studio Set Part block.
    Part,
    /// Studio Set Common, one byte per Part from the field's offset.
    Common,
}

/// A Part setting's definition: names, group, and range, default, display
/// and family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartSettingDef {
    /// The field this defines.
    pub field: PartSetting,
    /// The CLI's name for it (`key-lower`).
    pub name: &'static str,
    /// The label a control draws (`Key Lower`).
    pub label: &'static str,
    /// The group it belongs to.
    pub group: PartSettingGroup,
    /// Range, default, display, family and host name (the MIDI
    /// Implementation's wording within the Part, as the mixer's
    /// [`ParamDef::host_name`](crate::param_registry::ParamDef::host_name)).
    pub spec: FieldSpec,
}

/// Why a Part setting can't be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PartSettingError {
    /// The part index is past the device's last part.
    #[error("part index {0} out of range")]
    PartOutOfRange(u8),
    /// The value is outside the field's range.
    #[error("{} takes {min} to {max}, not {value}", field.name())]
    ValueOutOfRange {
        /// The field.
        field: PartSetting,
        /// The value refused.
        value: u8,
        /// The field's minimum.
        min: i32,
        /// The field's maximum.
        max: i32,
    },
}

/// What the SCALE group says while System Scale Tune Switch is OFF.
pub const SCALE_SWITCHED_OFF_NOTE: &str = "System Scale Tune is OFF: Part scales have no effect";

/// What the SCALE group says when the Part's tone ignores the scale (see
/// [`crate::relevance::tone_ignores_scale`]).
pub const SCALE_IGNORED_NOTE: &str = "This Part's tone ignores the scale";

/// Scale Tune Type CUSTOM: what the device sets the Type to once an offset
/// is written (`docs/midi/99-undocumented.md` §11).
pub const SCALE_TUNE_CUSTOM: u8 = 0;

/// First byte of the Part block a mixer load reads past the mixer dump:
/// Scale Tune Type (`00 2B`), through the Velocity Curve Type (`00 43`).
pub const SETTINGS_READ_FROM: [u8; 3] = [0x00, 0x00, 0x2B];

/// Bytes of the Part settings read from [`SETTINGS_READ_FROM`], `00 2B` to
/// `00 43`.
pub const SETTINGS_READ_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x00, 0x19);

/// Studio Set Common offset of Part 1's Voice Reserve; Part *n*'s follows.
const VOICE_RESERVE_OFFSET: u8 = 0x18;

/// The Voice Reserve block in Studio Set Common: its address and size, one
/// byte per Part.
pub fn voice_reserve_read() -> (Address, DataSize) {
    (
        address::studio_set_common([0x00, VOICE_RESERVE_OFFSET]),
        DataSize::from_value(u32::from(INTEGRA7.part_count)).expect("a byte per part"),
    )
}

/// Scale Tune Type names.
static SCALE_TUNE_TYPES: &[&str] = &[
    "CUSTOM",
    "EQUAL",
    "JUST-MAJ",
    "JUST-MIN",
    "PYTHAGORE",
    "KIRNBERGE",
    "MEANTONE",
    "WERCKMEIS",
    "ARABIC",
];

/// Scale Tune Key names.
static SCALE_TUNE_KEYS: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// A choice with TONE last: the tone's own setting.
static OFF_ON_TONE: &[&str] = &["OFF", "ON", "TONE"];
static MONO_POLY_TONE: &[&str] = &["MONO", "POLY", "TONE"];
static VELOCITY_CURVES: &[&str] = &["OFF", "1", "2", "3", "4"];

/// Everything the table says of a field.
struct Row {
    name: &'static str,
    label: &'static str,
    host_name: &'static str,
    group: PartSettingGroup,
    offset: u8,
    min: i32,
    max: i32,
    default: i32,
    format: ParamFormat,
    family: ParameterFamily,
}

impl PartSetting {
    /// Every Part setting, in registry order: by group, as the drawer
    /// stacks them.
    pub const ALL: [Self; 51] = [
        Self::KeyRangeLower,
        Self::KeyRangeUpper,
        Self::KeyFadeLower,
        Self::KeyFadeUpper,
        Self::VelocityRangeLower,
        Self::VelocityRangeUpper,
        Self::VelocityFadeLower,
        Self::VelocityFadeUpper,
        Self::VelocitySensOffset,
        Self::MonoPoly,
        Self::Legato,
        Self::VelocityCurve,
        Self::OctaveShift,
        Self::CoarseTune,
        Self::FineTune,
        Self::BendRange,
        Self::PortamentoSwitch,
        Self::PortamentoTime,
        Self::CutoffOffset,
        Self::ResonanceOffset,
        Self::AttackOffset,
        Self::DecayOffset,
        Self::ReleaseOffset,
        Self::VibratoRate,
        Self::VibratoDepth,
        Self::VibratoDelay,
        Self::RxProgramChange,
        Self::RxBankSelect,
        Self::RxPitchBend,
        Self::RxPolyPressure,
        Self::RxChannelPressure,
        Self::RxModulation,
        Self::RxVolume,
        Self::RxPan,
        Self::RxExpression,
        Self::RxHold1,
        Self::VoiceReserve,
        Self::ScaleTuneType,
        Self::ScaleTuneKey,
        Self::ScaleTuneC,
        Self::ScaleTuneCSharp,
        Self::ScaleTuneD,
        Self::ScaleTuneDSharp,
        Self::ScaleTuneE,
        Self::ScaleTuneF,
        Self::ScaleTuneFSharp,
        Self::ScaleTuneG,
        Self::ScaleTuneGSharp,
        Self::ScaleTuneA,
        Self::ScaleTuneASharp,
        Self::ScaleTuneB,
    ];

    /// The 12 Scale Tune offsets, C to B.
    pub const SCALE_TUNE_OFFSETS: [Self; 12] = [
        Self::ScaleTuneC,
        Self::ScaleTuneCSharp,
        Self::ScaleTuneD,
        Self::ScaleTuneDSharp,
        Self::ScaleTuneE,
        Self::ScaleTuneF,
        Self::ScaleTuneFSharp,
        Self::ScaleTuneG,
        Self::ScaleTuneGSharp,
        Self::ScaleTuneA,
        Self::ScaleTuneASharp,
        Self::ScaleTuneB,
    ];

    /// The field's definition.
    pub fn def(self) -> PartSettingDef {
        let row = self.row();
        PartSettingDef {
            field: self,
            name: row.name,
            label: row.label,
            group: row.group,
            spec: FieldSpec {
                min: row.min,
                max: row.max,
                default: row.default,
                format: row.format,
                family: row.family,
                host_name: row.host_name,
            },
        }
    }

    /// The CLI's name for the field (`key-lower`).
    pub fn name(self) -> &'static str {
        self.row().name
    }

    /// The field whose CLI name is `name`.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.name() == name)
    }

    /// The block the field lives in.
    pub fn block(self) -> PartSettingBlock {
        match self {
            Self::VoiceReserve => PartSettingBlock::Common,
            _ => PartSettingBlock::Part,
        }
    }

    /// The field's offset in its [`block`](Self::block): in the Part block,
    /// or in Studio Set Common for Part 1 (Part *n*'s byte follows).
    pub fn offset(self) -> u8 {
        self.row().offset
    }

    /// Bytes the value takes: 2 for the nibblized Portamento Time, else 1.
    pub fn width(self) -> usize {
        match self {
            Self::PortamentoTime => 2,
            _ => 1,
        }
    }

    /// The address `part`'s (0-based) value is written to.
    ///
    /// # Errors
    ///
    /// [`PartSettingError::PartOutOfRange`] for a part past the device's.
    pub fn address(self, part: u8) -> Result<Address, PartSettingError> {
        if part >= INTEGRA7.part_count {
            return Err(PartSettingError::PartOutOfRange(part));
        }
        Ok(match self.block() {
            PartSettingBlock::Part => address::studio_set_part(part, [0, 0, self.offset()]),
            PartSettingBlock::Common => address::studio_set_common([0, self.offset() + part]),
        })
    }

    /// The bytes that write `value`: the value, or its two nibbles (most
    /// significant first) for Portamento Time.
    pub fn encode(self, value: u8) -> Vec<u8> {
        match self.width() {
            2 => vec![value >> 4, value & 0x0F],
            _ => vec![value],
        }
    }

    /// The field's other end, if it is the Lower or Upper of a key or
    /// velocity range, and whether it is the Lower.
    fn range_partner(self) -> Option<(Self, bool)> {
        Some(match self {
            Self::KeyRangeLower => (Self::KeyRangeUpper, true),
            Self::KeyRangeUpper => (Self::KeyRangeLower, false),
            Self::VelocityRangeLower => (Self::VelocityRangeUpper, true),
            Self::VelocityRangeUpper => (Self::VelocityRangeLower, false),
            _ => return None,
        })
    }

    /// Whether the field is one of the 12 Scale Tune offsets.
    pub fn is_scale_tune_offset(self) -> bool {
        Self::SCALE_TUNE_OFFSETS.contains(&self)
    }

    /// The field's place in [`ALL`](Self::ALL).
    fn index(self) -> usize {
        self as usize
    }

    /// The registry's table: one row per field.
    #[rustfmt::skip]
    fn row(self) -> Row {
        use ParamFormat::{Named, Note, NumberOr, OnOff, Raw, SignedCenter};
        use ParameterFamily::{Amp, Common, Filter, Modulation, Pitch};
        use PartSettingGroup as G;
        let row = |name, label, host_name, group, offset, (min, max), default, format, family| Row {
            name,
            label,
            host_name,
            group,
            offset,
            min,
            max,
            default,
            format,
            family,
        };
        let last = |names: &[&str]| (0, names.len() as i32 - 1);
        const SIGNED: ParamFormat = SignedCenter(64);
        // A Sound Ctrl offset, -64 to +63.
        let sound = |name, label, host_name, offset, family| {
            row(name, label, host_name, G::Offset, offset, (0, 127), 64, SIGNED, family)
        };
        // A receive filter, on by default.
        let rx = |name, label, host_name, offset| {
            row(name, label, host_name, G::Midi, offset, (0, 1), 1, OnOff, Common)
        };
        // A Scale Tune offset, -64 to +63 cents.
        let scale = |name, label, host_name, key: u8| {
            row(name, label, host_name, G::Scale, 0x2D + key, (0, 127), 64, SIGNED, Pitch)
        };
        match self {
            Self::KeyRangeLower => row("key-lower", "Key Lower", "Keyboard Range Lower", G::Kbd, 0x1D, (0, 127), 0, Note, Pitch),
            Self::KeyRangeUpper => row("key-upper", "Key Upper", "Keyboard Range Upper", G::Kbd, 0x1E, (0, 127), 127, Note, Pitch),
            Self::KeyFadeLower => row("key-fade-lower", "Key Fade Lower", "Keyboard Fade Width Lower", G::Kbd, 0x1F, (0, 127), 0, Raw, Pitch),
            Self::KeyFadeUpper => row("key-fade-upper", "Key Fade Upper", "Keyboard Fade Width Upper", G::Kbd, 0x20, (0, 127), 0, Raw, Pitch),
            Self::VelocityRangeLower => row("velocity-lower", "Velo Lower", "Velocity Range Lower", G::Kbd, 0x21, (1, 127), 1, Raw, Amp),
            Self::VelocityRangeUpper => row("velocity-upper", "Velo Upper", "Velocity Range Upper", G::Kbd, 0x22, (0, 127), 127, Raw, Amp),
            Self::VelocityFadeLower => row("velocity-fade-lower", "Velo Fade Lower", "Velocity Fade Width Lower", G::Kbd, 0x23, (0, 127), 0, Raw, Amp),
            Self::VelocityFadeUpper => row("velocity-fade-upper", "Velo Fade Upper", "Velocity Fade Width Upper", G::Kbd, 0x24, (0, 127), 0, Raw, Amp),
            Self::VelocitySensOffset => row("velocity-sens", "Velo Sens", "Velocity Sens Offset", G::Kbd, 0x1C, (1, 127), 64, SIGNED, Amp),
            Self::MonoPoly => row("mono-poly", "Mono/Poly", "Mono/Poly", G::Kbd, 0x0D, last(MONO_POLY_TONE), 2, Named(MONO_POLY_TONE), Common),
            Self::Legato => row("legato", "Legato", "Legato Switch", G::Kbd, 0x0E, last(OFF_ON_TONE), 2, Named(OFF_ON_TONE), Common),
            Self::VelocityCurve => row("velocity-curve", "Velo Curve", "Velocity Curve Type", G::Kbd, 0x43, last(VELOCITY_CURVES), 0, Named(VELOCITY_CURVES), Amp),
            Self::OctaveShift => row("octave", "Octave", "Octave Shift", G::Pitch, 0x1B, (61, 67), 64, SIGNED, Pitch),
            Self::CoarseTune => row("coarse-tune", "Coarse", "Coarse Tune", G::Pitch, 0x0B, (16, 112), 64, SIGNED, Pitch),
            Self::FineTune => row("fine-tune", "Fine", "Fine Tune", G::Pitch, 0x0C, (14, 114), 64, SIGNED, Pitch),
            Self::BendRange => row("bend-range", "Bend Range", "Pitch Bend Range", G::Pitch, 0x0F, (0, 25), 25, NumberOr { top: 25, name: "TONE" }, Pitch),
            Self::PortamentoSwitch => row("portamento", "Portamento", "Portamento Switch", G::Pitch, 0x10, last(OFF_ON_TONE), 2, Named(OFF_ON_TONE), Pitch),
            Self::PortamentoTime => row("portamento-time", "Porta Time", "Portamento Time", G::Pitch, 0x11, (0, 128), 128, NumberOr { top: 128, name: "TONE" }, Pitch),
            Self::CutoffOffset => sound("cutoff", "Cutoff", "Cutoff Offset", 0x13, Filter),
            Self::ResonanceOffset => sound("resonance", "Resonance", "Resonance Offset", 0x14, Filter),
            Self::AttackOffset => sound("attack", "Attack", "Attack Time Offset", 0x15, Amp),
            Self::DecayOffset => sound("decay", "Decay", "Decay Time Offset", 0x16, Amp),
            Self::ReleaseOffset => sound("release", "Release", "Release Time Offset", 0x17, Amp),
            Self::VibratoRate => sound("vibrato-rate", "Vib Rate", "Vibrato Rate", 0x18, Modulation),
            Self::VibratoDepth => sound("vibrato-depth", "Vib Depth", "Vibrato Depth", 0x19, Modulation),
            Self::VibratoDelay => sound("vibrato-delay", "Vib Delay", "Vibrato Delay", 0x1A, Modulation),
            Self::RxProgramChange => rx("rx-program-change", "Rx PC", "Receive Program Change", crate::params::part::RECEIVE_PROGRAM_CHANGE[2]),
            Self::RxBankSelect => rx("rx-bank-select", "Rx BS", "Receive Bank Select", 0x3A),
            Self::RxPitchBend => rx("rx-pitch-bend", "Rx Bend", "Receive Pitch Bend", 0x3B),
            Self::RxPolyPressure => rx("rx-poly-pressure", "Rx Poly Aft", "Receive Polyphonic Key Pressure", 0x3C),
            Self::RxChannelPressure => rx("rx-channel-pressure", "Rx Ch Aft", "Receive Channel Pressure", 0x3D),
            Self::RxModulation => rx("rx-modulation", "Rx Mod", "Receive Modulation", 0x3E),
            Self::RxVolume => rx("rx-volume", "Rx Volume", "Receive Volume", 0x3F),
            Self::RxPan => rx("rx-pan", "Rx Pan", "Receive Pan", 0x40),
            Self::RxExpression => rx("rx-expression", "Rx Expression", "Receive Expression", 0x41),
            Self::RxHold1 => rx("rx-hold", "Rx Hold-1", "Receive Hold-1", 0x42),
            Self::VoiceReserve => row("voice-reserve", "Voice Reserve", "Voice Reserve", G::Voice, VOICE_RESERVE_OFFSET, (0, 64), 0, NumberOr { top: 64, name: "FULL" }, Common),
            Self::ScaleTuneType => row("scale-type", "Scale Type", "Scale Tune Type", G::Scale, 0x2B, last(SCALE_TUNE_TYPES), 1, Named(SCALE_TUNE_TYPES), Pitch),
            Self::ScaleTuneKey => row("scale-key", "Scale Key", "Scale Tune Key", G::Scale, 0x2C, last(SCALE_TUNE_KEYS), 0, Named(SCALE_TUNE_KEYS), Pitch),
            Self::ScaleTuneC => scale("scale-c", "C", "Scale Tune for C", 0),
            Self::ScaleTuneCSharp => scale("scale-c-sharp", "C#", "Scale Tune for C#", 1),
            Self::ScaleTuneD => scale("scale-d", "D", "Scale Tune for D", 2),
            Self::ScaleTuneDSharp => scale("scale-d-sharp", "D#", "Scale Tune for D#", 3),
            Self::ScaleTuneE => scale("scale-e", "E", "Scale Tune for E", 4),
            Self::ScaleTuneF => scale("scale-f", "F", "Scale Tune for F", 5),
            Self::ScaleTuneFSharp => scale("scale-f-sharp", "F#", "Scale Tune for F#", 6),
            Self::ScaleTuneG => scale("scale-g", "G", "Scale Tune for G", 7),
            Self::ScaleTuneGSharp => scale("scale-g-sharp", "G#", "Scale Tune for G#", 8),
            Self::ScaleTuneA => scale("scale-a", "A", "Scale Tune for A", 9),
            Self::ScaleTuneASharp => scale("scale-a-sharp", "A#", "Scale Tune for A#", 10),
            Self::ScaleTuneB => scale("scale-b", "B", "Scale Tune for B", 11),
        }
    }
}

/// Where a single-byte write at `address` lands among the Part settings:
/// the Part (0-based), the field, and which of its bytes (0, or 1 for
/// Portamento Time's low nibble). `None` for any other address.
pub fn locate(address: &Address) -> Option<(u8, PartSetting, usize)> {
    let [a, b, c, d] = *address.as_bytes();
    let studio_set = address::STUDIO_SET.as_bytes();
    if [a, b] != [studio_set[0], studio_set[1]] {
        return None;
    }
    let parts = INTEGRA7.part_count;
    let first_part = address::studio_set_part(0, [0, 0, 0]).as_bytes()[2];
    if c == 0 {
        let part = d.checked_sub(VOICE_RESERVE_OFFSET).filter(|p| *p < parts)?;
        return Some((part, PartSetting::VoiceReserve, 0));
    }
    let part = c.checked_sub(first_part).filter(|p| *p < parts)?;
    PartSetting::ALL
        .into_iter()
        .filter(|f| f.block() == PartSettingBlock::Part)
        .find_map(|f| {
            let byte = usize::from(d.checked_sub(f.offset())?);
            (byte < f.width()).then_some((part, f, byte))
        })
}

/// A Part's settings: one raw value per [`PartSetting`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartSettings([u8; PartSetting::ALL.len()]);

impl Default for PartSettings {
    /// Every field at its default.
    fn default() -> Self {
        Self(PartSetting::ALL.map(|f| f.def().spec.default as u8))
    }
}

impl PartSettings {
    /// The raw value of `field`.
    pub fn get(&self, field: PartSetting) -> u8 {
        self.0[field.index()]
    }

    /// Store a raw value of `field`, as the device holds it; no rule applies
    /// (see [`constrain`](Self::constrain)).
    pub fn set(&mut self, field: PartSetting, value: u8) {
        self.0[field.index()] = value;
    }

    /// The value a write of `value` to `field` stores: a key or velocity
    /// Lower held at or under its Upper, and an Upper at or over its Lower.
    /// Any other field takes `value`.
    pub fn constrain(&self, field: PartSetting, value: u8) -> u8 {
        match field.range_partner() {
            Some((upper, true)) => value.min(self.get(upper)),
            Some((lower, false)) => value.max(self.get(lower)),
            None => value,
        }
    }

    /// Take byte `byte` of `field` (see [`locate`]): the value itself, or
    /// one nibble of Portamento Time.
    pub fn set_byte(&mut self, field: PartSetting, byte: usize, value: u8) {
        let stored = match (field.width(), byte) {
            (2, 0) => (value & 0x0F) << 4 | (self.get(field) & 0x0F),
            (2, _) => (self.get(field) & 0xF0) | (value & 0x0F),
            _ => value,
        };
        self.set(field, stored);
    }

    /// Take the Part-block fields wholly inside `data`, a dump of the Part
    /// block from offset `from`. Fields it doesn't cover keep their value.
    pub fn apply_dump(&mut self, from: u8, data: &[u8]) {
        for field in PartSetting::ALL {
            if field.block() != PartSettingBlock::Part {
                continue;
            }
            let Some(start) = field.offset().checked_sub(from).map(usize::from) else {
                continue;
            };
            let Some(bytes) = data.get(start..start + field.width()) else {
                continue;
            };
            for (i, byte) in bytes.iter().enumerate() {
                self.set_byte(field, i, *byte);
            }
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for PartSettings {
    /// As a map from each field's CLI name to its raw value.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(PartSetting::ALL.len()))?;
        for field in PartSetting::ALL {
            map.serialize_entry(field.name(), &self.get(field))?;
        }
        map.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for PartSettings {
    /// From the map [`Serialize`](serde::Serialize) writes: an unknown name
    /// is skipped, and a field missing keeps its default.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let values = std::collections::HashMap::<String, u8>::deserialize(deserializer)?;
        let mut settings = Self::default();
        for (name, value) in values {
            if let Some(field) = PartSetting::from_name(&name) {
                settings.set(field, value);
            }
        }
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn all_lists_every_field_once_in_index_order() {
        for (i, field) in PartSetting::ALL.into_iter().enumerate() {
            assert_eq!(field.index(), i, "{field:?}");
            assert_eq!(field.def().field, field);
        }
    }

    #[test]
    fn names_are_unique_and_found_back() {
        let mut seen = HashSet::new();
        for field in PartSetting::ALL {
            let name = field.name();
            assert!(seen.insert(name), "{name} named twice");
            assert_eq!(PartSetting::from_name(name), Some(field));
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{name} is not kebab-case"
            );
        }
        assert_eq!(
            PartSetting::from_name("key-lower"),
            Some(PartSetting::KeyRangeLower)
        );
        assert_eq!(PartSetting::from_name("level"), None);
    }

    #[test]
    fn groups_follow_the_prd() {
        use PartSettingGroup as G;
        let count = |group| {
            PartSetting::ALL
                .iter()
                .filter(|f| f.def().group == group)
                .count()
        };
        assert_eq!(count(G::Kbd), 12);
        assert_eq!(count(G::Pitch), 6);
        assert_eq!(count(G::Offset), 8);
        assert_eq!(count(G::Midi), 10);
        assert_eq!(count(G::Voice), 1);
        assert_eq!(count(G::Scale), 14);
        // ALL lists the groups in the drawer's order, each in one run.
        let mut groups: Vec<_> = PartSetting::ALL.iter().map(|f| f.def().group).collect();
        groups.dedup();
        assert_eq!(groups, G::ALL);
    }

    /// Offset, range and default of each field, against
    /// `docs/midi/05-studio-set.md` (Studio Set Part, and Common for Voice
    /// Reserve) and the editor's `init` values.
    #[test]
    fn offsets_and_ranges_match_the_midi_implementation() {
        use PartSetting as F;
        #[rustfmt::skip]
        let expected: [(F, u8, i32, i32, i32); 51] = [
            (F::KeyRangeLower, 0x1D, 0, 127, 0),
            (F::KeyRangeUpper, 0x1E, 0, 127, 127),
            (F::KeyFadeLower, 0x1F, 0, 127, 0),
            (F::KeyFadeUpper, 0x20, 0, 127, 0),
            (F::VelocityRangeLower, 0x21, 1, 127, 1),
            (F::VelocityRangeUpper, 0x22, 0, 127, 127),
            (F::VelocityFadeLower, 0x23, 0, 127, 0),
            (F::VelocityFadeUpper, 0x24, 0, 127, 0),
            (F::VelocitySensOffset, 0x1C, 1, 127, 64),
            (F::MonoPoly, 0x0D, 0, 2, 2),
            (F::Legato, 0x0E, 0, 2, 2),
            (F::VelocityCurve, 0x43, 0, 4, 0),
            (F::OctaveShift, 0x1B, 61, 67, 64),
            (F::CoarseTune, 0x0B, 16, 112, 64),
            (F::FineTune, 0x0C, 14, 114, 64),
            (F::BendRange, 0x0F, 0, 25, 25),
            (F::PortamentoSwitch, 0x10, 0, 2, 2),
            (F::PortamentoTime, 0x11, 0, 128, 128),
            (F::CutoffOffset, 0x13, 0, 127, 64),
            (F::ResonanceOffset, 0x14, 0, 127, 64),
            (F::AttackOffset, 0x15, 0, 127, 64),
            (F::DecayOffset, 0x16, 0, 127, 64),
            (F::ReleaseOffset, 0x17, 0, 127, 64),
            (F::VibratoRate, 0x18, 0, 127, 64),
            (F::VibratoDepth, 0x19, 0, 127, 64),
            (F::VibratoDelay, 0x1A, 0, 127, 64),
            (F::RxProgramChange, 0x39, 0, 1, 1),
            (F::RxBankSelect, 0x3A, 0, 1, 1),
            (F::RxPitchBend, 0x3B, 0, 1, 1),
            (F::RxPolyPressure, 0x3C, 0, 1, 1),
            (F::RxChannelPressure, 0x3D, 0, 1, 1),
            (F::RxModulation, 0x3E, 0, 1, 1),
            (F::RxVolume, 0x3F, 0, 1, 1),
            (F::RxPan, 0x40, 0, 1, 1),
            (F::RxExpression, 0x41, 0, 1, 1),
            (F::RxHold1, 0x42, 0, 1, 1),
            (F::VoiceReserve, 0x18, 0, 64, 0),
            (F::ScaleTuneType, 0x2B, 0, 8, 1),
            (F::ScaleTuneKey, 0x2C, 0, 11, 0),
            (F::ScaleTuneC, 0x2D, 0, 127, 64),
            (F::ScaleTuneCSharp, 0x2E, 0, 127, 64),
            (F::ScaleTuneD, 0x2F, 0, 127, 64),
            (F::ScaleTuneDSharp, 0x30, 0, 127, 64),
            (F::ScaleTuneE, 0x31, 0, 127, 64),
            (F::ScaleTuneF, 0x32, 0, 127, 64),
            (F::ScaleTuneFSharp, 0x33, 0, 127, 64),
            (F::ScaleTuneG, 0x34, 0, 127, 64),
            (F::ScaleTuneGSharp, 0x35, 0, 127, 64),
            (F::ScaleTuneA, 0x36, 0, 127, 64),
            (F::ScaleTuneASharp, 0x37, 0, 127, 64),
            (F::ScaleTuneB, 0x38, 0, 127, 64),
        ];
        assert_eq!(
            expected.map(|(f, ..)| f),
            PartSetting::ALL,
            "the table lists every field"
        );
        for (field, offset, min, max, default) in expected {
            let spec = field.def().spec;
            assert_eq!(field.offset(), offset, "{field:?} offset");
            assert_eq!(
                (spec.min, spec.max, spec.default),
                (min, max, default),
                "{field:?}"
            );
        }
    }

    #[test]
    fn addresses_are_in_the_part_block_or_common() {
        assert_eq!(
            PartSetting::KeyRangeLower.address(0),
            Ok(Address::new(0x18, 0x00, 0x20, 0x1D))
        );
        assert_eq!(
            PartSetting::RxHold1.address(15),
            Ok(Address::new(0x18, 0x00, 0x2F, 0x42))
        );
        assert_eq!(
            PartSetting::VoiceReserve.address(0),
            Ok(Address::new(0x18, 0x00, 0x00, 0x18))
        );
        assert_eq!(
            PartSetting::VoiceReserve.address(15),
            Ok(Address::new(0x18, 0x00, 0x00, 0x27))
        );
        assert_eq!(
            PartSetting::CutoffOffset.address(16),
            Err(PartSettingError::PartOutOfRange(16))
        );
    }

    #[test]
    fn displays_follow_the_midi_implementation() {
        let show = |field: PartSetting, value| field.def().spec.format.format(value);
        assert_eq!(show(PartSetting::KeyRangeLower, 0), "C-1");
        assert_eq!(show(PartSetting::KeyRangeUpper, 127), "G9");
        assert_eq!(show(PartSetting::CoarseTune, 16), "-48");
        assert_eq!(show(PartSetting::FineTune, 114), "+50");
        assert_eq!(show(PartSetting::OctaveShift, 61), "-3");
        assert_eq!(show(PartSetting::VelocitySensOffset, 1), "-63");
        assert_eq!(show(PartSetting::BendRange, 24), "24");
        assert_eq!(show(PartSetting::BendRange, 25), "TONE");
        assert_eq!(show(PartSetting::PortamentoTime, 127), "127");
        assert_eq!(show(PartSetting::PortamentoTime, 128), "TONE");
        assert_eq!(show(PartSetting::VoiceReserve, 63), "63");
        assert_eq!(show(PartSetting::VoiceReserve, 64), "FULL");
        assert_eq!(show(PartSetting::MonoPoly, 2), "TONE");
        assert_eq!(show(PartSetting::VelocityCurve, 0), "OFF");
        assert_eq!(show(PartSetting::ScaleTuneType, 8), "ARABIC");
        assert_eq!(show(PartSetting::ScaleTuneKey, 11), "B");
        assert_eq!(show(PartSetting::RxHold1, 0), "OFF");
        assert_eq!(show(PartSetting::ScaleTuneB, 0), "-64");
    }

    #[test]
    fn every_part_block_field_is_read_by_the_mixer_dump_or_the_settings_read() {
        let mixer_dump = 0..crate::params::PART_MIXER_SIZE.value();
        let from = u32::from(SETTINGS_READ_FROM[2]);
        let settings = from..from + SETTINGS_READ_SIZE.value();
        for field in PartSetting::ALL {
            if field.block() != PartSettingBlock::Part {
                continue;
            }
            let span = u32::from(field.offset())..u32::from(field.offset()) + field.width() as u32;
            let within = |r: &std::ops::Range<u32>| r.start <= span.start && span.end <= r.end;
            assert!(
                within(&mixer_dump) || within(&settings),
                "{field:?} is not read"
            );
        }
    }

    #[test]
    fn portamento_time_is_two_nibbles() {
        assert_eq!(PartSetting::PortamentoTime.encode(128), vec![0x08, 0x00]);
        assert_eq!(PartSetting::PortamentoTime.encode(0x5A), vec![0x05, 0x0A]);
        assert_eq!(PartSetting::CutoffOffset.encode(70), vec![70]);
        let mut settings = PartSettings::default();
        let mut dump = vec![0u8; 0x2A];
        dump[0x11] = 0x07;
        dump[0x12] = 0x0F;
        settings.apply_dump(0, &dump);
        assert_eq!(settings.get(PartSetting::PortamentoTime), 0x7F);
    }

    #[test]
    fn a_dump_fills_only_the_fields_it_covers() {
        let mut settings = PartSettings::default();
        // 00 2B..00 43, each byte its offset's low bits.
        let dump: Vec<u8> = (0x2B..=0x43u8).map(|o| o & 0x0F).collect();
        settings.apply_dump(0x2B, &dump);
        assert_eq!(settings.get(PartSetting::ScaleTuneType), 0x0B);
        assert_eq!(settings.get(PartSetting::ScaleTuneB), 0x08);
        assert_eq!(settings.get(PartSetting::RxHold1), 0x02);
        assert_eq!(settings.get(PartSetting::VelocityCurve), 0x03);
        // Not in the dump: default.
        assert_eq!(settings.get(PartSetting::KeyRangeUpper), 127);
        assert_eq!(settings.get(PartSetting::VoiceReserve), 0);
    }

    #[test]
    fn a_range_never_crosses() {
        let mut settings = PartSettings::default();
        settings.set(PartSetting::KeyRangeLower, 40);
        settings.set(PartSetting::KeyRangeUpper, 60);
        assert_eq!(settings.constrain(PartSetting::KeyRangeLower, 70), 60);
        assert_eq!(settings.constrain(PartSetting::KeyRangeLower, 50), 50);
        assert_eq!(settings.constrain(PartSetting::KeyRangeUpper, 10), 40);
        settings.set(PartSetting::VelocityRangeLower, 30);
        settings.set(PartSetting::VelocityRangeUpper, 90);
        assert_eq!(settings.constrain(PartSetting::VelocityRangeLower, 100), 90);
        assert_eq!(settings.constrain(PartSetting::VelocityRangeUpper, 5), 30);
        assert_eq!(settings.constrain(PartSetting::CutoffOffset, 5), 5);
    }

    #[test]
    fn locate_finds_each_byte() {
        for part in [0, 15] {
            for field in PartSetting::ALL {
                let address = field.address(part).unwrap();
                assert_eq!(locate(&address), Some((part, field, 0)), "{field:?}");
            }
        }
        let low = PartSetting::PortamentoTime
            .address(3)
            .unwrap()
            .offset([0, 0, 0, 1]);
        assert_eq!(locate(&low), Some((3, PartSetting::PortamentoTime, 1)));
        // The Part's level, a Common byte past the reserves, Part 17.
        assert_eq!(locate(&Address::new(0x18, 0x00, 0x20, 0x09)), None);
        assert_eq!(locate(&Address::new(0x18, 0x00, 0x00, 0x28)), None);
        assert_eq!(locate(&Address::new(0x18, 0x00, 0x30, 0x1D)), None);
    }

    #[test]
    fn set_byte_takes_one_nibble_of_portamento_time() {
        let mut settings = PartSettings::default();
        settings.set(PartSetting::PortamentoTime, 0x00);
        settings.set_byte(PartSetting::PortamentoTime, 0, 0x07);
        settings.set_byte(PartSetting::PortamentoTime, 1, 0x0E);
        assert_eq!(settings.get(PartSetting::PortamentoTime), 0x7E);
    }
}
