//! XV series (Model ID `00 10`): where a DT1 lands.
//!
//! Reference: `docs/research/xv-patch-format.md` ("User and Temporary
//! areas") and `docs/research/foreign-rhythm-format.md` (XV Rhythm Set).

use super::{Placed, Unit};
use crate::librarian::foreign::report::Skipped;

/// XV Model ID.
pub(super) const MODEL: &[u8] = &[0x00, 0x10];

/// Highest first address byte of System and Setup (XV-5050/2020 put
/// System at `02`).
const LAST_SYSTEM: u8 = 0x02;
/// Temporary Performance.
const TEMPORARY_PERFORMANCE: u8 = 0x10;
/// First and last address bytes of the Performance parts' Temporary Patches
/// and Rhythms.
const PARTS: core::ops::RangeInclusive<u8> = 0x11..=0x18;
/// Patch-mode Temporary Patch and Rhythm.
const TEMPORARY: u8 = 0x1F;
/// User Performances.
const USER_PERFORMANCES: u8 = 0x20;
/// User Patches.
const USER_PATCHES: u8 = 0x30;
/// User Rhythms.
const USER_RHYTHMS: u8 = 0x40;
/// Bit of the second address byte that moves a Temporary Patch to its
/// Temporary Rhythm.
const RHYTHM_BIT: u8 = 0x10;
/// Bits of the second address byte that name a Rhythm's slot. The others
/// carry the Rhythm Tone offset.
const RHYTHM_SLOT: u8 = 0x70;

/// The unit a DT1 at `address` belongs to. A Patch's blocks share the
/// first two address bytes; a Rhythm's share the first and the slot bits
/// of the second.
pub(super) fn place(address: [u8; 4]) -> Option<Placed> {
    let [a, b, c, d] = address;
    let kind = match a {
        ..=LAST_SYSTEM => Skipped::System,
        TEMPORARY_PERFORMANCE | USER_PERFORMANCES => Skipped::Performance,
        USER_PATCHES => Skipped::Patch,
        USER_RHYTHMS => Skipped::Rhythm,
        _ if a == TEMPORARY || PARTS.contains(&a) => {
            if b & RHYTHM_BIT == 0 {
                Skipped::Patch
            } else {
                Skipped::Rhythm
            }
        }
        _ => return None,
    };
    let (key, block) = if kind == Skipped::Rhythm {
        (b & RHYTHM_SLOT, b & !RHYTHM_SLOT)
    } else {
        (b, 0)
    };
    Some(Placed {
        unit: Unit {
            kind,
            key: [a, key],
        },
        common: (block, c, d) == (0, 0, 0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(address: [u8; 4]) -> Option<Skipped> {
        place(address).map(|p| p.unit.kind)
    }

    #[test]
    fn places_patches_rhythms_and_the_rest() {
        // Patch mode, part 1's Tone 1, part 32 and User Patch 128's MFX.
        for patch in [
            [0x1F, 0, 0, 0],
            [0x11, 0, 0x20, 0],
            [0x18, 0x60, 0, 0],
            [0x30, 0x7F, 2, 0],
        ] {
            assert_eq!(kind(patch), Some(Skipped::Patch), "{patch:02X?}");
        }
        // Patch mode, part 10 and User Rhythm 4.
        for rhythm in [
            [0x1F, 0x10, 0, 0],
            [0x13, 0x30, 0x10, 0],
            [0x40, 0x30, 0, 0],
        ] {
            assert_eq!(kind(rhythm), Some(Skipped::Rhythm), "{rhythm:02X?}");
        }
        assert_eq!(kind([0x20, 0x3F, 0, 0]), Some(Skipped::Performance));
        assert_eq!(kind([0x02, 0, 0, 0]), Some(Skipped::System));
        assert_eq!(kind([0x50, 0, 0, 0]), None);
    }

    #[test]
    fn a_rhythm_is_one_unit_across_its_tones() {
        let common = place([0x40, 0x10, 0, 0]).unwrap();
        // Rhythm Tone for key 108: `00 10 00` + 2 × 87, with a 7-bit carry.
        let last_tone = place([0x40, 0x11, 0x3E, 0]).unwrap();
        assert_eq!(common.unit, last_tone.unit);
        assert!(common.common && !last_tone.common);
        assert_ne!(common.unit, place([0x40, 0x20, 0, 0]).unwrap().unit);
    }
}
