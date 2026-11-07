//! Tone baselines: a Part's Temporary Tone as the device reported it right
//! after a Preset or Expansion tone was selected.
//!
//! Those tones never change, so the bytes read back at that moment are what
//! the tone is; recall (ADR 0009) sends the selection and then only the
//! fields that differ. A User tone keeps no baseline, because its slot can be
//! overwritten after the project is saved.
//!
//! [`Baselines`] is sans-IO bookkeeping. A tone is read as the whole blocks
//! its layout documents, one RQ1 at a time with core's [`BlockReader`], which
//! places the packets of a reply by address (a reply may be short, or come as
//! several DT1s). The service asks for each block in the turn of the client
//! whose select started the reading, so the reads share the device with
//! everyone else's work and that client's later items wait for the tone.
//!
//! A reply may arrive after the Part's tone has changed again, so every
//! request that left is remembered with the reading it belonged to; a reply
//! to one of a reading that is gone is thrown away instead of being taken
//! for the new tone's. Presets never change, so a finished reading is kept by
//! tone, as offsets from the Part's Temporary Tone, for the next time any
//! Part selects it. An Expansion tone depends on which Expansion is loaded,
//! so those are kept only for the session.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;
use std::sync::Arc;

use integral_core::address::{Address, DataSize};
use integral_core::librarian::reader::{BlockData, BlockReader};
use integral_core::params::temporary_tone_base;

use crate::protocol::{ClientId, ToneSelect};

/// Bytes of a tone by their offset from its Part's Temporary Tone.
type Offsets = Arc<BTreeMap<u32, u8>>;

/// A Part's tone read back right after it was selected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneBaseline {
    part: u8,
    tone: ToneSelect,
    bytes: Offsets,
}

impl ToneBaseline {
    /// The Part's Temporary Tone, as a number to subtract from addresses.
    fn base(&self) -> u32 {
        temporary_tone_base(self.part).value()
    }

    /// The tone it is the baseline of.
    pub fn tone(&self) -> ToneSelect {
        self.tone
    }

    /// The `len` bytes from `address`, if the baseline has all of them.
    pub fn get(&self, address: &Address, len: usize) -> Option<Vec<u8>> {
        let start = address.value().checked_sub(self.base())?;
        (0..u32::try_from(len).ok()?)
            .map(|i| self.bytes.get(&(start + i)).copied())
            .collect()
    }

    /// The bytes as runs of consecutive addresses, in address order.
    pub fn runs(&self) -> Vec<(Address, Vec<u8>)> {
        let mut runs: Vec<(u32, Vec<u8>)> = Vec::new();
        for (&at, &byte) in self.bytes.iter() {
            match runs.last_mut() {
                Some((start, bytes)) if *start + bytes.len() as u32 == at => bytes.push(byte),
                _ => runs.push((at, vec![byte])),
            }
        }
        let base = self.base();
        runs.into_iter()
            .map(|(at, bytes)| {
                let address = Address::from_value(base + at).expect("a tone address");
                (address, bytes)
            })
            .collect()
    }
}

/// A client's [`Request::Baseline`](crate::protocol::Request::Baseline), answered.
pub(crate) struct Settled {
    pub client: ClientId,
    pub part: u8,
    /// The baseline, or `None` if the Part has none.
    pub baseline: Option<ToneBaseline>,
}

/// What [`Baselines`] wants the service to do next.
#[derive(Default)]
pub(crate) struct Outcome {
    /// Answers to give.
    pub settled: Vec<Settled>,
    /// Readings that may ask for their next block: the client, the Part and
    /// the reading's generation.
    pub resume: Vec<(ClientId, u8, u64)>,
}

/// What became of a DT1 from the device.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Reply {
    /// Not an answer to a baseline request.
    NotMine,
    /// The answer to a request of a reading that is gone: ignore it.
    Stale,
    /// Part of the baseline of the Part's current tone.
    Stored,
}

/// What a client's turn for a reading comes to.
pub(crate) enum Step {
    /// The reading it was for is gone.
    Skip,
    /// Send this RQ1.
    Send(Vec<u8>),
    /// Every block has been asked for and answered.
    Done,
}

/// A tone a client has just selected on a Part.
pub(crate) struct Selected {
    pub part: u8,
    pub tone: ToneSelect,
    /// The client whose turns the reading takes.
    pub client: ClientId,
    /// The tone is an Expansion's.
    pub expansion: bool,
}

/// What starting to read a tone comes to.
pub(crate) enum Begin {
    /// The tone was read before; this is its baseline for the Part.
    Cached(ToneBaseline),
    /// A reading started, with this generation: ask for its first block.
    Reading(u64),
    /// There is nothing to read.
    Nothing,
}

/// An RQ1 that left, or is about to, and has not been answered.
struct Outstanding {
    part: u8,
    generation: u64,
    rq1: Vec<u8>,
    /// The addresses ([`Address::value`]) it asks for.
    range: Range<u32>,
    /// When to give up, from the moment it left.
    deadline: Option<f64>,
}

/// A tone being read.
struct Reading {
    generation: u64,
    client: ClientId,
    tone: ToneSelect,
    expansion: bool,
    blocks: Vec<(Address, DataSize)>,
    reader: BlockReader,
    /// Blocks asked for so far.
    requested: usize,
    /// A block is asked for and not yet answered or given up on.
    in_flight: bool,
    /// Something may have changed the tone while it was read (another
    /// client's write into it, a Program Change that may reach the Part): the
    /// bytes may not be the preset's, so they serve the current push but are
    /// not kept for the next.
    dirty: bool,
}

/// A tone read once.
struct Kept {
    bytes: Offsets,
    expansion: bool,
}

/// Every Part's baseline, finished or being read.
#[derive(Default)]
pub(crate) struct Baselines {
    done: HashMap<u8, ToneBaseline>,
    reading: HashMap<u8, Reading>,
    outstanding: Vec<Outstanding>,
    /// Clients waiting for a Part's reading to end.
    waiters: Vec<(ClientId, u8)>,
    kept: HashMap<ToneSelect, Kept>,
    generations: u64,
}

impl Baselines {
    /// The baseline of `part`, if its tone was read whole.
    pub(crate) fn get(&self, part: u8) -> Option<&ToneBaseline> {
        self.done.get(&part)
    }

    /// Start reading the tone `selected` as the whole `blocks` of it. The
    /// baseline of the Part's previous tone, if any, is dropped.
    pub(crate) fn begin(
        &mut self,
        selected: Selected,
        blocks: Vec<(Address, DataSize)>,
        device_id: u8,
        out: &mut Outcome,
    ) -> Begin {
        let Selected {
            part,
            tone,
            client,
            expansion,
        } = selected;
        self.cancel(part, out);
        if let Some(kept) = self.kept.get(&tone) {
            let baseline = ToneBaseline {
                part,
                tone,
                bytes: kept.bytes.clone(),
            };
            self.done.insert(part, baseline.clone());
            return Begin::Cached(baseline);
        }
        if blocks.is_empty() {
            return Begin::Nothing;
        }
        self.generations += 1;
        let generation = self.generations;
        self.reading.insert(
            part,
            Reading {
                generation,
                client,
                tone,
                expansion,
                reader: BlockReader::new(device_id, blocks.clone()),
                blocks,
                requested: 0,
                in_flight: false,
                dirty: false,
            },
        );
        Begin::Reading(generation)
    }

    /// Something may have changed `part`'s tone: if it is being read, what is
    /// read is not kept for later.
    pub(crate) fn touch(&mut self, part: u8) {
        if let Some(reading) = self.reading.get_mut(&part) {
            reading.dirty = true;
        }
    }

    /// An Expansion was loaded: its tones are not what they were. Forget the
    /// ones read, and do not keep the ones being read.
    pub(crate) fn expansion_changed(&mut self) {
        self.kept.retain(|_, kept| !kept.expansion);
        for reading in self.reading.values_mut().filter(|r| r.expansion) {
            reading.dirty = true;
        }
    }

    /// Whether `client` must wait: a block it asked for is not answered, or
    /// an answer to a request for the tone its Part had before is still due
    /// (it would be taken for this one's).
    pub(crate) fn blocked(&self, client: ClientId) -> bool {
        self.reading.iter().any(|(part, reading)| {
            reading.client == client
                && (reading.in_flight
                    || self
                        .outstanding
                        .iter()
                        .any(|o| o.part == *part && o.generation != reading.generation))
        })
    }

    /// The client's turn for the reading of `part` with `generation`.
    pub(crate) fn step(&mut self, part: u8, generation: u64, out: &mut Outcome) -> Step {
        let Some(reading) = self
            .reading
            .get_mut(&part)
            .filter(|r| r.generation == generation && !r.in_flight)
        else {
            return Step::Skip;
        };
        let Some(rq1) = reading.reader.next_request() else {
            self.finish(part, out);
            return Step::Done;
        };
        let (address, size) = reading.blocks[reading.requested];
        reading.requested += 1;
        reading.in_flight = true;
        let start = address.value();
        self.outstanding.push(Outstanding {
            part,
            generation,
            rq1: rq1.clone(),
            range: start..start + size.value(),
            deadline: None,
        });
        Step::Send(rq1)
    }

    /// Forget `part`'s baseline and what is being read for it, and answer the
    /// clients waiting for it with none. The requests that already left stay
    /// remembered, so their answers are recognised as stale.
    pub(crate) fn cancel(&mut self, part: u8, out: &mut Outcome) {
        self.done.remove(&part);
        self.reading.remove(&part);
        self.settle(part, out);
    }

    /// [`cancel`](Self::cancel) every Part's.
    pub(crate) fn cancel_all(&mut self, out: &mut Outcome) {
        let parts: Vec<u8> = self
            .done
            .keys()
            .chain(self.reading.keys())
            .copied()
            .collect();
        for part in parts {
            self.cancel(part, out);
        }
        for (client, part) in std::mem::take(&mut self.waiters) {
            out.settled.push(Settled {
                client,
                part,
                baseline: None,
            });
        }
    }

    /// The session ended: drop everything but the Preset tones read, and
    /// return the clients that were waiting for a baseline.
    pub(crate) fn reset(&mut self) -> Vec<(ClientId, u8)> {
        self.done.clear();
        self.reading.clear();
        self.outstanding.clear();
        self.kept.retain(|_, kept| !kept.expansion);
        std::mem::take(&mut self.waiters)
    }

    /// A client asks for `part`'s baseline: the answer now, or `None` if it
    /// has to wait for the reading, in which case it is given later in
    /// [`Outcome::settled`].
    pub(crate) fn ask(&mut self, client: ClientId, part: u8) -> Option<Settled> {
        if self.reading.contains_key(&part) {
            self.waiters.push((client, part));
            return None;
        }
        Some(Settled {
            client,
            part,
            baseline: self.done.get(&part).cloned(),
        })
    }

    /// A message left for the device at `now_ms`: if it is one of the
    /// requests, its answer has until then plus `timeout_ms`.
    pub(crate) fn sent(&mut self, message: &[u8], now_ms: f64, timeout_ms: f64) {
        if let Some(request) = self
            .outstanding
            .iter_mut()
            .find(|o| o.deadline.is_none() && o.rq1 == message)
        {
            request.deadline = Some(now_ms + timeout_ms);
        }
    }

    /// Take a DT1 from the device if it answers a request.
    pub(crate) fn reply(&mut self, address: &Address, data: &[u8], out: &mut Outcome) -> Reply {
        let at = address.value();
        let Some(i) = self.outstanding.iter().position(|o| o.range.contains(&at)) else {
            return Reply::NotMine;
        };
        let (part, generation) = (self.outstanding[i].part, self.outstanding[i].generation);
        let Some(reading) = self
            .reading
            .get_mut(&part)
            .filter(|r| r.generation == generation && r.in_flight)
        else {
            self.outstanding.remove(i);
            return Reply::Stale;
        };
        reading.reader.on_dt1(address, data);
        if !reading.reader.is_waiting() {
            self.outstanding.remove(i);
            self.advance(part, out);
        }
        Reply::Stored
    }

    /// Give up on the requests unanswered at `now_ms`; a block that gets
    /// nothing is read as missing, and the reading goes on.
    pub(crate) fn expire(&mut self, now_ms: f64, out: &mut Outcome) {
        let (late, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.outstanding)
            .into_iter()
            .partition(|o| o.deadline.is_some_and(|d| d <= now_ms));
        self.outstanding = waiting;
        for request in late {
            let Some(reading) = self
                .reading
                .get_mut(&request.part)
                .filter(|r| r.generation == request.generation && r.in_flight)
            else {
                continue;
            };
            reading.reader.on_timeout();
            self.advance(request.part, out);
        }
    }

    /// When the next request times out.
    pub(crate) fn next_deadline(&self) -> Option<f64> {
        self.outstanding
            .iter()
            .filter_map(|o| o.deadline)
            .min_by(f64::total_cmp)
    }

    /// The block in flight for `part` is over: ask for the next, or finish.
    fn advance(&mut self, part: u8, out: &mut Outcome) {
        let Some(reading) = self.reading.get_mut(&part) else {
            return;
        };
        reading.in_flight = false;
        if reading.reader.is_done() {
            self.finish(part, out);
        } else {
            out.resume.push((reading.client, part, reading.generation));
        }
    }

    /// Every block has been asked for: keep what arrived.
    fn finish(&mut self, part: u8, out: &mut Outcome) {
        let Some(reading) = self.reading.remove(&part) else {
            return;
        };
        let base = temporary_tone_base(part).value();
        let mut bytes = BTreeMap::new();
        let mut whole = true;
        for block in reading.reader.result() {
            let start = block.address.value() - base;
            let (data, missing) = match block.data {
                BlockData::Complete(data) => (data, Vec::new()),
                BlockData::Gap { data, missing } => (data, missing),
                BlockData::Timeout => {
                    whole = false;
                    continue;
                }
            };
            whole &= missing.is_empty();
            for (i, byte) in (0u32..).zip(data) {
                if !missing.iter().any(|m| m.contains(&(i as usize))) {
                    bytes.insert(start + i, byte);
                }
            }
        }
        if !bytes.is_empty() {
            let bytes = Arc::new(bytes);
            if whole && !reading.dirty {
                self.kept.insert(
                    reading.tone,
                    Kept {
                        bytes: bytes.clone(),
                        expansion: reading.expansion,
                    },
                );
            }
            self.done.insert(
                part,
                ToneBaseline {
                    part,
                    tone: reading.tone,
                    bytes,
                },
            );
        }
        self.settle(part, out);
    }

    /// Answer the clients waiting for `part`.
    fn settle(&mut self, part: u8, out: &mut Outcome) {
        let (ready, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.waiters)
            .into_iter()
            .partition(|(_, p)| *p == part);
        self.waiters = waiting;
        for (client, part) in ready {
            out.settled.push(Settled {
                client,
                part,
                baseline: self.done.get(&part).cloned(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TONE: ToneSelect = ToneSelect {
        msb: 87,
        lsb: 64,
        pc: 0,
    };
    const CLIENT: ClientId = ClientId(1);

    fn sel(part: u8, tone: ToneSelect, expansion: bool) -> Selected {
        Selected {
            part,
            tone,
            client: CLIENT,
            expansion,
        }
    }

    fn block(part: u8, offset: u32, len: u32) -> (Address, DataSize) {
        (
            Address::from_value(temporary_tone_base(part).value() + offset).unwrap(),
            DataSize::from_value(len).unwrap(),
        )
    }

    fn start(
        b: &mut Baselines,
        part: u8,
        tone: ToneSelect,
        blocks: Vec<(Address, DataSize)>,
    ) -> u64 {
        let mut out = Outcome::default();
        match b.begin(sel(part, tone, false), blocks, 0x10, &mut out) {
            Begin::Reading(generation) => generation,
            _ => panic!("expected a reading"),
        }
    }

    fn step(b: &mut Baselines, part: u8, generation: u64) -> Step {
        b.step(part, generation, &mut Outcome::default())
    }

    fn answer(b: &mut Baselines, address: Address, bytes: &[u8]) -> (Reply, Outcome) {
        let mut out = Outcome::default();
        let reply = b.reply(&address, bytes, &mut out);
        (reply, out)
    }

    #[test]
    fn a_tone_is_read_block_by_block_and_kept_once_the_last_is_in() {
        let mut b = Baselines::default();
        let blocks = vec![block(2, 0, 4), block(2, 100, 2)];
        let generation = start(&mut b, 2, TONE, blocks.clone());
        assert!(b.ask(ClientId(7), 2).is_none(), "still reading");
        assert!(!b.blocked(CLIENT));

        assert!(matches!(step(&mut b, 2, generation), Step::Send(_)));
        assert!(b.blocked(CLIENT), "until the block is answered");
        // The block comes as two packets.
        let (reply, out) = answer(&mut b, blocks[0].0, &[1, 2]);
        assert_eq!(reply, Reply::Stored);
        assert!(out.resume.is_empty());
        let next = Address::from_value(blocks[0].0.value() + 2).unwrap();
        let (_, out) = answer(&mut b, next, &[3, 4]);
        assert_eq!(out.resume, [(CLIENT, 2, generation)]);
        assert!(!b.blocked(CLIENT));

        assert!(matches!(step(&mut b, 2, generation), Step::Send(_)));
        let (_, out) = answer(&mut b, blocks[1].0, &[5, 6]);
        assert!(out.resume.is_empty(), "that was the last");
        assert_eq!(out.settled.len(), 1, "the waiting client is answered");
        assert!(matches!(step(&mut b, 2, generation), Step::Skip));

        let baseline = b.get(2).expect("whole");
        assert_eq!(baseline.tone(), TONE);
        assert_eq!(baseline.get(&blocks[0].0, 4), Some(vec![1, 2, 3, 4]));
        assert_eq!(baseline.get(&blocks[1].0, 3), None);
        assert_eq!(baseline.runs().len(), 2);
    }

    #[test]
    fn a_short_reply_ends_with_the_timeout_and_keeps_what_arrived() {
        let mut b = Baselines::default();
        let blocks = vec![block(0, 0, 4)];
        let generation = start(&mut b, 0, TONE, blocks.clone());
        let Step::Send(rq1) = step(&mut b, 0, generation) else {
            panic!()
        };
        b.sent(&rq1, 1000.0, 2000.0);
        assert_eq!(b.next_deadline(), Some(3000.0));
        answer(&mut b, blocks[0].0, &[1, 2]);
        let mut out = Outcome::default();
        b.expire(3000.0, &mut out);
        assert_eq!(b.get(0).unwrap().get(&blocks[0].0, 2), Some(vec![1, 2]));
        assert_eq!(b.get(0).unwrap().get(&blocks[0].0, 3), None);
    }

    #[test]
    fn a_block_that_never_answers_leaves_no_baseline_and_answers_waiters_with_none() {
        let mut b = Baselines::default();
        let generation = start(&mut b, 0, TONE, vec![block(0, 0, 4)]);
        assert!(b.ask(ClientId(4), 0).is_none());
        let Step::Send(rq1) = step(&mut b, 0, generation) else {
            panic!()
        };
        b.sent(&rq1, 0.0, 2000.0);
        let mut out = Outcome::default();
        b.expire(2000.0, &mut out);
        assert!(b.get(0).is_none());
        assert_eq!(out.settled.len(), 1);
        assert!(out.settled[0].baseline.is_none());
    }

    #[test]
    fn a_reply_after_the_tone_changed_is_stale_even_after_a_cancel_of_everything() {
        let mut b = Baselines::default();
        let blocks = vec![block(1, 0, 2)];
        let generation = start(&mut b, 1, TONE, blocks.clone());
        step(&mut b, 1, generation);
        // A Studio Set select drops every baseline; the request is still out.
        b.cancel_all(&mut Outcome::default());
        let (reply, _) = answer(&mut b, blocks[0].0, &[9, 9]);
        assert_eq!(reply, Reply::Stale);
        let (reply, _) = answer(&mut b, blocks[0].0, &[9, 9]);
        assert_eq!(reply, Reply::NotMine, "once");
    }

    #[test]
    fn the_new_tone_waits_for_the_old_ones_answer() {
        let mut b = Baselines::default();
        let blocks = vec![block(1, 0, 2)];
        let old = start(&mut b, 1, TONE, blocks.clone());
        step(&mut b, 1, old);
        let newer = ToneSelect { pc: 1, ..TONE };
        start(&mut b, 1, newer, blocks.clone());
        assert!(b.blocked(CLIENT), "the answer for the first tone is due");
        let (reply, _) = answer(&mut b, blocks[0].0, &[1, 1]);
        assert_eq!(reply, Reply::Stale);
        assert!(!b.blocked(CLIENT));
    }

    #[test]
    fn a_tone_read_once_is_kept_for_any_part_and_expansion_tones_for_the_session() {
        let mut b = Baselines::default();
        let blocks = vec![block(0, 0, 2)];
        let mut out = Outcome::default();
        let generation = start(&mut b, 0, TONE, blocks);
        step(&mut b, 0, generation);
        answer(&mut b, block(0, 0, 2).0, &[7, 8]);
        let expansion = ToneSelect { msb: 93, ..TONE };
        let g = match b.begin(
            sel(0, expansion, true),
            vec![block(0, 0, 2)],
            0x10,
            &mut out,
        ) {
            Begin::Reading(g) => g,
            _ => panic!(),
        };
        step(&mut b, 0, g);
        answer(&mut b, block(0, 0, 2).0, &[1, 2]);

        // Another Part selects the same preset: nothing to read.
        let Begin::Cached(baseline) =
            b.begin(sel(5, TONE, false), vec![block(5, 0, 2)], 0x10, &mut out)
        else {
            panic!("kept")
        };
        assert_eq!(baseline.get(&block(5, 0, 2).0, 2), Some(vec![7, 8]));
        assert!(b.get(5).is_some());

        b.reset();
        assert!(matches!(
            b.begin(sel(3, TONE, false), vec![block(3, 0, 2)], 0x10, &mut out),
            Begin::Cached(_)
        ));
        assert!(matches!(
            b.begin(
                sel(3, expansion, true),
                vec![block(3, 0, 2)],
                0x10,
                &mut out
            ),
            Begin::Reading(_)
        ));
    }

    #[test]
    fn cancel_drops_the_baseline_and_the_asked_for_answer_is_none() {
        let mut b = Baselines::default();
        let generation = start(&mut b, 1, TONE, vec![block(1, 0, 1)]);
        step(&mut b, 1, generation);
        answer(&mut b, block(1, 0, 1).0, &[5]);
        assert!(b.get(1).is_some());
        b.cancel(1, &mut Outcome::default());
        assert!(b.get(1).is_none());
        assert!(b.ask(ClientId(1), 1).expect("at once").baseline.is_none());
        assert!(matches!(step(&mut b, 1, generation), Step::Skip));
    }

    #[test]
    fn a_reading_touched_meanwhile_serves_its_part_but_is_not_kept() {
        let mut b = Baselines::default();
        let mut out = Outcome::default();
        let generation = start(&mut b, 0, TONE, vec![block(0, 0, 2)]);
        step(&mut b, 0, generation);
        b.touch(0);
        answer(&mut b, block(0, 0, 2).0, &[7, 8]);
        assert!(b.get(0).is_some());
        assert!(matches!(
            b.begin(sel(1, TONE, false), vec![block(1, 0, 2)], 0x10, &mut out),
            Begin::Reading(_)
        ));
    }

    #[test]
    fn an_expansion_load_forgets_expansion_tones_read() {
        let mut b = Baselines::default();
        let mut out = Outcome::default();
        let expansion = ToneSelect { msb: 93, ..TONE };
        let Begin::Reading(g) = b.begin(
            sel(0, expansion, true),
            vec![block(0, 0, 2)],
            0x10,
            &mut out,
        ) else {
            panic!()
        };
        step(&mut b, 0, g);
        answer(&mut b, block(0, 0, 2).0, &[1, 2]);
        b.expansion_changed();
        assert!(matches!(
            b.begin(
                sel(1, expansion, true),
                vec![block(1, 0, 2)],
                0x10,
                &mut out
            ),
            Begin::Reading(_)
        ));
    }
}
