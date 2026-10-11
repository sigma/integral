//! What reading a file dropped or approximated: per file, and per converted
//! tone (the Conversion report).
//!
//! Neither has a severity: a tone is clean when its report is empty.

use core::fmt;

/// One thing a Foreign patch conversion dropped or approximated. Filled by
/// the per-family converters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ConversionItem {}

impl fmt::Display for ConversionItem {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {}
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
    /// A Foreign patch, with no converter yet.
    Patch,
    /// A Foreign Rhythm Set, with no converter yet.
    Rhythm,
    /// A Foreign Performance.
    Performance,
    /// Foreign System or Setup settings.
    System,
    /// INTEGRA-7 tone or Studio Set blocks that don't make a whole entry.
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
