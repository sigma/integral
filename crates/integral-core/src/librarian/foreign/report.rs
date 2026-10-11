//! What reading a file dropped or approximated: per file, and per converted
//! tone (the Conversion report).
//!
//! Neither has a severity. A tone is clean when its report holds only
//! [`ConversionItem::NotApplied`] items, the settings the Studio Set
//! restores.

use core::fmt;

use super::wave_map::{ForeignWave, WaveTarget};
use crate::waves;

/// Where in a converted entry a [`ConversionItem`] applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Location {
    /// A PCM Synth tone's partial (1–4).
    Partial(u8),
    /// A PCM Drum Kit key, by MIDI note number.
    Key(u8),
    /// A Wave layer of a PCM Drum Kit key.
    Wmt {
        /// The key, by MIDI note number.
        key: u8,
        /// The WMT (1–4).
        wmt: u8,
    },
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Partial(partial) => write!(f, "Partial {partial}"),
            Self::Key(key) => write!(f, "Key {key}"),
            Self::Wmt { key, wmt } => write!(f, "Key {key} WMT {wmt}"),
        }
    }
}

/// One thing a Foreign patch conversion dropped or approximated. Filled by
/// the per-family converters, and only for what the Patch uses: a dropped
/// setting is listed when it differs from its neutral value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "camelCase"))]
pub enum ConversionItem {
    /// A Wave with no INTEGRA-7 equivalent. Its partial or WMT is switched
    /// off.
    WaveUnmapped {
        /// The partial or WMT.
        at: Location,
        /// The Wave.
        source: ForeignWave,
    },
    /// A Wave mapped with low confidence.
    WaveApproximate {
        /// The partial or WMT.
        at: Location,
        /// The Wave.
        source: ForeignWave,
        /// The INTEGRA-7 Wave it was mapped to.
        target: WaveTarget,
    },
    /// The tone's effect, set to THRU.
    EffectDropped {
        /// The effect the Patch used.
        source: ForeignEffect,
    },
    /// A parameter with no INTEGRA-7 equivalent.
    ParamDropped {
        /// Where it was set, or `None` for a tone-wide parameter.
        at: Option<Location>,
        /// Its name on the source synth.
        param: &'static str,
    },
    /// A controller route that found no place among the four Matrix
    /// Controls, or whose source the INTEGRA-7 doesn't have.
    RoutingDropped {
        /// The partial (1–4).
        partial: u8,
        /// The source synth's controller (1–3).
        control: u8,
        /// The destination, as the source synth names it.
        destination: &'static str,
        /// The depth, from −63 to +63.
        sens: i8,
    },
    /// A Patch setting that lives in the Studio Set on the INTEGRA-7.
    NotApplied {
        /// The setting and its value, e.g. `Reverb SRV HALL, level 127`.
        what: String,
    },
}

impl ConversionItem {
    /// Whether the item makes its entry lossy: all but
    /// [`NotApplied`](Self::NotApplied), which the Studio Set restores.
    pub fn is_lossy(&self) -> bool {
        !matches!(self, Self::NotApplied { .. })
    }
}

/// How many items of `report` make its entry lossy: 0 when it is clean.
pub fn lossy_count(report: &[ConversionItem]) -> usize {
    report.iter().filter(|item| item.is_lossy()).count()
}

impl fmt::Display for ConversionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WaveUnmapped { at, source } => {
                write!(f, "{at}: {source} has no INTEGRA-7 Wave")
            }
            Self::WaveApproximate { at, source, target } => {
                write!(f, "{at}: {source} approximated by ")?;
                write!(f, "{} Wave {}", target.group.id(), target.number)?;
                match waves::name(target.group, target.number) {
                    Some(name) => write!(f, " ({name})"),
                    None => Ok(()),
                }
            }
            Self::EffectDropped { source } => {
                write!(f, "{source} dropped, MFX set to THRU")
            }
            Self::ParamDropped {
                at: Some(at),
                param,
            } => write!(f, "{at}: {param} dropped"),
            Self::ParamDropped { at: None, param } => write!(f, "{param} dropped"),
            Self::RoutingDropped {
                partial,
                control,
                destination,
                sens,
            } => write!(
                f,
                "Partial {partial}: Control {control} route to {destination} ({sens:+}) dropped"
            ),
            Self::NotApplied { what } => {
                write!(f, "{what} not applied; set it in the Studio Set")
            }
        }
    }
}

/// The effect block of a Foreign Patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum EffectFamily {
    /// The JV/XP EFX.
    JvEfx,
    /// The XV MFX.
    XvMfx,
}

/// One effect type of a Foreign Patch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ForeignEffect {
    /// Its effect block.
    pub family: EffectFamily,
    /// Its type number as the source synth shows it, from 1: JV EFX type
    /// `n` stores `n - 1`, XV MFX type `n` stores `n` (0 is THRU). Both
    /// number the 40 shared types alike (`docs/research/foreign-fx-map.md`).
    pub number: u8,
}

impl fmt::Display for ForeignEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let family = match self.family {
            EffectFamily::JvEfx => "EFX",
            EffectFamily::XvMfx => "MFX",
        };
        write!(f, "{family} type {}", self.number)
    }
}

/// The model a Foreign Patch was made on, when its bytes tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum SourceModel {
    /// JV-1080.
    Jv1080,
    /// XP-50, XP-60 or XP-80.
    Xp,
    /// JV-2080, JV-1010 or XP-30.
    Jv2080,
}

/// The Conversion report of one Pack entry: always empty for a native
/// entry. See [`lossy_count`] for when it is clean.
pub type ConversionReport = Vec<ConversionItem>;

/// What a group of skipped messages held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Skipped {
    /// A Foreign patch of a family with no converter yet.
    Patch,
    /// A Foreign Rhythm Set of a family with no converter yet.
    Rhythm,
    /// A Foreign Performance.
    Performance,
    /// Foreign System or Setup settings.
    System,
    /// An XV-5080 Multi-Partial Patch: it plays user samples, so it can't
    /// convert.
    MultiPartial,
    /// Blocks that don't make a whole entry or Foreign patch.
    Incomplete,
    /// Any other message: another manufacturer or model, a Universal
    /// message, an RQ1, or an address outside every entry.
    Other,
}

impl Skipped {
    /// What one or `count` of these are called.
    fn noun(self, count: usize) -> &'static str {
        let (one, many) = match self {
            Self::Patch => ("Foreign Patch", "Foreign Patches"),
            Self::Rhythm => ("Foreign Rhythm Set", "Foreign Rhythm Sets"),
            Self::Performance => ("Foreign Performance", "Foreign Performances"),
            Self::System => ("Foreign System block", "Foreign System blocks"),
            Self::MultiPartial => ("Multi-Partial Patch", "Multi-Partial Patches"),
            Self::Incomplete => ("incomplete entry", "incomplete entries"),
            Self::Other => ("other message", "other messages"),
        };
        if count == 1 { one } else { many }
    }
}

/// Something about the file as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "camelCase"))]
pub enum FileItem {
    /// Messages dropped for a checksum mismatch.
    BadChecksum {
        /// How many.
        count: usize,
    },
    /// Units the file holds that are not read as Pack entries.
    Skipped {
        /// What they are.
        kind: Skipped,
        /// How many.
        count: usize,
    },
    /// Entries dropped because an earlier one in the file is byte-identical.
    Duplicates {
        /// How many.
        count: usize,
    },
    /// Part of the file can't be read: a malformed MIDI file or track, or
    /// bytes outside any SysEx message.
    Unreadable,
}

impl fmt::Display for FileItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::BadChecksum { count } => write!(
                f,
                "{count} {} dropped for a bad checksum",
                if count == 1 { "message" } else { "messages" }
            ),
            Self::Skipped { kind, count } => {
                write!(f, "{count} {} skipped", kind.noun(count))
            }
            Self::Duplicates { count } => write!(
                f,
                "{count} duplicate {} dropped",
                if count == 1 { "entry" } else { "entries" }
            ),
            Self::Unreadable => f.write_str("part of the file is unreadable"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::librarian::foreign::wave_map::{Confidence, ForeignWaveGroup};
    use crate::waves::WaveGroup;

    #[test]
    fn conversion_items_read_as_sentences() {
        let source = ForeignWave {
            group: ForeignWaveGroup::SrJv80(2),
            number: 1,
        };
        let items = [
            ConversionItem::WaveUnmapped {
                at: Location::Partial(1),
                source,
            },
            ConversionItem::WaveApproximate {
                at: Location::Wmt { key: 36, wmt: 2 },
                source,
                target: WaveTarget {
                    group: WaveGroup::Srx(6),
                    number: 29,
                    confidence: Confidence::Low,
                },
            },
            ConversionItem::EffectDropped {
                source: ForeignEffect {
                    family: EffectFamily::JvEfx,
                    number: 2,
                },
            },
            ConversionItem::ParamDropped {
                at: Some(Location::Key(38)),
                param: "Receive Pan Mode",
            },
            ConversionItem::ParamDropped {
                at: None,
                param: "One Shot Mode",
            },
            ConversionItem::RoutingDropped {
                partial: 4,
                control: 2,
                destination: "CUT",
                sens: -10,
            },
            ConversionItem::NotApplied {
                what: "Reverb SRV HALL, level 127".into(),
            },
        ];
        let texts: Vec<String> = items.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [
                "Partial 1: SR-JV80-02 Wave 1 (Vl Sect A) has no INTEGRA-7 Wave",
                "Key 36 WMT 2: SR-JV80-02 Wave 1 (Vl Sect A) approximated by SRX-06 Wave 29 (Vl Sect A)",
                "EFX type 2 dropped, MFX set to THRU",
                "Key 38: Receive Pan Mode dropped",
                "One Shot Mode dropped",
                "Partial 4: Control 2 route to CUT (-10) dropped",
                "Reverb SRV HALL, level 127 not applied; set it in the Studio Set",
            ]
        );
    }

    #[test]
    fn only_not_applied_items_leave_an_entry_clean() {
        let not_applied = ConversionItem::NotApplied {
            what: "Tempo 90".into(),
        };
        let dropped = ConversionItem::ParamDropped {
            at: None,
            param: "One Shot Mode",
        };
        assert_eq!(lossy_count(&[]), 0);
        assert_eq!(lossy_count(&[not_applied.clone(), not_applied.clone()]), 0);
        assert_eq!(lossy_count(&[not_applied, dropped]), 1);
    }

    #[test]
    fn file_items_read_as_sentences() {
        let items = [
            FileItem::BadChecksum { count: 1 },
            FileItem::Skipped {
                kind: Skipped::Patch,
                count: 128,
            },
            FileItem::Skipped {
                kind: Skipped::Incomplete,
                count: 1,
            },
            FileItem::Duplicates { count: 2 },
            FileItem::Unreadable,
        ];
        let texts: Vec<String> = items.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [
                "1 message dropped for a bad checksum",
                "128 Foreign Patches skipped",
                "1 incomplete entry skipped",
                "2 duplicate entries dropped",
                "part of the file is unreadable",
            ]
        );
    }
}
