//! Packs: Studio Sets and tones that travel together in one `.SVD` file.
//!
//! A [`Pack`] holds each entry as SysEx blocks in the device's Temporary
//! layout (the block list of its [`UserMemorySpec`]). SVD is only the
//! encoding at the file boundary: [`Pack::from_svd`] reads the sound-export
//! and backup shapes, and [`Pack::to_svd`] writes the sound-export shape.
//!
//! A Studio Set Part that selects a User tone holds that tone's 0-based
//! position among the Pack's entries of its type, as Roland's export does.
//! In a backup every slot is present, so position and slot coincide.
//!
//! Reference: `docs/svd/01-container.md`, `docs/svd/05-studio-set.md`,
//! `docs/svd/11-sound-vs-backup.md`.

use thiserror::Error;

use crate::address;
use crate::device_spec::{INTEGRA7, USER_MEMORY, UserMemorySpec};
use crate::params::{self, part};
use crate::parse_helpers::parse_ascii_name_len;
use crate::svd::{ChunkType, SvdChunk, SvdError, SvdFile, blocks_category};
use crate::svd_convert::{ConvertError, svd_to_sysex, sysex_to_svd};
use crate::svd_specs::{
    PCMD_TONE_SPEC, PCMS_TONE_SPEC, SNA_TONE_SPEC, SND_TONE_SPEC, SNS_TONE_SPEC, SS_TONE_SPEC,
    SvdToneSpec,
};
use crate::user_write::BankSelect;

/// Chunks of a sound-export file, in directory order.
const SOUND_CHUNKS: [ChunkType; 6] = [
    ChunkType::StudioSet,
    ChunkType::PcmSynthTone,
    ChunkType::PcmDrumKit,
    ChunkType::SnSynthTone,
    ChunkType::SnAcousticTone,
    ChunkType::SnDrumKit,
];

/// Errors reading, building or writing a [`Pack`].
#[derive(Debug, Error)]
pub enum PackError {
    /// The file is not a valid SVD container.
    #[error(transparent)]
    Svd(#[from] SvdError),
    /// An entry does not decode with its type's codec.
    #[error(transparent)]
    Convert(#[from] ConvertError),
    /// The type has no codec (System Settings).
    #[error("unsupported type: {0}")]
    UnsupportedType(ChunkType),
    /// A chunk's entry size differs from its type's.
    #[error("{kind} entries are {actual} bytes, expected {expected}")]
    EntrySize {
        /// The chunk's type.
        kind: ChunkType,
        /// Entry size of the type.
        expected: usize,
        /// Entry size in the file.
        actual: usize,
    },
    /// Blocks don't match the type's Temporary layout.
    #[error("blocks don't match the {0} Temporary layout")]
    BlockLayout(ChunkType),
    /// A reference that this entry's Part can't hold.
    #[error("Part {part} can't hold {tone_ref:?}")]
    InvalidToneRef {
        /// The Part, 0-indexed.
        part: u8,
        /// The rejected reference.
        tone_ref: ToneRef,
    },
}

/// The tone a Studio Set Part selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneRef {
    /// A User tone of this Pack: the `index`-th entry of type `kind`
    /// (0-based, in Pack order).
    Pack {
        /// The tone's type.
        kind: ChunkType,
        /// Position among the Pack's entries of `kind`.
        index: u16,
    },
    /// Any other tone, by Bank Select (PC 0-based).
    Bank {
        /// Bank Select MSB.
        msb: u8,
        /// Bank Select LSB.
        lsb: u8,
        /// Program number, 0-based.
        pc: u8,
    },
}

impl ToneRef {
    /// Types a Part's Bank Select. A User tone bank reads as the Pack
    /// position `LSB × 128 + PC`; anything else stays a [`ToneRef::Bank`].
    pub fn from_bank_select(bs: BankSelect) -> Self {
        let index = u16::from(bs.lsb) * 128 + u16::from(bs.pc);
        tone_memories()
            .find(|(_, m)| m.bank_msb == bs.msb && index < m.slot_count)
            .map_or(
                Self::Bank {
                    msb: bs.msb,
                    lsb: bs.lsb,
                    pc: bs.pc,
                },
                |(kind, _)| Self::Pack { kind, index },
            )
    }

    /// The Bank Select a Part stores for this reference: a Pack position is
    /// written as the User slot of the same number, the LSB carrying past
    /// 128. `None` if it fits no User bank or a field exceeds 7 bits.
    pub fn bank_select(self) -> Option<BankSelect> {
        match self {
            Self::Pack { kind, index } => tone_memories()
                .find(|(k, _)| *k == kind)?
                .1
                .bank_select(index),
            Self::Bank { msb, lsb, pc } => {
                ((msb | lsb | pc) < 0x80).then_some(BankSelect { msb, lsb, pc })
            }
        }
    }
}

/// One Studio Set or tone of a [`Pack`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackEntry {
    kind: ChunkType,
    slot: Option<u16>,
    blocks: Vec<Vec<u8>>,
    /// The SVD entry this was decoded from, whose undocumented trailing
    /// bytes are written back on encode.
    svd: Option<Vec<u8>>,
}

impl PackEntry {
    /// An entry of type `kind` from its blocks, in the order of its
    /// [`UserMemorySpec`] block list. `slot` is the User slot it came from,
    /// if known.
    pub fn new(
        kind: ChunkType,
        slot: Option<u16>,
        blocks: Vec<Vec<u8>>,
    ) -> Result<Self, PackError> {
        let memory = user_memory(kind).ok_or(PackError::UnsupportedType(kind))?;
        let sizes = memory
            .blocks(memory.temporary_base(0))
            .map(|(_, size)| size.value() as usize);
        if !blocks.iter().map(Vec::len).eq(sizes) {
            return Err(PackError::BlockLayout(kind));
        }
        Ok(Self {
            kind,
            slot,
            blocks,
            svd: None,
        })
    }

    /// The entry's type.
    pub fn kind(&self) -> ChunkType {
        self.kind
    }

    /// The User slot (0-indexed) the entry came from: its index in a
    /// backup file, `None` in a sound-export file.
    pub fn slot(&self) -> Option<u16> {
        self.slot
    }

    /// SysEx blocks in the Temporary layout.
    pub fn blocks(&self) -> &[Vec<u8>] {
        &self.blocks
    }

    /// Name, from Common.
    pub fn name(&self) -> String {
        self.kind.name_length().map_or_else(String::new, |len| {
            parse_ascii_name_len(&self.blocks[0], len)
        })
    }

    /// Tone category byte, for the types that have one.
    pub fn category(&self) -> Option<u8> {
        blocks_category(&self.blocks, self.kind)
    }

    /// The tone of each Part of a Studio Set; empty for a tone.
    pub fn tone_refs(&self) -> Vec<ToneRef> {
        (0..INTEGRA7.part_count)
            .filter_map(|p| self.part_block(p))
            .map(|b| {
                let byte = |offset: [u8; 3]| self.blocks[b][usize::from(offset[2])];
                ToneRef::from_bank_select(BankSelect {
                    msb: byte(part::TONE_BANK_MSB),
                    lsb: byte(part::TONE_BANK_LSB),
                    pc: byte(part::TONE_PC),
                })
            })
            .collect()
    }

    /// Point Part `part` (0-indexed) of a Studio Set at `tone_ref`.
    pub fn set_tone_ref(&mut self, part: u8, tone_ref: ToneRef) -> Result<(), PackError> {
        let invalid = PackError::InvalidToneRef { part, tone_ref };
        let (Some(b), Some(bs)) = (self.part_block(part), tone_ref.bank_select()) else {
            return Err(invalid);
        };
        for (offset, value) in [
            (part::TONE_BANK_MSB, bs.msb),
            (part::TONE_BANK_LSB, bs.lsb),
            (part::TONE_PC, bs.pc),
        ] {
            self.blocks[b][usize::from(offset[2])] = value;
        }
        Ok(())
    }

    /// Index in `blocks` of Part `part`'s block, for a Studio Set.
    fn part_block(&self, part: u8) -> Option<usize> {
        if self.kind != ChunkType::StudioSet || part >= INTEGRA7.part_count {
            return None;
        }
        let addr = params::part_address(part, [0; 3]);
        user_memory(self.kind)?
            .blocks(address::STUDIO_SET)
            .position(|(a, _)| a == addr)
    }

    /// The SVD entry: undocumented trailing bytes come from the file the
    /// entry was read from, else take their INIT value.
    fn to_svd(&self) -> Result<Vec<u8>, PackError> {
        let spec = codec(self.kind)?;
        Ok(sysex_to_svd(&self.blocks, spec, self.svd.as_deref()))
    }
}

/// Studio Sets and tones of any type, moved together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pack {
    /// The entries. Within a type, their order is their Pack position.
    pub entries: Vec<PackEntry>,
}

impl Pack {
    /// Read a sound-export or backup SVD file. In a backup, each entry's
    /// slot is its index, and the system settings are ignored.
    pub fn from_svd(bytes: &[u8]) -> Result<Self, PackError> {
        let file = SvdFile::parse(bytes)?;
        let backup = file
            .chunks
            .iter()
            .any(|c| c.chunk_type == ChunkType::SystemSettings);
        let mut entries = Vec::new();
        for chunk in &file.chunks {
            let kind = chunk.chunk_type;
            if kind == ChunkType::SystemSettings || chunk.entries.is_empty() {
                continue;
            }
            let spec = codec(kind)?;
            if chunk.entry_size != spec.entry_size {
                return Err(PackError::EntrySize {
                    kind,
                    expected: spec.entry_size,
                    actual: chunk.entry_size,
                });
            }
            for (slot, svd) in (0..).zip(&chunk.entries) {
                entries.push(PackEntry {
                    kind,
                    slot: backup.then_some(slot),
                    blocks: svd_to_sysex(svd, spec)?,
                    svd: Some(svd.clone()),
                });
            }
        }
        Ok(Self { entries })
    }

    /// Write the sound-export shape: all six chunks, empty ones included.
    pub fn to_svd(&self) -> Result<Vec<u8>, PackError> {
        let chunks = SOUND_CHUNKS
            .into_iter()
            .map(|kind| {
                Ok(SvdChunk {
                    chunk_type: kind,
                    entry_size: kind.default_entry_size(),
                    entries: self
                        .of_kind(kind)
                        .map(PackEntry::to_svd)
                        .collect::<Result<_, PackError>>()?,
                })
            })
            .collect::<Result<_, PackError>>()?;
        Ok(SvdFile { chunks }.write())
    }

    /// Entries of type `kind`, in Pack position order.
    pub fn of_kind(&self, kind: ChunkType) -> impl Iterator<Item = &PackEntry> {
        self.entries.iter().filter(move |e| e.kind == kind)
    }
}

/// The codec of each supported type.
fn codec(kind: ChunkType) -> Result<&'static SvdToneSpec, PackError> {
    match kind {
        ChunkType::StudioSet => Ok(&SS_TONE_SPEC),
        ChunkType::SnSynthTone => Ok(&SNS_TONE_SPEC),
        ChunkType::SnAcousticTone => Ok(&SNA_TONE_SPEC),
        ChunkType::SnDrumKit => Ok(&SND_TONE_SPEC),
        ChunkType::PcmSynthTone => Ok(&PCMS_TONE_SPEC),
        ChunkType::PcmDrumKit => Ok(&PCMD_TONE_SPEC),
        ChunkType::SystemSettings => Err(PackError::UnsupportedType(kind)),
    }
}

/// The User memory spec of `kind`.
fn user_memory(kind: ChunkType) -> Option<&'static UserMemorySpec> {
    USER_MEMORY.iter().find(|m| m.cli_name == kind.cli_name())
}

/// Every tone type (Studio Set excluded) with its User memory spec.
fn tone_memories() -> impl Iterator<Item = (ChunkType, &'static UserMemorySpec)> {
    SOUND_CHUNKS
        .into_iter()
        .filter_map(|kind| Some((kind, user_memory(kind)?)))
        .filter(|(_, m)| m.tone_type.is_some())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_helpers::encode_ascii_name;

    const SNS: ChunkType = ChunkType::SnSynthTone;

    /// Blocks of a synthetic entry named `name`, decoded from an all-zero
    /// SVD entry so that every value round-trips.
    fn blocks(kind: ChunkType, name: &str) -> Vec<Vec<u8>> {
        let spec = codec(kind).unwrap();
        let mut blocks = svd_to_sysex(&vec![0; spec.entry_size], spec).unwrap();
        let len = kind.name_length().unwrap();
        blocks[0][..len].copy_from_slice(&encode_ascii_name(name, len));
        blocks
    }

    fn entry(kind: ChunkType, name: &str) -> PackEntry {
        PackEntry::new(kind, None, blocks(kind, name)).unwrap()
    }

    /// Part 1 → SN-S position 1, Part 2 → a preset, Part 3 → SN-S position
    /// 130 (LSB carry; broken in this Pack), the rest → SN-S position 0.
    const REFS: [(u8, ToneRef); 3] = [
        (
            0,
            ToneRef::Pack {
                kind: SNS,
                index: 1,
            },
        ),
        (
            1,
            ToneRef::Bank {
                msb: 89,
                lsb: 64,
                pc: 0,
            },
        ),
        (
            2,
            ToneRef::Pack {
                kind: SNS,
                index: 130,
            },
        ),
    ];

    fn studio_set() -> PackEntry {
        let mut set = entry(ChunkType::StudioSet, "Pack Test Set");
        for p in 0..16 {
            set.set_tone_ref(
                p,
                ToneRef::Pack {
                    kind: SNS,
                    index: 0,
                },
            )
            .unwrap();
        }
        for (p, r) in REFS {
            set.set_tone_ref(p, r).unwrap();
        }
        set
    }

    /// A sound-export file: one Studio Set and two SN-S tones, the second
    /// with a non-INIT tail after its end marker.
    fn sound_file() -> Vec<u8> {
        let pack = Pack {
            entries: vec![studio_set(), entry(SNS, "Tone A"), entry(SNS, "Tone B")],
        };
        let mut file = SvdFile::parse(&pack.to_svd().unwrap()).unwrap();
        file.chunks[3].entries[1][250..252].copy_from_slice(&[0x44, 0xC0]);
        file.write()
    }

    #[test]
    fn codec_sections_match_temporary_blocks() {
        for kind in SOUND_CHUNKS {
            let memory = user_memory(kind).unwrap();
            let sizes: Vec<usize> = memory
                .blocks(memory.temporary_base(0))
                .map(|(_, s)| s.value() as usize)
                .collect();
            let sections: Vec<usize> = codec(kind)
                .unwrap()
                .sections
                .iter()
                .map(|s| s.sysex_size())
                .collect();
            assert_eq!(sections, sizes, "{kind}");
            assert_eq!(codec(kind).unwrap().entry_size, kind.default_entry_size());
        }
    }

    #[test]
    fn sound_file_round_trips() {
        let bytes = sound_file();
        let pack = Pack::from_svd(&bytes).unwrap();
        assert_eq!(pack.to_svd().unwrap(), bytes);

        let names: Vec<_> = pack.entries.iter().map(PackEntry::name).collect();
        assert_eq!(names, ["Pack Test Set", "Tone A", "Tone B"]);
        assert!(pack.entries.iter().all(|e| e.slot().is_none()));

        let refs = pack.entries[0].tone_refs();
        assert_eq!(refs.len(), 16);
        for (p, r) in REFS {
            assert_eq!(refs[usize::from(p)], r);
        }
        assert_eq!(
            refs[15],
            ToneRef::Pack {
                kind: SNS,
                index: 0
            }
        );
        assert!(pack.entries[1].tone_refs().is_empty());
    }

    #[test]
    fn writes_sound_export_shape() {
        let bytes = sound_file();
        assert_eq!(&bytes[..6], b"\x00\x6ESVD1");
        let file = SvdFile::parse(&bytes).unwrap();
        let counts: Vec<_> = file
            .chunks
            .iter()
            .map(|c| (c.chunk_type, c.entries.len(), c.entry_size))
            .collect();
        assert_eq!(
            counts,
            SOUND_CHUNKS.map(|k| {
                let n = match k {
                    ChunkType::StudioSet => 1,
                    SNS => 2,
                    _ => 0,
                };
                (k, n, k.default_entry_size())
            })
        );
        // A fresh SN-S entry takes the INIT tail.
        assert_eq!(
            file.chunks[3].entries[0][246..252],
            [0x0E, 0x00, 0x00, 0x00, 0x02, 0xC0]
        );
    }

    #[test]
    fn pack_position_carries_into_lsb() {
        let set = studio_set();
        let part3 = set.part_block(2).unwrap();
        assert_eq!(set.blocks()[part3][6..9], [95, 1, 2]);
    }

    #[test]
    fn backup_entries_carry_their_slot() {
        let mut file = SvdFile::parse(&sound_file()).unwrap();
        file.chunks.push(SvdChunk {
            chunk_type: ChunkType::SystemSettings,
            entry_size: ChunkType::SystemSettings.default_entry_size(),
            entries: vec![vec![0x5A; ChunkType::SystemSettings.default_entry_size()]],
        });
        let pack = Pack::from_svd(&file.write()).unwrap();
        let slots: Vec<_> = pack.entries.iter().map(PackEntry::slot).collect();
        assert_eq!(slots, [Some(0), Some(0), Some(1)]);
        assert_eq!(pack.to_svd().unwrap(), sound_file());
    }

    #[test]
    fn system_settings_are_unsupported() {
        let sys = PackEntry::new(ChunkType::SystemSettings, None, vec![]);
        assert!(matches!(sys, Err(PackError::UnsupportedType(_))));
        let pcm = PackEntry::new(ChunkType::PcmSynthTone, None, vec![]);
        assert!(matches!(pcm, Err(PackError::BlockLayout(_))));
    }

    /// A backup-shaped file: every chunk non-empty, plus `SYSa`. Each tone
    /// entry carries data the codec doesn't model: the PCM Synth long name
    /// (a reserve) and non-zero bytes after the data sections.
    fn backup() -> SvdFile {
        let mut tones = Vec::new();
        for kind in SOUND_CHUNKS {
            let mut blocks = blocks(kind, kind.cli_name());
            if kind == ChunkType::PcmSynthTone {
                // Common 2 `00 00`–`00 0F`.
                blocks[7][..16].copy_from_slice(&encode_ascii_name("Long Name Tone", 16));
            }
            tones.push(PackEntry::new(kind, None, blocks).unwrap());
        }
        let mut file = SvdFile::parse(&Pack { entries: tones }.to_svd().unwrap()).unwrap();
        for chunk in &mut file.chunks {
            let spec = codec(chunk.chunk_type).unwrap();
            if spec.data_bytes() < spec.entry_size {
                *chunk.entries[0].last_mut().unwrap() = 0x5A;
            }
        }
        file.chunks.push(SvdChunk {
            chunk_type: ChunkType::SystemSettings,
            entry_size: ChunkType::SystemSettings.default_entry_size(),
            entries: vec![vec![0x5A; ChunkType::SystemSettings.default_entry_size()]],
        });
        file
    }

    #[test]
    fn backup_round_trips_every_type() {
        let file = backup();
        let pack = Pack::from_svd(&file.write()).unwrap();
        assert_eq!(pack.entries.len(), SOUND_CHUNKS.len());
        assert!(pack.entries.iter().all(|e| e.slot() == Some(0)));
        for (entry, kind) in pack.entries.iter().zip(SOUND_CHUNKS) {
            assert_eq!(entry.kind(), kind);
            assert_eq!(entry.name(), kind.cli_name());
        }
        let pcms = pack.of_kind(ChunkType::PcmSynthTone).next().unwrap();
        assert_eq!(&pcms.blocks()[7][..14], b"Long Name Tone");

        // Every entry comes back byte for byte; only `SYSa` is dropped.
        let written = SvdFile::parse(&pack.to_svd().unwrap()).unwrap();
        assert_eq!(written.chunks, file.chunks[..SOUND_CHUNKS.len()]);
    }

    #[test]
    fn drum_kit_banks_are_pack_refs() {
        let typed = |msb, lsb, pc| ToneRef::from_bank_select(BankSelect { msb, lsb, pc });
        let snd = ToneRef::Pack {
            kind: ChunkType::SnDrumKit,
            index: 63,
        };
        let pcmd = ToneRef::Pack {
            kind: ChunkType::PcmDrumKit,
            index: 31,
        };
        assert_eq!(typed(88, 0, 63), snd);
        assert_eq!(typed(86, 0, 31), pcmd);
        // Past the last User kit.
        assert_eq!(
            typed(86, 0, 32),
            ToneRef::Bank {
                msb: 86,
                lsb: 0,
                pc: 32
            }
        );

        let mut set = studio_set();
        set.set_tone_ref(4, snd).unwrap();
        set.set_tone_ref(5, pcmd).unwrap();
        let pack = Pack::from_svd(&Pack { entries: vec![set] }.to_svd().unwrap()).unwrap();
        let refs = pack.entries[0].tone_refs();
        assert_eq!((refs[4], refs[5]), (snd, pcmd));
    }

    #[test]
    fn bank_selects_type_as_pack_or_bank() {
        let typed = |msb, lsb, pc| ToneRef::from_bank_select(BankSelect { msb, lsb, pc });
        assert_eq!(
            typed(95, 3, 127),
            ToneRef::Pack {
                kind: SNS,
                index: 511
            }
        );
        assert_eq!(
            typed(87, 1, 0),
            ToneRef::Pack {
                kind: ChunkType::PcmSynthTone,
                index: 128
            }
        );
        // SN-S preset, User Studio Set bank, past the last User slot.
        for (msb, lsb, pc) in [(95, 64, 0), (85, 0, 0), (89, 2, 0)] {
            assert_eq!(typed(msb, lsb, pc), ToneRef::Bank { msb, lsb, pc });
        }
    }

    #[test]
    fn rejects_refs_a_part_cannot_hold() {
        let mut set = studio_set();
        let past = ToneRef::Pack {
            kind: SNS,
            index: 512,
        };
        let set_ref = ToneRef::Pack {
            kind: ChunkType::StudioSet,
            index: 0,
        };
        let wide = ToneRef::Bank {
            msb: 128,
            lsb: 0,
            pc: 0,
        };
        for r in [past, set_ref, wide] {
            assert!(set.set_tone_ref(0, r).is_err(), "{r:?}");
        }
        let ok = ToneRef::Pack {
            kind: SNS,
            index: 0,
        };
        assert!(set.set_tone_ref(16, ok).is_err());
        assert!(entry(SNS, "Tone").set_tone_ref(0, ok).is_err());
    }
}
