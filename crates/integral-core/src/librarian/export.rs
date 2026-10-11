//! Exporting Studio Sets and tones from the device into a [`Pack`].
//!
//! [`PackExport`] reads its sources without doing I/O: the host sends each
//! RQ1 that [`PackExport::next_action`] asks for through `DeviceState`'s
//! throttled queue, feeds every DT1 back through [`PackExport::on_read`],
//! and calls [`PackExport::on_timeout`] when its reply timeout elapses.
//! User slots are read directly, which is silent: nothing is loaded into a
//! Temporary area (`docs/midi/99-undocumented.md` §2).
//!
//! Studio Sets are read first. Every Part whose Bank Select names a User
//! tone, whatever its Receive or Mute state, pulls that tone into the Pack
//! (once, however many Parts or sets use it). Within a type, tones are
//! ordered by ascending User slot, and each Part reference is rewritten to
//! the tone's Pack position, as Roland's export does
//! (`docs/svd/11-sound-vs-backup.md`).
//!
//! A Part's Temporary tone has no slot: it sorts after every User tone of
//! its type, as if stored in the next free slot. It never stands in for the
//! User slot it was selected from, so a Studio Set referencing that slot
//! still gets the stored tone.

use core::fmt;

use thiserror::Error;

use super::pack::{Pack, PackEntry, ToneRef};
use super::reader::{BlockData, BlockReader};
use crate::address::{Address, DataSize};
use crate::device_spec::{INTEGRA7, USER_MEMORY, UserMemorySpec};
use crate::svd::ChunkType;

/// Something to export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "source", rename_all = "camelCase"))]
pub enum ExportSource {
    /// User slot `slot` (0-indexed) of type `kind`.
    User {
        /// The type.
        kind: ChunkType,
        /// The User slot, 0-indexed.
        slot: u16,
    },
    /// The Temporary tone of Part `part` (0-indexed), unsaved edits
    /// included. It has no slot.
    PartTone {
        /// The tone's type.
        kind: ChunkType,
        /// The Part, 0-indexed.
        part: u8,
    },
    /// The Temporary Studio Set, unsaved edits included. It has no slot.
    StudioSet,
}

impl ExportSource {
    /// The type of what this source reads.
    pub fn kind(self) -> ChunkType {
        match self {
            Self::User { kind, .. } | Self::PartTone { kind, .. } => kind,
            Self::StudioSet => ChunkType::StudioSet,
        }
    }

    /// Pack order: by type (Studio Sets first), then by slot; slotless
    /// sources last, by Part.
    fn sort_key(self) -> (usize, u16, u8) {
        let kind = memory(self.kind()).map_or(usize::MAX, |(i, _)| i);
        match self {
            Self::User { slot, .. } => (kind, slot, 0),
            Self::PartTone { part, .. } => (kind, u16::MAX, part),
            Self::StudioSet => (kind, u16::MAX, 0),
        }
    }

    /// The blocks to read, as `(address, size)`, or `None` if the source
    /// names nothing on the device.
    fn blocks(self) -> Option<Vec<(Address, DataSize)>> {
        let (_, m) = memory(self.kind())?;
        let base = match self {
            Self::User { slot, .. } => m.user_address(slot)?,
            Self::PartTone { part, .. } if m.tone_type.is_some() && part < INTEGRA7.part_count => {
                m.temporary_base(part)
            }
            Self::StudioSet => m.temporary_base(0),
            Self::PartTone { .. } => return None,
        };
        Some(m.blocks(base).collect())
    }
}

impl fmt::Display for ExportSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User { kind, slot } => write!(f, "User {kind} {}", slot + 1),
            Self::PartTone { kind, part } => write!(f, "Part {} {kind}", part + 1),
            Self::StudioSet => f.write_str("Temporary Studio Set"),
        }
    }
}

/// Why an export did not produce a Pack.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ExportError {
    /// The source names nothing the device holds.
    #[error("can't export {0}")]
    InvalidSource(ExportSource),
    /// A block of a source got no reply.
    #[error("{item}: no reply for the block at {address}")]
    Timeout {
        /// The source being read.
        item: ExportSource,
        /// Start of the block.
        address: Address,
    },
    /// A block of a source arrived with bytes missing.
    #[error("{item}: bytes missing in the block at {address}")]
    Gap {
        /// The source being read.
        item: ExportSource,
        /// Start of the block.
        address: Address,
    },
    /// The host cancelled the export.
    #[error("export cancelled")]
    Cancelled,
}

/// Where a source's read stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum EntryStatus {
    /// Not requested yet.
    Pending,
    /// Requests sent, blocks arriving.
    Reading,
    /// Every block arrived.
    Read,
    /// A block timed out or arrived with a gap.
    Failed,
}

/// What the host should do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportAction {
    /// Send this RQ1 and feed its DT1 replies to [`PackExport::on_read`].
    Read(Vec<u8>),
    /// Nothing to send: feed DT1s, or call [`PackExport::on_timeout`] once
    /// the reply timeout elapses.
    Wait,
    /// The export is over.
    Done(Result<Pack, ExportError>),
}

struct Item {
    source: ExportSource,
    status: EntryStatus,
    entry: Option<PackEntry>,
}

/// Reads sources into a [`Pack`]. Pure: the host does the I/O.
pub struct PackExport {
    device_id: u8,
    /// In Pack order; Studio Sets form a prefix, so they are read first.
    items: Vec<Item>,
    auto_included: Vec<ExportSource>,
    planned: bool,
    current: Option<(usize, BlockReader)>,
    outcome: Option<Result<Pack, ExportError>>,
}

impl PackExport {
    /// Prepare exporting `sources`; duplicates are read once.
    pub fn new(
        device_id: u8,
        sources: impl IntoIterator<Item = ExportSource>,
    ) -> Result<Self, ExportError> {
        let mut export = Self {
            device_id,
            items: Vec::new(),
            auto_included: Vec::new(),
            planned: false,
            current: None,
            outcome: None,
        };
        for source in sources {
            source.blocks().ok_or(ExportError::InvalidSource(source))?;
            export.add(source);
        }
        export.items.sort_by_key(|i| i.source.sort_key());
        Ok(export)
    }

    /// The User tones pulled in by Studio Set references and not among the
    /// sources, in Pack order. Complete once every Studio Set is read.
    pub fn auto_included(&self) -> &[ExportSource] {
        &self.auto_included
    }

    /// Every source with its status, in Pack order. Auto-included tones
    /// appear once every Studio Set is read.
    pub fn progress(&self) -> Vec<(ExportSource, EntryStatus)> {
        self.items.iter().map(|i| (i.source, i.status)).collect()
    }

    /// Stop the export. Reads borrow no Temporary area, so nothing needs
    /// restoring; late replies are ignored.
    pub fn cancel(&mut self) {
        if self.outcome.is_none() {
            self.current = None;
            self.outcome = Some(Err(ExportError::Cancelled));
        }
    }

    /// The next step.
    pub fn next_action(&mut self) -> ExportAction {
        loop {
            if let Some(outcome) = &self.outcome {
                return ExportAction::Done(outcome.clone());
            }
            if let Some((_, reader)) = &mut self.current {
                if !reader.is_done() {
                    return reader
                        .next_request()
                        .map_or(ExportAction::Wait, ExportAction::Read);
                }
                self.finish_read();
                continue;
            }
            let next = self
                .items
                .iter()
                .position(|i| i.status == EntryStatus::Pending);
            match next {
                Some(i) if self.planned || self.items[i].source.kind() == ChunkType::StudioSet => {
                    self.start_read(i)
                }
                _ if !self.planned => self.plan_tones(),
                _ => self.outcome = Some(Ok(self.pack())),
            }
        }
    }

    /// Feed an incoming DT1. Returns `true` if it belonged to the read in
    /// flight.
    pub fn on_read(&mut self, address: &Address, data: &[u8]) -> bool {
        self.current
            .as_mut()
            .is_some_and(|(_, reader)| reader.on_dt1(address, data))
    }

    /// The host's reply timeout elapsed: stop waiting for the current block.
    pub fn on_timeout(&mut self) {
        if let Some((_, reader)) = &mut self.current {
            reader.on_timeout();
        }
    }

    fn add(&mut self, source: ExportSource) -> bool {
        let new = self.items.iter().all(|i| i.source != source);
        if new {
            self.items.push(Item {
                source,
                status: EntryStatus::Pending,
                entry: None,
            });
        }
        new
    }

    fn start_read(&mut self, i: usize) {
        let item = &mut self.items[i];
        item.status = EntryStatus::Reading;
        let blocks = item.source.blocks().unwrap_or_default();
        self.current = Some((i, BlockReader::new(self.device_id, blocks)));
    }

    fn finish_read(&mut self) {
        let Some((i, reader)) = self.current.take() else {
            return;
        };
        let item = &mut self.items[i];
        let mut blocks = Vec::new();
        for read in reader.result() {
            let (source, address) = (item.source, read.address);
            let error = match read.data {
                BlockData::Complete(data) => {
                    blocks.push(data);
                    continue;
                }
                BlockData::Gap { .. } => ExportError::Gap {
                    item: source,
                    address,
                },
                BlockData::Timeout => ExportError::Timeout {
                    item: source,
                    address,
                },
            };
            item.status = EntryStatus::Failed;
            self.outcome = Some(Err(error));
            return;
        }
        let slot = match item.source {
            ExportSource::User { slot, .. } => Some(slot),
            _ => None,
        };
        item.entry = Some(
            PackEntry::new(item.source.kind(), slot, blocks)
                .expect("blocks were read from the type's own layout"),
        );
        item.status = EntryStatus::Read;
    }

    /// Every Studio Set is read: add the User tones their Parts reference.
    fn plan_tones(&mut self) {
        self.planned = true;
        let refs: Vec<_> = self
            .items
            .iter()
            .filter_map(|i| i.entry.as_ref())
            .flat_map(PackEntry::tone_refs)
            .filter_map(|r| match r {
                ToneRef::Pack { kind, index } => Some(ExportSource::User { kind, slot: index }),
                ToneRef::Bank { .. } => None,
            })
            .collect();
        for source in refs {
            if self.add(source) {
                self.auto_included.push(source);
            }
        }
        self.items.sort_by_key(|i| i.source.sort_key());
        self.auto_included.sort_by_key(|s| s.sort_key());
    }

    /// The Pack, with each Studio Set Part reference rewritten from User
    /// slot to Pack position.
    fn pack(&self) -> Pack {
        let position = |kind: ChunkType, slot: u16| {
            self.items
                .iter()
                .filter(|i| i.source.kind() == kind)
                .position(|i| i.source == ExportSource::User { kind, slot })
        };
        let mut entries: Vec<PackEntry> =
            self.items.iter().filter_map(|i| i.entry.clone()).collect();
        for entry in &mut entries {
            for (part, r) in (0..).zip(entry.tone_refs()) {
                let ToneRef::Pack { kind, index } = r else {
                    continue;
                };
                let index = position(kind, index)
                    .and_then(|p| u16::try_from(p).ok())
                    .expect("every referenced User tone is in the Pack");
                entry
                    .set_tone_ref(part, ToneRef::Pack { kind, index })
                    .expect("a Pack position never exceeds the User slot it replaces");
            }
        }
        Pack { entries }
    }
}

/// The User memory spec of `kind`, with its index in [`USER_MEMORY`].
fn memory(kind: ChunkType) -> Option<(usize, &'static UserMemorySpec)> {
    USER_MEMORY
        .iter()
        .enumerate()
        .find(|(_, m)| m.cli_name == kind.cli_name())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::parse_helpers::encode_ascii_name;

    const SNS: ChunkType = ChunkType::SnSynthTone;
    const PCMS: ChunkType = ChunkType::PcmSynthTone;
    const SET: ChunkType = ChunkType::StudioSet;

    fn user(kind: ChunkType, slot: u16) -> ExportSource {
        ExportSource::User { kind, slot }
    }

    /// Blocks and replies by block address.
    #[derive(Default)]
    struct Device(HashMap<Address, Vec<u8>>);

    impl Device {
        /// Store an entry named `name` at `source`, with Parts `(part, kind,
        /// slot)` selecting User tones and Part 16 a preset.
        fn put(&mut self, source: ExportSource, name: &str, refs: &[(u8, ChunkType, u16)]) {
            let (_, m) = memory(source.kind()).unwrap();
            let layout = source.blocks().unwrap();
            let mut blocks: Vec<Vec<u8>> = layout
                .iter()
                .map(|(_, s)| vec![0; s.value() as usize])
                .collect();
            let len = source.kind().name_length().unwrap();
            blocks[0][..len].copy_from_slice(&encode_ascii_name(name, len));
            let mut entry = PackEntry::new(source.kind(), None, blocks).unwrap();
            let preset = ToneRef::Bank {
                msb: 89,
                lsb: 64,
                pc: 0,
            };
            if m.tone_type.is_none() {
                entry.set_tone_ref(15, preset).unwrap();
            }
            for &(part, kind, slot) in refs {
                let bs = memory(kind).unwrap().1.bank_select(slot).unwrap();
                let r = ToneRef::Bank {
                    msb: bs.msb,
                    lsb: bs.lsb,
                    pc: bs.pc,
                };
                entry.set_tone_ref(part, r).unwrap();
            }
            for ((addr, _), data) in layout.into_iter().zip(entry.blocks()) {
                self.0.insert(addr, data.clone());
            }
        }

        /// Answer each RQ1 whole, or not at all.
        fn run(&self, export: &mut PackExport) -> Result<Pack, ExportError> {
            loop {
                match export.next_action() {
                    ExportAction::Read(rq1) => {
                        let addr = Address::new(rq1[7], rq1[8], rq1[9], rq1[10]);
                        match self.0.get(&addr) {
                            Some(data) => assert!(export.on_read(&addr, data)),
                            None => export.on_timeout(),
                        }
                    }
                    ExportAction::Wait => export.on_timeout(),
                    ExportAction::Done(result) => return result,
                }
            }
        }
    }

    fn names(pack: &Pack) -> Vec<String> {
        pack.entries.iter().map(PackEntry::name).collect()
    }

    fn pack_ref(kind: ChunkType, index: u16) -> ToneRef {
        ToneRef::Pack { kind, index }
    }

    /// User set 4 uses SN-S 101, 8 and PCM-S 201; the Temporary set shares
    /// SN-S 101 and adds SN-S 21. SN-S 41 and 8 are selected out of order,
    /// with Part 3's tone.
    fn device() -> Device {
        let mut device = Device::default();
        device.put(
            user(SET, 3),
            "User Set",
            &[(0, SNS, 100), (1, SNS, 7), (4, PCMS, 200)],
        );
        device.put(
            ExportSource::StudioSet,
            "Temp Set",
            &[(0, SNS, 100), (2, SNS, 20)],
        );
        for slot in [7, 20, 40, 100] {
            device.put(user(SNS, slot), &format!("SNS {}", slot + 1), &[]);
        }
        device.put(user(PCMS, 200), "PCMS 201", &[]);
        let part = ExportSource::PartTone { kind: SNS, part: 2 };
        device.put(part, "Part 3 Tone", &[]);
        device
    }

    fn sources() -> [ExportSource; 6] {
        [
            user(SNS, 40),
            ExportSource::PartTone { kind: SNS, part: 2 },
            ExportSource::StudioSet,
            user(SNS, 7),
            user(SET, 3),
            user(SNS, 40),
        ]
    }

    #[test]
    fn exports_sets_with_their_user_tones() {
        let mut export = PackExport::new(0x10, sources()).unwrap();
        let pack = device().run(&mut export).unwrap();

        assert_eq!(
            names(&pack),
            [
                "User Set",
                "Temp Set",
                "SNS 8",
                "SNS 21",
                "SNS 41",
                "SNS 101",
                "Part 3 Tone",
                "PCMS 201",
            ]
        );
        let slots: Vec<_> = pack.entries.iter().map(PackEntry::slot).collect();
        assert_eq!(
            slots,
            [
                Some(3),
                None,
                Some(7),
                Some(20),
                Some(40),
                Some(100),
                None,
                Some(200)
            ]
        );
        assert_eq!(
            export.auto_included(),
            [user(SNS, 20), user(SNS, 100), user(PCMS, 200)]
        );

        let user_set = pack.entries[0].tone_refs();
        assert_eq!(user_set[0], pack_ref(SNS, 3));
        assert_eq!(user_set[1], pack_ref(SNS, 0));
        assert_eq!(user_set[4], pack_ref(PCMS, 0));
        assert_eq!(
            user_set[15],
            ToneRef::Bank {
                msb: 89,
                lsb: 64,
                pc: 0
            }
        );
        let temp_set = pack.entries[1].tone_refs();
        assert_eq!(temp_set[0], pack_ref(SNS, 3));
        assert_eq!(temp_set[2], pack_ref(SNS, 1));
        // Zeroed Parts select no User bank.
        assert!(matches!(temp_set[5], ToneRef::Bank { .. }));

        assert!(
            export
                .progress()
                .iter()
                .all(|(_, s)| *s == EntryStatus::Read)
        );
    }

    #[test]
    fn reads_sets_before_planning_tones() {
        let device = device();
        let mut export = PackExport::new(0x10, sources()).unwrap();
        let ExportAction::Read(rq1) = export.next_action() else {
            panic!("expected a read");
        };
        let common = Address::new(0x2F, 0x03, 0x00, 0x00);
        assert_eq!(rq1[7..11], common.0);
        assert!(export.on_read(&common, &device.0[&common]));
        let progress = export.progress();
        assert_eq!(progress.len(), 5);
        assert_eq!(progress[0], (user(SET, 3), EntryStatus::Reading));
        assert!(export.auto_included().is_empty());
        device.run(&mut export).unwrap();
        assert_eq!(export.progress().len(), 8);
    }

    #[test]
    fn cancel_stops_and_ignores_late_replies() {
        let mut export = PackExport::new(0x10, [user(SNS, 0)]).unwrap();
        assert!(matches!(export.next_action(), ExportAction::Read(_)));
        export.cancel();
        assert_eq!(
            export.next_action(),
            ExportAction::Done(Err(ExportError::Cancelled))
        );
        let addr = memory(SNS).unwrap().1.user_address(0).unwrap();
        assert!(!export.on_read(&addr, &[0; 4]));
    }

    #[test]
    fn missing_reply_is_a_timeout() {
        let mut export = PackExport::new(0x10, [user(SNS, 9)]).unwrap();
        assert_eq!(
            device().run(&mut export),
            Err(ExportError::Timeout {
                item: user(SNS, 9),
                address: Address::new(0x50, 0x09, 0x00, 0x00),
            })
        );
        assert_eq!(export.progress()[0].1, EntryStatus::Failed);
    }

    #[test]
    fn partial_reply_is_a_gap() {
        let mut device = device();
        let mfx = Address::new(0x50, 0x07, 0x02, 0x00);
        device.0.get_mut(&mfx).unwrap().truncate(61);
        let mut export = PackExport::new(0x10, [user(SNS, 7)]).unwrap();
        assert_eq!(
            device.run(&mut export),
            Err(ExportError::Gap {
                item: user(SNS, 7),
                address: mfx,
            })
        );
    }

    #[test]
    fn rejects_sources_the_device_lacks() {
        for source in [
            user(SNS, 512),
            user(ChunkType::SystemSettings, 0),
            ExportSource::PartTone { kind: SET, part: 0 },
            ExportSource::PartTone {
                kind: SNS,
                part: 16,
            },
        ] {
            assert_eq!(
                PackExport::new(0x10, [source]).err(),
                Some(ExportError::InvalidSource(source))
            );
        }
    }

    #[test]
    fn nothing_to_export_is_an_empty_pack() {
        let mut export = PackExport::new(0x10, []).unwrap();
        assert_eq!(
            export.next_action(),
            ExportAction::Done(Ok(Pack::default()))
        );
    }
}
