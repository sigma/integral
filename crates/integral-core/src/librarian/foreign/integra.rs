//! INTEGRA-7 DT1s: which Temporary area or User slot they write, and the
//! Pack entry their blocks rebuild.
//!
//! Blocks are rebuilt by [`frame::assemble`].

use super::frame;
use crate::address::Address;
use crate::device_spec::{INTEGRA7, UserMemorySpec};
use crate::librarian::pack::{PackEntry, user_memory};
use crate::svd::{ChunkType, SOUND_CHUNKS};

/// One place an entry can be dumped from: a Temporary area or a User slot.
#[derive(Debug, Clone, Copy)]
pub(super) struct Area {
    kind: ChunkType,
    memory: &'static UserMemorySpec,
    base: Address,
    /// Address value one past the last block.
    end: u32,
    /// The User slot this area is, if it is one. It is the source address
    /// the entry was dumped from, for Studio Set Parts to relink to.
    slot: Option<u16>,
}

/// Every Temporary area (each Part's, for tones) and User slot of every
/// type.
pub(super) fn areas() -> Vec<Area> {
    let mut areas = Vec::new();
    for kind in SOUND_CHUNKS {
        let Some(memory) = user_memory(kind) else {
            continue;
        };
        let parts = if memory.tone_type.is_some() {
            INTEGRA7.part_count
        } else {
            1
        };
        let temporary = (0..parts).map(|p| (memory.temporary_base(p), None));
        let user = (0..memory.slot_count)
            .filter_map(|s| memory.user_address(s).map(|base| (base, Some(s))));
        for (base, slot) in temporary.chain(user) {
            let end = memory
                .blocks(base)
                .map(|(a, size)| a.value() + size.value())
                .max()
                .unwrap_or(base.value());
            areas.push(Area {
                kind,
                memory,
                base,
                end,
                slot,
            });
        }
    }
    areas
}

/// The index in `areas` of the area that `address` lies in.
pub(super) fn area_of(areas: &[Area], address: Address) -> Option<usize> {
    let at = address.value();
    areas
        .iter()
        .position(|a| (a.base.value()..a.end).contains(&at))
}

/// Whether `address` is the start of `area`, where its Common block sits.
pub(super) fn is_common(area: &Area, address: Address) -> bool {
    area.base == address
}

/// The entry that DT1s `(address, data)` aimed at `area` rebuild, or
/// `None` if a block is not fully covered.
pub(super) fn assemble<'a>(
    area: &Area,
    frames: impl IntoIterator<Item = (Address, &'a [u8])>,
) -> Option<PackEntry> {
    let layout: Vec<(u32, usize)> = area
        .memory
        .blocks(area.base)
        .map(|(a, size)| (a.value(), size.value() as usize))
        .collect();
    let blocks = frame::assemble(&layout, frames)?;
    PackEntry::new(area.kind, area.slot, blocks).ok()
}
