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

use std::collections::HashMap;

use crate::address::{Address, DataSize};
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
    CompEqKit, MixerField, MixerScope, ParamFormat, ParamGroup, mixer_field_address, mixer_param,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
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

    /// The raw value in `min..=max` that `text` reads as: the inverse of
    /// [`format`](Self::format), for a host that takes typed values.
    ///
    /// A plain number format reads the number. The other formats are too
    /// varied (names, tempo-synced notes, tables, piecewise scales) for a
    /// grammar each, so this searches the range, which has up to 65,536
    /// values, formatting each as it goes, in two passes. The lowest raw
    /// value wins a tie in either.
    ///
    /// 1. The text equals a value's text, ignoring case and whitespace
    ///    (`l 30` reads as `L30`, `1.5 s` as `1.5s`).
    /// 2. The text's leading number equals a value's leading number, and
    ///    its unit, if it gives one, is that value's (`+3` reads as
    ///    `+3 dB`, `50` as `50%`, `3dB` as `+3 dB` too).
    ///
    /// Returns `None` if nothing matches.
    pub fn parse(&self, text: &str, min: i32, max: i32) -> Option<i32> {
        if matches!(
            self,
            Self::Field(ParamFormat::Raw) | Self::Fx(FxUnit::Plain)
        ) {
            return text
                .trim()
                .parse::<i32>()
                .ok()
                .filter(|raw| (min..=max).contains(raw));
        }
        let wanted = squash(text);
        if wanted.is_empty() {
            return None;
        }
        if let Some(raw) = (min..=max).find(|&v| squash(&self.format(v)) == wanted) {
            return Some(raw);
        }
        let (number, unit) = split_number(&wanted)?;
        (min..=max).find(|&v| {
            let formatted = squash(&self.format(v));
            split_number(&formatted)
                .is_some_and(|(n, u)| n == number && (unit.is_empty() || u == unit))
        })
    }
}

/// `text` without whitespace, lower-cased: the form two texts are compared in.
fn squash(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The number `text` starts with (an optional sign, digits and one decimal
/// point) and the rest of it, or `None` if it starts with no number.
fn split_number(text: &str) -> Option<(f64, &str)> {
    let mut end = 0;
    let mut digits = false;
    let mut point = false;
    for (i, c) in text.char_indices() {
        let take = match c {
            '+' | '-' => i == 0,
            '.' if !point => {
                point = true;
                true
            }
            _ if c.is_ascii_digit() => {
                digits = true;
                true
            }
            _ => false,
        };
        if !take {
            break;
        }
        end = i + c.len_utf8();
    }
    if !digits {
        return None;
    }
    let number = text[..end].parse().ok()?;
    Some((number, &text[end..]))
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
    /// Where the parameter is on the device for Part 1 (and the first key,
    /// for a key's parameter): [`HostParam::device_address`] moves it to a
    /// [`Binding`].
    pub address: Address,
    /// How the address moves with the Binding.
    pub anchor: Anchor,
    /// Bytes the value takes on the device: 1, or its nibble count.
    pub width: usize,
    /// Added to the raw value to get the stored one (32768 for an effect
    /// slot, whose signed value is stored offset).
    pub bias: i32,
    /// For an effect type's parameter: which effect, and which type, it
    /// belongs to. The slot does something only while the effect has that
    /// type.
    pub effect: Option<EffectType>,
}

/// The effects whose slots change meaning with their type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    /// A tone's MFX.
    Mfx,
    /// The Studio Set's Chorus.
    Chorus,
    /// The Studio Set's Reverb.
    Reverb,
}

/// Which type of an effect a parameter is for, and where the effect keeps
/// its type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectType {
    /// The effect.
    pub kind: EffectKind,
    /// The type the parameter is for.
    pub type_id: u8,
    /// Where the effect's type byte is, for Part 1 if the parameter's
    /// anchor is a Part ([`HostParam::effect_type_address`]).
    pub type_address: Address,
}

/// How a parameter's address moves with the [`Binding`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Fixed in the Studio Set; the ID holds any Part.
    Absolute,
    /// In the Temporary Tone of the bound Part.
    Part,
    /// In the bound key's block of the bound Part's kit.
    Key(ToneType),
}

/// What a plugin instance is bound to: its Part and, for a Kit plugin bound
/// to a key, the key. The Studio Set plugin's IDs hold their Part, so it
/// needs none.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Binding {
    /// The Part (0-based).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub part: Option<u8>,
    /// The drum key (a MIDI note number).
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub key: Option<u8>,
}

impl Binding {
    /// Bound to nothing: as a filter, every binding.
    pub const ANY: Self = Self {
        part: None,
        key: None,
    };

    /// Bound to a Part.
    pub const fn of_part(part: u8) -> Self {
        Self {
            part: Some(part),
            key: None,
        }
    }

    /// This binding with a drum key.
    #[must_use]
    pub const fn with_key(mut self, key: u8) -> Self {
        self.key = Some(key);
        self
    }

    /// Whether this binding, used as a filter, takes a change at `change`:
    /// every index it sets is the same there.
    pub fn matches(&self, change: &Binding) -> bool {
        (self.part.is_none() || self.part == change.part)
            && (self.key.is_none() || self.key == change.key)
    }
}

/// Why a parameter has no address for a [`Binding`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BindingError {
    /// The parameter needs a Part and the binding has none.
    #[error("the binding needs a part")]
    NoPart,
    /// The parameter needs a key and the binding has none.
    #[error("the binding needs a key")]
    NoKey,
    /// The Part is past the device's last.
    #[error("part {0} is out of range")]
    PartOutOfRange(u8),
    /// The key is outside the kit's range.
    #[error(transparent)]
    Key(#[from] crate::device_spec::KeyOutOfRange),
}

impl HostParam {
    /// The raw value `text` reads as for this parameter (see
    /// [`HostFormat::parse`]).
    pub fn parse(&self, text: &str) -> Option<i32> {
        self.format.parse(text, self.min, self.max)
    }

    /// The address of the parameter for `binding`.
    ///
    /// # Errors
    ///
    /// [`BindingError`] if the parameter needs a Part or key the binding
    /// lacks or has out of range.
    pub fn device_address(&self, binding: &Binding) -> Result<Address, BindingError> {
        relocate(self.address, self.anchor, binding)
    }

    /// The address of the type byte of the effect the parameter belongs to,
    /// for `binding`, or `None` if it is not an effect type's parameter.
    ///
    /// # Errors
    ///
    /// As [`device_address`](Self::device_address).
    pub fn effect_type_address(&self, binding: &Binding) -> Option<Result<Address, BindingError>> {
        let effect = self.effect?;
        Some(relocate(effect.type_address, self.anchor, binding))
    }

    /// The bytes that set the parameter to the raw value `value`: the value
    /// itself, or its nibbles (most significant first), after the bias.
    pub fn encode(&self, value: i32) -> Vec<u8> {
        let stored = (value + self.bias) as u32;
        if self.width == 1 {
            return vec![stored as u8];
        }
        (0..self.width)
            .rev()
            .map(|i| ((stored >> (4 * i)) & 0x0F) as u8)
            .collect()
    }

    /// The raw value `data` holds, or `None` if it is not `width` bytes.
    pub fn decode(&self, data: &[u8]) -> Option<i32> {
        if data.len() != self.width {
            return None;
        }
        let stored = if self.width == 1 {
            i32::from(data[0])
        } else {
            data.iter()
                .fold(0, |acc, &b| (acc << 4) | i32::from(b & 0x0F))
        };
        Some(stored - self.bias)
    }
}

/// `address` (for Part 1 and the first key) moved to `binding`.
fn relocate(address: Address, anchor: Anchor, binding: &Binding) -> Result<Address, BindingError> {
    if anchor == Anchor::Absolute {
        return Ok(address);
    }
    let part = binding.part.ok_or(BindingError::NoPart)?;
    if part >= INTEGRA7.part_count {
        return Err(BindingError::PartOutOfRange(part));
    }
    let mut shift =
        params::temporary_tone_base(part).value() - params::temporary_tone_base(0).value();
    if let Anchor::Key(tone_type) = anchor {
        let key = binding.key.ok_or(BindingError::NoKey)?;
        shift += key_block(part, tone_type, key)?.value()
            - key_block(part, tone_type, key_range(tone_type).first)?.value();
    }
    Ok(shifted(address, shift))
}

/// `address` moved up by `delta` bytes.
fn shifted(address: Address, delta: u32) -> Address {
    let bytes = DataSize::from_value(delta).expect("a shift fits in 28 bits");
    address.offset(*bytes.as_bytes())
}

/// The key range of a drum tone type.
fn key_range(tone_type: ToneType) -> crate::device_spec::KeyRange {
    match tone_type {
        ToneType::SnDrum => INTEGRA7.sn_drum_keys,
        _ => INTEGRA7.pcm_drum_keys,
    }
}

/// Where `key`'s block is in `part`'s kit of `tone_type`.
fn key_block(
    part: u8,
    tone_type: ToneType,
    key: u8,
) -> Result<Address, crate::device_spec::KeyOutOfRange> {
    match tone_type {
        ToneType::SnDrum => sn_drum::snd_note_address(part, key),
        _ => pcm_drum::pcmd_partial_address(part, key),
    }
}

/// The Part whose Temporary Tone holds `address`, if any.
pub fn temporary_tone_part(address: &Address) -> Option<u8> {
    tone_part_of(address).map(|(part, _)| part)
}

/// The Part whose Temporary Tone holds `address`, and the address moved to
/// Part 1.
fn tone_part_of(address: &Address) -> Option<(u8, Address)> {
    let first = params::temporary_tone_base(0).value();
    let stride = params::temporary_tone_base(1).value() - first;
    let part = address.value().checked_sub(first)? / stride;
    let part = u8::try_from(part)
        .ok()
        .filter(|p| *p < INTEGRA7.part_count)?;
    Some((part, shifted_down(*address, u32::from(part) * stride)))
}

/// `address` moved down by `delta` bytes.
fn shifted_down(address: Address, delta: u32) -> Address {
    let bytes = DataSize::from_value(address.value() - delta).expect("an address fits in 28 bits");
    Address(*bytes.as_bytes())
}

/// A plugin and the position of one of its parameters in its list
/// ([`PluginKind::host_params`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParamHandle {
    /// The plugin.
    pub plugin: PluginKind,
    /// The position in the plugin's list.
    pub index: u32,
}

/// A parameter found at an address, with the binding that puts it there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Located {
    /// The parameter.
    pub handle: ParamHandle,
    /// The Part and key that address is for: only what the parameter's
    /// [`Anchor`] uses.
    pub binding: Binding,
}

/// Every plugin's host parameter list, searchable by ID and by address: the
/// one mapping between a parameter's ID and where it is on the device.
///
/// An effect's types share their slots' addresses, so an address can name
/// the slot of every type: [`locate`](Self::locate) returns them all, and the
/// caller picks by the active type.
pub struct ParamIndex {
    lists: Vec<Vec<HostParam>>,
    by_id: HashMap<(PluginKind, String), u32>,
    absolute: HashMap<Address, Vec<ParamHandle>>,
    part: HashMap<Address, Vec<ParamHandle>>,
    key: HashMap<Address, Vec<ParamHandle>>,
}

impl Default for ParamIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl ParamIndex {
    /// The index of every plugin's list.
    pub fn new() -> Self {
        let mut index = Self {
            lists: Vec::new(),
            by_id: HashMap::new(),
            absolute: HashMap::new(),
            part: HashMap::new(),
            key: HashMap::new(),
        };
        for plugin in PluginKind::ALL {
            let list = plugin.host_params();
            for (position, param) in list.iter().enumerate() {
                let position = u32::try_from(position).expect("a list is short");
                let handle = ParamHandle {
                    plugin,
                    index: position,
                };
                index.by_id.insert((plugin, param.id.clone()), position);
                let map = match param.anchor {
                    Anchor::Absolute => &mut index.absolute,
                    Anchor::Part => &mut index.part,
                    Anchor::Key(_) => &mut index.key,
                };
                map.entry(param.address).or_default().push(handle);
            }
            index.lists.push(list);
        }
        index
    }

    /// The parameter `handle` names.
    ///
    /// # Panics
    ///
    /// If `handle` is not from a list of this index.
    pub fn get(&self, handle: ParamHandle) -> &HostParam {
        &self.lists[Self::slot(handle.plugin)][handle.index as usize]
    }

    fn slot(plugin: PluginKind) -> usize {
        PluginKind::ALL
            .iter()
            .position(|p| *p == plugin)
            .expect("every plugin is in ALL")
    }

    /// The parameter of `plugin` with ID `id`.
    pub fn find(&self, plugin: PluginKind, id: &str) -> Option<ParamHandle> {
        let index = *self.by_id.get(&(plugin, id.to_string()))?;
        Some(ParamHandle { plugin, index })
    }

    /// Every parameter whose address for some binding is `address`.
    pub fn locate(&self, address: &Address) -> Vec<Located> {
        let mut found: Vec<Located> = self
            .absolute
            .get(address)
            .into_iter()
            .flatten()
            .map(|&handle| Located {
                handle,
                binding: Binding::ANY,
            })
            .collect();
        let Some((part, base)) = tone_part_of(address) else {
            return found;
        };
        let binding = Binding::of_part(part);
        found.extend(
            self.part
                .get(&base)
                .into_iter()
                .flatten()
                .map(|&handle| Located { handle, binding }),
        );
        for tone_type in [ToneType::SnDrum, ToneType::PcmDrum] {
            let range = key_range(tone_type);
            let Ok(first) = key_block(0, tone_type, range.first) else {
                continue;
            };
            for key in range.first..=range.last {
                let Ok(block) = key_block(0, tone_type, key) else {
                    continue;
                };
                let delta = block.value() - first.value();
                let Some(at) = base.value().checked_sub(delta) else {
                    continue;
                };
                let Some(at) = DataSize::from_value(at) else {
                    continue;
                };
                for &handle in self.key.get(&Address(*at.as_bytes())).into_iter().flatten() {
                    if self.get(handle).anchor == Anchor::Key(tone_type) {
                        found.push(Located {
                            handle,
                            binding: binding.with_key(key),
                        });
                    }
                }
            }
        }
        found
    }
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
    /// How the block's addresses move with the binding.
    anchor: Anchor,
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
                    address,
                    anchor: block.anchor,
                    width: field.width(),
                    bias: 0,
                    effect: None,
                },
            ));
        }
    }
}

/// One slot of an effect type, as a host parameter.
fn effect_param(
    (id_block, base, anchor, kind_at): (&str, Address, Anchor, (EffectKind, Address)),
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
        address: at(base, usize::from(slot) * EFFECT_SLOT_WIDTH),
        anchor,
        width: EFFECT_SLOT_WIDTH,
        bias: EFFECT_SLOT_BIAS,
        effect: Some(EffectType {
            kind: kind_at.0,
            type_id,
            type_address: kind_at.1,
        }),
    }
}

/// Bytes an effect parameter slot takes: four nibbles.
const EFFECT_SLOT_WIDTH: usize = 4;
/// What an effect slot holds beyond its signed value.
const EFFECT_SLOT_BIAS: i32 = 32768;

/// Add one effect type's parameters, `defs`, under `id_block`'s IDs. A
/// tempo-synced one adds its switch and its note, each at its own slot.
fn add_effect_type(
    entries: &mut Vec<Entry>,
    (address, anchor): (Address, Anchor),
    kind_at: (EffectKind, Address),
    (id_block, label): (&str, &str),
    type_id: u8,
    type_name: &str,
    defs: &[FxParamDef],
) {
    let group = group_path(&[label, type_name]);
    for def in defs {
        let name = def.qualified_name(label, type_name);
        let sync = def.sync.zip(def.note_param());
        let param = |slot, name, def: &FxParamDef| {
            let place = (id_block, address, anchor, kind_at);
            effect_param(place, type_id, slot, name, &group, def)
        };
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
                    address: at(base, usize::from(offset)),
                    anchor: Anchor::Part,
                    width: 1,
                    bias: 0,
                    effect: None,
                },
            ));
        }
    }
    let params_at = at(base, usize::from(MFX_PARAMS_START));
    let type_offset = mfx_header_offset(MfxHeaderField::Type, 0).expect("the type has a position");
    for type_id in 0..MFX_TYPE_NAMES.len() {
        let type_id = u8::try_from(type_id).expect("few MFX types");
        let def = mfx_type_def(type_id).expect("every named type has a definition");
        let label = (MFX_ID, MFX_LABEL);
        let place = (params_at, Anchor::Part);
        let kind_at = (EffectKind::Mfx, at(base, usize::from(type_offset)));
        add_effect_type(
            entries, place, kind_at, label, type_id, def.name, def.params,
        );
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
                    address,
                    anchor: Anchor::Part,
                    width: 1,
                    bias: 0,
                    effect: None,
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
                    address,
                    anchor: Anchor::Absolute,
                    width: 1,
                    bias: 0,
                    effect: None,
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
    let type_at =
        |field| mixer_field_address(field, MixerScope::Common).expect("in the common scope");
    let chorus_type_at = type_at(MixerField::ChorusType);
    let reverb_type_at = type_at(MixerField::ReverbType);
    let chorus_at = params::chorus_address(params::chorus::PARAM_BASE);
    for (type_id, type_name) in types(CHORUS_TYPE_HOST_NAMES) {
        let label = (CHORUS_ID, CHORUS_LABEL);
        add_effect_type(
            &mut entries,
            (chorus_at, Anchor::Absolute),
            (EffectKind::Chorus, chorus_type_at),
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
            (reverb_at, Anchor::Absolute),
            (EffectKind::Reverb, reverb_type_at),
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
        anchor: Anchor::Part,
    };
    add_fields(&mut entries, &common, SnsCommonField::ALL, sns_common_spec);
    for partial in 0..INTEGRA7.sn_synth_partial_count {
        let prefix = partial_prefix(usize::from(partial));
        let block = Block {
            base: sn_synth::sns_partial_address(0, partial),
            id: &format!("sns/partial{}", partial + 1),
            group: &prefix,
            prefix: &prefix,
            anchor: Anchor::Part,
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
        anchor: Anchor::Part,
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
        anchor: Anchor::Part,
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
            anchor: Anchor::Part,
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
        anchor: Anchor::Part,
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
        anchor: Anchor::Key(ToneType::SnDrum),
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
        anchor: Anchor::Part,
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
        anchor: Anchor::Key(ToneType::PcmDrum),
    };
    add_fields(&mut entries, &key, PcmdPartialField::ALL, pcmd_partial_spec);
    add_fields(&mut entries, &key, PcmdWmtField::ALL, pcmd_wmt_spec);
    ordered(entries)
}
