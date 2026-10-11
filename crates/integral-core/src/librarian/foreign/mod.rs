//! Reading a Pack from any file it can come in: an SVD file, raw SysEx
//! (`.syx`), or the SysEx events of a Standard MIDI File (`.mid`, behind the
//! `smf` feature).
//!
//! SysEx is read as Roland DT1 frames of any model. INTEGRA-7 DT1s that
//! cover a whole tone or Studio Set, in a Temporary area or a User slot,
//! become Pack entries; the import then stores them through Temporary as
//! usual. Foreign patch frames (JV/XP, XV) are grouped per source unit for
//! the per-family converters: JV/XP and XV Patches become PCM Synth tones,
//! each with its Conversion report. Units with no converter yet are skipped.
//! Everything else goes in the file-level report. XV `.svd` files hold the
//! same Patches, bit-packed, and convert the same way.
//!
//! Reference: `docs/research/xv-svd-format.md` (SVD routing).

pub mod frame;
mod integra;
mod jv;
mod pcm_tone;
pub mod report;
pub mod wave_map;
mod xv;

use std::collections::HashMap;

use crate::address::Address;
use crate::librarian::pack::{Pack, PackEntry, PackError};
use crate::librarian::plan::Origin;
use crate::svd::{self, SvdFamily, XvChunkType};
use crate::sysex;

use frame::FrameError;
pub use report::{
    ConversionItem, ConversionReport, EffectFamily, FileItem, ForeignEffect, Skipped,
};

/// First bytes of a Standard MIDI File.
const SMF_MAGIC: &[u8] = b"MThd";

/// The kind of file a Pack was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackFormat {
    /// An INTEGRA-7 SVD file.
    Svd,
    /// Raw SysEx (`.syx`).
    Sysex,
    /// A Standard MIDI File (`.mid`).
    Midi,
}

impl PackFormat {
    /// What a file of this format is called when planning from it: an SVD
    /// file is a sound export or a backup.
    pub fn label(self, origin: Origin) -> &'static str {
        match (self, origin) {
            (Self::Svd, Origin::Sound) => "sound export",
            (Self::Svd, Origin::Backup) => "backup",
            (Self::Sysex, _) => "SysEx dump",
            (Self::Midi, _) => "MIDI file",
        }
    }
}

/// A Pack read from a file, with what the reading dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRead {
    /// The kind of file.
    pub format: PackFormat,
    /// The entries read.
    pub pack: Pack,
    /// What the file held that is not in `pack`.
    pub file_report: Vec<FileItem>,
    /// The Conversion report of each entry, indexed like `pack.entries`.
    pub reports: Vec<ConversionReport>,
}

impl PackRead {
    /// A Pack of native entries: every Conversion report is empty.
    fn native(format: PackFormat, pack: Pack, file_report: Vec<FileItem>) -> Self {
        let reports = vec![ConversionReport::new(); pack.entries.len()];
        Self {
            format,
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
                return Ok(PackRead::native(
                    PackFormat::Svd,
                    Pack::from_svd(bytes)?,
                    vec![],
                ));
            }
            Some(SvdFamily::Xv) => return read_xv_svd(bytes),
            Some(SvdFamily::JvXp) => {
                return Err(PackError::UnsupportedFormat(
                    "JV/XP .svd files (SVD0) are not supported",
                ));
            }
            None => {}
        }
        if bytes.first() == Some(&frame::SYSEX_START) {
            Ok(read_sysex(bytes, PackFormat::Sysex, false))
        } else if bytes.starts_with(SMF_MAGIC) {
            read_smf(bytes)
        } else {
            Err(PackError::UnknownFormat)
        }
    }
}

/// Pack entries as units convert, with the tally of what was dropped.
struct Collector {
    read: PackRead,
    skipped: Vec<(Skipped, usize)>,
    duplicates: usize,
}

impl Collector {
    /// An empty Pack read from a file of `format`.
    fn new(format: PackFormat) -> Self {
        Self {
            read: PackRead::native(format, Pack::default(), vec![]),
            skipped: Vec::new(),
            duplicates: 0,
        }
    }

    /// Count `count` units skipped as `kind`.
    fn skip(&mut self, kind: Skipped, count: usize) {
        match self.skipped.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, n)) => *n += count,
            None => self.skipped.push((kind, count)),
        }
    }

    /// Add a converted unit, unless an earlier one is byte-identical, or
    /// count it skipped.
    fn add(&mut self, converted: Result<(PackEntry, ConversionReport), Skipped>) {
        match converted {
            Ok((entry, _)) if self.read.pack.entries.contains(&entry) => self.duplicates += 1,
            Ok((entry, report)) => {
                self.read.pack.entries.push(entry);
                self.read.reports.push(report);
            }
            Err(kind) => self.skip(kind, 1),
        }
    }

    /// The Pack read, with its file-level report.
    fn finish(self, bad_checksums: usize, unreadable: bool) -> PackRead {
        let Self {
            mut read,
            skipped,
            duplicates,
        } = self;
        if bad_checksums > 0 {
            read.file_report.push(FileItem::BadChecksum {
                count: bad_checksums,
            });
        }
        read.file_report.extend(
            skipped
                .into_iter()
                .map(|(kind, count)| FileItem::Skipped { kind, count }),
        );
        if duplicates > 0 {
            read.file_report
                .push(FileItem::Duplicates { count: duplicates });
        }
        if unreadable {
            read.file_report.push(FileItem::Unreadable);
        }
        read
    }
}

/// An XV-series `.svd` file: each Patch entry converts like a SysEx Patch
/// dump. Performances, Rhythm Sets and System settings are skipped.
fn read_xv_svd(bytes: &[u8]) -> Result<PackRead, PackError> {
    let mut collector = Collector::new(PackFormat::Svd);
    for chunk in svd::parse_xv(bytes)? {
        let kind = match chunk.chunk_type {
            XvChunkType::Patch => {
                for entry in &chunk.entries {
                    collector.add(xv::convert_svd(entry).map_err(Skipped::from));
                }
                continue;
            }
            XvChunkType::Performance => Skipped::Performance,
            XvChunkType::Rhythm => Skipped::Rhythm,
            XvChunkType::System => Skipped::System,
            XvChunkType::Aux => continue,
        };
        if !chunk.entries.is_empty() {
            collector.skip(kind, chunk.entries.len());
        }
    }
    Ok(collector.finish(0, false))
}

/// The SysEx stream of a Standard MIDI File: every track's `F0` events, with
/// their `F7` continuation packets.
#[cfg(feature = "smf")]
fn read_smf(bytes: &[u8]) -> Result<PackRead, PackError> {
    use midly::TrackEventKind;

    let Ok((_, tracks)) = midly::parse(bytes) else {
        return Ok(PackRead::native(
            PackFormat::Midi,
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
    Ok(read_sysex(&stream, PackFormat::Midi, unreadable))
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

/// Read a stream of SysEx messages from a file of `format`. `unreadable`
/// carries in from the container.
fn read_sysex(bytes: &[u8], format: PackFormat, mut unreadable: bool) -> PackRead {
    let split = frame::split(bytes);
    unreadable |= split.incomplete;
    let areas = integra::areas();

    let mut bad_checksums = 0;
    let mut collector = Collector::new(format);

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
                collector.skip(Skipped::Other, 1);
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
            collector.skip(Skipped::Other, 1);
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

    for Group { source, frames, .. } in groups {
        collector.add(match source {
            Source::Integra(i) => {
                let frames = frames.iter().map(|f| (Address(f.address), f.data));
                integra::assemble(&areas[i], frames)
                    .map(|entry| (entry, ConversionReport::new()))
                    .ok_or(Skipped::Incomplete)
            }
            Source::Xv(unit) if unit.kind == Skipped::Patch => {
                xv::convert(unit.key, &frames).map_err(Skipped::from)
            }
            Source::Jv(unit) if unit.kind == Skipped::Patch => jv::convert(unit.key, &frames),
            Source::Jv(unit) | Source::Xv(unit) => Err(unit.kind),
        });
    }
    collector.finish(bad_checksums, unreadable)
}

#[cfg(test)]
mod tests;
