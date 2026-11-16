//! Reading the mixer state from the device: the blocks to request, and how
//! each reply updates [`MixerState`].
//!
//! [`DeviceState::load_mixer`](crate::device::DeviceState::load_mixer)
//! queues the requests and applies the replies.

use crate::address::{Address, DataSize};
use crate::device_spec::{INTEGRA7, MIDI_CHANNELS};
use crate::factory_catalog;
use crate::mfx;
use crate::params::{self, part};
use crate::parse_helpers::parse_ascii_name_len;
use crate::part_settings::{self, PartSetting};
use crate::sn_acoustic::{SnaCommonField, SnaInstrument};
use crate::state::parse as state_parse;
use crate::state::{MixerState, PartState};
use crate::studio_set_settings;
use crate::svd::DRUMS_CATEGORY;

/// Size of an SN-A tone's category read: Category (Common `00 1B`) through
/// Inst Number (`00 21`), so the one RQ1 brings the tone's instrument too.
pub const SNA_CATEGORY_READ_SIZE: DataSize = DataSize::new(
    0x00,
    0x00,
    0x00,
    category_read_offset(SnaCommonField::InstNumber) as u8 + 1,
);

/// Where SN-A Common `field` sits in the reply to an SN-A tone's category
/// read, which starts at the Category.
pub const fn category_read_offset(field: SnaCommonField) -> usize {
    field as usize - SnaCommonField::Category as usize
}

/// Whether bank MSB `msb` holds SN-A tones, whose category read brings
/// the instrument too.
fn is_sna_bank(msb: u8) -> bool {
    params::tone_type_from_bank_msb(msb) == Some(params::ToneType::SnAcoustic)
}

/// A piece of mixer state read with one RQ1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixerBlock {
    /// Studio Set name.
    StudioSetName,
    /// Studio Set program number.
    StudioSetPc,
    /// System master level.
    MasterLevel,
    /// Solo part.
    SoloPart,
    /// A part's mixer settings, receive channel through output assign. It
    /// holds the Part's KBD, PITCH and OFFSET settings too.
    Part(u8),
    /// A part's settings past the mixer dump: Scale Tune, the receive
    /// filters and the Velocity Curve Type (`00 2B`-`00 43`).
    PartSettings(u8),
    /// Every part's Voice Reserve (Studio Set Common `00 18`-`00 27`).
    VoiceReserve,
    /// A part's EQ.
    PartEq(u8),
    /// Master EQ bands.
    MasterEq,
    /// Master EQ switch.
    MasterEqSwitch,
    /// Ext part level, chorus and reverb sends, and mute.
    ExtPart,
    /// Chorus type, level and output.
    ChorusCore,
    /// Chorus switch.
    ChorusSwitch,
    /// Chorus parameters.
    ChorusParams,
    /// Reverb type, level and output.
    ReverbCore,
    /// Reverb switch.
    ReverbSwitch,
    /// Reverb parameters.
    ReverbParams,
    /// Motional Surround common block.
    SurroundCommon,
    /// A part's Motional Surround position.
    PartSurround(u8),
    /// Tone Control 1–4 Source and the Studio Set Tempo.
    StudioSetControl,
    /// A MIDI channel's Phase Lock (0-15 = channel 1-16).
    PhaseLock(u8),
    /// Drum Comp+EQ switch, part and output assigns.
    DrumCompEqCommon,
    /// The Comp+EQ block of a part's drum kit, of bank MSB `msb`.
    CompEq {
        /// Part index (0-15).
        part: u8,
        /// The kit's bank MSB.
        msb: u8,
    },
    /// The name of a part's tone, of bank MSB `msb`.
    ToneName {
        /// Part index (0-15).
        part: u8,
        /// The tone's bank MSB.
        msb: u8,
    },
    /// The category of a part's tone, of bank MSB `msb`. An SN-A tone's
    /// read runs on to its instrument (Common `00 1B`-`00 21`, see
    /// [`SNA_CATEGORY_READ_SIZE`]).
    ToneCategory {
        /// Part index (0-15).
        part: u8,
        /// The tone's bank MSB.
        msb: u8,
    },
    /// The MFX Type of a part's tone, of bank MSB `msb`.
    MfxType {
        /// Part index (0-15).
        part: u8,
        /// The tone's bank MSB.
        msb: u8,
    },
}

impl MixerBlock {
    /// The blocks a mixer load starts with. Replies queue the rest (see
    /// [`apply`](Self::apply)).
    pub fn initial() -> Vec<Self> {
        use MixerBlock::*;
        let parts = |block: fn(u8) -> Self| (0..INTEGRA7.part_count).map(block);
        [StudioSetName, StudioSetPc, MasterLevel, SoloPart]
            .into_iter()
            .chain(parts(Part))
            .chain(parts(PartEq))
            .chain([
                MasterEq,
                MasterEqSwitch,
                ExtPart,
                ChorusCore,
                ChorusSwitch,
                ChorusParams,
                ReverbCore,
                ReverbSwitch,
                ReverbParams,
                SurroundCommon,
            ])
            .chain(parts(PartSurround))
            .chain([StudioSetControl])
            .chain((0..MIDI_CHANNELS).map(PhaseLock))
            .chain([DrumCompEqCommon])
            .chain(parts(PartSettings))
            .chain([VoiceReserve])
            .collect()
    }

    /// The tone name, category and MFX Type reads of `part`'s tone of bank
    /// MSB `msb`.
    pub fn tone_reads(part: u8, msb: u8) -> [Self; 3] {
        [
            Self::ToneName { part, msb },
            Self::ToneCategory { part, msb },
            Self::MfxType { part, msb },
        ]
    }

    /// A short human-readable name, e.g. for a list of blocks that failed
    /// to load.
    pub fn label(self) -> String {
        use MixerBlock::*;
        match self {
            StudioSetName => "Studio Set name".into(),
            StudioSetPc => "Studio Set number".into(),
            MasterLevel => "Master level".into(),
            SoloPart => "Solo part".into(),
            Part(p) => format!("Part {}", u32::from(p) + 1),
            PartSettings(p) => format!("Part {} settings", u32::from(p) + 1),
            VoiceReserve => "Voice Reserve".into(),
            PartEq(p) => format!("Part {} EQ", u32::from(p) + 1),
            MasterEq => "Master EQ".into(),
            MasterEqSwitch => "Master EQ switch".into(),
            ExtPart => "Ext part".into(),
            ChorusCore => "Chorus".into(),
            ChorusSwitch => "Chorus switch".into(),
            ChorusParams => "Chorus parameters".into(),
            ReverbCore => "Reverb".into(),
            ReverbSwitch => "Reverb switch".into(),
            ReverbParams => "Reverb parameters".into(),
            SurroundCommon => "Motional Surround".into(),
            PartSurround(p) => format!("Part {} surround", u32::from(p) + 1),
            StudioSetControl => "Tone Control sources and tempo".into(),
            PhaseLock(c) => format!("Channel {} Phase Lock", u32::from(c) + 1),
            DrumCompEqCommon => "Drum Comp+EQ".into(),
            CompEq { part, .. } => format!("Part {} Comp+EQ", u32::from(part) + 1),
            ToneName { part, .. } => format!("Part {} tone name", u32::from(part) + 1),
            ToneCategory { part, .. } => format!("Part {} tone category", u32::from(part) + 1),
            MfxType { part, .. } => format!("Part {} MFX type", u32::from(part) + 1),
        }
    }

    /// Whether the mixer can't be shown before this block arrives.
    pub fn is_primary(self) -> bool {
        use MixerBlock::*;
        matches!(
            self,
            StudioSetName | StudioSetPc | MasterLevel | SoloPart | Part(_)
        )
    }

    /// Whether this block reads `part`'s tone: its dump (bank and PC), its
    /// name, category, MFX Type or Comp+EQ block.
    pub fn reads_tone_of(self, part: u8) -> bool {
        match self {
            Self::Part(p) => p == part,
            Self::CompEq { part: p, .. }
            | Self::ToneName { part: p, .. }
            | Self::ToneCategory { part: p, .. }
            | Self::MfxType { part: p, .. } => p == part,
            _ => false,
        }
    }

    /// The address and size of the RQ1 that reads this block, or `None`
    /// when the device has nothing to read (e.g. a drum kit's category).
    pub fn request(self) -> Option<(Address, DataSize)> {
        use MixerBlock::*;
        Some(match self {
            StudioSetName => (params::STUDIO_SET_NAME, params::STUDIO_SET_NAME_SIZE),
            StudioSetPc => (params::SETUP_STUDIO_SET_PC, DataSize::ONE),
            MasterLevel => (params::SYSTEM_MASTER_LEVEL, DataSize::ONE),
            SoloPart => (params::SOLO_PART, DataSize::ONE),
            Part(p) => (
                params::part_address(p, part::RECEIVE_CHANNEL),
                params::PART_MIXER_SIZE,
            ),
            PartSettings(p) => (
                params::part_address(p, part_settings::SETTINGS_READ_FROM),
                part_settings::SETTINGS_READ_SIZE,
            ),
            VoiceReserve => part_settings::voice_reserve_read(),
            PartEq(p) => (
                params::part_eq_address(p, params::part_eq::SWITCH),
                params::PART_EQ_SIZE,
            ),
            MasterEq => (
                params::master_eq_address(params::master_eq::LOW_FREQ),
                params::MASTER_EQ_SIZE,
            ),
            MasterEqSwitch => (params::MASTER_EQ_SWITCH, DataSize::ONE),
            ExtPart => (params::EXT_PART_LEVEL, params::EXT_PART_SIZE),
            ChorusCore => (
                params::chorus_address(params::chorus::TYPE),
                params::CHORUS_CORE_SIZE,
            ),
            ChorusSwitch => (params::CHORUS_SWITCH, DataSize::ONE),
            ChorusParams => (
                params::chorus_address(params::chorus::PARAM_BASE),
                params::CHORUS_PARAMS_SIZE,
            ),
            ReverbCore => (
                params::reverb_address(params::reverb::TYPE),
                params::REVERB_CORE_SIZE,
            ),
            ReverbSwitch => (params::REVERB_SWITCH, DataSize::ONE),
            ReverbParams => (
                params::reverb_address(params::reverb::PARAM_BASE),
                params::REVERB_PARAMS_SIZE,
            ),
            SurroundCommon => (params::SURROUND_BASE, params::SURROUND_COMMON_SIZE),
            PartSurround(p) => (
                params::part_address(p, params::part_surround::LR),
                params::PART_SURROUND_SIZE,
            ),
            StudioSetControl => (
                params::TONE_CONTROL_1_SOURCE,
                params::STUDIO_SET_CONTROL_SIZE,
            ),
            PhaseLock(c) => (
                params::studio_set_midi_address(c, params::studio_set_midi::PHASE_LOCK),
                DataSize::ONE,
            ),
            DrumCompEqCommon => (
                params::DRUM_COMP_EQ_SWITCH,
                params::DRUM_COMP_EQ_COMMON_SIZE,
            ),
            CompEq { part, msb } => (
                params::comp_eq_block_address_for_bank(part, msb)?,
                params::COMP_EQ_BLOCK_SIZE,
            ),
            ToneName { part, msb } => (
                params::tone_name_address(part, params::tone_type_from_bank_msb(msb)?),
                params::TONE_NAME_SIZE,
            ),
            ToneCategory { part, msb } => (
                params::tone_category_address(part, msb)?,
                if is_sna_bank(msb) {
                    SNA_CATEGORY_READ_SIZE
                } else {
                    DataSize::ONE
                },
            ),
            // The device doesn't answer reads of a GM2 or ExPCM tone.
            MfxType { part, msb } => (
                mfx::mfx_address(
                    part,
                    params::is_editable_bank(msb)
                        .then(|| params::tone_type_from_bank_msb(msb))
                        .flatten()?,
                    mfx::mfx_header_offset(mfx::MfxHeaderField::Type, 0).ok()?,
                ),
                DataSize::ONE,
            ),
        })
    }

    /// Update `state` from this block's reply `data`, which must be the
    /// requested size. Returns the blocks the reply makes readable.
    pub fn apply(self, state: &mut MixerState, data: &[u8]) -> Vec<Self> {
        use MixerBlock::*;
        let byte = data[0];
        match self {
            StudioSetName => state.studio_set_name = parse_ascii_name_len(data, data.len()),
            StudioSetPc => state.studio_set_pc = byte,
            MasterLevel => state.master_level = byte,
            SoloPart => state.solo_part = byte,
            Part(p) => {
                let parsed = state_parse::parse_part_dump(data);
                let s = &mut state.parts[usize::from(p)];
                set_tone(
                    s,
                    parsed.tone_bank_msb,
                    parsed.tone_bank_lsb,
                    parsed.tone_pc,
                );
                s.receive_channel = parsed.receive_channel;
                s.level = parsed.level;
                s.pan = parsed.pan;
                s.muted = parsed.muted;
                s.chorus_send = parsed.chorus_send;
                s.reverb_send = parsed.reverb_send;
                s.output_assign = parsed.output_assign;
                s.settings.apply_dump(0, data);
                return Self::tone_reads(p, parsed.tone_bank_msb).to_vec();
            }
            PartSettings(p) => state.parts[usize::from(p)]
                .settings
                .apply_dump(part_settings::SETTINGS_READ_FROM[2], data),
            VoiceReserve => {
                for (part, reserve) in state.parts.iter_mut().zip(data) {
                    part.settings.set(PartSetting::VoiceReserve, *reserve);
                }
            }
            PartEq(p) => state.parts[usize::from(p)].eq = state_parse::parse_part_eq_dump(data),
            MasterEq => {
                let enabled = state.master_eq.enabled;
                state.master_eq = state_parse::parse_master_eq_dump(data);
                state.master_eq.enabled = enabled;
            }
            MasterEqSwitch => state.master_eq.enabled = byte == 1,
            ExtPart => {
                // The byte at `address` in the block read from the level.
                let at = |address: Address| {
                    data[(address.value() - params::EXT_PART_LEVEL.value()) as usize]
                };
                state.ext_level = at(params::EXT_PART_LEVEL);
                state.ext_chorus_send = at(params::EXT_PART_CHORUS_SEND);
                state.ext_reverb_send = at(params::EXT_PART_REVERB_SEND);
                state.ext_muted = at(params::EXT_PART_MUTE) == 1;
            }
            ChorusCore => {
                state.chorus.fx_type = data[usize::from(params::chorus::TYPE)];
                state.chorus.level = data[usize::from(params::chorus::LEVEL)];
                state.chorus.output_assign = data[usize::from(params::chorus::OUTPUT_ASSIGN)];
                state.chorus.output = data[usize::from(params::chorus::OUTPUT_SELECT)];
            }
            ChorusSwitch => state.chorus.enabled = byte == 1,
            ChorusParams => {
                state.chorus.params =
                    state_parse::decode_nib_params(data, params::CHORUS_PARAM_COUNT);
            }
            ReverbCore => {
                state.reverb.fx_type = data[usize::from(params::reverb::TYPE)];
                state.reverb.level = data[usize::from(params::reverb::LEVEL)];
                state.reverb.output = data[usize::from(params::reverb::OUTPUT_ASSIGN)];
            }
            ReverbSwitch => state.reverb.enabled = byte == 1,
            ReverbParams => {
                state.reverb.params =
                    state_parse::decode_nib_params(data, params::REVERB_PARAM_COUNT);
            }
            SurroundCommon => {
                let parts = state.surround.parts.clone();
                state.surround = state_parse::parse_surround_common(data);
                state.surround.parts = parts;
            }
            PartSurround(p) => {
                // L-R, (reserved), F-B, (reserved), Width, Ambience Send.
                state.surround.parts[usize::from(p)] =
                    state_parse::parse_surround_part(data[0], data[2], data[4], data[5]);
            }
            StudioSetControl => {
                studio_set_settings::apply_control_block(&mut state.studio_set_settings, data);
            }
            PhaseLock(c) => state.studio_set_settings.phase_lock[usize::from(c)] = byte == 1,
            DrumCompEqCommon => {
                let dce = &mut state.drum_comp_eq;
                dce.enabled = byte == 1;
                dce.part = data[1];
                dce.output_assigns.copy_from_slice(&data[2..8]);
                // Only the Comp+EQ part's drum kit holds the units.
                if let (true, Some(kit)) = (dce.enabled, state.parts.get(usize::from(dce.part))) {
                    return vec![CompEq {
                        part: dce.part,
                        msb: kit.tone_bank_msb,
                    }];
                }
            }
            CompEq { .. } => state.drum_comp_eq.units = state_parse::parse_comp_eq_block(data),
            ToneName { part, .. } => {
                let name = parse_ascii_name_len(data, data.len());
                if !name.is_empty() {
                    state.parts[usize::from(part)].tone_name = name;
                }
            }
            ToneCategory { part, msb } => {
                let s = &mut state.parts[usize::from(part)];
                s.tone_category = byte;
                if is_sna_bank(msb) {
                    let at = |field| data[category_read_offset(field)];
                    s.sna_instrument = Some(SnaInstrument {
                        variation: at(SnaCommonField::InstVariation),
                        number: at(SnaCommonField::InstNumber),
                    });
                }
            }
            MfxType { part, .. } => state.parts[usize::from(part)].mfx_type = Some(byte),
        }
        Vec::new()
    }
}

/// Set `part`'s tone, with the factory name and the drum kit category until
/// the device's own are read (see [`MixerBlock::tone_reads`]). The new tone
/// has its own MFX, unread.
pub(crate) fn set_tone(part: &mut PartState, msb: u8, lsb: u8, pc: u8) {
    part.mfx_type = None;
    part.sna_instrument = None;
    part.tone_bank_msb = msb;
    part.tone_bank_lsb = lsb;
    part.tone_pc = pc;
    part.tone_name = factory_catalog::factory_tone(msb, lsb, pc)
        .map(|t| t.name.to_string())
        .unwrap_or_default();
    part.tone_category = if params::is_drum_kit_bank(msb) {
        DRUMS_CATEGORY
    } else {
        0
    };
}
