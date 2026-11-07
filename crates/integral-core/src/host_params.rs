//! The parameter lists the DAW plugins hand to their host.
//!
//! One list per plugin kind (ADR 0007): the Studio Set plugin, a Tone plugin
//! per synth tone type, and a Kit plugin per drum kit type. A list is the
//! ordered [`HostParam`]s the plugin declares to the host, each with its ID,
//! name, group, raw range and value text. The lists carry no host types, so
//! the plugin crate turns them into its framework's parameters and core
//! still builds for `wasm32-unknown-unknown`.
//!
//! # IDs
//!
//! A host saves automation against the ID string, so an ID comes from where
//! the parameter lives on the device and never from a name (ADR 0008). It
//! is `<block>/<offset>`, the offset being the parameter's **linear byte
//! position** in its block (the field enum's discriminant), not the 7-bit
//! address notation of the MIDI Implementation: `0x80` is the byte after
//! `0x7F`, where the doc writes `01 00`. A parameter holding several values
//! adds `.<index>` (0-based).
//!
//! - **A tone block field:** `pcms/partial2/0x2A`, `sns/common/0x0E`. A
//!   drum key's field is under the key block (`pcmd/key/0x0E`) and carries
//!   no key number; a Comp+EQ unit's setting is `pcmd/compeq/0x1C`.
//! - **An MFX, Chorus or Reverb type parameter:** `mfx/<type>/<slot>`, the
//!   type's ID and the parameter's slot, and likewise `chorus/..` and
//!   `reverb/..`. The MFX header (type, sends, controls) is `mfx/head/<offset>`.
//! - **A Studio Set parameter:** the same form, the block being one of the
//!   Studio Set's ([`params::studio_set_blocks`]): `part07/0x09`,
//!   `part07/eq/0x00`, `common/0x3F`, `chorus/head/0x00`, `reverb/head/0x00`,
//!   `surround/0x0C`, `mastereq/0x05`. The Part is in the ID.
//!
//! A Tone or Kit ID carries no Part number, so a re-bind keeps automation.
//!
//! # Order
//!
//! By device address: each list is sorted by the address of its parameters
//! in the Temporary area (or the Studio Set), so the blocks come in the
//! order the device lays them out (Common, MFX, PMT, Partials, Common 2),
//! and an effect's types keep their type order, whichever type is active.
//! The order is the host's list order, so a snapshot test guards it.
//!
//! # Ranges
//!
//! The range is the raw one the device takes: a Chorus, Reverb or MFX
//! parameter's is its stored slot value (the raw value less 32768).
//!
//! # Tempo sync
//!
//! A tempo-synced effect parameter spans three slots under one name: a
//! switch (OFF reads the value in its own unit, ON the note), the value and
//! the note. All three are host parameters (`<value name> Tempo Sync`, the
//! value, `<value name> Note`), at their own slots.
//!
//! # Left out
//!
//! The Master Level lives in the System area, not the Studio Set, and ADR
//! 0007 gives the Studio Set plugin the Master EQ only.

use crate::address::Address;
use crate::block_field::BlockField;
use crate::device_spec::INTEGRA7;
use crate::field_spec::{
    FieldSpec, KEY_PREFIX, mfx_header_spec, partial_prefix, pcmd_common_spec, pcmd_common2_spec,
    pcmd_partial_spec, pcmd_wmt_spec, pcms_common_spec, pcms_common2_spec, pcms_partial_spec,
    pcms_pmt_spec, qualified_name, sna_common_spec, snd_common_spec, snd_note_spec,
    sns_common_spec, sns_partial_spec,
};
use crate::fx_params::{
    CHORUS_LABEL, CHORUS_TYPE_HOST_NAMES, FxParamDef, FxUnit, MFX_LABEL, REVERB_LABEL,
    REVERB_TYPE_HOST_NAMES, chorus_params, reverb_params,
};
use crate::mfx::{self, MFX_PARAMS_START, MfxHeaderField, mfx_header_offset};
use crate::mfx_params::{MFX_TYPE_NAMES, mfx_type_def};
use crate::param_registry::{
    CompEqKit, MixerScope, ParamFormat, ParamGroup, mixer_field_address, mixer_param,
    studio_set_scopes,
};
use crate::params::{self, ToneType, studio_set_block_offset, studio_set_blocks};
use crate::pcm_drum::{self, PcmdCommon2Field, PcmdCommonField, PcmdPartialField, PcmdWmtField};
use crate::pcm_synth::{self, PcmsCommon2Field, PcmsCommonField, PcmsPartialField, PcmsPmtField};
use crate::sn_acoustic::{self, SnaCommonField};
use crate::sn_drum::{self, SndCommonField, SndNoteField};
use crate::sn_synth::{self, SnsCommonField, SnsPartialField};

/// ID block of the MFX parameters.
const MFX_ID: &str = "mfx";
/// ID block of the MFX header (type, sends, controls), within the MFX.
const HEADER_ID: &str = "head";
/// ID block of the Chorus type parameters.
const CHORUS_ID: &str = "chorus";
/// ID block of the Reverb type parameters.
const REVERB_ID: &str = "reverb";
/// ID block of a drum key's fields, within a kit.
const KEY_ID: &str = "key";
/// ID block of a Drum Comp+EQ unit's settings, within a kit.
const COMP_EQ_ID: &str = "compeq";

/// Group of a block's fields that no prefix names.
const COMMON_GROUP: &str = "Common";
/// Group of the PMT fields.
const PMT_GROUP: &str = "PMT";
/// Group of the Common 2 fields.
const COMMON2_GROUP: &str = "Common 2";

/// The plugins of the family (ADR 0007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    /// What the whole Studio Set shares.
    StudioSet,
    /// A Tone plugin for SN Synth tones.
    SnSynth,
    /// A Tone plugin for SN Acoustic tones.
    SnAcoustic,
    /// A Tone plugin for PCM Synth tones.
    PcmSynth,
    /// A Kit plugin for SN Drum kits.
    SnDrum,
    /// A Kit plugin for PCM Drum kits.
    PcmDrum,
}

impl PluginKind {
    /// Every plugin of the family.
    pub const ALL: [Self; 6] = [
        Self::StudioSet,
        Self::SnSynth,
        Self::SnAcoustic,
        Self::PcmSynth,
        Self::SnDrum,
        Self::PcmDrum,
    ];

    /// The tone type a Part must hold for the plugin to act on it, or
    /// `None` for the Studio Set plugin, which acts on any.
    pub const fn tone_type(self) -> Option<ToneType> {
        match self {
            Self::StudioSet => None,
            Self::SnSynth => Some(ToneType::SnSynth),
            Self::SnAcoustic => Some(ToneType::SnAcoustic),
            Self::PcmSynth => Some(ToneType::PcmSynth),
            Self::SnDrum => Some(ToneType::SnDrum),
            Self::PcmDrum => Some(ToneType::PcmDrum),
        }
    }

    /// The plugin's host parameter list.
    pub fn host_params(self) -> Vec<HostParam> {
        match self {
            Self::StudioSet => studio_set_host_params(),
            Self::SnSynth => sn_synth_host_params(),
            Self::SnAcoustic => sn_acoustic_host_params(),
            Self::PcmSynth => pcm_synth_host_params(),
            Self::SnDrum => sn_drum_host_params(),
            Self::PcmDrum => pcm_drum_host_params(),
        }
    }
}

/// How a host parameter's raw value reads as text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostFormat {
    /// A tone, kit or Studio Set field's format.
    Field(ParamFormat),
    /// A Chorus, Reverb or MFX parameter's unit.
    Fx(FxUnit),
}

impl HostFormat {
    /// The text of raw value `raw`.
    pub fn format(&self, raw: i32) -> String {
        match self {
            Self::Field(format) => format.format(raw),
            Self::Fx(unit) => unit.format(raw),
        }
    }
}

/// One parameter of a plugin's host list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostParam {
    /// The ID the host saves automation against (see the module docs).
    pub id: String,
    /// The name in the host's list.
    pub name: String,
    /// The group the host lists it under: a `/`-separated path (`Partial 2`,
    /// `MFX/Phaser 1`), the form nih-plug's `param_map` takes. No segment
    /// holds a `/`.
    pub group: String,
    /// Minimum raw value (inclusive).
    pub min: i32,
    /// Maximum raw value (inclusive).
    pub max: i32,
    /// Default raw value.
    pub default: i32,
    /// How a raw value reads as text.
    pub format: HostFormat,
}

/// A list as its snapshot text: one `id<TAB>min<TAB>max` line per
/// parameter, which the checked-in snapshots hold so a change shows in
/// review (ADR 0008).
pub fn snapshot(list: &[HostParam]) -> String {
    list.iter()
        .map(|p| format!("{}\t{}\t{}\n", p.id, p.min, p.max))
        .collect()
}

/// A host parameter with the device address that orders it.
type Entry = (Address, HostParam);

/// Sort entries by address and drop the addresses. The sort is stable, so
/// entries at one address (every type of an effect) keep their order.
fn ordered(mut entries: Vec<Entry>) -> Vec<HostParam> {
    entries.sort_by_key(|(address, _)| address.value());
    entries.into_iter().map(|(_, param)| param).collect()
}

/// The address of byte `position` of the block at `base`.
fn at(base: Address, position: usize) -> Address {
    let position = u8::try_from(position).expect("a block position fits a byte");
    base.offset([0, 0, 0, position])
}

/// A group path from its segments. A `/` inside a segment (`Drum Comp/EQ`,
/// `OD/DS -> TouchWah`) would open a sub-group, so it becomes a `-`.
fn group_path(segments: &[&str]) -> String {
    segments
        .iter()
        .map(|segment| segment.replace('/', "-"))
        .collect::<Vec<_>>()
        .join("/")
}

/// A block of tone fields: where it starts, how its parameters are named
/// and grouped, and its ID.
struct Block<'a> {
    base: Address,
    id: &'a str,
    group: &'a str,
    /// The name prefix (empty for none).
    prefix: &'a str,
}

/// The ID of value `index` of a parameter at linear byte `offset` in
/// `block`: the offset, then the index for a parameter holding several
/// values.
fn offset_id(block: &str, offset: usize, index: Option<usize>) -> String {
    match index {
        Some(index) => format!("{block}/{offset:#04X}.{index}"),
        None => format!("{block}/{offset:#04X}"),
    }
}

/// Add every value of `fields`, in `block`.
fn add_fields<F: BlockField>(
    entries: &mut Vec<Entry>,
    block: &Block,
    fields: &[F],
    spec_of: impl Fn(F) -> FieldSpec,
) {
    for &field in fields {
        let spec = spec_of(field);
        for index in 0..field.count() {
            let address = at(block.base, field.position() + index * field.stride());
            entries.push((
                address,
                HostParam {
                    id: offset_id(
                        block.id,
                        field.position(),
                        (field.count() > 1).then_some(index),
                    ),
                    name: qualified_name(block.prefix, &spec.element_name(index)),
                    group: group_path(&[block.group]),
                    min: spec.min,
                    max: spec.max,
                    default: spec.default,
                    format: HostFormat::Field(spec.format.clone()),
                },
            ));
        }
    }
}

/// One slot of an effect type, as a host parameter.
fn effect_param(
    id_block: &str,
    type_id: u8,
    slot: u8,
    name: String,
    group: &str,
    def: &FxParamDef,
) -> HostParam {
    HostParam {
        id: format!("{id_block}/{type_id}/{slot}"),
        name,
        group: group.to_string(),
        min: def.min,
        max: def.max,
        default: def.default_value,
        format: HostFormat::Fx(def.unit),
    }
}

/// Add one effect type's parameters, `defs`, under `id_block`'s IDs. A
/// tempo-synced one adds its switch and its note, each at its own slot.
fn add_effect_type(
    entries: &mut Vec<Entry>,
    address: Address,
    (id_block, label): (&str, &str),
    type_id: u8,
    type_name: &str,
    defs: &[FxParamDef],
) {
    let group = group_path(&[label, type_name]);
    for def in defs {
        let name = def.qualified_name(label, type_name);
        let sync = def.sync.zip(def.note_param());
        let param =
            |slot, name, def: &FxParamDef| effect_param(id_block, type_id, slot, name, &group, def);
        if let Some((sync, _)) = &sync {
            entries.push((
                address,
                HostParam {
                    min: 0,
                    max: 1,
                    default: sync.switch_default,
                    format: HostFormat::Field(ParamFormat::OnOff),
                    ..param(sync.switch_index, format!("{name} Tempo Sync"), def)
                },
            ));
        }
        entries.push((address, param(def.index, name.clone(), def)));
        if let Some((sync, note)) = &sync {
            let note_name = format!("{name} Note");
            entries.push((address, param(sync.note_index, note_name, note)));
        }
    }
}

/// Add the MFX block of a tone type's tone: the header, then every type's
/// parameters, the active type or not.
fn add_mfx(entries: &mut Vec<Entry>, tone_type: ToneType) {
    let base = mfx::tone_mfx_block_address(0, tone_type);
    let block = format!("{MFX_ID}/{HEADER_ID}");
    for &field in MfxHeaderField::ALL {
        let spec = mfx_header_spec(field);
        for index in 0..field.count() {
            let offset = mfx_header_offset(field, u8::try_from(index).expect("few controls"))
                .expect("index below count");
            entries.push((
                at(base, usize::from(offset)),
                HostParam {
                    id: offset_id(
                        &block,
                        field.position(),
                        (field.count() > 1).then_some(index),
                    ),
                    name: spec.element_name(index),
                    group: MFX_LABEL.to_string(),
                    min: spec.min,
                    max: spec.max,
                    default: spec.default,
                    format: HostFormat::Field(spec.format.clone()),
                },
            ));
        }
    }
    let params_at = at(base, usize::from(MFX_PARAMS_START));
    for type_id in 0..MFX_TYPE_NAMES.len() {
        let type_id = u8::try_from(type_id).expect("few MFX types");
        let def = mfx_type_def(type_id).expect("every named type has a definition");
        let label = (MFX_ID, MFX_LABEL);
        add_effect_type(entries, params_at, label, type_id, def.name, def.params);
    }
}

/// Add a drum kit's Drum Comp+EQ units' own settings (a unit's Output
/// Assign is the Studio Set's). The settings sit in the kit, so a Kit
/// plugin bound to the whole kit owns them.
fn add_comp_eq(entries: &mut Vec<Entry>, kit_id: &str, tone_type: ToneType) {
    let kit = CompEqKit { part: 0, tone_type };
    let base = params::comp_eq_block_address(kit.part, kit.tone_type);
    for unit in 0..INTEGRA7.comp_eq_unit_count {
        let scope = MixerScope::CompEqUnit { kit, unit };
        let prefix = scope.host_prefix();
        for field in scope.kind().kit_fields() {
            let address = mixer_field_address(field, scope).expect("field is in the unit scope");
            let def = mixer_param(field);
            let offset = (address.value() - base.value()) as usize;
            entries.push((
                address,
                HostParam {
                    id: offset_id(&format!("{kit_id}/{COMP_EQ_ID}"), offset, None),
                    name: qualified_name(&prefix, def.host_name),
                    group: group_path(&[&prefix]),
                    min: def.min,
                    max: def.max,
                    default: def.default,
                    format: HostFormat::Field(def.format),
                },
            ));
        }
    }
}

/// The Studio Set plugin's list: every Part's mixer block and EQ, Ext, the
/// Master EQ, the Studio Set common block (with Motional Surround) and the
/// Drum Comp+EQ outputs ([`MixerScopeKind::studio_set_fields`]), then
/// Chorus and Reverb per type.
///
/// [`MixerScopeKind::studio_set_fields`]: crate::param_registry::MixerScopeKind::studio_set_fields
fn studio_set_host_params() -> Vec<HostParam> {
    let blocks = studio_set_blocks(INTEGRA7.part_count);
    let mut entries = Vec::new();
    for scope in studio_set_scopes() {
        let prefix = scope.host_prefix();
        for field in scope.kind().studio_set_fields() {
            let address = mixer_field_address(field, scope).expect("field is in the scope");
            let (block, offset) =
                studio_set_block_offset(&blocks, address).expect("a Studio Set address");
            let def = mixer_param(field);
            // The Chorus and Reverb settings in the common block group with
            // their effect's type parameters.
            let group = match def.group {
                ParamGroup::Chorus => CHORUS_LABEL,
                ParamGroup::Reverb => REVERB_LABEL,
                _ if prefix.is_empty() => COMMON_GROUP,
                _ => &prefix,
            };
            entries.push((
                address,
                HostParam {
                    id: offset_id(&block.id, offset as usize, None),
                    name: qualified_name(&prefix, def.host_name),
                    group: group_path(&[group]),
                    min: def.min,
                    max: def.max,
                    default: def.default,
                    format: HostFormat::Field(def.format),
                },
            ));
        }
    }
    let types = |names: &'static [&'static str]| {
        names
            .iter()
            .enumerate()
            .map(|(id, name)| (u8::try_from(id).expect("few types"), *name))
    };
    let chorus_at = params::chorus_address(params::chorus::PARAM_BASE);
    for (type_id, type_name) in types(CHORUS_TYPE_HOST_NAMES) {
        let label = (CHORUS_ID, CHORUS_LABEL);
        add_effect_type(
            &mut entries,
            chorus_at,
            label,
            type_id,
            type_name,
            chorus_params(type_id),
        );
    }
    let reverb_at = params::reverb_address(params::reverb::PARAM_BASE);
    for (type_id, type_name) in types(REVERB_TYPE_HOST_NAMES) {
        let label = (REVERB_ID, REVERB_LABEL);
        add_effect_type(
            &mut entries,
            reverb_at,
            label,
            type_id,
            type_name,
            reverb_params(type_id),
        );
    }
    ordered(entries)
}

/// The SN Synth Tone plugin's list: Common, Partials 1 to 3 and MFX.
fn sn_synth_host_params() -> Vec<HostParam> {
    let mut entries = Vec::new();
    let common = Block {
        base: sn_synth::sns_common_address(0),
        id: "sns/common",
        group: COMMON_GROUP,
        prefix: "",
    };
    add_fields(&mut entries, &common, SnsCommonField::ALL, sns_common_spec);
    for partial in 0..INTEGRA7.sn_synth_partial_count {
        let prefix = partial_prefix(usize::from(partial));
        let block = Block {
            base: sn_synth::sns_partial_address(0, partial),
            id: &format!("sns/partial{}", partial + 1),
            group: &prefix,
            prefix: &prefix,
        };
        add_fields(&mut entries, &block, SnsPartialField::ALL, sns_partial_spec);
    }
    add_mfx(&mut entries, ToneType::SnSynth);
    ordered(entries)
}

/// The SN Acoustic Tone plugin's list: Common and MFX.
fn sn_acoustic_host_params() -> Vec<HostParam> {
    let mut entries = Vec::new();
    let common = Block {
        base: sn_acoustic::sna_common_address(0),
        id: "sna/common",
        group: COMMON_GROUP,
        prefix: "",
    };
    add_fields(&mut entries, &common, SnaCommonField::ALL, sna_common_spec);
    add_mfx(&mut entries, ToneType::SnAcoustic);
    ordered(entries)
}

/// The PCM Synth Tone plugin's list: Common, PMT, Partials 1 to 4, Common 2
/// and MFX.
fn pcm_synth_host_params() -> Vec<HostParam> {
    let mut entries = Vec::new();
    let block = |base, id, group| Block {
        base,
        id,
        group,
        prefix: "",
    };
    let common = block(
        pcm_synth::pcms_common_address(0),
        "pcms/common",
        COMMON_GROUP,
    );
    add_fields(
        &mut entries,
        &common,
        PcmsCommonField::ALL,
        pcms_common_spec,
    );
    let pmt = block(pcm_synth::pcms_pmt_address(0), "pcms/pmt", PMT_GROUP);
    add_fields(&mut entries, &pmt, PcmsPmtField::ALL, pcms_pmt_spec);
    for partial in 0..INTEGRA7.pcm_synth_partial_count {
        let prefix = partial_prefix(usize::from(partial));
        let partial_block = Block {
            base: pcm_synth::pcms_partial_address(0, partial),
            id: &format!("pcms/partial{}", partial + 1),
            group: &prefix,
            prefix: &prefix,
        };
        add_fields(
            &mut entries,
            &partial_block,
            PcmsPartialField::ALL,
            pcms_partial_spec,
        );
    }
    let common2 = block(
        pcm_synth::pcms_common2_address(0),
        "pcms/common2",
        COMMON2_GROUP,
    );
    add_fields(
        &mut entries,
        &common2,
        PcmsCommon2Field::ALL,
        pcms_common2_spec,
    );
    add_mfx(&mut entries, ToneType::PcmSynth);
    ordered(entries)
}

/// The SN Drum Kit plugin's list: the kit-wide parameters (Common, MFX and
/// the Comp+EQ units) and one key's. A plugin bound to a key leaves the
/// kit-wide parameters inert, and one bound to the kit the key's.
fn sn_drum_host_params() -> Vec<HostParam> {
    let mut entries = Vec::new();
    let common = Block {
        base: sn_drum::snd_common_address(0),
        id: "snd/common",
        group: COMMON_GROUP,
        prefix: "",
    };
    add_fields(&mut entries, &common, SndCommonField::ALL, snd_common_spec);
    add_mfx(&mut entries, ToneType::SnDrum);
    add_comp_eq(&mut entries, "snd", ToneType::SnDrum);
    let key = Block {
        base: sn_drum::snd_note_address(0, INTEGRA7.sn_drum_keys.first)
            .expect("first key is in range"),
        id: &format!("snd/{KEY_ID}"),
        group: KEY_PREFIX,
        prefix: KEY_PREFIX,
    };
    add_fields(&mut entries, &key, SndNoteField::ALL, snd_note_spec);
    ordered(entries)
}

/// The PCM Drum Kit plugin's list: the kit-wide parameters (Common, Common
/// 2, MFX and the Comp+EQ units) and one key's, its WMT layers included. A
/// plugin bound to a key leaves the kit-wide parameters inert, and one
/// bound to the kit the key's.
fn pcm_drum_host_params() -> Vec<HostParam> {
    let mut entries = Vec::new();
    let block = |base, id, group| Block {
        base,
        id,
        group,
        prefix: "",
    };
    let common = block(
        pcm_drum::pcmd_common_address(0),
        "pcmd/common",
        COMMON_GROUP,
    );
    add_fields(
        &mut entries,
        &common,
        PcmdCommonField::ALL,
        pcmd_common_spec,
    );
    let common2 = block(
        pcm_drum::pcmd_common2_address(0),
        "pcmd/common2",
        COMMON2_GROUP,
    );
    add_fields(
        &mut entries,
        &common2,
        PcmdCommon2Field::ALL,
        pcmd_common2_spec,
    );
    add_mfx(&mut entries, ToneType::PcmDrum);
    add_comp_eq(&mut entries, "pcmd", ToneType::PcmDrum);
    let key = Block {
        base: pcm_drum::pcmd_partial_address(0, INTEGRA7.pcm_drum_keys.first)
            .expect("first key is in range"),
        id: &format!("pcmd/{KEY_ID}"),
        group: KEY_PREFIX,
        prefix: KEY_PREFIX,
    };
    add_fields(&mut entries, &key, PcmdPartialField::ALL, pcmd_partial_spec);
    add_fields(&mut entries, &key, PcmdWmtField::ALL, pcmd_wmt_spec);
    ordered(entries)
}
