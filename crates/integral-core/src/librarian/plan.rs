//! Import planning: where each entry of a [`Pack`] lands.
//!
//! [`plan_import`] proposes a destination for every entry; the host shows
//! the plan, lets the user [assign](ImportPlan::assign),
//! [unassign](ImportPlan::unassign) or [exclude](ImportPlan::exclude)
//! entries, and imports once nothing is [unplaced](ImportPlan::unplaced).
//!
//! The plan never puts an entry on a slot that isn't free by itself: only
//! [`ImportPlan::assign`] does, and the entry then shows among the
//! [overwrites](ImportPlan::overwrites).
//!
//! A plan made with [`plan_read`] also carries what reading the file
//! dropped: each entry's [Conversion report](ImportPlan::report) and the
//! [file-level report](ImportPlan::file_report), for the host to show
//! before anything is written.
//!
//! Reference: decisions #65 and #66 (`docs/svd/11-sound-vs-backup.md`).

use std::collections::HashSet;
use std::fmt;

use thiserror::Error;

use crate::catalog::CatalogEntry;
use crate::expansion_slots::ExpansionSlots;
use crate::expansions::{Expansion, expansion_of_bank};
use crate::svd::ChunkType;

use super::foreign::studio_set_fx::StudioSetEffect;
use super::foreign::{ConversionItem, FileItem, PackFormat, PackRead, SourceModel};
use super::pack::{Pack, ToneRef, user_memory};

/// What kind of file a [`Pack`] was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A sound export: entries fill the free slots of their type, lowest
    /// first.
    Sound,
    /// A full backup: each entry goes back to its original slot.
    Backup,
    /// A SysEx or MIDI dump: entries fill free slots as from a sound
    /// export, but a Studio Set Part's User tone is the device's User slot,
    /// not a Pack position. A Part whose tone the dump carries (said by its
    /// source slot) is relinked to where the import stores it; any other
    /// Part keeps the device's User slot of its number.
    Dump,
}

impl Origin {
    /// The origin of a Pack read from a file: a SysEx or MIDI file is a
    /// dump; otherwise only a backup gives its entries a slot.
    pub fn of(read: &PackRead) -> Self {
        if matches!(read.format, PackFormat::Sysex | PackFormat::Midi) {
            Self::Dump
        } else if read.pack.entries.iter().any(|e| e.slot().is_some()) {
            Self::Backup
        } else {
            Self::Sound
        }
    }
}

/// Where one entry of the Pack goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Placement {
    /// This User slot (0-indexed) of the entry's type.
    User(u16),
    /// The Temporary area: a tone to the Part the host picks, a Studio Set
    /// to the Temporary Studio Set.
    Temporary,
    /// No destination yet. Blocks the import.
    Unplaced,
    /// Left out of the import.
    Excluded,
}

/// Something the user should know before importing a Studio Set or a tone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    /// A Part points at a Pack position that holds no tone of its type. It
    /// is imported as is, pointing at the User slot of that number.
    BrokenReference {
        /// The Studio Set, as an index into the Pack's entries.
        set: usize,
        /// The Part, 0-indexed.
        part: u8,
        /// The missing tone's type.
        kind: ChunkType,
        /// The missing tone's Pack position.
        index: u16,
    },
    /// A Part points at a tone of the Pack that the plan doesn't store in a
    /// User slot (excluded or unplaced). It is imported as is, pointing at
    /// the User slot of that number.
    ToneNotImported {
        /// The Studio Set, as an index into the Pack's entries.
        set: usize,
        /// The Part, 0-indexed.
        part: u8,
        /// The tone's type.
        kind: ChunkType,
        /// The tone's Pack position.
        index: u16,
    },
    /// A Part plays a tone of an Expansion that is not loaded.
    ExpansionNotLoaded {
        /// The Studio Set, as an index into the Pack's entries.
        set: usize,
        /// The Part, 0-indexed.
        part: u8,
        /// The Expansion the tone needs.
        expansion: &'static Expansion,
    },
    /// A Part of a dumped Studio Set points at a User tone that the dump
    /// doesn't carry (or carries without saying where from). It is imported
    /// as is, pointing at that User slot.
    UnmatchedReference {
        /// The Studio Set, as an index into the Pack's entries.
        set: usize,
        /// The Part, 0-indexed.
        part: u8,
        /// The missing tone's type.
        kind: ChunkType,
        /// The User slot the Part points at, 0-indexed.
        slot: u16,
    },
    /// A PCM Synth tone or PCM Drum Kit of the Pack plays Waves of an
    /// Expansion that is not loaded.
    ToneExpansionNotLoaded {
        /// The tone, as an index into the Pack's entries.
        tone: usize,
        /// The Expansion its Waves need.
        expansion: &'static Expansion,
    },
}

impl Warning {
    /// The entry the warning is about (a Studio Set, or a tone for
    /// [`Warning::ToneExpansionNotLoaded`]), as an index into the Pack's
    /// entries.
    pub fn entry(&self) -> usize {
        match *self {
            Self::BrokenReference { set, .. }
            | Self::ToneNotImported { set, .. }
            | Self::UnmatchedReference { set, .. }
            | Self::ExpansionNotLoaded { set, .. } => set,
            Self::ToneExpansionNotLoaded { tone, .. } => tone,
        }
    }

    /// The Studio Set Part the warning is about, 0-indexed, if any.
    pub fn part(&self) -> Option<u8> {
        match *self {
            Self::BrokenReference { part, .. }
            | Self::ToneNotImported { part, .. }
            | Self::UnmatchedReference { part, .. }
            | Self::ExpansionNotLoaded { part, .. } => Some(part),
            Self::ToneExpansionNotLoaded { .. } => None,
        }
    }
}

impl fmt::Display for Warning {
    /// The message, without the entry's name.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::BrokenReference {
                part, kind, index, ..
            } => write!(
                f,
                "Part {}: no {kind} at Pack position {}",
                part + 1,
                index + 1
            ),
            Self::ToneNotImported {
                part, kind, index, ..
            } => write!(
                f,
                "Part {}: {kind} at Pack position {} is not imported",
                part + 1,
                index + 1
            ),
            Self::UnmatchedReference {
                part, kind, slot, ..
            } => write!(
                f,
                "Part {}: no {kind} from User slot {} in the file",
                part + 1,
                slot + 1
            ),
            Self::ExpansionNotLoaded {
                part, expansion, ..
            } => write!(f, "Part {}: needs {}, not loaded", part + 1, expansion.name),
            Self::ToneExpansionNotLoaded { expansion, .. } => {
                write!(f, "plays Waves of {}, not loaded", expansion.name)
            }
        }
    }
}

/// An edit the plan can't take.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PlanError {
    /// No entry at this index.
    #[error("no entry {0}")]
    NoEntry(usize),
    /// The entry's type has no User slot of this number.
    #[error("no {kind} User slot {}", slot + 1)]
    NoSlot {
        /// The entry's type.
        kind: ChunkType,
        /// The slot, 0-indexed.
        slot: u16,
    },
}

/// Where each entry of a [`Pack`] goes, and what to tell the user first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPlan {
    origin: Origin,
    kinds: Vec<ChunkType>,
    /// Each entry's slot in the file it came from, if any.
    slots: Vec<Option<u16>>,
    placements: Vec<Placement>,
    /// Free User slots, from the slot names.
    free: Vec<(ChunkType, u16)>,
    /// Warnings that no edit changes.
    warnings: Vec<Warning>,
    /// Each Part reference to a tone of the Pack, with that tone's entry
    /// index, for [`Warning::ToneNotImported`].
    pack_refs: Vec<(Warning, usize)>,
    /// Each entry's Conversion report.
    reports: Vec<Vec<ConversionItem>>,
    /// The model each entry was made on, when its bytes tell.
    sources: Vec<Option<SourceModel>>,
    /// The Studio Set Chorus and Reverb of each entry's Foreign patch.
    effects: Vec<Vec<StudioSetEffect>>,
    /// What the file held that is not in the Pack.
    file_report: Vec<FileItem>,
}

/// Plan the import of `pack`.
///
/// `slot_names` are the catalog entries of the User banks: a slot is free
/// when its name is one of its type's INIT names. A slot missing from
/// `slot_names` counts as taken. `loaded` holds the loaded Expansions, or
/// `None` when unknown: a Part playing a tone of an Expansion that isn't
/// loaded gets a warning, and none when unknown.
///
/// A PCM Synth tone or PCM Drum Kit playing Waves of an Expansion that
/// isn't loaded gets a warning too, whether converted or native.
///
/// A lone tone, or a lone Studio Set that references no tone of the Pack,
/// goes to Temporary. So does a dump's lone entry: relinking a Studio Set
/// needs another entry for its Part to select.
///
/// The plan's reports are empty: [`plan_read`] plans a Pack with the
/// reports of its reading.
pub fn plan_import(
    pack: &Pack,
    origin: Origin,
    slot_names: &[CatalogEntry],
    loaded: Option<&ExpansionSlots>,
) -> ImportPlan {
    let kinds: Vec<ChunkType> = pack.entries.iter().map(|e| e.kind()).collect();
    let free = free_slots(&kinds, slot_names);

    let mut warnings = Vec::new();
    let mut pack_refs = Vec::new();
    for (set, entry) in pack.entries.iter().enumerate() {
        for (part, tone_ref) in (0..).zip(entry.tone_refs()) {
            let warning = match tone_ref {
                ToneRef::Pack { kind, index } => {
                    match pointed_entry(pack, origin, kind, index) {
                        Some(tone) => {
                            let w = Warning::ToneNotImported {
                                set,
                                part,
                                kind,
                                index,
                            };
                            pack_refs.push((w, tone));
                            None
                        }
                        // A dump's Parts point at the device's User slots.
                        // With no tone of that kind in the dump, the
                        // reference stays as it is (#226).
                        None if origin == Origin::Dump && !kinds.contains(&kind) => None,
                        None if origin == Origin::Dump => Some(Warning::UnmatchedReference {
                            set,
                            part,
                            kind,
                            slot: index,
                        }),
                        None => Some(Warning::BrokenReference {
                            set,
                            part,
                            kind,
                            index,
                        }),
                    }
                }
                ToneRef::Bank { msb, lsb, .. } => expansion_of_bank(msb, lsb)
                    .filter(|e| loaded.is_some_and(|l| !l.contains(e)))
                    .map(|expansion| Warning::ExpansionNotLoaded {
                        set,
                        part,
                        expansion,
                    }),
            };
            warnings.extend(warning);
        }
        if let Some(loaded) = loaded {
            warnings.extend(
                entry
                    .wave_groups()
                    .into_iter()
                    .filter_map(|g| g.expansion())
                    .filter(|e| !loaded.contains(e))
                    .map(|expansion| Warning::ToneExpansionNotLoaded {
                        tone: set,
                        expansion,
                    }),
            );
        }
    }

    let single = match pack.entries.as_slice() {
        [entry] => {
            origin == Origin::Dump
                || !entry
                    .tone_refs()
                    .iter()
                    .any(|r| matches!(r, ToneRef::Pack { .. }))
        }
        _ => false,
    };
    let mut plan = ImportPlan {
        origin,
        kinds,
        slots: pack.entries.iter().map(|e| e.slot()).collect(),
        placements: vec![Placement::Unplaced; pack.entries.len()],
        free,
        warnings,
        pack_refs,
        reports: vec![Vec::new(); pack.entries.len()],
        sources: vec![None; pack.entries.len()],
        effects: vec![Vec::new(); pack.entries.len()],
        file_report: Vec::new(),
    };
    if single {
        plan.placements[0] = Placement::Temporary;
    } else {
        for entry in 0..plan.placements.len() {
            plan.placements[entry] = plan.destination(entry);
        }
    }
    plan
}

/// Plan the import of a Pack read from a file, as [`plan_import`] does,
/// keeping the reports of its reading.
pub fn plan_read(
    read: &PackRead,
    origin: Origin,
    slot_names: &[CatalogEntry],
    loaded: Option<&ExpansionSlots>,
) -> ImportPlan {
    let mut plan = plan_import(&read.pack, origin, slot_names, loaded);
    plan.reports.clone_from(&read.reports);
    plan.reports.resize(read.pack.entries.len(), Vec::new());
    plan.sources.clone_from(&read.sources);
    plan.sources.resize(read.pack.entries.len(), None);
    plan.effects.clone_from(&read.effects);
    plan.effects.resize(read.pack.entries.len(), Vec::new());
    plan.file_report.clone_from(&read.file_report);
    plan
}

/// Which entry of `pack` the tone a Studio Set Part selects points at, as
/// an index into its entries. From a sound export or a backup, a reference
/// names a Pack position: the `index`-th entry of `kind`. From a dump, it
/// names the device's User slot `index` of `kind`: the entry dumped from
/// that slot, if the dump carries one.
pub(crate) fn pointed_entry(
    pack: &Pack,
    origin: Origin,
    kind: ChunkType,
    index: u16,
) -> Option<usize> {
    match origin {
        Origin::Dump => pack
            .entries
            .iter()
            .position(|e| e.kind() == kind && e.slot() == Some(index)),
        Origin::Sound | Origin::Backup => pack
            .entries
            .iter()
            .enumerate()
            .filter(|&(_, e)| e.kind() == kind)
            .nth(usize::from(index))
            .map(|(i, _)| i),
    }
}

/// The free slots among `slot_names`, for the types in `kinds`, lowest
/// first.
fn free_slots(kinds: &[ChunkType], slot_names: &[CatalogEntry]) -> Vec<(ChunkType, u16)> {
    let mut free = Vec::new();
    for &kind in kinds {
        let Some(memory) = user_memory(kind) else {
            continue;
        };
        if free.iter().any(|&(k, _)| k == kind) {
            continue;
        }
        free.extend(
            slot_names
                .iter()
                .filter(|e| e.bank_msb == memory.bank_msb)
                .filter(|e| memory.is_free(&e.name))
                .map(|e| (kind, u16::from(e.bank_lsb) * 128 + u16::from(e.pc)))
                .filter(|&(_, s)| s < memory.slot_count),
        );
    }
    free.sort_by_key(|&(k, s)| (s, k.cli_name()));
    free.dedup();
    free
}

impl ImportPlan {
    /// What kind of file the Pack was read from.
    pub fn origin(&self) -> Origin {
        self.origin
    }

    /// Where each entry goes, indexed like the Pack's entries.
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    /// Entries with no destination yet. The import can't start until there
    /// are none.
    pub fn unplaced(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.placements.len()).filter(|&i| self.placements[i] == Placement::Unplaced)
    }

    /// Entries whose destination is a User slot that isn't free.
    pub fn overwrites(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.placements.len()).filter(|&i| match self.placements[i] {
            Placement::User(s) => !self.free.contains(&(self.kinds[i], s)),
            _ => false,
        })
    }

    /// The Conversion report of `entry`: what converting it from a Foreign
    /// patch dropped or approximated. Empty for a native entry or an index
    /// out of range.
    pub fn report(&self, entry: usize) -> &[ConversionItem] {
        self.reports.get(entry).map_or(&[], Vec::as_slice)
    }

    /// The Studio Set Chorus and Reverb `entry`'s Foreign patch converts
    /// to, for an audition to apply. Empty for a native entry or an index
    /// out of range.
    pub fn effects(&self, entry: usize) -> &[StudioSetEffect] {
        self.effects.get(entry).map_or(&[], Vec::as_slice)
    }

    /// The model `entry` was made on, when its bytes tell.
    pub fn source(&self, entry: usize) -> Option<SourceModel> {
        self.sources.get(entry).copied().flatten()
    }

    /// What the file held that is not in the Pack.
    pub fn file_report(&self) -> &[FileItem] {
        &self.file_report
    }

    /// Warnings about the entries that are not excluded, as the plan stands
    /// now.
    pub fn warnings(&self) -> impl Iterator<Item = Warning> + '_ {
        let not_imported = self
            .pack_refs
            .iter()
            .filter(|&&(_, tone)| !matches!(self.placements[tone], Placement::User(_)))
            .map(|&(w, _)| w);
        self.warnings
            .iter()
            .copied()
            .chain(not_imported)
            .filter(|w| self.placements[w.entry()] != Placement::Excluded)
    }

    /// Send `entry` to User slot `slot` (0-indexed) of its type, free or
    /// not. Another entry that was going there becomes unplaced.
    pub fn assign(&mut self, entry: usize, slot: u16) -> Result<(), PlanError> {
        let kind = *self.kinds.get(entry).ok_or(PlanError::NoEntry(entry))?;
        if user_memory(kind).is_none_or(|m| slot >= m.slot_count) {
            return Err(PlanError::NoSlot { kind, slot });
        }
        for (k, p) in self.kinds.iter().zip(&mut self.placements) {
            if *k == kind && *p == Placement::User(slot) {
                *p = Placement::Unplaced;
            }
        }
        self.placements[entry] = Placement::User(slot);
        Ok(())
    }

    /// Clear `entry`'s destination.
    pub fn unassign(&mut self, entry: usize) -> Result<(), PlanError> {
        self.set(entry, Placement::Unplaced)
    }

    /// Leave `entry` out of the import.
    pub fn exclude(&mut self, entry: usize) -> Result<(), PlanError> {
        self.set(entry, Placement::Excluded)
    }

    /// Take `entry` back after an [exclude](Self::exclude), placing it as
    /// [`plan_import`] would.
    pub fn include(&mut self, entry: usize) -> Result<(), PlanError> {
        self.set(entry, Placement::Unplaced)?;
        let placement = self.destination(entry);
        self.set(entry, placement)
    }

    /// Where `entry` goes by default: from a backup, its own slot if free;
    /// otherwise the lowest free slot of its type. Either way, never a slot
    /// another entry already takes.
    fn destination(&self, entry: usize) -> Placement {
        let kind = self.kinds[entry];
        let taken: HashSet<u16> = self
            .kinds
            .iter()
            .zip(&self.placements)
            .filter_map(|(&k, &p)| match p {
                Placement::User(s) if k == kind => Some(s),
                _ => None,
            })
            .collect();
        let slot = match self.origin {
            Origin::Backup => {
                self.slots[entry].filter(|s| self.free.contains(&(kind, *s)) && !taken.contains(s))
            }
            Origin::Sound | Origin::Dump => self
                .free
                .iter()
                .filter(|&&(k, _)| k == kind)
                .map(|&(_, s)| s)
                .find(|s| !taken.contains(s)),
        };
        slot.map_or(Placement::Unplaced, Placement::User)
    }

    fn set(&mut self, entry: usize, placement: Placement) -> Result<(), PlanError> {
        *self
            .placements
            .get_mut(entry)
            .ok_or(PlanError::NoEntry(entry))? = placement;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expansions::by_name;
    use crate::librarian::pack::PackEntry;

    const SET: ChunkType = ChunkType::StudioSet;
    const SNS: ChunkType = ChunkType::SnSynthTone;

    /// An all-zero entry of `kind`; a Studio Set's Parts select no Expansion.
    fn entry(kind: ChunkType, slot: Option<u16>) -> PackEntry {
        let memory = user_memory(kind).unwrap();
        let blocks = memory
            .blocks(memory.temporary_base(0))
            .map(|(_, size)| vec![0; size.value() as usize])
            .collect();
        PackEntry::new(kind, slot, blocks).unwrap()
    }

    fn set_with(refs: &[(u8, ToneRef)]) -> PackEntry {
        let mut set = entry(SET, None);
        for &(part, r) in refs {
            set.set_tone_ref(part, r).unwrap();
        }
        set
    }

    fn sns_ref(index: u16) -> ToneRef {
        ToneRef::Pack { kind: SNS, index }
    }

    /// SRX-05 tone 1.
    const SRX05: ToneRef = ToneRef::Bank {
        msb: 93,
        lsb: 4,
        pc: 0,
    };

    /// Catalog entries naming `kind`'s User slots 0, 1, … in order.
    fn names(kind: ChunkType, names: &[&str]) -> Vec<CatalogEntry> {
        let memory = user_memory(kind).unwrap();
        (0..)
            .zip(names)
            .map(|(slot, name)| {
                let bs = memory.bank_select(slot).unwrap();
                CatalogEntry {
                    bank_msb: bs.msb,
                    bank_lsb: bs.lsb,
                    pc: bs.pc,
                    name: (*name).into(),
                }
            })
            .collect()
    }

    fn plan_of(pack: &Pack, origin: Origin, slot_names: &[CatalogEntry]) -> ImportPlan {
        plan_import(pack, origin, slot_names, None)
    }

    fn pack(entries: Vec<PackEntry>) -> Pack {
        Pack { entries }
    }

    #[test]
    fn sound_fills_free_slots_lowest_first() {
        let p = pack(vec![
            set_with(&[(0, sns_ref(1))]),
            entry(SNS, None),
            entry(SNS, None),
        ]);
        let mut slot_names = names(SET, &["My Set", "INIT STUDIO"]);
        slot_names.extend(names(
            SNS,
            &["Lead", "INIT TONE", "Pad", "INIT TONE   ", "INIT TONE"],
        ));
        let plan = plan_of(&p, Origin::Sound, &slot_names);
        use Placement::User;
        assert_eq!(plan.placements(), [User(1), User(1), User(3)]);
        assert_eq!(plan.unplaced().count(), 0);
        assert_eq!(plan.overwrites().count(), 0);
        assert_eq!(plan.warnings().count(), 0);
    }

    #[test]
    fn shortage_leaves_entries_unplaced() {
        let p = pack(vec![entry(SNS, None), entry(SNS, None), entry(SNS, None)]);
        // Slot 2 is free but unread names count as taken.
        let plan = plan_of(&p, Origin::Sound, &names(SNS, &["Lead", "INIT TONE"]));
        assert_eq!(
            plan.placements(),
            [Placement::User(1), Placement::Unplaced, Placement::Unplaced]
        );
        assert_eq!(plan.unplaced().collect::<Vec<_>>(), [1, 2]);
        assert_eq!(plan.overwrites().count(), 0);
    }

    #[test]
    fn backup_restores_original_free_slots_only() {
        let p = pack(vec![
            entry(SNS, Some(0)),
            entry(SNS, Some(1)),
            entry(SNS, Some(2)),
        ]);
        let plan = plan_of(
            &p,
            Origin::Backup,
            &names(SNS, &["INIT TONE", "Lead", "INIT TONE"]),
        );
        assert_eq!(
            plan.placements(),
            [Placement::User(0), Placement::Unplaced, Placement::User(2)]
        );
        assert_eq!(plan.overwrites().count(), 0);
    }

    #[test]
    fn manual_assign_creates_overwrite() {
        let p = pack(vec![entry(SNS, None), entry(SNS, None)]);
        let mut plan = plan_of(&p, Origin::Sound, &names(SNS, &["Lead", "INIT TONE"]));
        assert_eq!(plan.unplaced().collect::<Vec<_>>(), [1]);

        plan.assign(1, 0).unwrap();
        assert_eq!(plan.overwrites().collect::<Vec<_>>(), [1]);
        assert_eq!(plan.unplaced().count(), 0);

        // Taking a planned slot unplaces its entry.
        plan.assign(1, 1).unwrap();
        assert_eq!(plan.placements(), [Placement::Unplaced, Placement::User(1)]);
        assert_eq!(plan.overwrites().count(), 0);

        plan.exclude(0).unwrap();
        assert_eq!(plan.unplaced().count(), 0);
        plan.unassign(0).unwrap();
        assert_eq!(plan.unplaced().collect::<Vec<_>>(), [0]);

        assert_eq!(plan.assign(2, 0), Err(PlanError::NoEntry(2)));
        assert_eq!(
            plan.assign(0, 512),
            Err(PlanError::NoSlot {
                kind: SNS,
                slot: 512
            })
        );
        assert_eq!(plan.exclude(2), Err(PlanError::NoEntry(2)));
    }

    #[test]
    fn include_places_like_the_plan() {
        let tones = || vec![entry(SNS, None), entry(SNS, None), entry(SNS, None)];
        let free = names(SNS, &["Lead", "INIT TONE", "INIT TONE", "INIT TONE"]);
        let mut plan = plan_of(&pack(tones()), Origin::Sound, &free);
        for i in 0..3 {
            plan.exclude(i).unwrap();
        }
        // Back in, an entry takes the lowest free slot no other entry holds.
        plan.include(2).unwrap();
        plan.include(0).unwrap();
        use Placement::{Excluded, User};
        assert_eq!(plan.placements(), [User(2), Excluded, User(1)]);
        assert_eq!(plan.include(3), Err(PlanError::NoEntry(3)));

        // From a backup, only its own slot, and only if free.
        let p = pack(vec![entry(SNS, Some(0)), entry(SNS, Some(3))]);
        let mut plan = plan_of(&p, Origin::Backup, &free);
        plan.exclude(0).unwrap();
        plan.exclude(1).unwrap();
        plan.include(0).unwrap();
        plan.include(1).unwrap();
        assert_eq!(plan.placements(), [Placement::Unplaced, User(3)]);
    }

    #[test]
    fn warns_on_broken_reference() {
        let p = pack(vec![set_with(&[(2, sns_ref(1))]), entry(SNS, None)]);
        let mut plan = plan_of(&p, Origin::Sound, &[]);
        let warnings: Vec<_> = plan.warnings().collect();
        assert_eq!(
            warnings,
            [Warning::BrokenReference {
                set: 0,
                part: 2,
                kind: SNS,
                index: 1
            }]
        );
        assert_eq!(
            warnings[0].to_string(),
            "Part 3: no SN Synth Tone at Pack position 2"
        );
        plan.exclude(0).unwrap();
        assert_eq!(plan.warnings().count(), 0);
    }

    #[test]
    fn warns_when_a_referenced_tone_is_not_imported() {
        let p = pack(vec![set_with(&[(2, sns_ref(0))]), entry(SNS, None)]);
        let mut plan = plan_of(&p, Origin::Sound, &names(SNS, &["INIT TONE"]));
        assert_eq!(plan.warnings().count(), 0);
        let not_imported = Warning::ToneNotImported {
            set: 0,
            part: 2,
            kind: SNS,
            index: 0,
        };
        plan.exclude(1).unwrap();
        assert_eq!(plan.warnings().collect::<Vec<_>>(), [not_imported]);
        assert_eq!(
            not_imported.to_string(),
            "Part 3: SN Synth Tone at Pack position 1 is not imported"
        );
        plan.unassign(1).unwrap();
        assert_eq!(plan.warnings().collect::<Vec<_>>(), [not_imported]);
        plan.include(1).unwrap();
        assert_eq!(plan.warnings().count(), 0);
    }

    /// A Studio Set whose Part 1 plays an SRX-05 tone, with its User tone.
    fn srx_pack() -> Pack {
        pack(vec![
            set_with(&[(0, SRX05), (1, sns_ref(0))]),
            entry(SNS, None),
        ])
    }

    fn srx_warnings(loaded: Option<&ExpansionSlots>) -> Vec<Warning> {
        let free = names(SNS, &["INIT TONE"]);
        plan_import(&srx_pack(), Origin::Sound, &free, loaded)
            .warnings()
            .collect()
    }

    #[test]
    fn srx_tone_with_srx_loaded() {
        let srx05 = by_name("SRX-05").unwrap();
        let loaded = ExpansionSlots::default().with(2, Some(srx05));
        assert_eq!(srx_warnings(Some(&loaded)), []);
    }

    #[test]
    fn srx_tone_with_srx_not_loaded() {
        let srx05 = by_name("SRX-05").unwrap();
        let loaded = ExpansionSlots::default().with(0, by_name("SRX-06"));
        let warnings = srx_warnings(Some(&loaded));
        assert_eq!(
            warnings,
            [Warning::ExpansionNotLoaded {
                set: 0,
                part: 0,
                expansion: srx05
            }]
        );
        assert_eq!(warnings[0].to_string(), "Part 1: needs SRX-05, not loaded");
    }

    #[test]
    fn srx_tone_with_unknown_slots() {
        assert_eq!(srx_warnings(None), []);
    }

    /// The XV fixture: one PCM Synth tone playing SRX-03 and INT Waves, with
    /// a Conversion report.
    fn united() -> PackRead {
        Pack::read(include_bytes!(
            "../../tests/fixtures/foreign/xv/srx03ap2-001.syx"
        ))
        .unwrap()
    }

    fn tone_warnings(read: &PackRead, loaded: Option<&ExpansionSlots>) -> Vec<Warning> {
        plan_read(read, Origin::Sound, &[], loaded)
            .warnings()
            .collect()
    }

    #[test]
    fn warns_when_a_tone_plays_waves_of_an_unloaded_expansion() {
        let read = united();
        let srx03 = by_name("SRX-03").unwrap();
        let other = ExpansionSlots::default().with(0, by_name("SRX-06"));
        let warnings = tone_warnings(&read, Some(&other));
        assert_eq!(
            warnings,
            [Warning::ToneExpansionNotLoaded {
                tone: 0,
                expansion: srx03
            }]
        );
        assert_eq!(warnings[0].entry(), 0);
        assert_eq!(warnings[0].part(), None);
        assert_eq!(warnings[0].to_string(), "plays Waves of SRX-03, not loaded");

        let loaded = ExpansionSlots::default().with(1, Some(srx03));
        assert_eq!(tone_warnings(&read, Some(&loaded)), []);
        assert_eq!(tone_warnings(&read, None), []);
    }

    #[test]
    fn warns_when_a_drum_kit_plays_waves_of_an_unloaded_expansion() {
        use crate::block_field::BlockField;
        use crate::parse_helpers::encode_nibble4;
        use crate::pcm_drum::PcmdWmtField as F;

        let mut kit = entry(ChunkType::PcmDrumKit, None);
        let mut blocks = kit.blocks().to_vec();
        // Key 21's first WMT plays SRX-07 Wave 1; every other WMT is off.
        let key = &mut blocks[3];
        let at = |field: F| field.position();
        key[at(F::Switch)] = 1;
        key[at(F::WaveGroupType)] = 1;
        key[at(F::WaveGroupId)..][..4].copy_from_slice(&encode_nibble4(7));
        key[at(F::WaveNumberL)..][..4].copy_from_slice(&encode_nibble4(1));
        kit = PackEntry::new(ChunkType::PcmDrumKit, None, blocks).unwrap();
        assert_eq!(kit.wave_groups(), [crate::waves::WaveGroup::Srx(7)]);

        let loaded = ExpansionSlots::default();
        let plan = plan_import(&pack(vec![kit]), Origin::Sound, &[], Some(&loaded));
        assert_eq!(
            plan.warnings().collect::<Vec<_>>(),
            [Warning::ToneExpansionNotLoaded {
                tone: 0,
                expansion: by_name("SRX-07").unwrap()
            }]
        );
    }

    #[test]
    fn a_read_plan_carries_the_reports() {
        let mut read = united();
        read.file_report.push(FileItem::Unreadable);
        let plan = plan_read(&read, Origin::Sound, &[], None);
        assert_eq!(plan.report(0), read.reports[0]);
        assert!(!plan.report(0).is_empty());
        assert_eq!(plan.report(1), []);
        assert_eq!(plan.file_report(), [FileItem::Unreadable]);
        assert_eq!(plan.source(0), None);

        read.sources[0] = Some(SourceModel::Xp);
        let plan = plan_read(&read, Origin::Sound, &[], None);
        assert_eq!(
            (plan.source(0), plan.source(1)),
            (Some(SourceModel::Xp), None)
        );

        let plain = plan_import(&read.pack, Origin::Sound, &[], None);
        assert_eq!((plain.report(0), plain.file_report()), (&[][..], &[][..]));
    }

    #[test]
    fn lone_tone_goes_to_temporary() {
        for origin in [Origin::Sound, Origin::Backup] {
            let plan = plan_of(&pack(vec![entry(SNS, Some(4))]), origin, &[]);
            assert_eq!(plan.placements(), [Placement::Temporary]);
            assert_eq!(plan.unplaced().count(), 0);
        }
    }

    #[test]
    fn lone_set_without_pack_tones_goes_to_temporary() {
        let plan = plan_of(&pack(vec![set_with(&[(0, SRX05)])]), Origin::Sound, &[]);
        assert_eq!(plan.placements(), [Placement::Temporary]);
    }

    #[test]
    fn a_dumped_set_keeps_its_device_references() {
        // A Studio Set dumped from the device, Part 3 on User SN-S tone 2.
        let set = set_with(&[(2, sns_ref(1))]);
        let memory = user_memory(SET).unwrap();
        let syx: Vec<u8> = (memory.blocks(memory.temporary_base(0)))
            .zip(set.blocks())
            .flat_map(|((address, _), data)| crate::sysex::build_dt1(0x10, &address, data))
            .collect();
        let read = Pack::read(&syx).unwrap();
        assert_eq!(Origin::of(&read), Origin::Dump);

        let plan = plan_read(&read, Origin::Dump, &names(SET, &["INIT STUDIO"]), None);
        assert_eq!(plan.warnings().count(), 0, "no broken reference");
        assert_eq!(plan.placements(), [Placement::Temporary]);
    }

    /// The `.syx` bytes that dump `entry` at User slot `slot` of its type.
    fn user_dump(entry: &PackEntry, slot: u16) -> Vec<u8> {
        let memory = user_memory(entry.kind()).unwrap();
        memory
            .blocks(memory.user_address(slot).unwrap())
            .zip(entry.blocks())
            .flat_map(|((address, _), data)| crate::sysex::build_dt1(0x10, &address, data))
            .collect()
    }

    #[test]
    fn a_dump_relinks_to_the_slots_its_tones_come_from() {
        // A Studio Set dumped from User slot 1, Part 3 on User SN-S slot 2,
        // with that slot's tone alongside.
        let set = set_with(&[(2, sns_ref(2))]);
        let syx: Vec<u8> = user_dump(&set, 1)
            .into_iter()
            .chain(user_dump(&entry(SNS, None), 2))
            .collect();
        let read = Pack::read(&syx).unwrap();
        assert_eq!(Origin::of(&read), Origin::Dump);
        let slots: Vec<_> = read.pack.entries.iter().map(|e| e.slot()).collect();
        assert_eq!(slots, [Some(1), Some(2)]);

        let mut slot_names = names(SET, &["INIT STUDIO"]);
        slot_names.extend(names(SNS, &["INIT TONE", "INIT TONE"]));
        let plan = plan_read(&read, Origin::of(&read), &slot_names, None);
        use Placement::User;
        assert_eq!(plan.placements(), [User(0), User(0)]);
        assert_eq!(plan.warnings().count(), 0, "the tone it plays is imported");

        // Excluding the tone leaves the reference unlinked again.
        let mut plan = plan_read(&read, Origin::of(&read), &slot_names, None);
        plan.exclude(1).unwrap();
        assert_eq!(
            plan.warnings().collect::<Vec<_>>(),
            [Warning::ToneNotImported {
                set: 0,
                part: 2,
                kind: SNS,
                index: 2
            }]
        );
    }

    #[test]
    fn a_dump_warns_when_no_dumped_tone_matches_a_reference() {
        // Part 3 on User SN-S slot 2; the dump's only SN-S tone is from
        // slot 9.
        let set = set_with(&[(2, sns_ref(2))]);
        let syx: Vec<u8> = user_dump(&set, 0)
            .into_iter()
            .chain(user_dump(&entry(SNS, None), 9))
            .collect();
        let read = Pack::read(&syx).unwrap();
        let plan = plan_read(
            &read,
            Origin::of(&read),
            &names(SET, &["INIT STUDIO"]),
            None,
        );
        let warnings: Vec<_> = plan.warnings().collect();
        assert_eq!(
            warnings,
            [Warning::UnmatchedReference {
                set: 0,
                part: 2,
                kind: SNS,
                slot: 2
            }]
        );
        assert_eq!(
            warnings[0].to_string(),
            "Part 3: no SN Synth Tone from User slot 3 in the file"
        );
    }

    #[test]
    fn a_dump_stays_silent_without_tones_of_the_referenced_kind() {
        // Part 3 on User SN-S slot 2; the dump carries only a PCM-S tone.
        let set = set_with(&[(2, sns_ref(2))]);
        let syx: Vec<u8> = user_dump(&set, 0)
            .into_iter()
            .chain(user_dump(&entry(ChunkType::PcmSynthTone, None), 3))
            .collect();
        let read = Pack::read(&syx).unwrap();
        let plan = plan_read(
            &read,
            Origin::of(&read),
            &names(SET, &["INIT STUDIO"]),
            None,
        );
        assert_eq!(plan.warnings().count(), 0);
    }

    #[test]
    fn set_with_pack_tones_goes_to_user_slots() {
        let slot_names = names(SET, &["INIT STUDIO"]);
        let lone = pack(vec![set_with(&[(0, sns_ref(0))])]);
        let plan = plan_of(&lone, Origin::Sound, &slot_names);
        assert_eq!(plan.placements(), [Placement::User(0)]);
        assert_eq!(plan.warnings().count(), 1);

        let with_tone = pack(vec![set_with(&[(0, sns_ref(0))]), entry(SNS, None)]);
        let plan = plan_of(&with_tone, Origin::Sound, &slot_names);
        assert_eq!(plan.placements(), [Placement::User(0), Placement::Unplaced]);
    }
}
