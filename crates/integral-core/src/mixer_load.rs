//! Reading the mixer state from the device: the blocks to request, and how
//! each reply updates [`MixerState`].
//!
//! [`DeviceState::load_mixer`](crate::device::DeviceState::load_mixer)
//! queues the requests and applies the replies.

use crate::address::{Address, DataSize};
use crate::device_spec::INTEGRA7;
use crate::factory_catalog;
use crate::params::{self, part};
use crate::parse_helpers::parse_ascii_name_len;
use crate::state::parse as state_parse;
use crate::state::{MixerState, PartState};
use crate::svd::DRUMS_CATEGORY;

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
    /// A part's mixer settings, receive channel through output assign.
    Part(u8),
    /// A part's EQ.
    PartEq(u8),
    /// Master EQ bands.
    MasterEq,
    /// Master EQ switch.
    MasterEqSwitch,
    /// Ext part level.
    ExtLevel,
    /// Ext part mute.
    ExtMute,
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
    /// The category of a part's tone, of bank MSB `msb`.
    ToneCategory {
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
                ExtLevel,
                ExtMute,
                ChorusCore,
                ChorusSwitch,
                ChorusParams,
                ReverbCore,
                ReverbSwitch,
                ReverbParams,
                SurroundCommon,
            ])
            .chain(parts(PartSurround))
            .chain([DrumCompEqCommon])
            .collect()
    }

    /// The tone name and category reads of `part`'s tone of bank MSB `msb`.
    pub fn tone_reads(part: u8, msb: u8) -> [Self; 2] {
        [
            Self::ToneName { part, msb },
            Self::ToneCategory { part, msb },
        ]
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
    /// name, category or Comp+EQ block.
    pub fn reads_tone_of(self, part: u8) -> bool {
        match self {
            Self::Part(p) => p == part,
            Self::CompEq { part: p, .. }
            | Self::ToneName { part: p, .. }
            | Self::ToneCategory { part: p, .. } => p == part,
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
            PartEq(p) => (
                params::part_eq_address(p, params::part_eq::SWITCH),
                params::PART_EQ_SIZE,
            ),
            MasterEq => (
                params::master_eq_address(params::master_eq::LOW_FREQ),
                params::MASTER_EQ_SIZE,
            ),
            MasterEqSwitch => (params::MASTER_EQ_SWITCH, DataSize::ONE),
            ExtLevel => (params::EXT_PART_LEVEL, DataSize::ONE),
            ExtMute => (params::EXT_PART_MUTE, DataSize::ONE),
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
            ToneCategory { part, msb } => {
                (params::tone_category_address(part, msb)?, DataSize::ONE)
            }
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
                return Self::tone_reads(p, parsed.tone_bank_msb).to_vec();
            }
            PartEq(p) => state.parts[usize::from(p)].eq = state_parse::parse_part_eq_dump(data),
            MasterEq => {
                let enabled = state.master_eq.enabled;
                state.master_eq = state_parse::parse_master_eq_dump(data);
                state.master_eq.enabled = enabled;
            }
            MasterEqSwitch => state.master_eq.enabled = byte == 1,
            ExtLevel => state.ext_level = byte,
            ExtMute => state.ext_muted = byte == 1,
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
            ToneCategory { part, .. } => state.parts[usize::from(part)].tone_category = byte,
        }
        Vec::new()
    }
}

/// Set `part`'s tone, with the factory name and the drum kit category until
/// the device's own are read (see [`MixerBlock::tone_reads`]).
pub(crate) fn set_tone(part: &mut PartState, msb: u8, lsb: u8, pc: u8) {
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
