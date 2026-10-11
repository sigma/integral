//! Catalog of the INTEGRA-7's Expansions.
//!
//! The device stores 19 Expansions (SRX-01 to SRX-12, ExSN1 to ExSN6, ExPCM).
//! Only those loaded into one of the four Expansion slots play. Each entry
//! carries the value that selects it in a slot load, how many slots it
//! takes, and for SRX its Wave Group ID. Its tone banks live in
//! [`TONE_BANK_GROUPS`](crate::tone_banks::TONE_BANK_GROUPS), each pointing
//! back at its Expansion.
//!
//! Reference: `docs/midi/99-undocumented.md`, section 5.

use crate::tone_banks::{self, ToneBank};

/// Family an Expansion belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ExpansionKind {
    /// SRX wave expansion (PCM tones and drum kits).
    Srx,
    /// SuperNATURAL expansion.
    ExSn,
    /// The ExPCM expansion.
    ExPcm,
}

/// One Expansion in the catalog.
#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Expansion {
    /// Display name (e.g. "SRX-03", "ExSN1", "ExPCM").
    pub name: &'static str,
    /// Family.
    pub kind: ExpansionKind,
    /// Value selecting this Expansion in a slot read or load (`01`–`13`).
    pub load_value: u8,
    /// Number of Expansion slots it takes when loaded.
    pub slot_cost: u8,
    /// Wave Group ID of its waves in a PCM partial (SRX only; = SRX number).
    pub wave_group: Option<u8>,
}

impl Expansion {
    /// The tone banks this Expansion provides.
    pub fn banks(&'static self) -> impl Iterator<Item = &'static ToneBank> {
        tone_banks::TONE_BANK_GROUPS
            .iter()
            .flat_map(|group| group.banks)
            .filter(move |bank| bank.expansion.is_some_and(|e| std::ptr::eq(e, self)))
    }
}

const fn srx(name: &'static str, number: u8) -> Expansion {
    Expansion {
        name,
        kind: ExpansionKind::Srx,
        load_value: number,
        slot_cost: 1,
        wave_group: Some(number),
    }
}

const fn exsn(name: &'static str, number: u8) -> Expansion {
    Expansion {
        name,
        kind: ExpansionKind::ExSn,
        load_value: 0x0C + number,
        slot_cost: 1,
        wave_group: None,
    }
}

/// Every Expansion, in load-value order.
pub static EXPANSIONS: [Expansion; 19] = [
    srx("SRX-01", 1),
    srx("SRX-02", 2),
    srx("SRX-03", 3),
    srx("SRX-04", 4),
    srx("SRX-05", 5),
    srx("SRX-06", 6),
    srx("SRX-07", 7),
    srx("SRX-08", 8),
    srx("SRX-09", 9),
    srx("SRX-10", 10),
    srx("SRX-11", 11),
    srx("SRX-12", 12),
    exsn("ExSN1", 1),
    exsn("ExSN2", 2),
    exsn("ExSN3", 3),
    exsn("ExSN4", 4),
    exsn("ExSN5", 5),
    exsn("ExSN6", 6),
    Expansion {
        name: "ExPCM",
        kind: ExpansionKind::ExPcm,
        load_value: 0x13,
        slot_cost: 4,
        wave_group: None,
    },
];

/// The Expansion selected by a slot value, or `None` for OFF or unknown.
pub fn by_load_value(value: u8) -> Option<&'static Expansion> {
    EXPANSIONS.iter().find(|e| e.load_value == value)
}

/// The Expansion with this name, ignoring case.
pub fn by_name(name: &str) -> Option<&'static Expansion> {
    EXPANSIONS
        .iter()
        .find(|e| e.name.eq_ignore_ascii_case(name))
}

/// The Expansion providing the tone bank at (MSB, LSB), if any.
pub fn expansion_of_bank(msb: u8, lsb: u8) -> Option<&'static Expansion> {
    tone_banks::find_bank(msb, lsb)?.expansion
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name_of(msb: u8, lsb: u8) -> Option<&'static str> {
        expansion_of_bank(msb, lsb).map(|e| e.name)
    }

    #[test]
    fn load_values_are_contiguous() {
        for (i, e) in EXPANSIONS.iter().enumerate() {
            assert_eq!(usize::from(e.load_value), i + 1, "{}", e.name);
            assert_eq!(by_load_value(e.load_value), Some(e));
        }
        assert_eq!(by_load_value(0), None);
        assert_eq!(by_load_value(0x14), None);
    }

    #[test]
    fn slot_cost_and_wave_group() {
        assert_eq!(by_name("ExPCM").unwrap().slot_cost, 4);
        assert_eq!(by_name("SRX-07").unwrap().slot_cost, 1);
        assert_eq!(by_name("SRX-07").unwrap().wave_group, Some(7));
        assert_eq!(by_name("ExSN3").unwrap().wave_group, None);
    }

    #[test]
    fn name_lookup_ignores_case() {
        assert_eq!(by_name("srx-03").unwrap().load_value, 3);
        assert_eq!(by_name("EXSN6").unwrap().load_value, 0x12);
        assert_eq!(by_name("expcm").unwrap().load_value, 0x13);
        assert_eq!(by_name("SRX-13"), None);
        assert_eq!(by_name("OFF"), None);
    }

    #[test]
    fn every_srx_tone_lsb_maps_to_its_srx() {
        let lsbs: [(&str, &[u8]); 12] = [
            ("SRX-01", &[0]),
            ("SRX-02", &[1]),
            ("SRX-03", &[2]),
            ("SRX-04", &[3]),
            ("SRX-05", &[4, 5, 6]),
            ("SRX-06", &[7, 8, 9, 10]),
            ("SRX-07", &[11, 12, 13, 14]),
            ("SRX-08", &[15, 16, 17, 18]),
            ("SRX-09", &[19, 20, 21, 22]),
            ("SRX-10", &[23]),
            ("SRX-11", &[24]),
            ("SRX-12", &[26]),
        ];
        for (name, lsbs) in lsbs {
            for &lsb in lsbs {
                assert_eq!(name_of(93, lsb), Some(name), "MSB 93 LSB {lsb}");
            }
        }
        assert_eq!(name_of(93, 25), None, "gap at MSB 93 LSB 25");
        assert_eq!(name_of(93, 27), None);
    }

    #[test]
    fn every_srx_drum_lsb_maps_to_its_srx() {
        for (lsb, name) in [
            (0, "SRX-01"),
            (2, "SRX-03"),
            (4, "SRX-05"),
            (7, "SRX-06"),
            (11, "SRX-07"),
            (15, "SRX-08"),
            (19, "SRX-09"),
        ] {
            assert_eq!(name_of(92, lsb), Some(name), "MSB 92 LSB {lsb}");
        }
        assert_eq!(name_of(92, 1), None);
    }

    #[test]
    fn exsn_and_expcm_banks() {
        for (lsb, name) in [
            (96, "ExSN1"),
            (97, "ExSN2"),
            (98, "ExSN3"),
            (99, "ExSN4"),
            (100, "ExSN5"),
        ] {
            assert_eq!(name_of(89, lsb), Some(name));
        }
        assert_eq!(name_of(88, 101), Some("ExSN6"));
        for lsb in 0..4 {
            assert_eq!(name_of(97, lsb), Some("ExPCM"));
        }
        assert_eq!(name_of(96, 0), Some("ExPCM"));
    }

    #[test]
    fn non_expansion_banks_have_none() {
        assert_eq!(name_of(89, 64), None);
        assert_eq!(name_of(87, 0), None);
        assert_eq!(name_of(121, 0), None);
    }

    #[test]
    fn every_expansion_provides_banks() {
        for e in &EXPANSIONS {
            assert!(e.banks().next().is_some(), "{}", e.name);
        }
        assert_eq!(by_name("SRX-02").unwrap().banks().count(), 1);
        assert_eq!(by_name("SRX-05").unwrap().banks().count(), 2);
        assert_eq!(by_name("ExPCM").unwrap().banks().count(), 2);
    }

    #[test]
    fn expansion_group_is_in_catalog_order() {
        let group = tone_banks::TONE_BANK_GROUPS
            .iter()
            .find(|g| g.label == "Expansion")
            .unwrap();
        let order: Vec<u8> = group
            .banks
            .iter()
            .map(|b| b.expansion.unwrap().load_value)
            .collect();
        assert!(order.is_sorted(), "{order:?}");
    }
}
