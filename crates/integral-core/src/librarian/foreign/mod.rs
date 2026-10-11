//! Reading a Pack from any file it can come in: an SVD file, raw SysEx
//! (`.syx`), or the SysEx events of a Standard MIDI File (`.mid`, behind the
//! `smf` feature).
//!
//! SysEx is read as Roland DT1 frames of any model. INTEGRA-7 DT1s that
//! cover a whole tone or Studio Set, in a Temporary area or a User slot,
//! become Pack entries; the import then stores them through Temporary as
//! usual. Foreign patch frames (JV/XP, XV) are grouped per source unit for
//! the per-family converters; until a family has one, its units are
//! skipped. Everything else goes in the file-level report.
//!
//! Reference: `docs/research/xv-svd-format.md` (SVD routing).

pub mod frame;
mod integra;
mod jv;
pub mod report;
pub mod wave_map;
mod xv;

use std::collections::HashMap;

use crate::address::Address;
use crate::librarian::pack::{Pack, PackError};
use crate::svd::SvdFamily;
use crate::sysex;

use frame::FrameError;
pub use report::{ConversionItem, ConversionReport, FileItem, Skipped};

/// First bytes of a Standard MIDI File.
const SMF_MAGIC: &[u8] = b"MThd";

/// A Pack read from a file, with what the reading dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackRead {
    /// The entries read.
    pub pack: Pack,
    /// What the file held that is not in `pack`.
    pub file_report: Vec<FileItem>,
    /// The Conversion report of each entry, indexed like `pack.entries`.
    pub reports: Vec<ConversionReport>,
}

impl PackRead {
    /// A Pack of native entries: every Conversion report is empty.
    fn native(pack: Pack, file_report: Vec<FileItem>) -> Self {
        let reports = vec![ConversionReport::new(); pack.entries.len()];
        Self {
            pack,
            file_report,
            reports,
        }
    }
}

impl Pack {
    /// Read a Pack file of any supported format, recognized from its first
    /// bytes. SVD files are routed on their directory's model ID.
    pub fn read(bytes: &[u8]) -> Result<PackRead, PackError> {
        match SvdFamily::of(bytes) {
            Some(SvdFamily::Integra) => {
                return Ok(PackRead::native(Pack::from_svd(bytes)?, vec![]));
            }
            Some(SvdFamily::Xv) => {
                return Err(PackError::UnsupportedFormat(
                    "XV .svd files are not supported yet",
                ));
            }
            Some(SvdFamily::JvXp) => {
                return Err(PackError::UnsupportedFormat(
                    "JV/XP .svd files (SVD0) are not supported",
                ));
            }
            None => {}
        }
        if bytes.first() == Some(&frame::SYSEX_START) {
            Ok(read_sysex(bytes, false))
        } else if bytes.starts_with(SMF_MAGIC) {
            read_smf(bytes)
        } else {
            Err(PackError::UnknownFormat)
        }
    }
}

/// The SysEx stream of a Standard MIDI File: every track's `F0` events, with
/// their `F7` continuation packets.
#[cfg(feature = "smf")]
fn read_smf(bytes: &[u8]) -> Result<PackRead, PackError> {
    use midly::TrackEventKind;

    let Ok((_, tracks)) = midly::parse(bytes) else {
        return Ok(PackRead::native(
            Pack::default(),
            vec![FileItem::Unreadable],
        ));
    };
    let mut stream = Vec::new();
    let mut unreadable = false;
    for track in tracks {
        let Ok(mut events) = track else {
            unreadable = true;
            continue;
        };
        // A malformed event ends the track early, without an error.
        while !events.unread().is_empty() {
            match events.next().map(|e| e.map(|e| e.kind)) {
                Some(Ok(TrackEventKind::SysEx(data))) => {
                    stream.push(frame::SYSEX_START);
                    stream.extend_from_slice(data);
                }
                Some(Ok(TrackEventKind::Escape(data))) => stream.extend_from_slice(data),
                Some(Ok(_)) => {}
                Some(Err(_)) | None => {
                    unreadable = true;
                    break;
                }
            }
        }
    }
    Ok(read_sysex(&stream, unreadable))
}

#[cfg(not(feature = "smf"))]
fn read_smf(_: &[u8]) -> Result<PackRead, PackError> {
    Err(PackError::UnsupportedFormat(
        "MIDI files are not supported in this build",
    ))
}

/// A unit of Foreign frames: one Patch, Rhythm Set, Performance or System
/// dump.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Unit {
    /// What it is. Until its family has a converter, a unit is skipped as
    /// this.
    kind: Skipped,
    /// Address bytes that every block of the unit shares.
    key: [u8; 2],
}

/// Where a Foreign DT1 lands.
struct Placed {
    unit: Unit,
    /// Whether it starts at the unit's Common block.
    common: bool,
}

/// What a group of frames is read as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Source {
    /// An INTEGRA-7 area, by index into [`integra::areas`].
    Integra(usize),
    /// A JV/XP unit.
    Jv(Unit),
    /// An XV unit.
    Xv(Unit),
}

/// The frames of one unit.
struct Group<'a> {
    source: Source,
    /// Whether a frame started at the unit's Common block. The next one
    /// that does starts the next unit.
    has_common: bool,
    frames: Vec<frame::Frame<'a>>,
}

/// Read a stream of SysEx messages. `unreadable` carries in from the
/// container.
fn read_sysex(bytes: &[u8], mut unreadable: bool) -> PackRead {
    let split = frame::split(bytes);
    unreadable |= split.incomplete;
    let areas = integra::areas();

    let mut bad_checksums = 0;
    let mut skipped: Vec<(Skipped, usize)> = Vec::new();
    let mut skip = |kind| match skipped.iter_mut().find(|(k, _)| *k == kind) {
        Some((_, n)) => *n += 1,
        None => skipped.push((kind, 1)),
    };

    // Frames grouped per unit, in the order units start.
    let mut groups: Vec<Group<'_>> = Vec::new();
    let mut open: HashMap<Source, usize> = HashMap::new();
    for raw in split.messages {
        let frame = match frame::parse(raw) {
            Ok(frame) => frame,
            Err(FrameError::BadChecksum) => {
                bad_checksums += 1;
                continue;
            }
            Err(FrameError::NotDt1) => {
                skip(Skipped::Other);
                continue;
            }
        };
        let placed = match frame.model {
            m if m == sysex::MODEL_ID => {
                let address = Address(frame.address);
                integra::area_of(&areas, address)
                    .map(|i| (Source::Integra(i), integra::is_common(&areas[i], address)))
            }
            jv::MODEL => jv::place(frame.address).map(|p| (Source::Jv(p.unit), p.common)),
            xv::MODEL => xv::place(frame.address).map(|p| (Source::Xv(p.unit), p.common)),
            _ => None,
        };
        let Some((source, common)) = placed else {
            skip(Skipped::Other);
            continue;
        };
        let group = match open.get(&source) {
            Some(&g) if !(common && groups[g].has_common) => g,
            _ => {
                groups.push(Group {
                    source,
                    has_common: false,
                    frames: Vec::new(),
                });
                open.insert(source, groups.len() - 1);
                groups.len() - 1
            }
        };
        groups[group].has_common |= common;
        groups[group].frames.push(frame);
    }

    let mut pack = Pack::default();
    let mut duplicates = 0;
    for Group { source, frames, .. } in groups {
        match source {
            Source::Integra(i) => {
                let frames = frames.iter().map(|f| (Address(f.address), f.data));
                match integra::assemble(&areas[i], frames) {
                    Some(entry) if pack.entries.contains(&entry) => duplicates += 1,
                    Some(entry) => pack.entries.push(entry),
                    None => skip(Skipped::Incomplete),
                }
            }
            Source::Jv(unit) | Source::Xv(unit) => skip(unit.kind),
        }
    }

    let mut file_report = Vec::new();
    if bad_checksums > 0 {
        file_report.push(FileItem::BadChecksum {
            count: bad_checksums,
        });
    }
    file_report.extend(
        skipped
            .into_iter()
            .map(|(kind, count)| FileItem::Skipped { kind, count }),
    );
    if duplicates > 0 {
        file_report.push(FileItem::Duplicates { count: duplicates });
    }
    if unreadable {
        file_report.push(FileItem::Unreadable);
    }
    PackRead::native(pack, file_report)
}

#[cfg(test)]
mod tests;
