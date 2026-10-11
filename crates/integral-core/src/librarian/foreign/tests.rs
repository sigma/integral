//! Pack reading, from bytes built in the tests.

use super::*;
use crate::address::Address;
use crate::device_spec::INTEGRA7;
use crate::librarian::pack::{PackEntry, user_memory};
use crate::parse_helpers::encode_ascii_name;
use crate::svd::ChunkType;
use crate::sysex::build_dt1;

const SNS: ChunkType = ChunkType::SnSynthTone;
const SNA: ChunkType = ChunkType::SnAcousticTone;

/// Blocks of an entry named `name`, every other byte `fill`.
fn blocks(kind: ChunkType, name: &str, fill: u8) -> Vec<Vec<u8>> {
    let memory = user_memory(kind).unwrap();
    let mut blocks: Vec<Vec<u8>> = memory
        .blocks(memory.temporary_base(0))
        .map(|(_, size)| vec![fill; size.value() as usize])
        .collect();
    let len = kind.name_length().unwrap();
    blocks[0][..len].copy_from_slice(&encode_ascii_name(name, len));
    blocks
}

/// One DT1 per block, aimed at `base`.
fn dump(kind: ChunkType, base: Address, blocks: &[Vec<u8>]) -> Vec<u8> {
    let memory = user_memory(kind).unwrap();
    memory
        .blocks(base)
        .zip(blocks)
        .flat_map(|((address, _), data)| build_dt1(0x10, &address, data))
        .collect()
}

/// The Temporary base of `kind` for Part `part` (0-indexed).
fn temporary(kind: ChunkType, part: u8) -> Address {
    user_memory(kind).unwrap().temporary_base(part)
}

fn read(bytes: &[u8]) -> PackRead {
    Pack::read(bytes).unwrap()
}

fn only_entry(read: &PackRead) -> &PackEntry {
    assert_eq!(read.pack.entries.len(), 1, "{:?}", read.file_report);
    assert_eq!(read.reports, [ConversionReport::new()]);
    &read.pack.entries[0]
}

fn skipped(kind: Skipped, count: usize) -> FileItem {
    FileItem::Skipped { kind, count }
}

#[test]
fn every_type_reads_from_any_part() {
    for kind in crate::svd::SOUND_CHUNKS {
        let tone = blocks(kind, "Dumped", 0);
        let read = read(&dump(kind, temporary(kind, 2), &tone));
        let entry = only_entry(&read);
        assert_eq!((entry.kind(), entry.blocks()), (kind, &tone[..]), "{kind}");
        assert_eq!(entry.slot(), None);
        assert!(read.file_report.is_empty(), "{:?}", read.file_report);
    }
}

#[test]
fn reads_a_studio_set() {
    let kind = ChunkType::StudioSet;
    let set = blocks(kind, "Dumped Set", 0);
    let read = read(&dump(kind, temporary(kind, 0), &set));
    assert_eq!(only_entry(&read).name(), "Dumped Set");
}

#[test]
fn reads_user_slots_like_temporary() {
    let kind = ChunkType::PcmDrumKit;
    let memory = user_memory(kind).unwrap();
    let kit = blocks(kind, "User Kit", 0);
    let user = read(&dump(kind, memory.user_address(31).unwrap(), &kit));
    let temp = read(&dump(kind, temporary(kind, 9), &kit));
    assert_eq!(user, temp);
    assert_eq!(only_entry(&user).slot(), None);
}

#[test]
fn ignores_the_device_id() {
    let mut bytes = dump(SNA, temporary(SNA, 0), &blocks(SNA, "Dev", 0));
    for (i, _) in bytes
        .clone()
        .iter()
        .enumerate()
        .filter(|(_, b)| **b == 0xF0)
    {
        bytes[i + 2] = 0x1F;
    }
    assert_eq!(only_entry(&read(&bytes)).name(), "Dev");
}

#[test]
fn rebuilds_blocks_from_addresses() {
    let tone = blocks(SNA, "Spanning", 0x11);
    let memory = user_memory(SNA).unwrap();
    let base = temporary(SNA, 4);
    let layout: Vec<_> = memory.blocks(base).collect();

    // One DT1 from Common through MFX; the gap between them is junk.
    let (mfx, mfx_size) = layout[1];
    let span = (mfx.value() + mfx_size.value() - base.value()) as usize;
    let mut data = vec![0x7F; span];
    for ((address, _), block) in layout.iter().zip(&tone) {
        let at = (address.value() - base.value()) as usize;
        data[at..at + block.len()].copy_from_slice(block);
    }
    let spanning = build_dt1(0x10, &base, &data);
    assert_eq!(only_entry(&read(&spanning)).blocks(), tone);

    // MFX first, then Common split in two, the halves out of order.
    let (head, tail) = tone[0].split_at(10);
    let split: Vec<u8> = [
        build_dt1(0x10, &mfx, &tone[1]),
        build_dt1(0x10, &base.offset([0, 0, 0, 10]), tail),
        build_dt1(0x10, &base, head),
    ]
    .concat();
    assert_eq!(only_entry(&read(&split)).blocks(), tone);
}

#[test]
fn reads_several_entries_and_drops_duplicates() {
    let a = dump(SNS, temporary(SNS, 0), &blocks(SNS, "Tone A", 0));
    let b = dump(SNS, temporary(SNS, 0), &blocks(SNS, "Tone B", 0));
    let set = dump(
        ChunkType::StudioSet,
        temporary(ChunkType::StudioSet, 0),
        &blocks(ChunkType::StudioSet, "Set", 0),
    );
    let read = read(&[&a[..], &b, &set, &a].concat());
    let names: Vec<_> = read.pack.entries.iter().map(PackEntry::name).collect();
    assert_eq!(names, ["Tone A", "Tone B", "Set"]);
    assert_eq!(read.reports.len(), 3);
    assert_eq!(read.file_report, [FileItem::Duplicates { count: 1 }]);
}

#[test]
fn drops_bad_checksums_and_reports_incomplete_entries() {
    let mut bytes = dump(SNA, temporary(SNA, 0), &blocks(SNA, "Broken", 0));
    // The last byte of the first message is its F7; corrupt its checksum.
    let first_end = bytes.iter().position(|&b| b == 0xF7).unwrap();
    bytes[first_end - 1] ^= 1;
    let read = read(&bytes);
    assert!(read.pack.entries.is_empty());
    assert_eq!(
        read.file_report,
        [
            FileItem::BadChecksum { count: 1 },
            skipped(Skipped::Incomplete, 1)
        ]
    );
}

#[test]
fn skips_other_messages() {
    let gm_on = [0xF0, 0x7E, 0x7F, 0x09, 0x01, 0xF7];
    let rq1 = crate::sysex::build_rq1(
        0x10,
        &Address::new(0x19, 0, 0, 0),
        &crate::address::DataSize::ONE,
    );
    // System Common, outside every entry.
    let system = build_dt1(0x10, &Address::new(0x02, 0, 0, 0), &[0]);
    let read = read(&[&gm_on[..], &rq1, &system].concat());
    assert!(read.pack.entries.is_empty());
    assert_eq!(read.file_report, [skipped(Skipped::Other, 3)]);
}

/// A Roland DT1 of another model.
fn foreign(model: &[u8], address: [u8; 4], data: &[u8]) -> Vec<u8> {
    let payload: Vec<u8> = address.iter().chain(data).copied().collect();
    let sum = crate::sysex::checksum(&payload);
    [&[0xF0, 0x41, 0x10], model, &[0x12], &payload, &[sum, 0xF7]].concat()
}

#[test]
fn skips_foreign_units_until_they_convert() {
    let xv = xv::MODEL;
    let jv = jv::MODEL;
    let bytes = [
        // Two XV Patches dumped to the same Temporary Patch: Common comes again.
        foreign(xv, [0x1F, 0, 0, 0], &[0x20; 79]),
        foreign(xv, [0x1F, 0, 0x20, 0], &[0; 137]),
        foreign(xv, [0x1F, 0, 0, 0], &[0x20; 79]),
        // One XV User Rhythm, Common and its last Rhythm Tone.
        foreign(xv, [0x40, 0x10, 0, 0], &[0x20; 18]),
        foreign(xv, [0x40, 0x11, 0x3E, 0], &[0; 193]),
        // A JV User Patch and a JV Performance.
        foreign(jv, [0x11, 0x05, 0, 0], &[0x20; 72]),
        foreign(jv, [0x11, 0x05, 0x10, 0], &[0; 129]),
        foreign(jv, [0x10, 0x00, 0, 0], &[0x20; 64]),
        // A model we don't know.
        foreign(&[0x46], [0x00, 0, 0, 0], &[0]),
    ]
    .concat();
    let read = read(&bytes);
    assert!(read.pack.entries.is_empty());
    assert_eq!(
        read.file_report,
        [
            skipped(Skipped::Other, 1),
            skipped(Skipped::Patch, 3),
            skipped(Skipped::Rhythm, 1),
            skipped(Skipped::Performance, 1),
        ]
    );
}

#[test]
fn reports_bytes_outside_messages() {
    let mut bytes = dump(SNA, temporary(SNA, 0), &blocks(SNA, "Trailing", 0));
    bytes.extend_from_slice(b"\r\n");
    let read = read(&bytes);
    assert_eq!(only_entry(&read).name(), "Trailing");
    assert_eq!(read.file_report, [FileItem::Unreadable]);
}

#[test]
fn routes_svd_files_on_their_model() {
    let pack = Pack {
        entries: vec![PackEntry::new(SNS, None, blocks(SNS, "From SVD", 0)).unwrap()],
    };
    let svd = pack.to_svd().unwrap();
    assert_eq!(read(&svd).pack, Pack::from_svd(&svd).unwrap());

    let mut xv = svd.clone();
    xv[0x14..0x18].copy_from_slice(b"XV88");
    let mut jv = svd.clone();
    jv[2..6].copy_from_slice(b"SVD0");
    for bytes in [xv, jv] {
        assert!(matches!(
            Pack::read(&bytes),
            Err(PackError::UnsupportedFormat(_))
        ));
    }
    assert!(matches!(Pack::read(b"RIFF"), Err(PackError::UnknownFormat)));
}

#[test]
fn every_part_has_a_temporary_area() {
    let areas = integra::areas();
    for part in 0..INTEGRA7.part_count {
        let base = temporary(SNS, part);
        assert!(integra::area_of(&areas, base).is_some(), "Part {part}");
    }
}

#[cfg(feature = "smf")]
mod smf {
    use super::*;
    use midly::num::{u15, u28};
    use midly::{Format, Header, Smf, Timing, TrackEvent, TrackEventKind};

    fn event(kind: TrackEventKind<'_>) -> TrackEvent<'_> {
        TrackEvent {
            delta: u28::new(0),
            kind,
        }
    }

    fn smf(tracks: Vec<Vec<TrackEvent<'_>>>) -> Vec<u8> {
        let mut smf = Smf::new(Header::new(
            Format::Parallel,
            Timing::Metrical(u15::new(96)),
        ));
        smf.tracks = tracks;
        let mut out = Vec::new();
        smf.write_std(&mut out).unwrap();
        out
    }

    #[test]
    fn reads_sysex_split_across_f0_and_f7_events() {
        let tone = blocks(SNA, "From MIDI", 0);
        let bytes = dump(SNA, temporary(SNA, 1), &tone);
        let first_end = bytes.iter().position(|&b| b == 0xF7).unwrap();
        let (first, second) = bytes.split_at(first_end + 1);
        let (head, tail) = first.split_at(20);
        let file = smf(vec![
            vec![event(TrackEventKind::Meta(midly::MetaMessage::EndOfTrack))],
            vec![
                // `F0` events leave out the F0 itself.
                event(TrackEventKind::SysEx(&head[1..])),
                event(TrackEventKind::Escape(tail)),
                event(TrackEventKind::SysEx(&second[1..])),
            ],
        ]);
        let read = read(&file);
        assert_eq!(only_entry(&read).blocks(), tone);
        assert!(read.file_report.is_empty(), "{:?}", read.file_report);
    }

    #[test]
    fn running_status_before_any_status_is_unreadable() {
        let tone = blocks(SNA, "Then Junk", 0);
        let bytes = dump(SNA, temporary(SNA, 0), &tone);
        let messages: Vec<&[u8]> = frame::split(&bytes).messages;
        let events = messages
            .iter()
            .map(|m| event(TrackEventKind::SysEx(&m[1..])))
            .collect();
        let mut file = smf(vec![events]);
        // A second track whose first event has no status byte.
        file.extend_from_slice(b"MTrk\0\0\0\x03\x00\x40\x40");
        let read = read(&file);
        assert_eq!(only_entry(&read).name(), "Then Junk");
        assert_eq!(read.file_report, [FileItem::Unreadable]);
    }

    #[test]
    fn a_malformed_header_is_unreadable() {
        let read = read(b"MThd\0\0");
        assert!(read.pack.entries.is_empty());
        assert_eq!(read.file_report, [FileItem::Unreadable]);
    }
}
