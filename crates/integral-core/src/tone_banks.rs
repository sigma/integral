//! Tone bank definitions for the INTEGRA-7.
//!
//! Each bank may span multiple LSBs (e.g. SN Acoustic Preset uses LSB 64–65,
//! with up to 128 tones per LSB).  The catalog query returns tones per single
//! MSB/LSB, so callers query each LSB separately and merge.
//!
//! Reference: `docs/midi/03-bank-select-tables.md`

use crate::expansions::{EXPANSIONS, Expansion};
use crate::params::tone_type_from_bank_msb;

/// A single tone bank identified by its Bank Select MSB and one or more LSB
/// values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneBank {
    /// Human-readable name (e.g. "Preset", "User").
    pub label: &'static str,
    /// Bank Select MSB value.
    pub msb: u8,
    /// Bank Select LSB values.  Each LSB holds up to 128 tones (PC 0–127).
    pub lsbs: &'static [u8],
    /// The Expansion providing this bank, or `None` for built-in banks.
    pub expansion: Option<&'static Expansion>,
}

/// A labelled group of related tone banks (e.g. "SN Acoustic").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneBankGroup {
    /// Group display name.
    pub label: &'static str,
    /// Short tag for the group's tones in mixed lists (e.g. "SN-A").
    pub badge: &'static str,
    /// Banks within this group.
    pub banks: &'static [ToneBank],
}

/// Label of the user-memory bank in each writable tone group.
pub const USER_LABEL: &str = "User";

/// All tone bank groups available on the INTEGRA-7.
pub static TONE_BANK_GROUPS: &[ToneBankGroup] = &[
    ToneBankGroup {
        label: "SN Acoustic",
        badge: "SN-A",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 89,
                lsbs: &[64, 65],
                expansion: None,
            },
            ToneBank {
                label: USER_LABEL,
                msb: 89,
                lsbs: &[0, 1],
                expansion: None,
            },
        ],
    },
    ToneBankGroup {
        label: "SN Synth",
        badge: "SN-S",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 95,
                lsbs: &[64, 65, 66, 67, 68, 69, 70, 71, 72],
                expansion: None,
            },
            ToneBank {
                label: USER_LABEL,
                msb: 95,
                lsbs: &[0, 1, 2, 3],
                expansion: None,
            },
        ],
    },
    ToneBankGroup {
        label: "SN Drum",
        badge: "SN-D",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 88,
                lsbs: &[64],
                expansion: None,
            },
            ToneBank {
                label: USER_LABEL,
                msb: 88,
                lsbs: &[0],
                expansion: None,
            },
        ],
    },
    ToneBankGroup {
        label: "PCM Synth",
        badge: "PCM-S",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 87,
                lsbs: &[64, 65, 66, 67, 68, 69, 70],
                expansion: None,
            },
            ToneBank {
                label: USER_LABEL,
                msb: 87,
                lsbs: &[0, 1],
                expansion: None,
            },
        ],
    },
    ToneBankGroup {
        label: "PCM Drum",
        badge: "PCM-D",
        banks: &[
            ToneBank {
                label: "Preset",
                msb: 86,
                lsbs: &[64],
                expansion: None,
            },
            ToneBank {
                label: USER_LABEL,
                msb: 86,
                lsbs: &[0],
                expansion: None,
            },
        ],
    },
    ToneBankGroup {
        label: "GM2",
        badge: "GM2",
        banks: &[
            ToneBank {
                label: "Tone",
                msb: 121,
                lsbs: &[0, 1],
                expansion: None,
            },
            ToneBank {
                label: "Drum",
                msb: 120,
                lsbs: &[0],
                expansion: None,
            },
        ],
    },
    ToneBankGroup {
        label: "Expansion",
        badge: "EXP",
        banks: &[
            ToneBank {
                label: "SRX-01",
                msb: 93,
                lsbs: &[0],
                expansion: Some(&EXPANSIONS[0]),
            },
            ToneBank {
                label: "SRX-01 Drum",
                msb: 92,
                lsbs: &[0],
                expansion: Some(&EXPANSIONS[0]),
            },
            ToneBank {
                label: "SRX-02",
                msb: 93,
                lsbs: &[1],
                expansion: Some(&EXPANSIONS[1]),
            },
            ToneBank {
                label: "SRX-03",
                msb: 93,
                lsbs: &[2],
                expansion: Some(&EXPANSIONS[2]),
            },
            ToneBank {
                label: "SRX-03 Drum",
                msb: 92,
                lsbs: &[2],
                expansion: Some(&EXPANSIONS[2]),
            },
            ToneBank {
                label: "SRX-04",
                msb: 93,
                lsbs: &[3],
                expansion: Some(&EXPANSIONS[3]),
            },
            ToneBank {
                label: "SRX-05",
                msb: 93,
                lsbs: &[4, 5, 6],
                expansion: Some(&EXPANSIONS[4]),
            },
            ToneBank {
                label: "SRX-05 Drum",
                msb: 92,
                lsbs: &[4],
                expansion: Some(&EXPANSIONS[4]),
            },
            ToneBank {
                label: "SRX-06",
                msb: 93,
                lsbs: &[7, 8, 9, 10],
                expansion: Some(&EXPANSIONS[5]),
            },
            ToneBank {
                label: "SRX-06 Drum",
                msb: 92,
                lsbs: &[7],
                expansion: Some(&EXPANSIONS[5]),
            },
            ToneBank {
                label: "SRX-07",
                msb: 93,
                lsbs: &[11, 12, 13, 14],
                expansion: Some(&EXPANSIONS[6]),
            },
            ToneBank {
                label: "SRX-07 Drum",
                msb: 92,
                lsbs: &[11],
                expansion: Some(&EXPANSIONS[6]),
            },
            ToneBank {
                label: "SRX-08",
                msb: 93,
                lsbs: &[15, 16, 17, 18],
                expansion: Some(&EXPANSIONS[7]),
            },
            ToneBank {
                label: "SRX-08 Drum",
                msb: 92,
                lsbs: &[15],
                expansion: Some(&EXPANSIONS[7]),
            },
            ToneBank {
                label: "SRX-09",
                msb: 93,
                lsbs: &[19, 20, 21, 22],
                expansion: Some(&EXPANSIONS[8]),
            },
            ToneBank {
                label: "SRX-09 Drum",
                msb: 92,
                lsbs: &[19],
                expansion: Some(&EXPANSIONS[8]),
            },
            ToneBank {
                label: "SRX-10",
                msb: 93,
                lsbs: &[23],
                expansion: Some(&EXPANSIONS[9]),
            },
            ToneBank {
                label: "SRX-11",
                msb: 93,
                lsbs: &[24],
                expansion: Some(&EXPANSIONS[10]),
            },
            ToneBank {
                label: "SRX-12",
                msb: 93,
                lsbs: &[26],
                expansion: Some(&EXPANSIONS[11]),
            },
            ToneBank {
                label: "ExSN1",
                msb: 89,
                lsbs: &[96],
                expansion: Some(&EXPANSIONS[12]),
            },
            ToneBank {
                label: "ExSN2",
                msb: 89,
                lsbs: &[97],
                expansion: Some(&EXPANSIONS[13]),
            },
            ToneBank {
                label: "ExSN3",
                msb: 89,
                lsbs: &[98],
                expansion: Some(&EXPANSIONS[14]),
            },
            ToneBank {
                label: "ExSN4",
                msb: 89,
                lsbs: &[99],
                expansion: Some(&EXPANSIONS[15]),
            },
            ToneBank {
                label: "ExSN5",
                msb: 89,
                lsbs: &[100],
                expansion: Some(&EXPANSIONS[16]),
            },
            ToneBank {
                label: "ExSN6 Drum",
                msb: 88,
                lsbs: &[101],
                expansion: Some(&EXPANSIONS[17]),
            },
            ToneBank {
                label: "ExPCM Tone",
                msb: 97,
                lsbs: &[0, 1, 2, 3],
                expansion: Some(&EXPANSIONS[18]),
            },
            ToneBank {
                label: "ExPCM Drum",
                msb: 96,
                lsbs: &[0],
                expansion: Some(&EXPANSIONS[18]),
            },
        ],
    },
];

impl ToneBank {
    /// Short tag for this bank's tones in mixed lists: its Expansion's
    /// name, else its group's badge.
    pub fn badge(&'static self) -> &'static str {
        match self.expansion {
            Some(expansion) => expansion.name,
            None => group_of(self).map_or("", |group| group.badge),
        }
    }
}

/// The group of [`TONE_BANK_GROUPS`] holding `bank`, or `None` for a bank
/// outside the table.
pub fn group_of(bank: &ToneBank) -> Option<&'static ToneBankGroup> {
    TONE_BANK_GROUPS
        .iter()
        .find(|group| group.banks.iter().any(|b| std::ptr::eq(b, bank)))
}

/// Find the bank that contains a given (MSB, LSB) pair.
pub fn find_bank(msb: u8, lsb: u8) -> Option<&'static ToneBank> {
    for group in TONE_BANK_GROUPS {
        for bank in group.banks {
            if bank.msb == msb && bank.lsbs.contains(&lsb) {
                return Some(bank);
            }
        }
    }
    None
}

/// The user bank a tone selected by `msb` can be saved into.
///
/// Tones save into the user bank of their tone type, so e.g. an ExPCM or
/// GM2 tone (PCM Synth type) saves into the PCM Synth user bank.
pub fn user_bank_for(msb: u8) -> Option<&'static ToneBank> {
    let tone_type = tone_type_from_bank_msb(msb)?;
    TONE_BANK_GROUPS
        .iter()
        .flat_map(|group| group.banks)
        .find(|bank| {
            bank.label == USER_LABEL && tone_type_from_bank_msb(bank.msb) == Some(tone_type)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_bank_for_native_types() {
        for (msb, lsbs) in [
            (89, &[0, 1][..]),
            (95, &[0, 1, 2, 3][..]),
            (88, &[0][..]),
            (87, &[0, 1][..]),
            (86, &[0][..]),
        ] {
            let bank = user_bank_for(msb).unwrap();
            assert_eq!((bank.label, bank.msb, bank.lsbs), (USER_LABEL, msb, lsbs));
        }
    }

    #[test]
    fn user_bank_for_expansion_and_gm2() {
        assert_eq!(user_bank_for(97).unwrap().msb, 87); // ExPCM Tone
        assert_eq!(user_bank_for(121).unwrap().msb, 87); // GM2 Tone
        assert_eq!(user_bank_for(96).unwrap().msb, 86); // ExPCM Drum
        assert_eq!(user_bank_for(120).unwrap().msb, 86); // GM2 Drum
    }

    #[test]
    fn user_bank_for_unknown_msb() {
        assert!(user_bank_for(0).is_none());
    }

    #[test]
    fn find_sn_acoustic_preset() {
        let bank = find_bank(89, 64).unwrap();
        assert_eq!(bank.label, "Preset");
        assert_eq!(bank.msb, 89);
    }

    #[test]
    fn find_sn_synth_preset_last_lsb() {
        let bank = find_bank(95, 72).unwrap();
        assert_eq!(bank.label, "Preset");
        assert_eq!(bank.msb, 95);
    }

    #[test]
    fn find_expansion_expcm() {
        let bank = find_bank(97, 2).unwrap();
        assert_eq!(bank.label, "ExPCM Tone");
    }

    #[test]
    fn unknown_bank_returns_none() {
        assert!(find_bank(0, 0).is_none());
    }

    #[test]
    fn group_and_badge_of_banks() {
        let user_pcm = user_bank_for(97).unwrap();
        assert_eq!(group_of(user_pcm).unwrap().label, "PCM Synth");
        assert_eq!(user_pcm.badge(), "PCM-S");
        let srx05_drum = find_bank(92, 4).unwrap();
        assert_eq!(group_of(srx05_drum).unwrap().label, "Expansion");
        assert_eq!(srx05_drum.badge(), "SRX-05");
        assert_eq!(find_bank(96, 0).unwrap().badge(), "ExPCM");
        let copy = user_pcm.clone();
        assert!(group_of(&copy).is_none(), "only banks of the table");
    }

    #[test]
    fn total_bank_count() {
        let count: usize = TONE_BANK_GROUPS.iter().map(|g| g.banks.len()).sum();
        assert_eq!(count, 39);
    }
}
