//! Pack reading, from bytes built in the tests.

use super::*;
use crate::address::Address;
use crate::device_spec::INTEGRA7;
use crate::librarian::pack::{PackEntry, user_memory};
use crate::parse_helpers::encode_ascii_name;
use crate::svd::{ChunkType, SvdError};
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
fn records_the_file_format() {
    let tone = blocks(SNS, "Dumped", 0);
    let syx = read(&dump(SNS, temporary(SNS, 0), &tone));
    assert_eq!(syx.format, PackFormat::Sysex);
    assert_eq!(syx.format.label(Origin::Sound), "SysEx dump");
    let svd = read(&syx.pack.to_svd().unwrap());
    assert_eq!(svd.format, PackFormat::Svd);
    assert_eq!(svd.format.label(Origin::Sound), "sound export");
    assert_eq!(svd.format.label(Origin::Backup), "backup");
    assert_eq!(PackFormat::Midi.label(Origin::Sound), "MIDI file");
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
        // An incomplete XV User Rhythm: Common and its last Rhythm Tone.
        foreign(xv, [0x40, 0x10, 0, 0], &[0x20; 18]),
        foreign(xv, [0x40, 0x11, 0x3E, 0], &[0; 193]),
        // An incomplete JV User Patch and a JV Performance.
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
            skipped(Skipped::Incomplete, 4),
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

    // An XV model routes to the XV reader, which rejects INTEGRA-7 chunks.
    let mut xv = svd.clone();
    xv[0x14..0x18].copy_from_slice(b"XV88");
    assert!(matches!(
        Pack::read(&xv),
        Err(PackError::Svd(SvdError::InvalidChunkType(code))) if code == *b"PRFb"
    ));
    let mut jv = svd.clone();
    jv[2..6].copy_from_slice(b"SVD0");
    assert!(matches!(
        Pack::read(&jv),
        Err(PackError::UnsupportedFormat(_))
    ));
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
    use crate::svd_convert::pack_sections;
    use crate::svd_specs::XV_PATCH_SECTIONS;

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
                "MFX type 6 dropped, MFX set to THRU",
                "Chorus CHORUS, level 127 not applied; set it in the Studio Set",
                "Reverb SRV HALL, level 127 not applied; set it in the Studio Set",
            ]
        );
    }

    /// An `.svd` chunk: code, model, entry size and entries.
    type Chunk = (&'static [u8; 4], &'static [u8; 4], usize, Vec<Vec<u8>>);

    /// An XV `.svd` file with the given chunks.
    fn xv_svd(chunks: &[Chunk]) -> Vec<u8> {
        let be = |n: usize| u32::try_from(n).unwrap().to_be_bytes();
        let dir_end = 16 + 16 * chunks.len();
        let mut header = u16::try_from(dir_end - 2).unwrap().to_be_bytes().to_vec();
        header.extend_from_slice(b"SVD1");
        header.resize(16, 0);
        let mut data = Vec::new();
        for (code, model, size, entries) in chunks {
            let chunk_size = 16 + size * entries.len();
            header.extend_from_slice(*code);
            header.extend_from_slice(*model);
            header.extend_from_slice(&be(dir_end + data.len()));
            header.extend_from_slice(&be(chunk_size));
            data.extend_from_slice(&be(entries.len()));
            data.extend_from_slice(&be(*size));
            data.extend_from_slice(&[0, 0, 0, 0x10, 0, 0, 0, 0]);
            entries.iter().for_each(|e| data.extend_from_slice(e));
        }
        [header, data].concat()
    }

    #[test]
    fn reads_an_xv_svd_patch_like_its_sysex_dump() {
        // The fixture's blocks, bit-packed into a 956-byte entry: no Split
        // Key data, then the 18-byte block seen in 12-chunk files.
        let mut entry = pack_sections(&data(UNITED), &XV_PATCH_SECTIONS);
        assert_eq!(entry.len(), 574);
        // "United !" as 7-bit characters, MSB first.
        assert_eq!(entry[..7], [0xAB, 0xBB, 0x4F, 0x4C, 0xB9, 0x10, 0x21]);
        entry.extend_from_slice(&[
            0x02, 0x00, 0xFE, 0, 0, 0, 0, 0x20, 0x20, 0x20, 0x20, 0, 0, 0x32, 0x32, 0x80, 0x80, 0,
        ]);
        entry.resize(956, 0);
        let svd = xv_svd(&[
            (b"PRFa", b"NEXY", 0, vec![]),
            (b"MLTa", b"NEXY", 2, vec![vec![0; 2]; 3]),
            (b"PATa", b"XV88", 956, vec![entry.clone(), entry]),
            (b"CTGa", b"XV88", 1, vec![vec![1]; 2]),
            (b"RHYa", b"XV88", 1, vec![vec![0]]),
            (b"SYSa", b"XV88", 1, vec![vec![0]]),
        ]);

        let read = read(&svd);
        let sysex = super::read(UNITED);
        assert_eq!(read.format.label(Origin::Sound), "XV SVD file");
        assert_eq!(read.pack, sysex.pack);
        assert_eq!(read.reports, sysex.reports);
        assert_eq!(
            read.file_report,
            [
                skipped(Skipped::Performance, 3),
                skipped(Skipped::Rhythm, 1),
                skipped(Skipped::System, 1),
                FileItem::Duplicates { count: 1 },
            ]
        );
    }

    #[test]
    fn a_short_xv_svd_entry_is_incomplete() {
        let svd = xv_svd(&[(b"PATa", b"XV88", 4, vec![vec![0; 4]])]);
        let read = read(&svd);
        assert!(read.pack.entries.is_empty());
        assert_eq!(read.file_report, [skipped(Skipped::Incomplete, 1)]);
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
                    at: Location::Partial(1),
                    source: ForeignWave {
                        group: ForeignWaveGroup::SrJv80(1),
                        number: 1
                    },
                },
                ConversionItem::WaveUnmapped {
                    at: Location::Partial(2),
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
            b[5][0x38] = 1; // Wave Tempo Sync
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
    fn reports_the_tempo_only_when_a_tone_syncs_to_it() {
        let tempo = |edit: fn(&mut [Vec<u8>])| {
            let (_, report) = converted(&patch([0x30, 0x00], |b| {
                b[0][0x1F..0x21].copy_from_slice(&[0x06, 0x0E]); // Tempo 110, PATCH
                edit(b);
            }));
            report.iter().map(ToString::to_string).collect::<Vec<_>>()
        };
        assert!(tempo(|_| {}).is_empty());
        // Tone 2 syncs, but is off.
        assert!(tempo(|b| b[6][0x38] = 1).is_empty());
        let reported = ["Tempo 110 not applied; set it in the Studio Set"];
        assert_eq!(
            tempo(|b| b[5][0x6E..0x70].copy_from_slice(&[0x08, 0x02])),
            reported
        ); // LFO1 Rate 1/16
        assert_eq!(
            tempo(|b| b[5][0x7C..0x7E].copy_from_slice(&[0x08, 0x02])),
            reported
        ); // LFO2 Rate
        assert_eq!(
            tempo(|b| b[5][0x0A..0x0C].copy_from_slice(&[0x08, 0x02])),
            reported
        ); // Delay Time
        // Clock Source SYSTEM: the Patch Tempo is unused.
        assert!(
            tempo(|b| {
                b[5][0x38] = 1;
                b[0][0x1E] = 1;
            })
            .is_empty()
        );
    }

    #[test]
    fn a_clean_patch_has_an_empty_report() {
        let (entry, report) = converted(&patch([0x30, 0x7F], |_| {}));
        assert_eq!(entry.name(), "Byte Built");
        assert!(report.is_empty(), "{report:?}");
    }
}

mod jv_patches {
    use super::*;
    use crate::librarian::foreign::wave_map::{ForeignWave, ForeignWaveGroup};
    use crate::parse_helpers::{nibble2, nibble4};

    /// User Patches 002 `AwesomeChoir`, 007 `Bright Lead`, 041 `Softpad`
    /// and 045 `Winds Of Chg`, cut from the public-domain `80TO1080.SYX`
    /// (see the fixture's `SOURCE.md`). All four are JV-1080 Patches.
    const BANK: &[u8] = include_bytes!("../../../tests/fixtures/foreign/jv/80to1080-cut.syx");

    /// Offsets (address byte 3) of Patch Common and Tones 1–4.
    const OFFSETS: [u8; 5] = [0x00, 0x10, 0x12, 0x14, 0x16];

    /// A JV Patch of 5 DT1s at `key` with a Common of `common_len` bytes:
    /// Tone 1 on and playing INT-A Wave 1, every setting at its JV INIT or
    /// a neutral value, then `edit`ed. Blocks are Common and Tones 1–4.
    fn patch(key: [u8; 2], common_len: usize, edit: impl FnOnce(&mut [Vec<u8>])) -> Vec<u8> {
        let mut blocks = vec![vec![0u8; common_len]];
        blocks.extend((0..4).map(|_| vec![0u8; 0x81]));
        blocks[0][..12].copy_from_slice(b"Byte Built  ");
        blocks[0][0x0C..0x2E].copy_from_slice(&jv::INIT_EFFECTS);
        for tone in &mut blocks[1..] {
            tone[0x0C..0x10].copy_from_slice(&[1, 127, 0, 127]); // Velocity, Key ranges
            tone[0x11] = 1; // Volume Control ON
            tone[0x14] = 1; // Pan Control CONTINUOUS
        }
        blocks[1][0x00] = 1; // Tone 1 Switch
        blocks[1][0x01..0x03].copy_from_slice(&[0, 1]); // INT-A
        edit(&mut blocks);
        OFFSETS
            .into_iter()
            .zip(&blocks)
            .flat_map(|(c, data)| foreign(jv::MODEL, [key[0], key[1], c, 0], data))
            .collect()
    }

    fn converted(bytes: &[u8]) -> (PackEntry, ConversionReport) {
        let mut read = read(bytes);
        assert_eq!(read.pack.entries.len(), 1, "{:?}", read.file_report);
        assert!(read.file_report.is_empty(), "{:?}", read.file_report);
        (read.pack.entries.remove(0), read.reports.remove(0))
    }

    fn texts(report: &ConversionReport) -> Vec<String> {
        report.iter().map(ToString::to_string).collect()
    }

    /// Matrix Controls 1–4 (36 bytes), and each partial's 16 Control
    /// Switches.
    fn matrix(entry: &PackEntry) -> (Vec<u8>, Vec<Vec<u8>>) {
        let blocks = entry.blocks();
        let switches = blocks[3..7].iter().map(|p| p[0x17..0x27].to_vec());
        (blocks[0][0x2B..0x4F].to_vec(), switches.collect())
    }

    /// Matrix Controls from `(source, [(destination, sens)])`, unused slots
    /// OFF at sens 0.
    fn controls(mcs: &[(u8, &[(u8, u8)])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for m in 0..4 {
            let (source, slots) = mcs.get(m).copied().unwrap_or((0, &[]));
            bytes.push(source);
            for k in 0..4 {
                let (dest, sens) = slots.get(k).copied().unwrap_or((0, 64));
                bytes.extend([dest, sens]);
            }
        }
        bytes
    }

    fn waves(partial: &[u8]) -> (u8, u16, u16, u16) {
        (
            partial[0x27],
            nibble4(partial, 0x28),
            nibble4(partial, 0x2C),
            nibble4(partial, 0x30),
        )
    }

    #[test]
    fn the_fixture_is_four_patches_of_5_dt1s() {
        assert_eq!(BANK.len(), 4 * 643);
        assert_eq!(frame::split(BANK).messages.len(), 20);
    }

    #[test]
    fn converts_the_fixture() {
        let read = read(BANK);
        assert!(read.file_report.is_empty(), "{:?}", read.file_report);
        let names: Vec<String> = read.pack.entries.iter().map(PackEntry::name).collect();
        assert_eq!(
            names,
            ["AwesomeChoir", "Bright Lead", "Softpad", "Winds Of Chg"]
        );
        assert!(
            read.pack
                .entries
                .iter()
                .all(|e| e.kind() == ChunkType::PcmSynthTone && e.category() == Some(0))
        );
        let studio = " not applied; set it in the Studio Set";
        let reports: Vec<Vec<String>> = read.reports.iter().map(texts).collect();
        assert_eq!(
            reports,
            [
                vec![
                    format!(
                        "Chorus level 127, rate 16, depth 30, pre-delay 69, feedback 0, output REV{studio}"
                    ),
                    format!("Reverb PAN-DLY, level 90, time 127, HF damp BYP, feedback 25{studio}"),
                ],
                vec![
                    format!(
                        "Chorus level 70, rate 24, depth 28, pre-delay 69, feedback 0, output MIX{studio}"
                    ),
                    format!(
                        "Reverb STAGE2, level 100, time 60, HF damp 4000Hz, feedback 0{studio}"
                    ),
                    "Partial 3: PCM card 3 Wave 30 has no INTEGRA-7 Wave".into(),
                ],
                vec![
                    format!(
                        "Chorus level 80, rate 12, depth 11, pre-delay 69, feedback 0, output MIX{studio}"
                    ),
                    format!(
                        "Reverb STAGE2, level 127, time 94, HF damp 4000Hz, feedback 0{studio}"
                    ),
                ],
                vec![
                    format!(
                        "Chorus level 127, rate 45, depth 13, pre-delay 69, feedback 0, output REV{studio}"
                    ),
                    format!(
                        "Reverb STAGE2, level 127, time 127, HF damp 4000Hz, feedback 42{studio}"
                    ),
                    "Partial 3: Delay Mode KEY-INTERVAL dropped".into(),
                ],
            ]
        );

        let [choir, lead, pad, winds] = &read.pack.entries[..] else {
            unreachable!()
        };
        // SR-JV80-04 Waves 126 and 128 land on SRX-07, INT-A 249 on INT.
        let choir_waves: Vec<_> = choir.blocks()[3..7].iter().map(|p| waves(p)).collect();
        assert_eq!(
            choir_waves,
            [
                (1, 7, 603, 0),
                (1, 7, 605, 0),
                (1, 7, 604, 0),
                (0, 1, 464, 0)
            ]
        );
        // Tone 3 plays a PCM-card Wave: no Wave, switched off.
        let pmt = &lead.blocks()[2];
        assert_eq!([pmt[0x05], pmt[0x0E], pmt[0x17], pmt[0x20]], [1, 1, 0, 0]);
        assert_eq!(waves(&lead.blocks()[5]), (0, 1, 0, 0));

        // Controllers: MOD, then AFT and EXPRESSION, packed by first use.
        let on = [1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            matrix(choir),
            (
                controls(&[
                    (1, &[(9, 70)]),
                    (97, &[(4, 74), (9, 74), (11, 63), (17, 75)]),
                    (11, &[(4, 87)]),
                ]),
                vec![on.to_vec(), on.to_vec(), vec![0; 16], on.to_vec()]
            )
        );
        // AFT needs two Matrix Controls, so all four are in use.
        assert_eq!(
            matrix(pad),
            (
                controls(&[
                    (1, &[(9, 76), (9, 75)]),
                    (97, &[(9, 78), (2, 80), (9, 72), (17, 73)]),
                    (97, &[(4, 84), (10, 56), (18, 71), (9, 76)]),
                    (11, &[(2, 85)]),
                ]),
                vec![
                    vec![1, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
                    vec![1, 0, 0, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0],
                    vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0],
                    vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0],
                ]
            )
        );
        // Tone 4 has the negated MOD depth: REVERSE.
        let shared = [1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut reversed = shared;
        reversed[0] = 2;
        assert_eq!(
            matrix(winds),
            (
                controls(&[(1, &[(9, 73)]), (97, &[(2, 77), (9, 73)])]),
                vec![
                    vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    shared.to_vec(),
                    shared.to_vec(),
                    reversed.to_vec(),
                ]
            )
        );
        // Velocity Range Switch ON: ranges copied, Velocity Control ON.
        let pmt = &winds.blocks()[2];
        assert_eq!(pmt[0x04], 1);
        assert_eq!(pmt[0x05 + 9 + 5..0x05 + 9 + 9], [71, 127, 0, 0]);
        // KEY-INTERVAL: NORMAL with no delay.
        let partial = &winds.blocks()[5];
        assert_eq!((partial[0x09], nibble2(partial, 0x0A)), (0, 0));
    }

    /// Roland's own conversion of SR-JV80-12 Patch `Spooky Hits` (JV-2080
    /// format, User 059 of `HipHop2.syx`) into XV format (`SRJV12PB.mid`,
    /// User 059, from Roland's 2002 XV patch sets for SR-JV80 boards). Only
    /// the controller bytes, taken from the corpus pair family R02 of
    /// `docs/research/jv-xv-roland-pairs.md`.
    mod spooky_hits {
        /// JV Patch Control Source 2 and 3: SYS-CTRL1, SYS-CTRL2.
        pub const JV_SOURCES: [u8; 2] = [1, 2];
        /// JV Tone 1–4 Controller 1–3 Destination/Depth 1–4 (`15`–`2C`);
        /// all four Tones are on.
        pub const JV_CONTROLLERS: [[u8; 24]; 4] = [
            [
                0x04, 0x30, 0x00, 0x3F, 0x00, 0x3F, 0x00, 0x3F, 0x09, 0x48, 0x00, 0x3F, 0x00, 0x3F,
                0x00, 0x3F, 0x04, 0x59, 0x00, 0x3F, 0x00, 0x3F, 0x00, 0x3F,
            ],
            [
                0x08, 0x7E, 0x03, 0x24, 0x01, 0x00, 0x00, 0x3F, 0x09, 0x48, 0x00, 0x3F, 0x00, 0x3F,
                0x00, 0x3F, 0x04, 0x59, 0x00, 0x3F, 0x00, 0x3F, 0x00, 0x3F,
            ],
            [
                0x09, 0x3C, 0x04, 0x3A, 0x02, 0x3A, 0x00, 0x3F, 0x09, 0x48, 0x00, 0x3F, 0x00, 0x3F,
                0x00, 0x3F, 0x04, 0x59, 0x00, 0x3F, 0x00, 0x3F, 0x00, 0x3F,
            ],
            [
                0x04, 0x3D, 0x08, 0x00, 0x02, 0x54, 0x03, 0x0F, 0x09, 0x48, 0x00, 0x3F, 0x00, 0x3F,
                0x00, 0x3F, 0x04, 0x59, 0x00, 0x3F, 0x00, 0x3F, 0x00, 0x3F,
            ],
        ];
        /// XV Patch Common Matrix Control 1–4 (`2B`–`4E`).
        pub const XV_MATRIX: [u8; 36] = [
            1, 4, 49, 8, 127, 3, 37, 1, 1, // MOD
            99, 9, 73, 0, 64, 0, 64, 0, 64, // CTRL2
            100, 4, 90, 0, 64, 0, 64, 0, 64, // CTRL3
            1, 9, 61, 4, 59, 2, 59, 4, 62, // MOD again
        ];
        /// XV Tone 1–4 Control 1–4 Switch 1–4 (`17`–`26`).
        pub const XV_SWITCHES: [[u8; 16]; 4] = [
            [1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
            [0, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0],
            [0, 2, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 1],
        ];
    }

    #[test]
    fn packs_matrix_controls_as_roland_does() {
        let bytes = patch([0x11, 0x3A], 0x4A, |b| {
            b[0][0x3A..0x3C].copy_from_slice(&spooky_hits::JV_SOURCES);
            for (tone, controllers) in b[1..].iter_mut().zip(spooky_hits::JV_CONTROLLERS) {
                tone[0x00] = 1;
                tone[0x15..0x2D].copy_from_slice(&controllers);
            }
        });
        let (entry, report) = converted(&bytes);
        assert_eq!(
            matrix(&entry),
            (
                spooky_hits::XV_MATRIX.to_vec(),
                spooky_hits::XV_SWITCHES.map(|s| s.to_vec()).to_vec()
            )
        );
        // Tone 4's last two MOD routes find no Matrix Control left.
        assert_eq!(
            texts(&report),
            [
                "Partial 4: Control 1 route to CUT (+21) dropped",
                "Partial 4: Control 1 route to RES (-48) dropped",
            ]
        );
    }

    #[test]
    fn tells_the_model_from_the_common_length() {
        let edit = |b: &mut [Vec<u8>]| {
            b[0][0x2C..0x2E].copy_from_slice(&[0x06, 0x04]); // Tempo 100
            b[1][0x34] = 1; // LFO1 External Sync CLOCK
            b[1][0x09] = 2; // KEY-INTERVAL / PLAYMATE
        };
        let model = |bytes: &[u8]| read(bytes).sources;
        let (_, report) = converted(&patch([0x03, 0x00], 0x48, edit));
        assert_eq!(
            model(&patch([0x03, 0x00], 0x48, edit)),
            [Some(SourceModel::Jv1080)]
        );
        assert_eq!(
            model(&patch([0x03, 0x00], 0x49, edit)),
            [Some(SourceModel::Xp)]
        );
        assert_eq!(
            model(&patch([0x03, 0x00], 0x4A, edit)),
            [Some(SourceModel::Jv2080)]
        );
        assert_eq!(
            texts(&report),
            [
                "Partial 1: Delay Mode KEY-INTERVAL dropped",
                "Tempo 100 not applied; set it in the Studio Set",
            ]
        );
        // XP: Clock Source SEQUENCER, so the Patch Tempo is unused.
        let (_, report) = converted(&patch([0x02, 0x00], 0x49, |b| {
            edit(b);
            b[0][0x48] = 1;
        }));
        assert_eq!(texts(&report), ["Partial 1: Delay Mode PLAYMATE dropped"]);
        // JV-2080: Patch Category 2 (EP) carries over by number.
        let (entry, report) = converted(&patch([0x02, 0x0F], 0x4A, |b| {
            edit(b);
            b[0][0x49] = 2;
        }));
        assert_eq!(entry.category(), Some(5));
        assert_eq!(entry.blocks()[0][0x0C], 2);
        assert_eq!(report.len(), 2);
        // A longer Common reads as the JV-2080's; a shorter one is not a
        // Patch.
        assert_eq!(
            converted(&patch([0x11, 0,], 0x4C, |_| {})).0.name(),
            "Byte Built"
        );
        let short = read(&patch([0x11, 0x01], 0x47, |_| {}));
        assert!(short.pack.entries.is_empty());
        assert_eq!(short.file_report, [skipped(Skipped::Incomplete, 1)]);
    }

    #[test]
    fn reports_the_tempo_only_when_a_tone_syncs_to_it() {
        let tempo = |edit: fn(&mut [Vec<u8>])| {
            let (_, report) = converted(&patch([0x11, 0x00], 0x48, |b| {
                b[0][0x2C..0x2E].copy_from_slice(&[0x06, 0x04]); // Tempo 100
                edit(b);
            }));
            texts(&report)
        };
        assert!(tempo(|_| {}).is_empty());
        // Tone 2 syncs, but is off.
        assert!(tempo(|b| b[2][0x3C] = 2).is_empty());
        let reported = ["Tempo 100 not applied; set it in the Studio Set"];
        assert_eq!(tempo(|b| b[1][0x3C] = 2), reported); // LFO2 External Sync TAP
        assert_eq!(tempo(|b| b[1][0x09] = 3), reported); // Delay Mode CLOCK-SYNC
    }

    #[test]
    fn applies_the_mapping_rows() {
        let (entry, report) = converted(&patch([0x11, 0x00], 0x48, |b| {
            b[0][0x33] = 1; // Key Assign SOLO
            b[0][0x41] = 6; // Octave Shift +3
            let t = &mut b[1];
            t[0x08] = 15; // FXM Depth 16
            t[0x2D] = 2; // LFO1 SAW
            t[0x3D] = 60; // Coarse Tune +12
            t[0x41] = 24; // P-Env Depth +12
            t[0x42] = 125; // P-Env Velocity Sens +150 %
            t[0x43] = 13; // P-Env Velocity Time 1 +70 %
            t[0x52] = 15; // Cutoff Keyfollow +200
            t[0x6D] = 0; // Level Time Keyfollow −100
            t[0x4A] = 126; // P-Env Level 1 +63
        }));
        assert!(report.is_empty(), "{report:?}");
        let [common, _, pmt, p, ..] = entry.blocks() else {
            unreachable!()
        };
        assert_eq!((common[0x16], common[0x13], common[0x18]), (0, 67, 1));
        assert_eq!(
            (p[0x37], p[0x6D], p[0x01], p[0x3A], p[0x3B], p[0x3C]),
            (16, 2, 76, 76, 127, 108)
        );
        assert_eq!((p[0x4A], p[0x65], p[0x44]), (84, 54, 127));
        // Start levels: Pitch L0 = 64, TVF L0 = 0.
        assert_eq!((p[0x43], p[0x59]), (64, 0));
        // Velocity Range Switch OFF: 1–127, no fade, Velocity Control ON.
        assert_eq!(pmt[0x04], 1);
        assert_eq!(pmt[0x05..0x0E], [1, 0, 127, 0, 0, 1, 127, 0, 0]);
    }

    #[test]
    fn copies_cross_fade_to_both_widths() {
        let (entry, _) = converted(&patch([0x11, 0x00], 0x48, |b| {
            b[0][0x40] = 1;
            b[1][0x0B..0x0E].copy_from_slice(&[20, 30, 100]);
        }));
        assert_eq!(entry.blocks()[2][0x05 + 5..0x05 + 9], [30, 100, 20, 20]);
    }

    #[test]
    fn converts_synced_delays_and_lfo_rates() {
        let (entry, report) = converted(&patch([0x11, 0x00], 0x48, |b| {
            for (tone, (mode, time)) in b[1..].iter_mut().zip([(3, 24), (4, 5), (7, 120), (5, 10)])
            {
                tone[0x00] = 1;
                tone[0x09..0x0B].copy_from_slice(&[mode, time]);
            }
            b[1][0x2F] = 48; // LFO1 Rate, CLOCK-synced: 1/4
            b[1][0x34] = 1;
            b[1][0x37] = 100; // LFO2 Rate, free
        }));
        assert!(report.is_empty(), "{report:?}");
        let delays: Vec<(u8, u16)> = entry.blocks()[3..7]
            .iter()
            .map(|p| (p[0x09], nibble2(p, 0x0A)))
            .collect();
        assert_eq!(delays, [(0, 137), (0, 130), (0, 149), (2, 10)]);
        let p = &entry.blocks()[3];
        assert_eq!((nibble2(p, 0x6E), nibble2(p, 0x7C)), (140, 100));
    }

    #[test]
    fn reports_what_the_integra_cannot_hold() {
        let (entry, report) = converted(&patch([0x11, 0x00], 0x48, |b| {
            b[0][0x0C] = 5; // EFX ENHANCER
            b[0][0x19] = 1; // EFX Output Assign OUTPUT 1
            b[0][0x3E] = 1; // Control 2 HOLD
            b[0][0x3A] = 15; // Control 2 source PLAYMATE
            let t = &mut b[1];
            t[0x1D..0x1F].copy_from_slice(&[2, 73]); // Control 2 → CUT +10
            t[0x11] = 0; // Volume Control OFF
            t[0x14] = 2; // Pan Control KEY-ON
            t[0x7D] = 1; // Output EFX
            t[0x7F..0x81].copy_from_slice(&[10, 20]);
            b[2][0x00] = 1;
            b[2][0x7D] = 2; // Output OUTPUT 1
            b[2][0x7F..0x81].copy_from_slice(&[30, 40]);
        }));
        assert_eq!(
            texts(&report),
            [
                "EFX type 6 dropped, MFX set to THRU",
                "EFX Output Assign OUTPUT 1 not applied; set it in the Studio Set",
                "Control 2 Hold/Peak dropped",
                "Partial 1: Control 2 route to CUT (+10) dropped",
                "Partial 1: Volume Control Switch dropped",
                "Partial 1: Pan Control Switch dropped",
                "Partial 2 Output Assign OUTPUT 1 not applied; set it in the Studio Set",
            ]
        );
        let p = &entry.blocks()[3..5];
        // Sends, output reserve, Receive Bender/Expression/Hold-1, then
        // the Receive Pan Mode reserve at KEY-ON.
        assert_eq!(p[0][0x0D..=0x15], [127, 127, 10, 20, 0, 0, 1, 0, 1]);
        assert_eq!(p[1][0x0D..=0x11], [127, 127, 30, 40, 1]);
        assert_eq!(entry.blocks()[1][0x00], 0);
    }

    #[test]
    fn a_clean_patch_has_an_empty_report() {
        let (entry, report) = converted(&patch([0x11, 0x7F], 0x48, |_| {}));
        assert_eq!(entry.name(), "Byte Built");
        assert!(report.is_empty(), "{report:?}");
        // INT-A Wave 1 is INT 19.
        assert_eq!(waves(&entry.blocks()[3]), (0, 1, 19, 0));
        let unmapped = ConversionItem::WaveUnmapped {
            at: Location::Partial(1),
            source: ForeignWave {
                group: ForeignWaveGroup::PcmCard(3),
                number: 30,
            },
        };
        assert_eq!(
            unmapped.to_string(),
            "Partial 1: PCM card 3 Wave 30 has no INTEGRA-7 Wave"
        );
    }
}

mod jv_rhythms {
    use super::*;
    use crate::librarian::foreign::wave_map::{ForeignWave, ForeignWaveGroup};
    use crate::parse_helpers::nibble4;
    use crate::pcm_drum::{PcmdPartialField as F, PcmdWmtField as W, pcmd_wmt_byte};

    /// User Rhythm Setup 1.
    const USER_1: [u8; 2] = [0x10, 0x40];

    /// A Rhythm Note of `key` at a neutral value: Tone Switch on, INT-A
    /// Wave 1, Source Key `key`, Velocity Sens 0 %, Velocity Time Sens 0 %,
    /// P-Env flat, output MIX.
    fn note(key: u8) -> Vec<u8> {
        let mut note = vec![0u8; 0x3A];
        note[0x00] = 1; // Tone Switch
        note[0x01..0x03].copy_from_slice(&[0, 1]); // INT-A
        note[0x05] = 1; // Wave Gain 0 dB
        note[0x09..0x0C].copy_from_slice(&[1, 1, 1]); // Volume, Hold-1, Pan CONTINUOUS
        note[0x0C] = key; // Source Key
        note[0x0D] = 50; // Fine Tune 0
        note[0x0F] = 12; // P-Env Depth 0
        for at in [0x10, 0x1D, 0x1F, 0x2A] {
            note[at] = 50; // Velocity Sens 0 %
        }
        for at in [0x11, 0x20, 0x2B] {
            note[at] = 7; // Velocity Time Sens 0 %
        }
        note[0x16..0x1A].fill(63); // P-Env Levels 0
        note[0x1A..0x1C].copy_from_slice(&[1, 127]); // LPF, Cutoff 127
        note[0x1E] = 63; // F-Env Depth 0
        note[0x29] = 127; // Tone Level
        note[0x30..0x33].copy_from_slice(&[127, 127, 0]); // A-Env Levels
        note[0x33] = 64; // Pan
        note[0x35] = 64; // Alternate Pan Depth
        note[0x37] = 127; // Mix/EFX Send Level
        note
    }

    /// A Rhythm Setup at `slot`: Common, then the Notes of `keys` built by
    /// [`note`] and `edit`ed.
    fn kit(
        slot: [u8; 2],
        keys: impl IntoIterator<Item = u8>,
        mut edit: impl FnMut(u8, &mut [u8]),
    ) -> Vec<u8> {
        let mut bytes = foreign(jv::MODEL, [slot[0], slot[1], 0, 0], b"Byte Kit    ");
        for key in keys {
            let mut note = note(key);
            edit(key, &mut note);
            bytes.extend(foreign(jv::MODEL, [slot[0], slot[1], key, 0], &note));
        }
        bytes
    }

    fn converted(bytes: &[u8]) -> (PackEntry, ConversionReport) {
        let mut read = read(bytes);
        assert_eq!(read.pack.entries.len(), 1, "{:?}", read.file_report);
        assert!(read.file_report.is_empty(), "{:?}", read.file_report);
        (read.pack.entries.remove(0), read.reports.remove(0))
    }

    /// The Partial block of `key`.
    fn partial(entry: &PackEntry, key: u8) -> &[u8] {
        &entry.blocks()[3 + usize::from(key - 21)]
    }

    /// WMT `wmt` (0-indexed) of a Partial: switch, Wave Group Type and ID,
    /// Wave Number L.
    fn wmt(partial: &[u8], wmt: usize) -> (u8, u8, u16, u16) {
        (
            partial[pcmd_wmt_byte(wmt, W::Switch)],
            partial[pcmd_wmt_byte(wmt, W::WaveGroupType)],
            nibble4(partial, pcmd_wmt_byte(wmt, W::WaveGroupId)),
            nibble4(partial, pcmd_wmt_byte(wmt, W::WaveNumberL)),
        )
    }

    #[test]
    fn converts_each_key_to_the_same_integra_key() {
        let bytes = kit(USER_1, 35..=98, |key, note| note[0x29] = key);
        let (entry, report) = converted(&bytes);
        assert_eq!(entry.kind(), ChunkType::PcmDrumKit);
        assert_eq!(entry.name(), "Byte Kit");
        assert_eq!(&entry.blocks()[91][..16], b"Byte Kit        ");
        assert!(report.is_empty(), "{report:?}");
        for key in 21..=108u8 {
            let p = partial(&entry, key);
            if (35..=98).contains(&key) {
                assert_eq!(
                    (p[F::CoarseTune as usize], p[F::Level as usize]),
                    (key, key)
                );
                // INT-A Wave 1 is INT 19.
                assert_eq!(wmt(p, 0), (1, 0, 1, 19), "key {key}");
            } else {
                assert_eq!(wmt(p, 0).0, 0, "key {key}");
            }
            assert!((1..4).all(|w| wmt(p, w).0 == 0), "key {key}");
        }
        // The kit has no effects of its own: MFX THRU.
        assert_eq!(entry.blocks()[1][0], 0);
    }

    #[test]
    fn fills_what_the_jv_has_no_source_for_as_roland_does() {
        let (entry, _) = converted(&kit(USER_1, [36], |_, note| {
            note[0x38..0x3A].copy_from_slice(&[10, 20]);
        }));
        let p = partial(&entry, 36);
        let byte = |f: F| p[f as usize];
        assert_eq!(byte(F::WmtVelocityControl), 1);
        assert_eq!(p[pcmd_wmt_byte(0, W::RandomPanSwitch)], 1);
        assert_eq!(p[pcmd_wmt_byte(0, W::AlternatePanSwitch)], 1);
        assert_eq!(p[pcmd_wmt_byte(0, W::Level)], 127);
        assert_eq!(byte(F::TvfCutoffVelocityCurve), 0);
        assert_eq!(byte(F::TvfEnvVelocityCurve), 1);
        assert_eq!(byte(F::TvaLevelVelocityCurve), 1);
        assert_eq!((byte(F::PitchEnvLevel), byte(F::TvfEnvLevel)), (64, 0));
        assert_eq!((byte(F::OutputAssign), byte(F::OneShotMode)), (0, 0));
        // Both send pairs, as Roland's XV conversion holds them.
        assert_eq!(p[0x17..=0x1A], [10, 20, 10, 20]);
        assert_eq!(p[0x80 + 0x42], 64);
    }

    /// A JV Note offset, the Partial offset it lands at, and `(JV,
    /// INTEGRA-7)` value pairs.
    type Pairs = (usize, usize, &'static [(u8, u8)]);

    /// Roland's own conversion of the SR-JV80-16 "Orch Set": every distinct
    /// `(JV, INTEGRA-7)` value pair its 64 keys show, per Note field, with
    /// the JV Note offset and the Partial offset it lands at. Read from the
    /// JV Rhythm Setup `OrchII-1.syx` (Roland-XV Yahoo group archive) and
    /// Roland's 2002 XV re-issue `SRJV16R.mid` (bwalk.com.au), whose Rhythm
    /// Tone is the INTEGRA-7 Partial; see `foreign-rhythm-format.md`,
    /// section 4. Only these value pairs are taken from the files.
    const ORCH_SET: [Pairs; 17] = [
        // Offset rows.
        (
            0x0D,
            0x10,
            &[
                (0, 14),
                (1, 15),
                (10, 24),
                (30, 44),
                (35, 49),
                (36, 50),
                (38, 52),
                (40, 54),
                (50, 64),
                (60, 74),
                (65, 79),
                (67, 81),
                (75, 89),
                (100, 114),
            ],
        ), // Fine Tune
        (0x0F, 0x80 + 0x15, &[(12, 64)]), // P-Env Depth
        (0x16, 0x80 + 0x1E, &[(63, 64)]), // P-Env Level 1
        (0x17, 0x80 + 0x1F, &[(63, 64)]), // P-Env Level 2
        (0x18, 0x80 + 0x20, &[(63, 64)]), // P-Env Level 3
        (0x19, 0x80 + 0x21, &[(63, 64)]), // P-Env Level 4
        (
            0x1E,
            0x80 + 0x28,
            &[
                (63, 64),
                (68, 69),
                (69, 70),
                (74, 75),
                (76, 77),
                (83, 84),
                (87, 88),
                (93, 94),
            ],
        ), // F-Env Depth
        // Rescale rows.
        (0x10, 0x80 + 0x16, &[(50, 64)]), // P-Env Velocity Sens
        (0x1D, 0x80 + 0x27, &[(50, 64)]), // Resonance Velocity Sens
        (
            0x1F,
            0x80 + 0x2A,
            &[(50, 64), (65, 73), (93, 91), (106, 100)],
        ), // F-Env Velocity Sens
        (
            0x2A,
            0x80 + 0x37,
            &[
                (80, 83),
                (85, 86),
                (90, 89),
                (96, 93),
                (100, 96),
                (105, 99),
                (106, 100),
                (120, 116),
            ],
        ), // A-Env Velocity Sens
        (0x11, 0x80 + 0x17, &[(7, 64)]),  // P-Env Velocity Time → Time 1
        (0x11, 0x80 + 0x18, &[(7, 64)]),  // … and Time 4
        (0x20, 0x80 + 0x2B, &[(7, 64)]),  // F-Env Velocity Time → Time 1
        (0x20, 0x80 + 0x2C, &[(7, 64)]),  // … and Time 4
        (0x2B, 0x80 + 0x38, &[(7, 64), (8, 70), (9, 76), (11, 89)]), // A-Env Velocity Time → Time 1
        (0x2B, 0x80 + 0x39, &[(7, 64), (8, 70), (9, 76), (11, 89)]), // … and Time 4
    ];

    #[test]
    fn matches_rolands_orch_set_on_every_offset_and_rescale_row() {
        for (from, to, pairs) in ORCH_SET {
            let keys = 35..35 + pairs.len() as u8;
            let bytes = kit(USER_1, keys.clone(), |key, note| {
                note[from] = pairs[usize::from(key - 35)].0;
            });
            let (entry, _) = converted(&bytes);
            for (key, &(jv, roland)) in keys.zip(pairs) {
                let ours = partial(&entry, key)[to];
                assert_eq!(ours, roland, "JV {from:02X} = {jv} → {to:03X}");
            }
        }
    }

    #[test]
    fn reports_what_the_integra_cannot_hold() {
        let bytes = kit(USER_1, 40..=45, |key, note| match key {
            40 => note[0x0B] = 2,                            // Pan Control KEY-ON
            41 => note[0x36] = 1,                            // Output Assign EFX
            42 => note[0x36] = 2,                            // OUTPUT 1
            43 => note[0x01..0x03].copy_from_slice(&[1, 3]), // PCM card 3
            44 => note[0x01..0x03].copy_from_slice(&[2, 6]), // SR-JV80-06 Dance
            _ => note[0x00] = 0,                             // A Note switched off is not reported.
        });
        let bytes = [bytes, kit(USER_1, [], |_, _| {})].concat();
        let read = read(&bytes);
        // The second Common starts another kit, with no Notes.
        assert_eq!(read.file_report, [skipped(Skipped::Incomplete, 1)]);
        let (entry, report) = (&read.pack.entries[0], &read.reports[0]);
        let texts: Vec<String> = report.iter().map(ToString::to_string).collect();
        assert_eq!(
            texts,
            [
                "Key 40: Pan Control Switch dropped",
                "Key 42 Output Assign OUTPUT 1 not applied; set it in the Studio Set",
                "Key 43 WMT 1: PCM card 3 Wave 1 has no INTEGRA-7 Wave",
                "Key 44 WMT 1: SR-JV80-06 Wave 1 has no INTEGRA-7 Wave",
                "Output Assign EFX dropped",
            ]
        );
        assert_eq!(partial(entry, 40)[0x1F], 1); // Receive Pan Mode KEY-ON
        for key in [43, 44] {
            assert_eq!(wmt(partial(entry, key), 0), (0, 0, 1, 0));
        }
        assert_eq!(
            report[3],
            ConversionItem::WaveUnmapped {
                at: Location::Wmt { key: 44, wmt: 1 },
                source: ForeignWave {
                    group: ForeignWaveGroup::SrJv80(6),
                    number: 1
                },
            }
        );
    }

    #[test]
    fn reads_every_rhythm_area() {
        // Temporary (Part 10), User 2 and the JV-1080 card.
        for slot in [[0x02, 0x09], [0x10, 0x41], [0x20, 0x40]] {
            let (entry, _) = converted(&kit(slot, [60], |_, _| {}));
            assert_eq!(wmt(partial(&entry, 60), 0).0, 1);
        }
    }
}

mod xv_rhythms {
    use super::*;
    use crate::parse_helpers::nibble4;
    use crate::pcm_drum::{
        PCMD_WMT_COUNT, PcmDrumPartial, PcmdPartialField as F, PcmdWmtField as W,
        encode_pcmd_partial, pcmd_wmt_byte,
    };

    /// User Rhythm 2.
    const USER_2: [u8; 2] = [0x40, 0x10];
    /// Address bytes 1 and 2 of the Common, MFX, Chorus and Reverb blocks.
    const COMMONS: [[u8; 2]; 4] = [[0, 0], [0, 2], [0, 4], [0, 6]];

    /// Common, MFX, Chorus and Reverb at neutral values: Tempo 120, Output
    /// Assign TONE, MFX THRU with Dry Send 127, Chorus and Reverb OFF.
    fn commons() -> [Vec<u8>; 4] {
        let mut common = vec![0u8; 18];
        common[..12].copy_from_slice(b"XV Byte Kit ");
        common[0x0C] = 127; // Rhythm Level
        common[0x0E..0x10].copy_from_slice(&[0x07, 0x08]); // Tempo 120
        common[0x11] = 13; // Output Assign TONE
        let mut mfx = vec![0u8; 145];
        mfx[0x01] = 127; // MFX Dry Send Level
        [common, mfx, vec![0; 52], vec![0; 83]]
    }

    /// A Rhythm Tone of `key`: WMT1 on and playing INT Wave 100, Coarse
    /// Tune `key`, output A, every other byte a marker.
    fn tone(key: u8) -> Vec<u8> {
        let mut tone = vec![0u8; 193];
        tone[..12].copy_from_slice(b"Tone        ");
        tone[F::Level as usize] = 100;
        tone[F::CoarseTune as usize] = key;
        tone[0x17..0x1B].copy_from_slice(&[11, 12, 13, 14]); // MFX and non-MFX sends
        tone[F::OutputAssign as usize] = 1; // A
        tone[pcmd_wmt_byte(0, W::Switch)] = 1;
        tone[pcmd_wmt_byte(0, W::WaveGroupId) + 3] = 1; // INT
        tone[pcmd_wmt_byte(0, W::WaveNumberL)..][..4].copy_from_slice(&[0, 0, 6, 4]); // 100
        tone
    }

    /// The DT1 of block `offset` (7-bit, below the slot bits) at `slot`.
    fn block(slot: [u8; 2], offset: u16, data: &[u8]) -> Vec<u8> {
        let [hi, lo] = [(offset >> 7) as u8, (offset & 0x7F) as u8];
        foreign(xv::MODEL, [slot[0], slot[1] | hi, lo, 0], data)
    }

    /// A Rhythm Set at `slot`: [`commons`], then the Tones of `keys`,
    /// `edit`ed.
    fn kit(
        slot: [u8; 2],
        keys: impl IntoIterator<Item = u8>,
        edit: impl FnOnce(&mut [Vec<u8>; 4], &mut [(u8, Vec<u8>)]),
    ) -> Vec<u8> {
        let mut commons = commons();
        let mut tones: Vec<(u8, Vec<u8>)> = keys.into_iter().map(|k| (k, tone(k))).collect();
        edit(&mut commons, &mut tones);
        let mut bytes = Vec::new();
        for ([_, c], data) in COMMONS.into_iter().zip(&commons) {
            bytes.extend(block(slot, u16::from(c), data));
        }
        for (key, data) in &tones {
            bytes.extend(block(slot, 0x10 + 2 * u16::from(key - 21), data));
        }
        bytes
    }

    fn converted(bytes: &[u8]) -> (PackEntry, ConversionReport) {
        let mut read = read(bytes);
        assert_eq!(read.pack.entries.len(), 1, "{:?}", read.file_report);
        assert!(read.file_report.is_empty(), "{:?}", read.file_report);
        (read.pack.entries.remove(0), read.reports.remove(0))
    }

    fn partial(entry: &PackEntry, key: u8) -> &[u8] {
        &entry.blocks()[3 + usize::from(key - 21)]
    }

    fn texts(report: &ConversionReport) -> Vec<String> {
        report.iter().map(ToString::to_string).collect()
    }

    /// The bytes of WMT `w` (0-indexed) in `partial`.
    fn wmt(partial: &[u8], w: usize) -> &[u8] {
        let len = pcmd_wmt_byte(1, W::Switch) - pcmd_wmt_byte(0, W::Switch);
        &partial[pcmd_wmt_byte(w, W::Switch)..][..len]
    }

    /// Reset the switched-off WMTs of `partial` to the INIT WMT.
    fn reset_off_wmts(partial: &mut [u8]) {
        let init = encode_pcmd_partial(&PcmDrumPartial::default());
        for w in 0..PCMD_WMT_COUNT {
            if partial[pcmd_wmt_byte(w, W::Switch)] == 0 {
                let at = pcmd_wmt_byte(w, W::Switch);
                let len = wmt(&init, w).len();
                partial[at..at + len].copy_from_slice(wmt(&init, w));
            }
        }
    }

    #[test]
    fn copies_each_key_to_the_same_integra_key_on_part() {
        let bytes = kit(USER_2, 21..=108, |_, _| {});
        let (entry, report) = converted(&bytes);
        assert!(report.is_empty(), "{report:?}");
        assert_eq!(entry.kind(), ChunkType::PcmDrumKit);
        assert_eq!(entry.name(), "XV Byte Kit");
        let blocks = entry.blocks();
        assert_eq!(blocks[0], commons()[0]);
        assert_eq!(blocks[1], commons()[1]);
        for key in 21..=108u8 {
            let mut expected = tone(key);
            // Output A would be COMP+EQ1: every key goes to PART.
            expected[F::OutputAssign as usize] = 0;
            expected.extend([0, 64]); // One Shot Mode, reserve `01 42`
            reset_off_wmts(&mut expected);
            assert_eq!(partial(&entry, key), expected, "key {key}");
        }
    }

    #[test]
    fn a_switched_off_wmt_gets_the_init_wmt() {
        let bytes = kit(USER_2, [22], |_, tones| {
            let tone = &mut tones[0].1;
            // WMT1 on, WMT2 off holding out-of-range bytes, as `XV 5080 All.mid`.
            tone[pcmd_wmt_byte(1, W::WaveGroupId)..][..4].copy_from_slice(&[0, 0, 0, 9]);
            tone[pcmd_wmt_byte(1, W::CoarseTune)] = 0;
            tone[pcmd_wmt_byte(1, W::VelocityRangeLower)] = 0;
        });
        let (entry, report) = converted(&bytes);
        assert!(report.is_empty(), "{report:?}");
        let init = encode_pcmd_partial(&PcmDrumPartial::default());
        let mut expected = tone(22);
        expected[F::OutputAssign as usize] = 0;
        assert_eq!(wmt(partial(&entry, 22), 0), wmt(&expected, 0));
        for w in 1..PCMD_WMT_COUNT {
            assert_eq!(wmt(partial(&entry, 22), w), wmt(&init, w), "WMT {}", w + 1);
        }
    }

    #[test]
    fn a_key_not_sent_stays_off() {
        let (entry, _) = converted(&kit(USER_2, [36, 38], |_, _| {}));
        for key in 21..=108u8 {
            let on = partial(&entry, key)[pcmd_wmt_byte(0, W::Switch)];
            assert_eq!(on, u8::from(key == 36 || key == 38), "key {key}");
        }
    }

    #[test]
    fn applies_the_fix_rows() {
        let bytes = kit(USER_2, 36..=40, |commons, tones| {
            commons[0][0x10] = 1; // One Shot Mode
            for (key, tone) in tones.iter_mut() {
                let wmt = |f| pcmd_wmt_byte(0, f);
                match key {
                    36 => tone[F::OutputAssign as usize] = 0, // MFX
                    37 => {
                        tone[wmt(W::WaveGroupType)] = 2; // SRX-06
                        tone[wmt(W::WaveGroupId) + 3] = 6;
                    }
                    38 => {
                        tone[wmt(W::WaveGroupType)] = 1; // SR-JV80-02 Wave 1
                        tone[wmt(W::WaveGroupId) + 3] = 2;
                        tone[wmt(W::WaveNumberL) + 2..][..2].copy_from_slice(&[0, 1]);
                    }
                    39 => tone[wmt(W::WaveGroupType)] = 3, // SAMPLE
                    _ => tone[0x1F] = 1,                   // Receive Pan Mode KEY-ON
                }
            }
        });
        let (entry, report) = converted(&bytes);
        let wave = |key, f| nibble4(partial(&entry, key), pcmd_wmt_byte(0, f));
        let group_type = |key| partial(&entry, key)[pcmd_wmt_byte(0, W::WaveGroupType)];
        // An MFX Tone's sends go where the INTEGRA-7 reads them.
        assert_eq!(partial(&entry, 36)[0x17..=0x1B], [11, 12, 11, 12, 0]);
        assert_eq!(partial(&entry, 37)[0x17..=0x1B], [11, 12, 13, 14, 0]);
        assert_eq!(
            (
                group_type(37),
                wave(37, W::WaveGroupId),
                wave(37, W::WaveNumberL)
            ),
            (1, 6, 100)
        );
        assert_eq!(
            (
                group_type(38),
                wave(38, W::WaveGroupId),
                wave(38, W::WaveNumberL)
            ),
            (1, 6, 29)
        );
        assert_eq!(partial(&entry, 39)[pcmd_wmt_byte(0, W::Switch)], 0);
        assert_eq!(partial(&entry, 40)[0x1F], 1);
        for key in 36..=40 {
            assert_eq!(partial(&entry, key)[F::OneShotMode as usize], 1);
        }
        assert_eq!(entry.blocks()[0][0x10], 1);
        assert_eq!(
            texts(&report),
            [
                "Key 40: Receive Pan Mode dropped",
                "Key 39 WMT 1: SAMPLE Wave 100 has no INTEGRA-7 Wave",
            ]
        );
    }

    #[test]
    fn reports_effects_and_settings_that_live_in_the_studio_set() {
        let bytes = kit(USER_2, [36], |commons, tones| {
            tones[0].1[pcmd_wmt_byte(0, W::WaveTempoSync)] = 1;
            commons[0][0x0E..0x10].copy_from_slice(&[0x05, 0x0B]); // Tempo 91
            commons[0][0x11] = 1; // Output Assign A
            commons[1][0x00] = 7; // MFX Type
            commons[1][0x01] = 0; // MFX Dry Send Level
            commons[1][0x04] = 2; // MFX Output Assign
            commons[3][0x00..0x02].copy_from_slice(&[3, 100]); // SRV HALL
        });
        let (entry, report) = converted(&bytes);
        let blocks = entry.blocks();
        assert_eq!(blocks[0][0x0E..0x12], [0x05, 0x0B, 0, 13]);
        assert_eq!(
            (blocks[1][0x00], blocks[1][0x01], blocks[1][0x04]),
            (0, 127, 0)
        );
        assert_eq!(
            texts(&report),
            [
                "Tempo 91 not applied; set it in the Studio Set",
                "Output Assign A not applied; set it in the Studio Set",
                "MFX type 7 dropped, MFX set to THRU",
                "MFX Dry Send Level dropped",
                "Reverb SRV HALL, level 100 not applied; set it in the Studio Set",
            ]
        );
    }

    #[test]
    fn reports_the_tempo_only_when_a_switched_on_wmt_syncs_to_it() {
        let tempo = |sync_wmt: usize| {
            let bytes = kit(USER_2, [36], |commons, tones| {
                tones[0].1[pcmd_wmt_byte(sync_wmt, W::WaveTempoSync)] = 1;
                commons[0][0x0E..0x10].copy_from_slice(&[0x05, 0x0B]); // Tempo 91
            });
            texts(&converted(&bytes).1)
        };
        assert_eq!(tempo(0), ["Tempo 91 not applied; set it in the Studio Set"]);
        // WMT2 is off.
        assert!(tempo(1).is_empty());
    }

    #[test]
    fn needs_every_common_block() {
        let mut bytes = kit(USER_2, [36], |_, _| {});
        // Drop the Reverb, the fourth DT1.
        let messages = frame::split(&bytes).messages;
        let reverb = messages[3].len();
        let start: usize = messages[..3].iter().map(|m| m.len()).sum();
        bytes.drain(start..start + reverb);
        let read = read(&bytes);
        assert!(read.pack.entries.is_empty());
        assert_eq!(read.file_report, [skipped(Skipped::Incomplete, 1)]);
    }

    #[test]
    fn reads_the_temporary_rhythms() {
        // Patch mode, and Performance part 10.
        for slot in [[0x1F, 0x10], [0x13, 0x30]] {
            let (entry, _) = converted(&kit(slot, [60], |_, _| {}));
            assert_eq!(partial(&entry, 60)[pcmd_wmt_byte(0, W::Switch)], 1);
        }
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
