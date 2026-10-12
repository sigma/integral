//! Signal routing of the PCM Synth Tone Structure Types: how a pair of
//! partials (1 & 2, or 3 & 4) is wired, after the INTEGRA-7 Parameter Guide
//! (PMT tab). The pair's upper partial is 1 (or 3), the lower 2 (or 4).

/// Which partial of the pair a block belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum Side {
    /// Partial 1 (or 3).
    Upper,
    /// Partial 2 (or 4).
    Lower,
}

/// A block of the signal path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", content = "side", rename_all = "lowercase")
)]
pub enum Block {
    /// Wave generator (WAVE).
    Wave(Side),
    /// TVF (FILTER).
    Filter(Side),
    /// TVA (AMP).
    Amp(Side),
    /// The booster, set by Booster 1&2 (3&4).
    Booster,
}

/// How the two paths come together.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum JoinKind {
    /// Summed.
    Mix,
    /// Summed through the booster.
    Booster,
    /// Ring modulated.
    Ring,
}

/// Where and how the two paths join.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Join {
    /// How they combine.
    pub kind: JoinKind,
    /// Whether the lower path is also mixed in dry, beside the join.
    pub lower_dry: bool,
    /// Blocks after the join, in signal order.
    pub after: &'static [Block],
}

/// The wiring of one Structure Type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Routing {
    /// The upper partial's blocks up to the join (or its whole path).
    pub upper: &'static [Block],
    /// The lower partial's blocks up to the join (or its whole path).
    pub lower: &'static [Block],
    /// `None` when the two partials stay independent (Type 1).
    pub join: Option<Join>,
}

use Block::{Amp, Booster, Filter, Wave};
use Side::{Lower, Upper};

const FULL_UPPER: &[Block] = &[Wave(Upper), Filter(Upper), Amp(Upper)];
const AMP_UPPER: &[Block] = &[Wave(Upper), Amp(Upper)];
const WAVE_LOWER: &[Block] = &[Wave(Lower)];
const BOTH_FILTERS: &[Block] = &[Filter(Upper), Filter(Lower), Amp(Lower)];
const LOWER_FILTER: &[Block] = &[Filter(Lower), Amp(Lower)];

const fn join(kind: JoinKind, lower_dry: bool, after: &'static [Block]) -> Option<Join> {
    Some(Join {
        kind,
        lower_dry,
        after,
    })
}

/// The routing of Structure Type `structure` (0-9, shown as 1-10); `None`
/// out of range.
pub fn routing(structure: u8) -> Option<Routing> {
    use JoinKind::{Booster as Boost, Mix, Ring};
    let (upper, lower, join) = match structure {
        0 => (
            FULL_UPPER,
            &[Wave(Lower), Filter(Lower), Amp(Lower)][..],
            None,
        ),
        1 => (AMP_UPPER, WAVE_LOWER, join(Mix, false, BOTH_FILTERS)),
        2 => (
            AMP_UPPER,
            WAVE_LOWER,
            join(
                Mix,
                false,
                &[Filter(Upper), Booster, Filter(Lower), Amp(Lower)],
            ),
        ),
        3 => (AMP_UPPER, WAVE_LOWER, join(Boost, false, BOTH_FILTERS)),
        4 => (AMP_UPPER, WAVE_LOWER, join(Ring, false, BOTH_FILTERS)),
        5 => (AMP_UPPER, WAVE_LOWER, join(Ring, true, BOTH_FILTERS)),
        6 => (FULL_UPPER, WAVE_LOWER, join(Ring, false, LOWER_FILTER)),
        7 => (FULL_UPPER, WAVE_LOWER, join(Ring, true, LOWER_FILTER)),
        8 => (
            FULL_UPPER,
            &[Wave(Lower), Filter(Lower)][..],
            join(Ring, false, &[Amp(Lower)]),
        ),
        9 => (
            FULL_UPPER,
            &[Wave(Lower), Filter(Lower)][..],
            join(Ring, true, &[Amp(Lower)]),
        ),
        _ => return None,
    };
    Some(Routing { upper, lower, join })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks(r: &Routing) -> Vec<Block> {
        let after = r.join.map_or(&[][..], |j| j.after);
        r.upper
            .iter()
            .chain(r.lower)
            .chain(after)
            .copied()
            .collect()
    }

    #[test]
    fn every_type_runs_each_stage_once() {
        for s in 0..10 {
            let r = routing(s).unwrap();
            let all = blocks(&r);
            for side in [Upper, Lower] {
                for stage in [Wave(side), Filter(side), Amp(side)] {
                    let n = all.iter().filter(|b| **b == stage).count();
                    assert_eq!(n, 1, "type {} {stage:?}", s + 1);
                }
            }
            assert_eq!(r.upper.first(), Some(&Wave(Upper)));
            assert_eq!(r.lower.first(), Some(&Wave(Lower)));
        }
        assert_eq!(routing(10), None);
    }

    #[test]
    fn joins_follow_the_guide() {
        assert_eq!(routing(0).unwrap().join, None);
        let kind = |s| routing(s).unwrap().join.unwrap().kind;
        assert_eq!(kind(1), JoinKind::Mix);
        assert_eq!(kind(3), JoinKind::Booster);
        assert!((4..10).all(|s| kind(s) == JoinKind::Ring));
        // Type 3 boosts after the upper filter, not at the join.
        assert!(
            routing(2)
                .unwrap()
                .join
                .unwrap()
                .after
                .contains(&Block::Booster)
        );
        // Types 6, 8 and 10 also mix in the lower partial dry.
        let dry: Vec<u8> = (1..10)
            .filter(|&s| routing(s).unwrap().join.unwrap().lower_dry)
            .collect();
        assert_eq!(dry, [5, 7, 9]);
    }
}
