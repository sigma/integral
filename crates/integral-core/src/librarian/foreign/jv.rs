//! JV/XP series (Model ID `6A`): where a DT1 lands.
//!
//! Reference: `docs/research/jv-patch-format.md` ("Where a Patch lives")
//! and `docs/research/foreign-rhythm-format.md` (JV/XP Rhythm Setup).

use super::{Placed, Unit};
use crate::librarian::foreign::report::Skipped;

/// JV/XP Model ID.
pub(super) const MODEL: &[u8] = &[0x6A];

/// Performance-mode Temporary area: Patches of Parts 1–16, and the Rhythm
/// Setup at Part 10's place.
const TEMPORARY_PARTS: u8 = 0x02;
/// The Temporary Rhythm Setup's second address byte.
const TEMPORARY_RHYTHM: u8 = 0x09;
/// Patch-mode Temporary Patch.
const TEMPORARY_PATCH: u8 = 0x03;
/// User Performances and Rhythm Setups.
const USER_PERFORMANCES: u8 = 0x10;
/// User Patches.
const USER_PATCHES: u8 = 0x11;
/// Data-card Performances and Rhythm Setup (JV-1080).
const CARD_PERFORMANCES: u8 = 0x20;
/// Data-card Patches (JV-1080).
const CARD_PATCHES: u8 = 0x21;
/// Second address byte of the first Rhythm Setup among Performances.
const RHYTHMS: u8 = 0x40;
/// System.
const SYSTEM: u8 = 0x00;
/// Temporary Performance.
const TEMPORARY_PERFORMANCE: u8 = 0x01;

/// The unit a DT1 at `address` belongs to. Every block of a unit shares
/// the first two address bytes, and Common sits at offset `00 00`.
pub(super) fn place(address: [u8; 4]) -> Option<Placed> {
    let [a, b, c, d] = address;
    let kind = match (a, b) {
        (SYSTEM, _) => Skipped::System,
        (TEMPORARY_PERFORMANCE, _) => Skipped::Performance,
        (TEMPORARY_PARTS, TEMPORARY_RHYTHM) => Skipped::Rhythm,
        (TEMPORARY_PARTS, 0x00..=0x0F) | (TEMPORARY_PATCH | USER_PATCHES | CARD_PATCHES, _) => {
            Skipped::Patch
        }
        (USER_PERFORMANCES | CARD_PERFORMANCES, RHYTHMS..) => Skipped::Rhythm,
        (USER_PERFORMANCES | CARD_PERFORMANCES, _) => Skipped::Performance,
        _ => return None,
    };
    Some(Placed {
        unit: Unit { kind, key: [a, b] },
        common: (c, d) == (0, 0),
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
        for patch in [
            [0x03, 0, 0, 0],
            [0x02, 0x0F, 0, 0],
            [0x11, 0x7F, 0, 0],
            [0x21, 5, 0, 0],
        ] {
            assert_eq!(kind(patch), Some(Skipped::Patch), "{patch:02X?}");
        }
        for rhythm in [
            [0x02, 0x09, 0x23, 0],
            [0x10, 0x41, 0, 0],
            [0x20, 0x40, 0, 0],
        ] {
            assert_eq!(kind(rhythm), Some(Skipped::Rhythm), "{rhythm:02X?}");
        }
        assert_eq!(kind([0x10, 0x1F, 0, 0]), Some(Skipped::Performance));
        assert_eq!(kind([0x01, 0x00, 0x10, 0]), Some(Skipped::Performance));
        assert_eq!(kind([0x00, 0x00, 0x00, 0x05]), Some(Skipped::System));
        assert_eq!(kind([0x7F, 0, 0, 0]), None);
    }

    #[test]
    fn a_patch_is_one_unit() {
        let common = place([0x11, 0x05, 0, 0]).unwrap();
        let tone = place([0x11, 0x05, 0x12, 0]).unwrap();
        assert_eq!(common.unit, tone.unit);
        assert!(common.common && !tone.common);
    }
}
