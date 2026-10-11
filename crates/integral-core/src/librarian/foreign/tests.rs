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
fn skips_foreign_units_with_no_converter() {
    let xv = xv::MODEL;
    let jv = jv::MODEL;
    let bytes = [
        // Two incomplete XV Patches dumped to the same Temporary Patch:
        // Common comes again.
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
            skipped(Skipped::Incomplete, 2),
            skipped(Skipped::Rhythm, 1),
            skipped(Skipped::Patch, 1),
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

mod xv_patches {
    use super::*;
    use crate::librarian::foreign::wave_map::{ForeignWave, ForeignWaveGroup};
    use crate::parse_helpers::nibble4;

    /// User Patch 001 `United !` of Roland's `SRX03AP2.mid` (see the
    /// fixture's `SOURCE.md`).
    const UNITED: &[u8] = include_bytes!("../../../tests/fixtures/foreign/xv/srx03ap2-001.syx");

    /// The data of each DT1 in `bytes`.
    fn data(bytes: &[u8]) -> Vec<Vec<u8>> {
        frame::split(bytes)
            .messages
            .into_iter()
            .map(|m| frame::parse(m).unwrap().data.to_vec())
            .collect()
    }

    /// An XV Patch of 9 DT1s at `key`: tone 1 on and playing INT Wave 1,
    /// every other byte 0, then `edit`ed. Blocks are Common, MFX, Chorus,
    /// Reverb, TMT and Tones 1–4.
    fn patch(key: [u8; 2], edit: impl FnOnce(&mut [Vec<u8>])) -> Vec<u8> {
        let mut blocks: Vec<Vec<u8>> = [79, 145, 52, 83, 41, 137, 137, 137, 137]
            .into_iter()
            .map(|n| vec![0; n])
            .collect();
        blocks[0][..12].copy_from_slice(b"Byte Built  ");
        blocks[0][0x1F..0x21].copy_from_slice(&[0x07, 0x08]); // Tempo 120
        blocks[0][0x27] = 13; // Patch Output Assign TONE
        blocks[1][0x01] = 127; // MFX Dry Send Level
        blocks[4][0x05] = 1; // TMT1 Tone Switch
        blocks[5][0x2F] = 1; // Wave Number L 1
        edit(&mut blocks);
        let offsets = [0x00, 0x02, 0x04, 0x06, 0x10, 0x20, 0x22, 0x24, 0x26];
        offsets
            .into_iter()
            .zip(&blocks)
            .flat_map(|(c, data)| foreign(xv::MODEL, [key[0], key[1], c, 0], data))
            .collect()
    }

    fn converted(bytes: &[u8]) -> (PackEntry, ConversionReport) {
        let mut read = read(bytes);
        assert_eq!(read.pack.entries.len(), 1, "{:?}", read.file_report);
        assert!(read.file_report.is_empty(), "{:?}", read.file_report);
        (read.pack.entries.remove(0), read.reports.remove(0))
    }

    #[test]
    fn the_fixture_is_one_patch_of_9_dt1s() {
        assert_eq!(UNITED.len(), 1056);
        assert_eq!(frame::split(UNITED).messages.len(), 9);
    }

    #[test]
    fn converts_the_fixture() {
        let xv = data(UNITED);
        let (entry, report) = converted(UNITED);

        // The blocks copy byte for byte, then these fields change.
        let mut common = xv[0].clone();
        common.push(0); // Common `00 4F`, a reserve the XV doesn't have
        let mut mfx = xv[1].clone();
        mfx[0x00] = 0; // MFX Type 6 → THRU
        mfx[0x04] = 0; // MFX Output Assign (a reserve) cleared
        let pmt = xv[4].clone();
        let partials: Vec<Vec<u8>> = (0..4)
            .map(|i| {
                let mut p = xv[5 + i].clone();
                p.push(0); // LFO Step Type
                p.extend([64; 16]); // LFO Steps 1–16
                match i {
                    // SRX-03 Waves through the MFX: Wave Group Type SRX, and
                    // the "(MFX)" sends 0/30 become the sends.
                    0 | 1 => {
                        p[0x27] = 1;
                        p[0x0D..=0x11].copy_from_slice(&[127, 127, 0, 30, 0]);
                    }
                    // INT Waves to output A: the "(non MFX)" sends stay,
                    // and the unused pair already holds 127.
                    2 => assert_eq!(p[0x0D..=0x11], [127, 127, 50, 80, 1]),
                    _ => assert_eq!(p[0x0D..=0x11], [127, 127, 0, 80, 1]),
                }
                p
            })
            .collect();
        let mut common2 = b"United !        ".to_vec();
        common2.extend([1, 0, 0, 64]); // Ac.Piano, Tone Number 0, Phrase Oct 0
        common2.resize(60, 0);
        let expected = [vec![common, mfx, pmt], partials, vec![common2]].concat();

        assert_eq!(entry.kind(), ChunkType::PcmSynthTone);
        assert_eq!(entry.name(), "United !");
        assert_eq!(entry.category(), Some(1));
        assert_eq!(entry.blocks(), expected);
        // Waves: SRX-03 1/2 and 7/8, INT 420/421 and 414/415.
        let waves: Vec<_> = expected[3..7]
            .iter()
            .map(|p| {
                (
                    p[0x27],
                    nibble4(p, 0x28),
                    nibble4(p, 0x2C),
                    nibble4(p, 0x30),
                )
            })
            .collect();
        assert_eq!(
            waves,
            [
                (1, 3, 1, 2),
                (1, 3, 7, 8),
                (0, 1, 420, 421),
                (0, 1, 414, 415)
            ]
        );
        let texts: Vec<String> = report.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [
                "effect dropped, MFX set to THRU",
                "Chorus CHORUS, level 127 not applied; set it in the Studio Set",
                "Reverb SRV HALL, level 127 not applied; set it in the Studio Set",
            ]
        );
    }

    #[test]
    fn rejects_multi_partial_patches() {
        let multi = patch([0x30, 0x05], |b| b[0][0x0D] = 1);
        // A Split Key block, which sits where the INTEGRA-7 has Common 2.
        let split_key = foreign(xv::MODEL, [0x30, 0x05, 0x30, 0], &[0; 12]);
        let read = read(&[multi, split_key].concat());
        assert!(read.pack.entries.is_empty());
        assert_eq!(read.file_report, [skipped(Skipped::MultiPartial, 1)]);
    }

    #[test]
    fn maps_sr_jv80_waves_onto_srx() {
        // Part 5's Temporary Patch; tone 1 plays SR-JV80-02 Waves 1 and 2.
        let bytes = patch([0x12, 0x00], |b| {
            b[5][0x27] = 1;
            b[5][0x28..0x34].copy_from_slice(&[0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 2]);
        });
        let (entry, report) = converted(&bytes);
        let p = &entry.blocks()[3];
        assert_eq!(
            (
                p[0x27],
                nibble4(p, 0x28),
                nibble4(p, 0x2C),
                nibble4(p, 0x30)
            ),
            (1, 6, 29, 30)
        );
        assert!(report.is_empty(), "{report:?}");
    }

    #[test]
    fn reports_waves_it_cannot_map_and_mutes_their_partial() {
        // SR-JV80-01 Wave 1, with no match, and a SAMPLE Wave.
        let bytes = patch([0x1F, 0x00], |b| {
            b[5][0x27] = 1;
            b[5][0x2B] = 1;
            b[4][0x05 + 9] = 1;
            b[6][0x27] = 3;
            b[6][0x2F] = 4;
        });
        let (entry, report) = converted(&bytes);
        let pmt = &entry.blocks()[2];
        assert_eq!((pmt[0x05], pmt[0x05 + 9]), (0, 0));
        assert_eq!(
            report,
            [
                ConversionItem::WaveUnmapped {
                    partial: 1,
                    source: ForeignWave {
                        group: ForeignWaveGroup::SrJv80(1),
                        number: 1
                    },
                },
                ConversionItem::WaveUnmapped {
                    partial: 2,
                    source: ForeignWave {
                        group: ForeignWaveGroup::Sample,
                        number: 4
                    },
                },
            ]
        );
    }

    #[test]
    fn reports_settings_that_live_in_the_studio_set() {
        let bytes = patch([0x30, 0x00], |b| {
            b[0][0x1F..0x21].copy_from_slice(&[0x06, 0x0E]); // Tempo 110, PATCH
            b[0][0x27] = 1; // Patch Output Assign A
            b[0][0x21] = 1; // One Shot Mode ON
            b[2][..2].copy_from_slice(&[2, 0]); // DELAY at level 0: silent
            b[3][..2].copy_from_slice(&[1, 90]); // REVERB, level 90
            b[5][0x15] = 1; // Receive Pan Mode KEY-ON
        });
        let (entry, report) = converted(&bytes);
        assert_eq!(entry.blocks()[0][0x27], 13);
        let texts: Vec<String> = report.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [
                "Tempo 110 not applied; set it in the Studio Set",
                "One Shot Mode dropped",
                "Output Assign A not applied; set it in the Studio Set",
                "Reverb REVERB, level 90 not applied; set it in the Studio Set",
                "Partial 1: Receive Pan Mode dropped",
            ]
        );
    }

    #[test]
    fn a_clean_patch_has_an_empty_report() {
        let (entry, report) = converted(&patch([0x30, 0x7F], |_| {}));
        assert_eq!(entry.name(), "Byte Built");
        assert!(report.is_empty(), "{report:?}");
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
