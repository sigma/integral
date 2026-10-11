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
//! Reference: decisions #65 and #66 (`docs/svd/11-sound-vs-backup.md`).

use std::fmt;

use thiserror::Error;

use crate::catalog::CatalogEntry;
use crate::expansion_slots::ExpansionSlots;
use crate::expansions::{Expansion, expansion_of_bank};
use crate::svd::ChunkType;

use super::pack::{Pack, ToneRef, user_memory};

/// What kind of file a [`Pack`] was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A sound export: entries fill the free slots of their type, lowest
    /// first.
    Sound,
    /// A full backup: each entry goes back to its original slot.
    Backup,
}

/// Where one entry of the Pack goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Something the user should know before importing a Studio Set.
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
    /// A Part plays a tone of an Expansion that is not loaded.
    ExpansionNotLoaded {
        /// The Studio Set, as an index into the Pack's entries.
        set: usize,
        /// The Part, 0-indexed.
        part: u8,
        /// The Expansion the tone needs.
        expansion: &'static Expansion,
    },
}

impl Warning {
    /// The Studio Set the warning is about, as an index into the Pack's
    /// entries.
    pub fn set(&self) -> usize {
        match *self {
            Self::BrokenReference { set, .. } | Self::ExpansionNotLoaded { set, .. } => set,
        }
    }
}

impl fmt::Display for Warning {
    /// The message, without the Studio Set's name.
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
            Self::ExpansionNotLoaded {
                part, expansion, ..
            } => write!(f, "Part {}: needs {}, not loaded", part + 1, expansion.name),
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
    kinds: Vec<ChunkType>,
    placements: Vec<Placement>,
    /// Free User slots, from the slot names.
    free: Vec<(ChunkType, u16)>,
    warnings: Vec<Warning>,
}

/// Plan the import of `pack`.
///
/// `slot_names` are the catalog entries of the User banks: a slot is free
/// when its name is one of its type's INIT names. A slot missing from
/// `slot_names` counts as taken. `loaded` holds the loaded Expansions, or
/// `None` when unknown: a Part playing a tone of an Expansion that isn't
/// loaded gets a warning, and none when unknown.
///
/// A lone tone, or a lone Studio Set that references no tone of the Pack,
/// goes to Temporary.
pub fn plan_import(
    pack: &Pack,
    origin: Origin,
    slot_names: &[CatalogEntry],
    loaded: Option<&ExpansionSlots>,
) -> ImportPlan {
    let kinds: Vec<ChunkType> = pack.entries.iter().map(|e| e.kind()).collect();
    let free = free_slots(&kinds, slot_names);

    let mut warnings = Vec::new();
    for (set, entry) in pack.entries.iter().enumerate() {
        for (part, tone_ref) in (0..).zip(entry.tone_refs()) {
            let warning = match tone_ref {
                ToneRef::Pack { kind, index } => pack
                    .of_kind(kind)
                    .nth(usize::from(index))
                    .is_none()
                    .then_some(Warning::BrokenReference {
                        set,
                        part,
                        kind,
                        index,
                    }),
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
    }

    let single = match pack.entries.as_slice() {
        [entry] => !entry
            .tone_refs()
            .iter()
            .any(|r| matches!(r, ToneRef::Pack { .. })),
        _ => false,
    };
    let placements = if single {
        vec![Placement::Temporary]
    } else {
        match origin {
            Origin::Sound => {
                let mut free = free.clone();
                kinds
                    .iter()
                    .map(|&kind| match free.iter().position(|&(k, _)| k == kind) {
                        Some(i) => Placement::User(free.remove(i).1),
                        None => Placement::Unplaced,
                    })
                    .collect()
            }
            Origin::Backup => pack
                .entries
                .iter()
                .map(|e| match e.slot() {
                    Some(s) if free.contains(&(e.kind(), s)) => Placement::User(s),
                    _ => Placement::Unplaced,
                })
                .collect(),
        }
    };

    ImportPlan {
        kinds,
        placements,
        free,
        warnings,
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
                .filter(|e| memory.init_names.contains(&e.name.trim_end()))
                .map(|e| (kind, u16::from(e.bank_lsb) * 128 + u16::from(e.pc)))
                .filter(|&(_, s)| s < memory.slot_count),
        );
    }
    free.sort_by_key(|&(k, s)| (s, k.cli_name()));
    free.dedup();
    free
}

impl ImportPlan {
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

    /// Warnings about the Studio Sets that are not excluded.
    pub fn warnings(&self) -> impl Iterator<Item = &Warning> {
        self.warnings
            .iter()
            .filter(|w| self.placements[w.set()] != Placement::Excluded)
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

    /// Clear `entry`'s destination, or take it back after an
    /// [exclude](Self::exclude).
    pub fn unassign(&mut self, entry: usize) -> Result<(), PlanError> {
        self.set(entry, Placement::Unplaced)
    }

    /// Leave `entry` out of the import.
    pub fn exclude(&mut self, entry: usize) -> Result<(), PlanError> {
        self.set(entry, Placement::Excluded)
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
    fn warns_on_broken_reference() {
        let p = pack(vec![set_with(&[(2, sns_ref(1))]), entry(SNS, None)]);
        let mut plan = plan_of(&p, Origin::Sound, &[]);
        let warnings: Vec<_> = plan.warnings().copied().collect();
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

    /// A Studio Set whose Part 1 plays an SRX-05 tone, with its User tone.
    fn srx_pack() -> Pack {
        pack(vec![
            set_with(&[(0, SRX05), (1, sns_ref(0))]),
            entry(SNS, None),
        ])
    }

    fn srx_warnings(loaded: Option<&ExpansionSlots>) -> Vec<Warning> {
        plan_import(&srx_pack(), Origin::Sound, &[], loaded)
            .warnings()
            .copied()
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
