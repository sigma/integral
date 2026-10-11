//! What reading a file dropped or approximated: per file, and per converted
//! tone (the Conversion report).
//!
//! Neither has a severity: a tone is clean when its report is empty.

use core::fmt;

use super::wave_map::{ForeignWave, WaveTarget};
use crate::waves;

/// One thing a Foreign patch conversion dropped or approximated. Filled by
/// the per-family converters, and only for what the Patch uses: a dropped
/// setting is listed when it differs from its neutral value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "type", rename_all = "camelCase"))]
pub enum ConversionItem {
    /// A Wave with no INTEGRA-7 equivalent. Its partial is switched off.
    WaveUnmapped {
        /// The partial (1–4).
        partial: u8,
        /// The Wave.
        source: ForeignWave,
    },
    /// A Wave mapped with low confidence.
    WaveApproximate {
        /// The partial (1–4).
        partial: u8,
        /// The Wave.
        source: ForeignWave,
        /// The INTEGRA-7 Wave it was mapped to.
        target: WaveTarget,
    },
    /// The tone's effect, set to THRU.
    EffectDropped,
    /// A parameter with no INTEGRA-7 equivalent.
    ParamDropped {
        /// The partial (1–4), or `None` for a tone-wide parameter.
        partial: Option<u8>,
        /// Its name on the source synth.
        param: &'static str,
    },
    /// A Patch setting that lives in the Studio Set on the INTEGRA-7.
    NotApplied {
        /// The setting and its value, e.g. `Reverb SRV HALL, level 127`.
        what: String,
    },
}

impl fmt::Display for ConversionItem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WaveUnmapped { partial, source } => {
                write!(f, "Partial {partial}: {source} has no INTEGRA-7 Wave")
            }
            Self::WaveApproximate {
                partial,
                source,
                target,
            } => {
                write!(f, "Partial {partial}: {source} approximated by ")?;
                write!(f, "{} Wave {}", target.group.id(), target.number)?;
                match waves::name(target.group, target.number) {
                    Some(name) => write!(f, " ({name})"),
                    None => Ok(()),
                }
            }
            Self::EffectDropped => f.write_str("effect dropped, MFX set to THRU"),
            Self::ParamDropped {
                partial: Some(partial),
                param,
            } => write!(f, "Partial {partial}: {param} dropped"),
            Self::ParamDropped {
                partial: None,
                param,
            } => write!(f, "{param} dropped"),
            Self::NotApplied { what } => {
                write!(f, "{what} not applied; set it in the Studio Set")
            }
        }
    }
}

/// The Conversion report of one Pack entry: empty when it is clean, and
/// always empty for a native entry.
pub type ConversionReport = Vec<ConversionItem>;

/// What a group of skipped messages held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum Skipped {
    /// A Foreign patch of a family with no converter yet.
    Patch,
    /// A Foreign Rhythm Set, with no converter yet.
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
            ConversionItem::WaveUnmapped { partial: 1, source },
            ConversionItem::WaveApproximate {
                partial: 2,
                source,
                target: WaveTarget {
                    group: WaveGroup::Srx(6),
                    number: 29,
                    confidence: Confidence::Low,
                },
            },
            ConversionItem::EffectDropped,
            ConversionItem::ParamDropped {
                partial: Some(3),
                param: "Receive Pan Mode",
            },
            ConversionItem::ParamDropped {
                partial: None,
                param: "One Shot Mode",
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
                "Partial 2: SR-JV80-02 Wave 1 (Vl Sect A) approximated by SRX-06 Wave 29 (Vl Sect A)",
                "effect dropped, MFX set to THRU",
                "Partial 3: Receive Pan Mode dropped",
                "One Shot Mode dropped",
                "Reverb SRV HALL, level 127 not applied; set it in the Studio Set",
            ]
        );
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
