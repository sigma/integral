//! Importing a [`Pack`] into User memory, following an [`ImportPlan`].
//!
//! [`PackImport`] does no I/O: the host sends what
//! [`PackImport::next_action`] asks for through `DeviceState`'s throttled
//! queue (raw, so DT1s to the same address are not coalesced), feeds read
//! replies to [`PackImport::on_read`] and write replies to
//! [`PackImport::on_write_reply`], and calls [`PackImport::on_timeout`]
//! when a wait runs out. When the import is done, the host refreshes mixer
//! state: the Temporary areas changed.
//!
//! Storing goes through a Temporary area plus the write request, because a
//! direct DT1 to a User slot is RAM-only (`docs/midi/99-undocumented.md`
//! §2 and §4). The import:
//!
//! 1. **Snapshots** the Temporary areas it borrows: the borrowed Part's
//!    Studio Set Part block and Temporary tone for tones; the whole
//!    Temporary Studio Set and all 16 Temporary tones for Studio Sets,
//!    because loading a set replaces them. Each Part's tone may be of any
//!    type, so the Part blocks are read first and each Part's tone is read
//!    in the layout of the type its Bank Select MSB names. A Part whose MSB
//!    names no type has no tone snapshot: restoring its Part block reloads
//!    its tone, without unsaved edits.
//! 2. **Stores tones**: selects the destination slot on the borrowed Part,
//!    so the Part plays a tone of the right type, DT1s the tone over it and
//!    sends the write request.
//! 3. **Stores Studio Sets**, each Part that selects a tone of the Pack
//!    relinked to the slot that tone was stored in: DT1 into the Temporary
//!    Studio Set, then the write request.
//! 4. **Restores** the snapshot by DT1: Studio Set blocks first (their
//!    Bank Selects reload the Parts' tones), then the tones over them.
//!
//! [`PackImport::reinit`] stores INIT into User slots the same way, but
//! sends the device's Init instead of DT1ing data
//! (`docs/midi/99-undocumented.md` §7): for a tone it selects the slot on
//! the borrowed Part and sends Tone Init, for a Studio Set it sends Studio
//! Set Init. It awaits "Init finished" before the write request.
//!
//! A plan whose only entry goes to Temporary ([`plan_import`] sends
//! nothing else there) loads it there (a tone into
//! the borrowed Part, selecting User slot 1 of its type first) and stops:
//! no snapshot, no write, no restore.
//!
//! Cancel and failure stop between slots: a write already requested is
//! awaited, the rest is left unstored, and the snapshot is restored. A
//! snapshot read failure aborts before anything is sent.
//!
//! Reference: decisions #65 and #66.
//!
//! [`plan_import`]: super::plan::plan_import

use std::collections::VecDeque;

use thiserror::Error;

use super::foreign::studio_set_fx;
use super::pack::{Pack, PackEntry, ToneRef, user_memory};
use super::plan::{ImportPlan, Placement, pointed_entry};
use super::reader::{BlockData, BlockReader};
use crate::address::{self, Address, DataSize};
use crate::device_spec::{INTEGRA7, USER_MEMORY, UserMemorySpec};
use crate::initialize::{InitTarget, build_init_request, is_init_finished};
use crate::params::{self, part};
use crate::svd::ChunkType;
use crate::sysex;
use crate::user_write::{
    BankSelect, WritePhase, WriteSource, build_write_request, parse_write_reply,
};

/// How long to wait for "write finished" (the device takes 2.3–3.0 s).
pub const WRITE_TIMEOUT_MS: f64 = 5_000.0;

/// Why an import can't start, or stopped early.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ImportError {
    /// The plan was made for another Pack.
    #[error("the plan doesn't match the Pack")]
    PlanMismatch,
    /// These entries have no destination.
    #[error("entries {0:?} have no destination")]
    Unplaced(Vec<usize>),
    /// The borrowed Part doesn't exist.
    #[error("no Part {}", .0 + 1)]
    InvalidPart(u8),
    /// A snapshot block didn't arrive whole; nothing was sent.
    #[error("snapshot: no complete reply for the block at {0}")]
    Snapshot(Address),
    /// No "Init finished" or "write finished" for this entry within
    /// [`WRITE_TIMEOUT_MS`].
    #[error("entry {0}: the device didn't confirm the write")]
    WriteTimeout(usize),
    /// There is no User slot of this number.
    #[error("no User slot {}", .0 + 1)]
    InvalidSlot(u16),
    /// The host cancelled the import.
    #[error("import cancelled")]
    Cancelled,
    /// Effects apply only to a single entry loaded into Temporary.
    #[error("effects apply only to a single entry loaded into Temporary")]
    EffectsNeedTemporary,
}

/// Where an entry stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub enum EntryStatus {
    /// Not stored (yet).
    Pending,
    /// Being sent, or its write awaits confirmation.
    Storing,
    /// Stored in its User slot.
    Stored,
    /// Loaded into Temporary, not stored.
    Loaded,
    /// Its write was not confirmed: the slot's content is unknown.
    Failed,
    /// Left out by the plan.
    Excluded,
}

/// What the import did, indexed like the Pack's entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportReport {
    /// Each entry's final status; [`EntryStatus::Pending`] means not stored.
    pub statuses: Vec<EntryStatus>,
    /// Why the import stopped early, if it did.
    pub error: Option<ImportError>,
}

/// What the host should do next.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportAction {
    /// Send this message; nothing comes back.
    Send(Vec<u8>),
    /// Send this RQ1 and feed its DT1 replies to [`PackImport::on_read`].
    Read(Vec<u8>),
    /// Nothing to send: feed DT1s, or call [`PackImport::on_timeout`] once
    /// the reply timeout elapses.
    Wait,
    /// Feed DT1s to [`PackImport::on_write_reply`] until it returns `true`,
    /// or call [`PackImport::on_timeout`] after `timeout_ms`. Awaits an Init
    /// as well as a write.
    AwaitWrite {
        /// How long to wait for "write finished", in ms.
        timeout_ms: f64,
    },
    /// The import is over; refresh mixer state.
    Done(ImportReport),
}

/// One entry's messages, and where to write it.
struct Job {
    entry: usize,
    sends: Vec<Vec<u8>>,
    /// Whether `sends` ends with an Init to await.
    init: bool,
    write: Option<(BankSelect, WriteSource)>,
}

/// A write request: the slot and what to store in it.
type Write = (BankSelect, WriteSource);

/// The device reply the import waits for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Awaiting {
    Init,
    Write,
}

/// Stores a [`Pack`] into User memory. Pure: the host does the I/O.
pub struct PackImport {
    device_id: u8,
    statuses: Vec<EntryStatus>,
    /// Studio Set Parts whose tone the snapshot covers.
    parts: Vec<u8>,
    /// In store order: tones, then Studio Sets.
    jobs: VecDeque<Job>,
    /// Messages to send before anything else.
    outbox: VecDeque<Vec<u8>>,
    /// The job being sent, whether its Init is still to await, and its
    /// write.
    current: Option<(usize, bool, Option<Write>)>,
    awaiting: Option<Awaiting>,
    /// The snapshot read in flight; `tones` once Part tones are read.
    reader: Option<(BlockReader, bool)>,
    /// Blocks read, in restore order; `None` when nothing is borrowed.
    snapshot: Option<Vec<(Address, Vec<u8>)>>,
    error: Option<ImportError>,
    done: bool,
}

impl PackImport {
    /// Prepare importing `pack` as `plan` says, borrowing Part `part`
    /// (0-indexed; the selected Part) to store tones. With `apply_effects`,
    /// a single entry loaded into Temporary also sets the Temporary Studio
    /// Set's Chorus and Reverb from its Conversion report.
    pub fn new(
        device_id: u8,
        pack: &Pack,
        plan: &ImportPlan,
        part: u8,
        apply_effects: bool,
    ) -> Result<Self, ImportError> {
        let placements = plan.placements();
        if placements.len() != pack.entries.len() {
            return Err(ImportError::PlanMismatch);
        }
        let unplaced: Vec<usize> = plan.unplaced().collect();
        if !unplaced.is_empty() {
            return Err(ImportError::Unplaced(unplaced));
        }
        if part >= INTEGRA7.part_count {
            return Err(ImportError::InvalidPart(part));
        }

        let dt1s = |memory: &UserMemorySpec, base: Address, entry: &PackEntry| {
            memory
                .blocks(base)
                .zip(entry.blocks())
                .map(|((addr, _), data)| sysex::build_dt1(device_id, &addr, data))
                .collect::<Vec<_>>()
        };
        let mut jobs = Vec::new();
        for (i, (entry, &placement)) in pack.entries.iter().zip(placements).enumerate() {
            let memory = user_memory(entry.kind()).ok_or(ImportError::PlanMismatch)?;
            let (slot, write) = match placement {
                Placement::User(slot) => (slot, true),
                Placement::Temporary => (0, false),
                Placement::Unplaced | Placement::Excluded => continue,
            };
            let target = memory.bank_select(slot).ok_or(ImportError::PlanMismatch)?;
            let job = if entry.kind() == ChunkType::StudioSet {
                // Parts whose tone the Pack stores are relinked to its slot;
                // a dump's other Parts keep the device's User slots.
                let set = relink(pack, plan, entry);
                Job {
                    entry: i,
                    sends: dt1s(memory, address::STUDIO_SET, &set),
                    init: false,
                    write: write.then_some((target, WriteSource::StudioSet)),
                }
            } else {
                let mut sends = vec![select(device_id, part, target)];
                sends.extend(dt1s(memory, memory.temporary_base(part), entry));
                Job {
                    entry: i,
                    sends,
                    init: false,
                    write: write.then_some((target, WriteSource::Part(part))),
                }
            };
            jobs.push(job);
        }
        jobs.sort_by_key(|j| pack.entries[j.entry].kind() == ChunkType::StudioSet);
        if apply_effects {
            match jobs.as_mut_slice() {
                [job] if job.write.is_none() => job.sends.extend(studio_set_fx::effects_dt1s(
                    device_id,
                    plan.effects(job.entry),
                )),
                _ => return Err(ImportError::EffectsNeedTemporary),
            }
        }

        let stores_sets = jobs
            .iter()
            .any(|j| j.write.is_some() && pack.entries[j.entry].kind() == ChunkType::StudioSet);
        let statuses = placements
            .iter()
            .map(|&p| match p {
                Placement::Excluded => EntryStatus::Excluded,
                _ => EntryStatus::Pending,
            })
            .collect();
        Ok(Self::start(device_id, statuses, jobs, stores_sets, part))
    }

    /// Prepare storing INIT into the User `slots` (0-indexed) of type
    /// `kind`, borrowing Part `part` (0-indexed) to store tones. Statuses
    /// are indexed like `slots`.
    pub fn reinit(
        device_id: u8,
        kind: ChunkType,
        slots: &[u16],
        part: u8,
    ) -> Result<Self, ImportError> {
        let memory = user_memory(kind).ok_or(ImportError::PlanMismatch)?;
        if part >= INTEGRA7.part_count {
            return Err(ImportError::InvalidPart(part));
        }
        let stores_sets = kind == ChunkType::StudioSet;
        let jobs = slots
            .iter()
            .enumerate()
            .map(|(i, &slot)| {
                let target = memory
                    .bank_select(slot)
                    .ok_or(ImportError::InvalidSlot(slot))?;
                let (sends, source) = if stores_sets {
                    let init = build_init_request(device_id, InitTarget::StudioSet);
                    (vec![init], WriteSource::StudioSet)
                } else {
                    // Tone Init keeps the tone's type: select one of the right type first.
                    let init = build_init_request(device_id, InitTarget::Tone(part));
                    (
                        vec![select(device_id, part, target), init],
                        WriteSource::Part(part),
                    )
                };
                Ok(Job {
                    entry: i,
                    sends,
                    init: true,
                    write: Some((target, source)),
                })
            })
            .collect::<Result<_, _>>()?;
        let statuses = vec![EntryStatus::Pending; slots.len()];
        Ok(Self::start(device_id, statuses, jobs, stores_sets, part))
    }

    /// An import of `jobs`, snapshotting what they borrow: the whole
    /// Temporary Studio Set and every Part's tone if they store Studio
    /// Sets, else Part `part`'s block and tone.
    fn start(
        device_id: u8,
        statuses: Vec<EntryStatus>,
        jobs: Vec<Job>,
        stores_sets: bool,
        part: u8,
    ) -> Self {
        let set_memory = user_memory(ChunkType::StudioSet).expect("Studio Sets have a layout");
        let set_blocks: Vec<(Address, DataSize)> = set_memory.blocks(address::STUDIO_SET).collect();
        let (parts, blocks) = if stores_sets {
            ((0..INTEGRA7.part_count).collect(), set_blocks)
        } else {
            let at = params::part_address(part, [0; 3]);
            let block = set_blocks.into_iter().filter(|&(a, _)| a == at).collect();
            (vec![part], block)
        };
        let borrows = jobs.iter().any(|j| j.write.is_some());

        Self {
            device_id,
            statuses,
            parts,
            jobs: jobs.into(),
            outbox: VecDeque::new(),
            current: None,
            awaiting: None,
            reader: borrows.then(|| (BlockReader::new(device_id, blocks), false)),
            snapshot: borrows.then(Vec::new),
            error: None,
            done: false,
        }
    }

    /// Each entry's status, indexed like the Pack's entries.
    pub fn progress(&self) -> &[EntryStatus] {
        &self.statuses
    }

    /// Stop between slots: a write already requested is still awaited,
    /// then the snapshot is restored. Before anything was sent, the import
    /// ends at once; once restoring, it has nothing left to cancel.
    pub fn cancel(&mut self) {
        let idle = self.reader.is_none() && self.current.is_none() && self.jobs.is_empty();
        if self.done || self.error.is_some() || idle {
            return;
        }
        self.error = Some(ImportError::Cancelled);
        if self.reader.take().is_some() {
            self.snapshot = None;
        }
        // An awaited Init is dropped: its slot is left unstored.
        if self.awaiting != Some(Awaiting::Write)
            && let Some((entry, ..)) = self.current.take()
        {
            self.awaiting = None;
            self.outbox.clear();
            self.statuses[entry] = EntryStatus::Pending;
        }
        self.jobs.clear();
    }

    /// The next step.
    pub fn next_action(&mut self) -> ImportAction {
        loop {
            if self.done {
                return ImportAction::Done(ImportReport {
                    statuses: self.statuses.clone(),
                    error: self.error.clone(),
                });
            }
            if let Some((reader, _)) = &mut self.reader {
                if !reader.is_done() {
                    return reader
                        .next_request()
                        .map_or(ImportAction::Wait, ImportAction::Read);
                }
                self.finish_read();
                continue;
            }
            if self.awaiting.is_some() {
                return ImportAction::AwaitWrite {
                    timeout_ms: WRITE_TIMEOUT_MS,
                };
            }
            if let Some(msg) = self.outbox.pop_front() {
                return ImportAction::Send(msg);
            }
            if let Some((entry, init, write)) = self.current {
                if init {
                    self.current = Some((entry, false, write));
                    self.awaiting = Some(Awaiting::Init);
                    continue;
                }
                match write {
                    Some((target, source)) => {
                        self.awaiting = Some(Awaiting::Write);
                        return ImportAction::Send(build_write_request(
                            self.device_id,
                            target,
                            source,
                        ));
                    }
                    None => {
                        self.statuses[entry] = EntryStatus::Loaded;
                        self.current = None;
                    }
                }
                continue;
            }
            if let Some(job) = self.jobs.pop_front() {
                self.statuses[job.entry] = EntryStatus::Storing;
                self.outbox = job.sends.into();
                self.current = Some((job.entry, job.init, job.write));
                continue;
            }
            if let Some(snapshot) = self.snapshot.take() {
                self.outbox = snapshot
                    .iter()
                    .map(|(addr, data)| sysex::build_dt1(self.device_id, addr, data))
                    .collect();
                continue;
            }
            self.done = true;
        }
    }

    /// Feed an incoming DT1. Returns `true` if it belonged to the snapshot
    /// read in flight.
    pub fn on_read(&mut self, address: &Address, data: &[u8]) -> bool {
        self.reader
            .as_mut()
            .is_some_and(|(reader, _)| reader.on_dt1(address, data))
    }

    /// Feed an incoming DT1 while awaiting an Init or a write. Returns
    /// `true` if it was the awaited "Init finished" or "write finished".
    pub fn on_write_reply(&mut self, address: &Address, data: &[u8]) -> bool {
        if self.awaiting == Some(Awaiting::Init) {
            let finished = is_init_finished(address, data);
            if finished {
                self.awaiting = None;
            }
            return finished;
        }
        let Some((entry, _, Some((target, source)))) = self.current else {
            return false;
        };
        let finished = self.awaiting == Some(Awaiting::Write)
            && parse_write_reply(address, data).is_some_and(|r| {
                r.phase == WritePhase::Finished && r.target == target && r.source == source
            });
        if finished {
            self.statuses[entry] = EntryStatus::Stored;
            self.current = None;
            self.awaiting = None;
        }
        finished
    }

    /// The host's timeout elapsed: stop waiting for the current snapshot
    /// block, or give up on the awaited Init or write and stop.
    pub fn on_timeout(&mut self) {
        if let Some((reader, _)) = &mut self.reader {
            reader.on_timeout();
        } else if self.awaiting.is_some()
            && let Some((entry, ..)) = self.current.take()
        {
            self.awaiting = None;
            self.statuses[entry] = EntryStatus::Failed;
            self.error.get_or_insert(ImportError::WriteTimeout(entry));
            self.jobs.clear();
        }
    }

    /// A snapshot read is over: keep its blocks, then read the Parts'
    /// tones, or abort if a block is missing.
    fn finish_read(&mut self) {
        let Some((reader, tones)) = self.reader.take() else {
            return;
        };
        let snapshot = self.snapshot.get_or_insert_with(Vec::new);
        for read in reader.result() {
            match read.data {
                BlockData::Complete(data) => snapshot.push((read.address, data)),
                BlockData::Gap { .. } | BlockData::Timeout => {
                    self.snapshot = None;
                    self.jobs.clear();
                    self.error = Some(ImportError::Snapshot(read.address));
                    self.done = true;
                    return;
                }
            }
        }
        if tones {
            return;
        }
        let msb_at = |p: u8| params::part_address(p, part::TONE_BANK_MSB);
        let mut blocks = Vec::new();
        for &p in &self.parts {
            let msb = snapshot.iter().find_map(|(addr, data)| {
                let at = msb_at(p).value().checked_sub(addr.value())?;
                data.get(at as usize).copied()
            });
            let memory = msb
                .and_then(params::tone_type_from_bank_msb)
                .and_then(|tt| USER_MEMORY.iter().find(|m| m.tone_type == Some(tt)));
            if let Some(m) = memory {
                blocks.extend(m.blocks(m.temporary_base(p)));
            }
        }
        self.reader = Some((BlockReader::new(self.device_id, blocks), true));
    }
}

/// The DT1 that points Part `part` at the User slot `target`, so the Part
/// plays a tone of its type.
fn select(device_id: u8, part: u8, target: BankSelect) -> Vec<u8> {
    sysex::build_dt1(
        device_id,
        &params::part_address(part, part::TONE_BANK_MSB),
        &[target.msb, target.lsb, target.pc],
    )
}

/// `set` with each Part that selects a tone of the Pack pointed at the User
/// slot the plan stores that tone in: from a sound export or backup, a
/// reference names a Pack position; from a dump, the User slot the tone was
/// dumped from. Other references are left as they are: a broken, excluded
/// or unmatched one points at the User slot of its number.
fn relink(pack: &Pack, plan: &ImportPlan, set: &PackEntry) -> PackEntry {
    let placements = plan.placements();
    let origin = plan.origin();
    let mut set = set.clone();
    for (p, r) in (0..).zip(set.tone_refs()) {
        let ToneRef::Pack { kind, index } = r else {
            continue;
        };
        let slot = pointed_entry(pack, origin, kind, index).and_then(|i| match placements[i] {
            Placement::User(slot) => Some(slot),
            _ => None,
        });
        if let Some(slot) = slot {
            set.set_tone_ref(p, ToneRef::Pack { kind, index: slot })
                .expect("a planned slot exists");
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::librarian::plan::{Origin, plan_import, plan_read};
    use crate::parse_helpers::encode_ascii_name;

    const SET: ChunkType = ChunkType::StudioSet;
    const SNS: ChunkType = ChunkType::SnSynthTone;
    const PCMS: ChunkType = ChunkType::PcmSynthTone;
    const PART: u8 = 2;
    const FINISHED: Address = Address::new(0x0F, 0x00, 0x10, 0x02);
    const STARTED: Address = Address::new(0x0F, 0x00, 0x10, 0x01);

    /// An entry of `kind` named `name`, every other byte `fill`.
    fn entry(kind: ChunkType, name: &str, fill: u8) -> PackEntry {
        let m = user_memory(kind).unwrap();
        let mut blocks: Vec<Vec<u8>> = m
            .blocks(m.temporary_base(0))
            .map(|(_, size)| vec![fill; size.value() as usize])
            .collect();
        let len = kind.name_length().unwrap();
        blocks[0][..len].copy_from_slice(&encode_ascii_name(name, len));
        PackEntry::new(kind, None, blocks).unwrap()
    }

    fn bank(msb: u8, lsb: u8, pc: u8) -> BankSelect {
        BankSelect { msb, lsb, pc }
    }

    /// What the host saw, in order.
    #[derive(Debug, Clone, PartialEq)]
    enum Event {
        Read(Address),
        Dt1(Address, Vec<u8>),
        Write(BankSelect, WriteSource),
        Init(InitTarget),
    }

    /// Byte-addressed Temporary memory; `writes` says whether the device
    /// confirms writes.
    struct Device {
        mem: BTreeMap<u32, u8>,
        writes: bool,
        events: Vec<Event>,
    }

    impl Device {
        /// Every Temporary area, filled with each byte's low 7 address
        /// bits. Part 1 plays an SN-S tone, Part 2 a PCM-S tone, Part 3
        /// (the borrowed one) an SN-S tone; other Parts select no type.
        fn new() -> Self {
            let mut device = Self {
                mem: BTreeMap::new(),
                writes: true,
                events: Vec::new(),
            };
            let set = user_memory(SET).unwrap();
            device.fill(set.blocks(address::STUDIO_SET));
            for m in USER_MEMORY.iter().filter(|m| m.tone_type.is_some()) {
                for p in 0..INTEGRA7.part_count {
                    device.fill(m.blocks(m.temporary_base(p)));
                }
            }
            for (p, msb) in (0..INTEGRA7.part_count).zip([95, 87, 95]) {
                device.put(params::part_address(p, part::TONE_BANK_MSB), &[msb]);
            }
            for p in 3..INTEGRA7.part_count {
                device.put(params::part_address(p, part::TONE_BANK_MSB), &[0]);
            }
            device
        }

        fn fill(&mut self, blocks: impl Iterator<Item = (Address, DataSize)>) {
            for (addr, size) in blocks {
                for v in addr.value()..addr.value() + size.value() {
                    self.mem.insert(v, (v & 0x7F) as u8);
                }
            }
        }

        fn put(&mut self, addr: Address, data: &[u8]) {
            for (v, &b) in (addr.value()..).zip(data) {
                self.mem.insert(v, b);
            }
        }

        fn get(&self, addr: Address, len: u32) -> Option<Vec<u8>> {
            (addr.value()..addr.value() + len)
                .map(|v| self.mem.get(&v).copied())
                .collect()
        }

        /// Drive `import` to the end.
        fn run(&mut self, import: &mut PackImport) -> ImportReport {
            loop {
                if let Some(report) = self.step(import) {
                    return report;
                }
            }
        }

        /// One action; the report once done.
        fn step(&mut self, import: &mut PackImport) -> Option<ImportReport> {
            match import.next_action() {
                ImportAction::Read(rq1) => {
                    let addr = Address::new(rq1[7], rq1[8], rq1[9], rq1[10]);
                    let size = DataSize::new(rq1[11], rq1[12], rq1[13], rq1[14]);
                    self.events.push(Event::Read(addr));
                    match self.get(addr, size.value()) {
                        Some(data) => assert!(import.on_read(&addr, &data)),
                        None => import.on_timeout(),
                    }
                }
                ImportAction::Send(msg) if msg[6] == 0x11 && msg[9] == 0x18 => {
                    self.events.push(Event::Init(match msg[10] {
                        0x00 => InitTarget::StudioSet,
                        _ => InitTarget::Tone(msg[13]),
                    }));
                }
                ImportAction::Send(msg) if msg[6] == 0x11 => {
                    let source = match msg[14] {
                        0x7F => WriteSource::StudioSet,
                        p => WriteSource::Part(p),
                    };
                    let target = bank(msg[11], msg[12], msg[13]);
                    self.events.push(Event::Write(target, source));
                }
                ImportAction::Send(msg) => {
                    assert!(msg.len() <= 256);
                    let addr = Address::new(msg[7], msg[8], msg[9], msg[10]);
                    let data = msg[11..msg.len() - 2].to_vec();
                    self.put(addr, &data);
                    self.events.push(Event::Dt1(addr, data));
                }
                ImportAction::AwaitWrite { timeout_ms } => {
                    assert_eq!(timeout_ms, WRITE_TIMEOUT_MS);
                    if let Some(Event::Init(_)) = self.events.last() {
                        let started = Address::new(0x0F, 0x00, 0x18, 0x01);
                        let finished = Address::new(0x0F, 0x00, 0x18, 0x02);
                        assert!(!import.on_write_reply(&started, &[0; 4]));
                        if self.writes {
                            assert!(import.on_write_reply(&finished, &[0; 4]));
                        } else {
                            import.on_timeout();
                        }
                        return None;
                    }
                    let Some(&Event::Write(t, s)) = self.events.last() else {
                        panic!("awaiting a write that wasn't requested");
                    };
                    let src = match s {
                        WriteSource::StudioSet => 0x7F,
                        WriteSource::Part(p) => p,
                    };
                    let reply = [0, 0, 0, t.msb, t.lsb, t.pc, src];
                    assert!(!import.on_write_reply(&STARTED, &reply));
                    if self.writes {
                        assert!(import.on_write_reply(&FINISHED, &reply));
                    } else {
                        import.on_timeout();
                    }
                }
                ImportAction::Wait => import.on_timeout(),
                ImportAction::Done(report) => return Some(report),
            }
            None
        }

        /// Whether every snapshotted byte is back as in `before`. Only the
        /// borrowed Part's PCM-S area may differ: the Part plays SN-S, so it
        /// is not snapshotted, and it is inactive once restored.
        fn restored(&self, before: &BTreeMap<u32, u8>) -> bool {
            let pcms = user_memory(PCMS).unwrap();
            let inactive: Vec<_> = pcms
                .blocks(pcms.temporary_base(PART))
                .map(|(a, s)| a.value()..a.value() + s.value())
                .collect();
            self.mem.len() == before.len()
                && self
                    .mem
                    .iter()
                    .filter(|&(&v, _)| !inactive.iter().any(|r| r.contains(&v)))
                    .all(|(v, b)| before.get(v) == Some(b))
        }

        /// The Bank Select Part `p` had when the Studio Set was written.
        fn stored_bank(&self, p: u8) -> [u8; 3] {
            let write_set = self
                .events
                .iter()
                .position(|e| matches!(e, Event::Write(_, WriteSource::StudioSet)))
                .unwrap();
            self.events[..write_set]
                .iter()
                .rev()
                .find_map(|e| match e {
                    Event::Dt1(a, d) if *a == params::part_address(p, [0; 3]) => {
                        d[6..9].try_into().ok()
                    }
                    _ => None,
                })
                .unwrap()
        }

        fn writes(&self) -> Vec<(BankSelect, WriteSource)> {
            self.events
                .iter()
                .filter_map(|e| match e {
                    Event::Write(t, s) => Some((*t, *s)),
                    _ => None,
                })
                .collect()
        }
    }

    /// A Studio Set whose Part 1 plays SN-S tone 2 of the Pack and Part 2
    /// SN-S tone 1, with three SN-S tones (the third unreferenced) and a
    /// PCM-S tone.
    fn pack() -> Pack {
        let mut set = entry(SET, "Set", 0);
        set.set_tone_ref(
            0,
            ToneRef::Pack {
                kind: SNS,
                index: 1,
            },
        )
        .unwrap();
        set.set_tone_ref(
            1,
            ToneRef::Pack {
                kind: SNS,
                index: 0,
            },
        )
        .unwrap();
        Pack {
            entries: vec![
                set,
                entry(SNS, "Tone A", 1),
                entry(SNS, "Tone B", 2),
                entry(PCMS, "Pcm", 3),
                entry(SNS, "Tone C", 4),
            ],
        }
    }

    /// Set → 5, Tone A → 10, Tone B → 130, PCM → 1; Tone C excluded.
    fn plan(pack: &Pack) -> ImportPlan {
        let mut plan = plan_import(pack, Origin::Sound, &[], None);
        for (entry, slot) in [(0, 4), (1, 9), (2, 129), (3, 0)] {
            plan.assign(entry, slot).unwrap();
        }
        plan.exclude(4).unwrap();
        plan
    }

    #[test]
    fn snapshots_stores_tones_then_relinked_sets_and_restores() {
        let pack = pack();
        let mut device = Device::new();
        let before = device.mem.clone();
        let mut import = PackImport::new(0x10, &pack, &plan(&pack), PART, false).unwrap();
        let report = device.run(&mut import);

        use EntryStatus::*;
        assert_eq!(report.statuses, [Stored, Stored, Stored, Stored, Excluded]);
        assert_eq!(report.error, None);
        assert_eq!(
            device.writes(),
            [
                (bank(95, 0, 9), WriteSource::Part(PART)),
                (bank(95, 1, 1), WriteSource::Part(PART)),
                (bank(87, 0, 0), WriteSource::Part(PART)),
                (bank(85, 0, 4), WriteSource::StudioSet),
            ]
        );

        // Snapshot first: the set, then the tones of Parts 1–3 by type.
        let reads = device
            .events
            .iter()
            .take_while(|e| matches!(e, Event::Read(_)))
            .count();
        let count = |kind: ChunkType| {
            let m = user_memory(kind).unwrap();
            m.blocks(m.temporary_base(0)).count()
        };
        // Parts 1 and 3 play SN-S, Part 2 PCM-S.
        assert_eq!(reads, count(SET) + 2 * count(SNS) + count(PCMS));
        assert_eq!(
            device.events[count(SET)],
            Event::Read(user_memory(SNS).unwrap().temporary_base(0))
        );

        // Each tone selects its slot on the borrowed Part, then loads.
        let select = params::part_address(PART, part::TONE_BANK_MSB);
        assert_eq!(device.events[reads], Event::Dt1(select, vec![95, 0, 9]));
        let sns_base = user_memory(SNS).unwrap().temporary_base(PART);
        assert_eq!(
            device.events[reads + 1],
            Event::Dt1(sns_base, pack.entries[1].blocks()[0].clone())
        );

        // The set's Parts point at the slots their tones were stored in.
        assert_eq!(device.stored_bank(0), [95, 1, 1]);
        assert_eq!(device.stored_bank(1), [95, 0, 9]);

        // Restored: every Temporary byte is back.
        assert!(matches!(device.events.last(), Some(Event::Dt1(..))));
        assert!(device.restored(&before));
    }

    #[test]
    fn excluded_tone_keeps_the_raw_reference() {
        let pack = pack();
        let mut plan = plan(&pack);
        plan.exclude(2).unwrap();
        let mut device = Device::new();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        device.run(&mut import);

        // Part 1 played Tone B (Pack position 2): it stays on User slot 2.
        assert_eq!(device.stored_bank(0), [95, 0, 1]);
        assert_eq!(device.stored_bank(1), [95, 0, 9]);
    }

    #[test]
    fn a_dump_keeps_its_device_references() {
        let pack = pack();
        let mut plan = plan_import(&pack, Origin::Dump, &[], None);
        for (entry, slot) in [(0, 4), (1, 9), (2, 129), (3, 0)] {
            plan.assign(entry, slot).unwrap();
        }
        plan.exclude(4).unwrap();
        let mut device = Device::new();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        device.run(&mut import);

        // The Parts still play User slots 2 and 1, as on the dumping device.
        assert_eq!(device.stored_bank(0), [95, 0, 1]);
        assert_eq!(device.stored_bank(1), [95, 0, 0]);
    }

    /// The `.syx` bytes dumping `entry` at User slot `slot` of its type.
    fn user_dump(entry: &PackEntry, slot: u16) -> Vec<u8> {
        let memory = user_memory(entry.kind()).unwrap();
        memory
            .blocks(memory.user_address(slot).unwrap())
            .zip(entry.blocks())
            .flat_map(|((address, _), data)| crate::sysex::build_dt1(0x10, &address, data))
            .collect()
    }

    /// A set dumped from User slot 4, Part 1 on User SN-S slot 2 (and Part
    /// 2 on slot 0 when `with_part2`), with `slot`'s SN-S tone alongside.
    fn dumped_pack(tone_slot: u16, with_part2: bool) -> (Pack, ImportPlan) {
        let mut set = entry(SET, "Dumped", 0);
        set.set_tone_ref(
            0,
            ToneRef::Pack {
                kind: SNS,
                index: 2,
            },
        )
        .unwrap();
        if with_part2 {
            set.set_tone_ref(
                1,
                ToneRef::Pack {
                    kind: SNS,
                    index: 0,
                },
            )
            .unwrap();
        }
        let mut bytes = user_dump(&set, 4);
        bytes.extend(user_dump(&entry(SNS, "Tone", 1), tone_slot));
        let read = Pack::read(&bytes).unwrap();
        let plan = plan_read(&read, Origin::of(&read), &[], None);
        (read.pack, plan)
    }

    #[test]
    fn a_dump_with_its_tone_relinks_the_set() {
        let (pack, mut plan) = dumped_pack(2, true);
        plan.assign(0, 5).unwrap();
        plan.assign(1, 9).unwrap();
        // Part 2 plays User slot 0, which the dump doesn't carry; Part 1's
        // tone is imported, so only it warns.
        assert_eq!(plan.warnings().count(), 1);
        let mut device = Device::new();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        device.run(&mut import);

        // Part 1 now plays the slot the tone was stored in; Part 2 keeps
        // the User slot 0 it played on the dumping device.
        assert_eq!(device.stored_bank(0), [95, 0, 9]);
        assert_eq!(device.stored_bank(1), [95, 0, 0]);
    }

    #[test]
    fn a_dump_with_an_unrelated_tone_keeps_the_device_reference() {
        let (pack, mut plan) = dumped_pack(9, false);
        assert_eq!(plan.warnings().count(), 1);
        plan.assign(0, 5).unwrap();
        plan.assign(1, 7).unwrap();
        let mut device = Device::new();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        device.run(&mut import);

        // Part 1 still plays User slot 2, as on the dumping device — not
        // the slot 7 the unrelated tone was stored in.
        assert_eq!(device.stored_bank(0), [95, 0, 2]);
    }

    #[test]
    fn tones_only_snapshot_the_borrowed_part() {
        let pack = Pack {
            entries: vec![entry(SNS, "A", 1), entry(SNS, "B", 2)],
        };
        let mut plan = plan_import(&pack, Origin::Sound, &[], None);
        plan.assign(0, 0).unwrap();
        plan.assign(1, 1).unwrap();
        let mut device = Device::new();
        let before = device.mem.clone();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        device.run(&mut import);

        let reads: Vec<_> = device
            .events
            .iter()
            .filter_map(|e| match e {
                Event::Read(a) => Some(*a),
                _ => None,
            })
            .collect();
        assert_eq!(reads.len(), 1 + 5);
        assert_eq!(reads[0], params::part_address(PART, [0; 3]));
        assert_eq!(device.writes().len(), 2);
        assert!(device.restored(&before));
    }

    #[test]
    fn cancel_awaits_the_write_then_restores() {
        let pack = pack();
        let mut device = Device::new();
        let before = device.mem.clone();
        let mut import = PackImport::new(0x10, &pack, &plan(&pack), PART, false).unwrap();
        while device.writes().is_empty() {
            assert!(device.step(&mut import).is_none());
        }
        import.cancel();
        let report = device.run(&mut import);

        use EntryStatus::*;
        assert_eq!(
            report.statuses,
            [Pending, Stored, Pending, Pending, Excluded]
        );
        assert_eq!(report.error, Some(ImportError::Cancelled));
        assert_eq!(device.writes().len(), 1);
        assert!(device.restored(&before));
        // Cancelling again, or after the end, changes nothing.
        import.cancel();
        assert_eq!(import.next_action(), ImportAction::Done(report));
    }

    #[test]
    fn cancel_mid_entry_drops_it() {
        let pack = pack();
        let mut device = Device::new();
        let before = device.mem.clone();
        let mut import = PackImport::new(0x10, &pack, &plan(&pack), PART, false).unwrap();
        while !matches!(device.events.last(), Some(Event::Dt1(..))) {
            device.step(&mut import);
        }
        import.cancel();
        let report = device.run(&mut import);
        assert_eq!(report.statuses[1], EntryStatus::Pending);
        assert!(device.writes().is_empty());
        assert!(device.restored(&before));
    }

    #[test]
    fn cancel_during_snapshot_sends_nothing() {
        let pack = pack();
        let mut device = Device::new();
        let mut import = PackImport::new(0x10, &pack, &plan(&pack), PART, false).unwrap();
        device.step(&mut import);
        import.cancel();
        let report = device.run(&mut import);
        assert_eq!(report.error, Some(ImportError::Cancelled));
        assert!(device.events.iter().all(|e| matches!(e, Event::Read(_))));
    }

    #[test]
    fn write_timeout_fails_the_entry_and_restores() {
        let pack = pack();
        let mut device = Device::new();
        device.writes = false;
        let before = device.mem.clone();
        let mut import = PackImport::new(0x10, &pack, &plan(&pack), PART, false).unwrap();
        let report = device.run(&mut import);

        use EntryStatus::*;
        assert_eq!(
            report.statuses,
            [Pending, Failed, Pending, Pending, Excluded]
        );
        assert_eq!(report.error, Some(ImportError::WriteTimeout(1)));
        assert_eq!(device.writes().len(), 1);
        assert!(device.restored(&before));
    }

    /// The events after the snapshot reads.
    fn after_snapshot(device: &Device) -> &[Event] {
        let reads = device
            .events
            .iter()
            .take_while(|e| matches!(e, Event::Read(_)))
            .count();
        &device.events[reads..]
    }

    #[test]
    fn reinit_tones_select_the_slot_init_then_write() {
        let mut device = Device::new();
        let before = device.mem.clone();
        let mut import = PackImport::reinit(0x10, SNS, &[9, 129], PART).unwrap();
        let report = device.run(&mut import);

        assert_eq!(report.statuses, [EntryStatus::Stored; 2]);
        assert_eq!(report.error, None);
        let select = params::part_address(PART, part::TONE_BANK_MSB);
        assert_eq!(
            after_snapshot(&device)[..6],
            [
                Event::Dt1(select, vec![95, 0, 9]),
                Event::Init(InitTarget::Tone(PART)),
                Event::Write(bank(95, 0, 9), WriteSource::Part(PART)),
                Event::Dt1(select, vec![95, 1, 1]),
                Event::Init(InitTarget::Tone(PART)),
                Event::Write(bank(95, 1, 1), WriteSource::Part(PART)),
            ]
        );
        assert!(device.restored(&before));
    }

    #[test]
    fn reinit_sets_snapshot_everything_and_init_the_studio_set() {
        let mut device = Device::new();
        let before = device.mem.clone();
        let mut import = PackImport::reinit(0x10, SET, &[4], PART).unwrap();
        let report = device.run(&mut import);

        assert_eq!(report.statuses, [EntryStatus::Stored]);
        let count = |kind: ChunkType| {
            let m = user_memory(kind).unwrap();
            m.blocks(m.temporary_base(0)).count()
        };
        let reads = device.events.len() - after_snapshot(&device).len();
        assert_eq!(reads, count(SET) + 2 * count(SNS) + count(PCMS));
        assert_eq!(
            after_snapshot(&device)[..2],
            [
                Event::Init(InitTarget::StudioSet),
                Event::Write(bank(85, 0, 4), WriteSource::StudioSet),
            ]
        );
        assert!(device.restored(&before));
    }

    #[test]
    fn reinit_cancel_during_init_drops_the_slot() {
        let mut device = Device::new();
        let before = device.mem.clone();
        let mut import = PackImport::reinit(0x10, SNS, &[9, 10], PART).unwrap();
        while !matches!(device.events.last(), Some(Event::Init(_))) {
            device.step(&mut import);
        }
        import.cancel();
        let report = device.run(&mut import);
        assert_eq!(report.statuses, [EntryStatus::Pending; 2]);
        assert_eq!(report.error, Some(ImportError::Cancelled));
        assert!(device.writes().is_empty());
        assert!(device.restored(&before));
    }

    #[test]
    fn reinit_unconfirmed_init_fails_the_slot() {
        let mut device = Device::new();
        device.writes = false;
        let mut import = PackImport::reinit(0x10, SNS, &[9, 10], PART).unwrap();
        let report = device.run(&mut import);
        use EntryStatus::*;
        assert_eq!(report.statuses, [Failed, Pending]);
        assert_eq!(report.error, Some(ImportError::WriteTimeout(0)));
        assert!(device.writes().is_empty());
    }

    #[test]
    fn reinit_rejects_missing_slots_and_parts() {
        let err = |r: Result<PackImport, ImportError>| r.err();
        assert_eq!(
            err(PackImport::reinit(0x10, SNS, &[512], PART)),
            Some(ImportError::InvalidSlot(512))
        );
        assert_eq!(
            err(PackImport::reinit(0x10, SNS, &[0], 16)),
            Some(ImportError::InvalidPart(16))
        );
    }

    #[test]
    fn snapshot_failure_aborts_before_sending() {
        let pack = pack();
        let mut device = Device::new();
        // Part 2's PCM-S Common 2 doesn't answer.
        let pcms = user_memory(PCMS).unwrap();
        let (common2, _) = pcms.blocks(pcms.temporary_base(1)).last().unwrap();
        device.mem.remove(&common2.value());
        let mut import = PackImport::new(0x10, &pack, &plan(&pack), PART, false).unwrap();
        let report = device.run(&mut import);

        assert_eq!(report.error, Some(ImportError::Snapshot(common2)));
        assert!(device.events.iter().all(|e| matches!(e, Event::Read(_))));
        assert!(
            report
                .statuses
                .iter()
                .all(|s| matches!(s, EntryStatus::Pending | EntryStatus::Excluded))
        );
    }

    #[test]
    fn lone_tone_loads_into_temporary() {
        let pack = Pack {
            entries: vec![entry(PCMS, "Lone", 5)],
        };
        let plan = plan_import(&pack, Origin::Sound, &[], None);
        assert_eq!(plan.placements(), [Placement::Temporary]);
        let mut device = Device::new();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        let report = device.run(&mut import);

        assert_eq!(report.statuses, [EntryStatus::Loaded]);
        assert_eq!(report.error, None);
        assert!(device.writes().is_empty());
        let pcms = user_memory(PCMS).unwrap();
        let blocks = pcms.blocks(pcms.temporary_base(PART)).count();
        assert_eq!(device.events.len(), 1 + blocks);
        assert_eq!(
            device.events[0],
            Event::Dt1(
                params::part_address(PART, part::TONE_BANK_MSB),
                vec![87, 0, 0]
            )
        );
        let common = pcms.temporary_base(PART);
        let len = pack.entries[0].blocks()[0].len() as u32;
        assert_eq!(
            device.get(common, len).as_deref(),
            Some(&pack.entries[0].blocks()[0][..])
        );
    }

    #[test]
    fn lone_set_loads_into_temporary() {
        let pack = Pack {
            entries: vec![entry(SET, "Lone Set", 0)],
        };
        let plan = plan_import(&pack, Origin::Sound, &[], None);
        let mut device = Device::new();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        let report = device.run(&mut import);

        assert_eq!(report.statuses, [EntryStatus::Loaded]);
        assert!(device.writes().is_empty());
        assert_eq!(
            device.events.len(),
            user_memory(SET)
                .unwrap()
                .blocks(address::STUDIO_SET)
                .count()
        );
        assert!(
            device
                .events
                .iter()
                .all(|e| matches!(e, Event::Dt1(a, _) if a.0[0] == 0x18))
        );
    }

    #[test]
    fn rejects_unready_plans() {
        let pack = pack();
        let plan = plan_import(&pack, Origin::Sound, &[], None);
        assert_eq!(
            PackImport::new(0x10, &pack, &plan, PART, false).err(),
            Some(ImportError::Unplaced(vec![0, 1, 2, 3, 4]))
        );
        let ready = self::plan(&pack);
        assert_eq!(
            PackImport::new(0x10, &pack, &ready, 16, false).err(),
            Some(ImportError::InvalidPart(16))
        );
        let other = Pack {
            entries: vec![entry(SNS, "A", 0)],
        };
        assert_eq!(
            PackImport::new(0x10, &other, &ready, PART, false).err(),
            Some(ImportError::PlanMismatch)
        );
        // Effects apply only to an audition.
        assert_eq!(
            PackImport::new(0x10, &pack, &ready, PART, true).err(),
            Some(ImportError::EffectsNeedTemporary)
        );
    }

    #[test]
    fn every_block_fits_one_sysex_message() {
        for m in USER_MEMORY {
            for (addr, size) in m.blocks(m.temporary_base(15)) {
                let data = vec![0; size.value() as usize];
                assert!(sysex::build_dt1(0x10, &addr, &data).len() <= 256, "{addr}");
            }
        }
    }

    #[test]
    fn empty_plan_does_nothing() {
        let pack = Pack {
            entries: vec![entry(SNS, "A", 0), entry(SNS, "B", 0)],
        };
        let mut plan = plan_import(&pack, Origin::Sound, &[], None);
        plan.exclude(0).unwrap();
        plan.exclude(1).unwrap();
        let mut import = PackImport::new(0x10, &pack, &plan, PART, false).unwrap();
        assert!(matches!(import.next_action(), ImportAction::Done(r) if r.error.is_none()));
    }
}
