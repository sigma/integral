//! The device's side of an instance's host parameters: what other writers
//! and the device itself do to them, and the host hears of.
//!
//! The [`Follower`] runs on the instance's worker thread, fed the client's
//! events. It never calls the host: it posts [`HostTask`]s
//! ([`Instance::post`]) for the main thread. It does four things:
//!
//! - **A single-field change by another writer** (`Message::Changed`) is a
//!   gesture on that host parameter: begin, perform, and end once
//!   [`GESTURE_END`] has passed without another change to it. Automation in
//!   write or touch mode records it (ADR 0008, decision 5). The window is
//!   the follower's own and independent of the service's pacing: with many
//!   clients taking turns a change can reach the instance late, and a drag
//!   then arrives as one gesture or as several, whichever the gaps make it.
//! - **A bulk event, a (re)connection, a new session or lost events** make the
//!   values out of date. They are read again (below) and replaced silently,
//!   with one notification to the host (decision 6).
//! - **The instance ignores its own changes** as gestures: those tagged with
//!   its own client ID or its editor's ([`Instance::editor_client`]). A bulk
//!   event is never ignored on that account: a tone the editor loaded changed
//!   the values. An own change to an effect's type still counts as the
//!   device's new type.
//! - **It keeps the device's effect types** ([`ParamBank::device_type`]) from
//!   every change and read, whether or not the host can be told, because the
//!   host's value of a type parameter can be stale and a slot of the wrong
//!   type must not be written.
//!
//! # Writing on and off
//!
//! The instance writes the host's changes ([`Instance::arm`]) once it has
//! read the device the first time, and not while the link is down. An editor
//! opening, a new session or a bulk event about an MFX, Chorus or Reverb do
//! not stop it: what the device's side gives the host is marked
//! ([`ParamBank::expect`]) so that it is not written back. A host edit made
//! while writing is off is kept and written when it is back on: the host wins.
//!
//! # The Part's tone type
//!
//! A Tone or Kit plugin acts only on a Part that holds a tone of its type
//! (ADR 0007, [`ToneFit`]). Until it knows, the fit is [`ToneFit::Unknown`]
//! and the instance neither writes nor reads, so that a host edit cannot land
//! in a tone of another type; an edit made meanwhile is kept for when it is
//! back on. It learns the type from the service where it can:
//!
//! - **A bulk event that selected a tone** (`selected`) names it when the
//!   service knows it (it selected it, saw the select written whole, or read
//!   it): the type is taken from that, with no question to the device. A
//!   bulk of parameters (`selected` off) changes nothing of the type.
//! - **A selection the service cannot name** (a Program Change on the note
//!   port, a Studio Set switch or mixer load, a partial select write) is
//!   settled by the service: it reads the Part's select itself and announces
//!   it in another bulk event, with the tone (or without it again if the read
//!   failed). The instance goes [`ToneFit::Unknown`] and does not ask the
//!   device meanwhile, since the answer to its own request could be of the
//!   selection before; a probe under way is dropped and its answer ignored.
//!   Only if the service says nothing for [`SETTLE_WAIT`] does it ask.
//! - **Otherwise it asks the device**, with no bulk event to wait for: on a
//!   new Binding, a connection, a new session, or lost events. The Part's
//!   three select bytes ([`params::tone_select_block`]) are read as a raw RQ1
//!   like the read's. The request waits in the instance's queue of the
//!   service, so an answer can be late (sixteen instances starting together,
//!   a tone being read back, a recall): after [`READ_QUIET`] it is asked
//!   again after a pause that doubles up to [`PROBE_BACKOFF_MAX`], the
//!   instance stays unsure, and a late answer to any of the requests counts.
//!
//! - **Fits:** the read and the arming below follow, as for the first time.
//!   Tones of the GM2 and ExPCM banks count as fitting (their type is PCM
//!   Synth) though the device does not let them be edited: writes to them
//!   have no effect.
//! - **Mismatch:** the instance is inert. It writes nothing (a host edit is
//!   taken as seen, and the ones kept before are dropped: the device's
//!   values win when the type fits again), takes no change from the device,
//!   reads nothing and reports
//!   [`Notice::TypeMismatch`](super::Notice::TypeMismatch). Its parameter
//!   list stays as it is. The editor's button loads the plugin's default tone
//!   ([`Instance::request_default_tone`], selected by the follower as a
//!   `ChangeTone`); the service announces it with the tone, the Part fits,
//!   and instead of a read that would wait behind the service's own read-back
//!   of the tone, the follower takes the values from the Part's baseline
//!   (`Request::Baseline`) as the result of the read.
//!
//! A tone changed on the device's front panel is not announced to the
//! service (#464): the instance keeps the type it had until the next event
//! that asks. A Program Change on the note port is announced only once the
//! service has the receive settings that say which Parts it reaches, up to
//! `RX_WAIT_MS` after it, and does not name the tone, so the instance asks.
//!
//! The recall (#458) selects tones as part of its push: when the tone it
//! selects is of the instance's own type, [`Instance::selected`] sets the fit
//! so that the instance can be armed once the push is done.
//! # The read
//!
//! The values come from the same service path the editor reads the device
//! with: a request for each whole block that holds a parameter of the
//! instance's ([`plan`]; a tone's blocks are core's `tone_read_blocks`, a
//! key's block is cut from the key's parameters), sent as raw RQ1 through the
//! paced queue, with the raw replies switched on for the duration. A typed
//! read per parameter would take a 40 ms slot each, minutes for a PCM Synth
//! tone. The read ends when every block has arrived, or when the device has
//! been quiet for [`READ_QUIET`], whichever is first; what did not arrive
//! keeps the host's value. When the host cannot be told (no context) or the
//! values are not to be applied, only the blocks holding effect types are read.
//!
//! **The project wins.** An instance whose state the host restored does not
//! take the device's values on its first read (ADR 0009): the project's are
//! the push's to send (the recall, #458, must therefore run after this first
//! read has settled the instance, and write with `Push`, which bypasses the
//! service's cache). Later reads take the device's values.
//!
//! A change another writer makes while the read is under way is patched into
//! the bytes read, so the older bytes of the reply do not undo it.
//!
//! No part of this runs on the audio thread.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use integral_core::address::{Address, DataSize};
use integral_core::device_spec::USER_MEMORY;
use integral_core::host_params::{Anchor, Binding, PluginKind};
use integral_core::params;
use integral_core::sysex;
use integral_device_service::protocol::{
    BaselineRun, BulkKind, ClientId, Message, ParamId, Request, ToneSelect,
};
use integral_device_service::{Client, ClientEvent};

use super::host::HostTask;
use super::instance::{Instance, ToneFit};

/// How long a parameter goes without a change by another writer before the
/// gesture on it ends.
pub const GESTURE_END: Duration = Duration::from_millis(300);

/// How long after the host refused a bulk replace the follower tries again.
const REPLACE_RETRY: Duration = Duration::from_millis(250);

/// How long a read waits for the device to say something, from the request
/// and again from each reply. The service sends at most one message per 40 ms
/// and the device answers each in turn.
pub const READ_QUIET: Duration = Duration::from_secs(1);

/// The longest a read goes on, replies or not.
const READ_AT_MOST: Duration = Duration::from_secs(30);

/// How long after an unanswered probe the next is sent, doubling up to
/// [`PROBE_BACKOFF_MAX`].
const PROBE_BACKOFF_MIN: Duration = Duration::from_secs(1);

/// The longest wait between two probes of a device that does not answer.
const PROBE_BACKOFF_MAX: Duration = Duration::from_secs(16);

/// How long after a selection the service could not name the instance waits
/// for the service's own bulk event with the tone, before it asks the device
/// itself. The service reads the select first, and gives up on a read that is
/// not answered after a few tries.
const SETTLE_WAIT: Duration = Duration::from_secs(10);

/// What the follower asks of its connection.
pub trait Wire {
    /// Receive changes in `binding` (see `Request::Subscribe`).
    fn subscribe(&self, binding: Binding);
    /// Stop receiving changes in `binding`.
    fn unsubscribe(&self, binding: Binding);
    /// Turn on or off the device's raw SysEx.
    fn receive_raw(&self, on: bool);
    /// Send one SysEx message through the paced queue.
    fn raw(&self, bytes: Vec<u8>);
    /// Select a tone on a Part (`Request::ChangeTone`).
    fn change_tone(&self, part: u8, msb: u8, lsb: u8, pc: u8);
    /// Ask for the baseline of a Part's tone (`Request::Baseline`).
    fn baseline(&self, part: u8);
}

impl Wire for Client {
    fn subscribe(&self, binding: Binding) {
        Client::subscribe(self, binding);
    }

    fn unsubscribe(&self, binding: Binding) {
        Client::unsubscribe(self, binding);
    }

    fn receive_raw(&self, on: bool) {
        Client::receive_raw(self, on);
    }

    fn raw(&self, bytes: Vec<u8>) {
        Client::raw(self, bytes);
    }

    fn change_tone(&self, part: u8, msb: u8, lsb: u8, pc: u8) {
        Client::send(self, Request::ChangeTone { part, msb, lsb, pc });
    }

    fn baseline(&self, part: u8) {
        Client::send(
            self,
            Request::Baseline {
                binding: Binding::of_part(part),
            },
        );
    }
}

/// The subscription an instance bound to `binding` needs: its Part (a
/// key's changes are told by Part), or everything for the Studio Set plugin.
fn subscription(binding: Binding) -> Binding {
    Binding {
        part: binding.part,
        key: None,
    }
}

/// A gesture on parameter `index` that ends at `deadline` unless there is
/// another change.
struct Gesture {
    index: usize,
    deadline: Instant,
}

/// A read of the Part's tone select (see the module docs).
struct Probe {
    /// Where the select's first byte, the Bank Select MSB, is.
    address: Address,
    /// When this request is taken to be lost and asked again, or `None` once
    /// it has been: the follower still takes a late reply.
    deadline: Option<Instant>,
}

/// A run of bytes to read, in address units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block {
    start: u32,
    len: u32,
}

/// A read of the instance's blocks in progress.
struct Reading {
    binding: Binding,
    blocks: Vec<Block>,
    /// The bytes received that are in a block, by address.
    bytes: HashMap<u32, u8>,
    /// Addresses a change by another writer set during the read: a reply
    /// made before it must not undo it.
    patched: std::collections::HashSet<u32>,
    /// Whether the values are to be applied to the host, or only the effect
    /// types learned.
    apply: bool,
    /// When the read gives up unless the device says something.
    deadline: Instant,
    /// When the read gives up whatever the device says.
    last_chance: Instant,
}

impl Reading {
    fn complete(&self) -> bool {
        self.bytes.len() == self.blocks.iter().map(|b| b.len as usize).sum::<usize>()
    }

    /// Take `data` at `address` for the bytes that are in a block, except
    /// those a change has set.
    fn take(&mut self, address: u32, data: &[u8]) {
        for (offset, byte) in data.iter().enumerate() {
            let at = address + offset as u32;
            if !self.patched.contains(&at) && self.in_block(at) {
                self.bytes.insert(at, *byte);
            }
        }
    }

    /// Set `data` at `address`, for good: a change another writer made.
    fn patch(&mut self, address: u32, data: &[u8]) {
        for (offset, byte) in data.iter().enumerate() {
            let at = address + offset as u32;
            if self.in_block(at) {
                self.bytes.insert(at, *byte);
                self.patched.insert(at);
            }
        }
    }

    fn in_block(&self, at: u32) -> bool {
        self.blocks
            .iter()
            .any(|b| (b.start..b.start + b.len).contains(&at))
    }
}

/// The follower of one instance.
pub struct Follower {
    instance: Arc<Instance>,
    /// The device ID the read requests carry; the service replaces it.
    device_id: u8,
    /// This connection's client ID, once welcomed.
    me: Option<ClientId>,
    connected: bool,
    /// The link to the device is up.
    link_up: bool,
    /// The Binding the follower has set up for.
    bound: Option<Binding>,
    subscribed: Option<Binding>,
    /// The values are out of date and must be read.
    stale: bool,
    /// The host could be told of changes at the last step.
    had_edits: bool,
    /// No read has finished yet (a restored instance keeps its project's
    /// values through it).
    first_read: bool,
    reading: Option<Reading>,
    /// The Part's tone type must be (re)read before anything else.
    needs_probe: bool,
    probe: Option<Probe>,
    /// When to send the next probe, after one went unanswered.
    probe_retry: Option<Instant>,
    probe_backoff: Duration,
    /// The device's raw SysEx is switched on, for a read or a probe.
    raw_on: bool,
    /// The editor's default tone was selected by this follower, and the
    /// Part's baseline is asked for when the service announces it.
    own_default_tone: bool,
    /// The Part whose baseline was asked for in place of a read.
    baseline_part: Option<u8>,
    gestures: Vec<Gesture>,
    /// When to post a replace the host did not take again.
    retry_at: Option<Instant>,
}

impl Follower {
    /// A follower of `instance`, whose read requests carry `device_id` (the
    /// service puts its own in any Roland message, so it is a placeholder).
    /// It does nothing until it is connected and the instance is bound and
    /// settled.
    pub fn new(instance: Arc<Instance>, device_id: u8) -> Self {
        Self {
            instance,
            device_id,
            me: None,
            connected: false,
            link_up: false,
            bound: None,
            subscribed: None,
            stale: true,
            had_edits: false,
            first_read: true,
            reading: None,
            needs_probe: false,
            probe: None,
            probe_retry: None,
            probe_backoff: PROBE_BACKOFF_MIN,
            raw_on: false,
            own_default_tone: false,
            baseline_part: None,
            gestures: Vec::new(),
            retry_at: None,
        }
    }

    /// How long until the follower next has something to do on its own: a
    /// gesture to end, a read to give up.
    pub fn next_wake(&self, now: Instant) -> Option<Duration> {
        self.gestures
            .iter()
            .map(|g| g.deadline)
            .chain(self.reading.as_ref().map(|r| r.deadline.min(r.last_chance)))
            .chain(self.probe.as_ref().and_then(|p| p.deadline))
            .chain(self.probe_retry)
            .min()
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    /// Take the next event, if there was one, and carry out what follows
    /// from it and from the time `now`.
    pub fn step(&mut self, event: Option<&ClientEvent>, now: Instant, wire: &dyn Wire) {
        if let Some(event) = event {
            self.on_event(event, now, wire);
        }
        self.follow_binding(wire);
        // The host becoming reachable is a reason to read: until then the
        // device's values could not be applied.
        let has_edits = self.instance.bank.edits().is_some();
        if has_edits && !self.had_edits {
            self.stale = true;
        }
        self.had_edits = has_edits;
        self.end_gestures(now);
        self.retry_replace(now);
        self.load_default_tone(wire);
        self.finish_probe(now);
        self.probe_if_due(now, wire);
        self.read_if_due(now, wire);
        self.finish_read(now);
        self.sync_raw(wire);
    }

    /// Switch the device's raw SysEx on while a read or a probe wants it.
    fn sync_raw(&mut self, wire: &dyn Wire) {
        let wanted = self.reading.is_some() || self.probe.is_some();
        if wanted != self.raw_on {
            self.raw_on = wanted;
            wire.receive_raw(wanted);
        }
    }

    /// End every gesture still open: the instance is going away.
    pub fn end_all(&mut self) {
        for gesture in self.gestures.drain(..) {
            self.instance.post(HostTask::End(gesture.index as u32));
        }
    }

    fn on_event(&mut self, event: &ClientEvent, now: Instant, wire: &dyn Wire) {
        match event {
            ClientEvent::Connected { client, session } => {
                self.me = Some(*client);
                self.connected = true;
                self.link_up = session.is_some();
                self.out_of_date(true);
            }
            ClientEvent::Disconnected | ClientEvent::Incompatible { .. } => {
                self.me = None;
                self.connected = false;
                self.link_up = false;
                self.instance.disarm();
                self.forget_fit();
                self.out_of_date(true);
            }
            ClientEvent::Overflowed => self.out_of_date(true),
            ClientEvent::Message(Message::LinkUp { .. }) => {
                self.link_up = true;
                self.out_of_date(true);
            }
            ClientEvent::Message(Message::LinkDown) => {
                self.link_up = false;
                self.instance.disarm();
                self.forget_fit();
                self.out_of_date(true);
            }
            ClientEvent::Message(Message::Changed {
                writer,
                param,
                binding,
                value,
            }) => self.on_changed(*writer, param, binding, *value, now),
            ClientEvent::Message(Message::Bulk {
                writer,
                kind,
                binding,
                selected,
                tone,
            }) if self.bulk_concerns_us(*kind, binding) => {
                self.on_bulk(*writer, *kind, *selected, tone.as_ref(), now, wire);
            }
            ClientEvent::Message(Message::Baseline {
                binding,
                tone,
                runs,
            }) if self.baseline_part.is_some() && binding.part == self.baseline_part => {
                self.baseline_part = None;
                match self.bound {
                    Some(mine) if tone.is_some() && self.instance.fit() == ToneFit::Fits => {
                        self.apply_baseline(mine, runs, now);
                    }
                    // None to read it from: read the device after all.
                    _ => self.stale = true,
                }
            }
            ClientEvent::Message(Message::Sysex { bytes }) => {
                if let Ok(dt1) = sysex::parse_dt1(bytes) {
                    if let Some(reading) = &mut self.reading {
                        reading.take(dt1.address.value(), &dt1.data);
                        reading.deadline = now + READ_QUIET;
                    }
                    // Late or not: the answer to a probe is the Part's tone.
                    if self
                        .probe
                        .as_ref()
                        .is_some_and(|p| p.address == dt1.address)
                        && let Some(&msb) = dt1.data.first()
                    {
                        self.resolve_fit(ToneFit::of(self.instance.kind(), msb));
                    }
                }
            }
            _ => {}
        }
    }

    /// A bulk event concerning this instance. One that did not select a tone
    /// leaves the Part's type as it was. One that did says which tone, if the
    /// service knows, and the type is taken from that; else it must be asked.
    fn on_bulk(
        &mut self,
        writer: ClientId,
        kind: BulkKind,
        selected: bool,
        tone: Option<&ToneSelect>,
        now: Instant,
        wire: &dyn Wire,
    ) {
        let announced = match (kind, tone) {
            (BulkKind::Tone, Some(tone)) if selected && self.checks_tone() => Some(tone.msb),
            _ => None,
        };
        let retype = selected && matches!(kind, BulkKind::Tone | BulkKind::StudioSet);
        self.out_of_date(false);
        if let Some(msb) = announced {
            self.resolve_fit(ToneFit::of(self.instance.kind(), msb));
        } else if retype {
            self.await_service(now);
        }
        // The tone this follower selected was just read by the service: its
        // baseline is the values to take, which a read of ours would only
        // get after the service's own.
        if self.own_default_tone
            && selected
            && Some(writer) == self.me
            && self.instance.fit() == ToneFit::Fits
            && let Some(part) = self.bound.and_then(|b| b.part)
        {
            self.own_default_tone = false;
            self.stale = false;
            self.baseline_part = Some(part);
            wire.baseline(part);
        }
    }

    /// A selection the service could not name: it reads the Part's select
    /// itself and announces the tone. The instance does not ask the device
    /// meanwhile, since an answer could be of the selection before: a probe
    /// under way is dropped, its answer ignored, and only if the service says
    /// nothing for [`SETTLE_WAIT`] does it ask. A known mismatch is kept
    /// until the word comes.
    fn await_service(&mut self, now: Instant) {
        if !self.checks_tone() {
            return;
        }
        self.needs_probe = false;
        self.probe = None;
        self.probe_backoff = PROBE_BACKOFF_MIN;
        self.probe_retry = Some(now + SETTLE_WAIT);
        if self.instance.fit() == ToneFit::Fits {
            self.instance.set_fit(ToneFit::Unknown);
        }
    }

    /// Take the values of the Part's baseline as the result of a read.
    fn apply_baseline(&mut self, mine: Binding, runs: &[BaselineRun], now: Instant) {
        let mut blocks = Vec::new();
        let mut bytes = HashMap::new();
        for run in runs {
            let start = Address(run.address).value();
            blocks.push(Block {
                start,
                len: run.bytes.len() as u32,
            });
            for (k, byte) in run.bytes.iter().enumerate() {
                bytes.insert(start + k as u32, *byte);
            }
        }
        let reading = Reading {
            binding: mine,
            blocks,
            bytes,
            patched: Default::default(),
            apply: self.applies(),
            deadline: now,
            last_chance: now,
        };
        self.conclude(reading);
    }

    /// Whether a read's values go to the host: it can be told, and the
    /// project's stand on a restored instance's first read.
    fn applies(&self) -> bool {
        self.instance.bank.edits().is_some() && !(self.first_read && self.instance.was_restored())
    }

    /// The values are out of date: drop a read under way and read again when
    /// possible. With `retype`, the Part's tone may be of another type too:
    /// the instance stops writing (a Mismatch stays reported, as it is
    /// probably still true) until it has asked the device.
    fn out_of_date(&mut self, retype: bool) {
        // What the effects' types were says nothing of what they are now:
        // writing a slot meanwhile could land on a type just loaded.
        self.instance.bank.forget_types();
        self.stale = true;
        self.reading = None;
        self.baseline_part = None;
        if retype && self.checks_tone() {
            self.needs_probe = true;
            self.probe = None;
            self.probe_retry = None;
            self.probe_backoff = PROBE_BACKOFF_MIN;
            if self.instance.fit() == ToneFit::Fits {
                self.instance.set_fit(ToneFit::Unknown);
            }
        }
    }

    /// The Part's tone type is known: set the fit and stop asking.
    fn resolve_fit(&mut self, fit: ToneFit) {
        self.instance.set_fit(fit);
        self.needs_probe = false;
        self.probe = None;
        self.probe_retry = None;
        self.probe_backoff = PROBE_BACKOFF_MIN;
    }

    /// Whether the plugin acts only on some tone type.
    fn checks_tone(&self) -> bool {
        self.instance.kind().tone_type().is_some()
    }

    /// What the instance knew of the Part is gone (a new Binding or the
    /// connection): not even a Mismatch is still reported.
    fn forget_fit(&mut self) {
        self.own_default_tone = false;
        self.instance.clear_default_tone();
        if self.checks_tone() {
            self.instance.set_fit(ToneFit::Unknown);
        }
    }

    /// Ask the device which tone the Part holds, if the instance needs to
    /// know and can ask.
    fn probe_if_due(&mut self, now: Instant, wire: &dyn Wire) {
        let Some(binding) = self.bound else { return };
        if !self.needs_probe
            || self.probe.as_ref().is_some_and(|p| p.deadline.is_some())
            || !self.connected
            || !self.link_up
            || !self.instance.is_settled()
            || !self.instance.kind().accepts(&binding)
        {
            return;
        }
        let Some(at) = binding.part else { return };
        self.needs_probe = false;
        let (address, size) = params::tone_select_block(at);
        self.probe = Some(Probe {
            address,
            deadline: Some(now + READ_QUIET),
        });
        // Raw replies on before the request leaves.
        self.sync_raw(wire);
        wire.raw(sysex::build_rq1(self.device_id, &address, &size));
    }

    /// An unanswered probe is asked again after a growing wait, while its
    /// late answer is still taken. The instance is not taken to fit meanwhile:
    /// the request may only be queued behind others, but a Part of another
    /// type must not be written.
    fn finish_probe(&mut self, now: Instant) {
        if let Some(probe) = &mut self.probe
            && probe.deadline.is_some_and(|d| d <= now)
        {
            probe.deadline = None;
            self.probe_retry = Some(now + self.probe_backoff);
            self.probe_backoff = (self.probe_backoff * 2).min(PROBE_BACKOFF_MAX);
        }
        if self.probe_retry.is_some_and(|at| at <= now) {
            self.probe_retry = None;
            self.needs_probe = true;
        }
    }

    /// Load the plugin's default tone onto the Part, if the editor asked and
    /// the Part is known to hold another type. A request that cannot be
    /// carried out is dropped, and the editor told.
    fn load_default_tone(&mut self, wire: &dyn Wire) {
        if !self.instance.take_default_tone_request() {
            return;
        }
        if let Some(binding) = self.bound
            && let Some(part) = binding.part
            && let Some(tone) = self.instance.default_tone()
            && self.instance.is_inert()
            && self.connected
            && self.link_up
        {
            self.own_default_tone = true;
            wire.change_tone(part, tone.msb, tone.lsb, tone.pc);
        } else {
            self.instance.drop_default_tone();
        }
    }

    /// Whether a bulk event of `kind` in `binding` changed what this
    /// instance's parameters hold. A Studio Set load changes every Part's
    /// tone as well.
    fn bulk_concerns_us(&self, kind: BulkKind, binding: &Binding) -> bool {
        let Some(mine) = self.bound else { return false };
        let studio_set = self.instance.kind() == PluginKind::StudioSet;
        match kind {
            BulkKind::StudioSet => true,
            BulkKind::Chorus | BulkKind::Reverb => studio_set,
            BulkKind::Tone | BulkKind::Mfx => !studio_set && binding.part == mine.part,
        }
    }

    /// Set up for a new Binding: subscribe to its changes and read its
    /// values.
    fn follow_binding(&mut self, wire: &dyn Wire) {
        let now = self.instance.binding.get();
        if now == self.bound {
            return;
        }
        self.bound = now;
        let wanted = now.map(subscription);
        if wanted != self.subscribed {
            if let Some(old) = self.subscribed.take() {
                wire.unsubscribe(old);
            }
            if let Some(new) = wanted {
                wire.subscribe(new);
            }
            self.subscribed = wanted;
        }
        self.forget_fit();
        self.out_of_date(true);
    }

    fn on_changed(
        &mut self,
        writer: ClientId,
        param: &ParamId,
        binding: &Binding,
        value: i32,
        now: Instant,
    ) {
        let bank = &self.instance.bank;
        let Some(mine) = self.bound else { return };
        // Inert, or not sure the Part holds our type: the device's changes
        // are not ours to follow (a read follows if it does).
        if param.plugin != bank.kind() || self.instance.fit() != ToneFit::Fits {
            return;
        }
        let Some(i) = bank.position(&param.id) else {
            return;
        };
        let host = &bank.list()[i].0;
        let ours = match host.anchor {
            Anchor::Absolute => true,
            Anchor::Part => binding.part == mine.part,
            Anchor::Key(_) => binding.part == mine.part && binding.key == mine.key,
        };
        if !ours || !bank.placed(i, &mine) {
            return;
        }
        let value = value.clamp(host.min, host.max);
        // What the device has now, whoever changed it and whether or not the
        // host can be told.
        if bank.is_holder(i) {
            bank.set_device_type(i, value);
        }
        if let Some(reading) = &mut self.reading
            && reading.binding == mine
            && let Ok(address) = host.device_address(&mine)
        {
            // The reply to the read may hold the value before this one.
            reading.patch(address.value(), &host.encode(value));
        }
        if Some(writer) == self.me || Some(writer) == self.instance.editor_client() {
            return;
        }
        if !bank.is_active(i) || bank.edits().is_none() || bank.value(i) == value {
            return;
        }
        let deadline = now + GESTURE_END;
        if let Some(gesture) = self.gestures.iter_mut().find(|g| g.index == i) {
            gesture.deadline = deadline;
        } else if self.instance.post(HostTask::Begin(i as u32)) {
            self.gestures.push(Gesture { index: i, deadline });
        } else {
            return;
        }
        bank.expect(i, value);
        self.instance.post(HostTask::Perform(i as u32, value));
    }

    /// End the gestures that have been quiet long enough. An `End` the ring
    /// refused is tried again at the next step.
    fn end_gestures(&mut self, now: Instant) {
        let instance = &self.instance;
        self.gestures.retain(|gesture| {
            gesture.deadline > now || !instance.post(HostTask::End(gesture.index as u32))
        });
    }

    /// Post again a replace the host handed back, once its time has come.
    fn retry_replace(&mut self, now: Instant) {
        if self.instance.bank.take_retry() {
            self.retry_at = Some(now + REPLACE_RETRY);
        }
        if self.retry_at.is_some_and(|at| at <= now) {
            self.retry_at = None;
            self.instance.post(HostTask::Replace);
        }
    }

    /// Start the read if the values are out of date and it can be done.
    fn read_if_due(&mut self, now: Instant, wire: &dyn Wire) {
        let Some(binding) = self.bound else { return };
        if !self.stale
            || self.reading.is_some()
            || self.instance.fit() != ToneFit::Fits
            || !self.connected
            || !self.link_up
            || !self.instance.is_settled()
            || !self.instance.kind().accepts(&binding)
        {
            return;
        }
        self.stale = false;
        // The project's values stand on a restored instance's first read.
        let apply = self.applies();
        let blocks = plan(&self.instance, &binding, !apply);
        if blocks.is_empty() {
            self.first_read = false;
            self.instance.arm();
            return;
        }
        let requests: Vec<Vec<u8>> = blocks
            .iter()
            .filter_map(|block| {
                let address = Address::from_value(block.start)?;
                let size = DataSize::from_value(block.len)?;
                Some(sysex::build_rq1(self.device_id, &address, &size))
            })
            .collect();
        self.reading = Some(Reading {
            binding,
            blocks,
            bytes: HashMap::new(),
            patched: Default::default(),
            apply,
            deadline: now + READ_QUIET,
            last_chance: now + READ_AT_MOST,
        });
        // Raw replies on before the requests leave.
        self.sync_raw(wire);
        for rq1 in requests {
            wire.raw(rq1);
        }
    }

    /// Apply the read once it is complete or the device has gone quiet.
    fn finish_read(&mut self, now: Instant) {
        if !self
            .reading
            .as_ref()
            .is_some_and(|r| r.complete() || r.deadline <= now || r.last_chance <= now)
        {
            return;
        }
        let reading = self.reading.take().expect("checked");
        self.conclude(reading);
    }

    /// Give the host what a finished read found, and arm the instance.
    fn conclude(&mut self, reading: Reading) {
        self.first_read = false;
        let bank = &self.instance.bank;
        let Read { types, changes } = values(&self.instance, &reading);
        for (holder, value) in types {
            bank.set_device_type(holder, value);
        }
        // What the host edited meanwhile is the host's.
        let changes: Vec<(usize, i32)> = changes
            .into_iter()
            .filter(|(i, _)| !bank.is_dirty(*i))
            .collect();
        if reading.apply && !changes.is_empty() {
            for (i, value) in &changes {
                bank.expect(*i, *value);
            }
            bank.queue_replace(changes);
            if !self.instance.post(HostTask::Replace) {
                // Not taken: nothing will come of the queued values.
                for (i, _) in bank.take_replace() {
                    bank.settle(i);
                }
            }
        }
        self.instance.arm();
    }
}

/// The blocks to read for an instance bound to `binding`: every whole block
/// of the device that holds one of its parameters, or, with `types_only`,
/// one of the parameters that hold an effect's type.
fn plan(instance: &Instance, binding: &Binding, types_only: bool) -> Vec<Block> {
    let bank = &instance.bank;
    let spans: Vec<(u32, u32)> = bank
        .list()
        .iter()
        .enumerate()
        .filter(|(i, _)| bank.placed(*i, binding) && (!types_only || bank.is_holder(*i)))
        .filter_map(|(_, (host, _))| {
            let at = host.device_address(binding).ok()?.value();
            Some((at, at + host.width as u32))
        })
        .collect();
    if binding.key.is_some() {
        // A key's block is not in the tone's read plan: one run over its
        // parameters.
        let start = spans.iter().map(|s| s.0).min();
        let end = spans.iter().map(|s| s.1).max();
        return match (start, end) {
            (Some(start), Some(end)) => vec![Block {
                start,
                len: end - start,
            }],
            _ => Vec::new(),
        };
    }
    let kind = instance.kind();
    let Some(spec) = USER_MEMORY.iter().find(|s| s.tone_type == kind.tone_type()) else {
        return Vec::new();
    };
    spec.tone_read_blocks(binding.part.unwrap_or(0))
        .into_iter()
        .map(|(address, size)| Block {
            start: address.value(),
            len: size.value(),
        })
        .filter(|block| {
            spans
                .iter()
                .any(|(start, end)| *start < block.start + block.len && block.start < *end)
        })
        .collect()
}

/// What a read gives (see [`values`]): the effect types it read (holder
/// position, type), and the values that differ from the host's, for a parameter whose bytes all
/// arrived and, for an effect's slot, only if its type is the effect's (as
/// read, else as the device is believed to have it).
fn values(instance: &Instance, reading: &Reading) -> Read {
    let bank = &instance.bank;
    let binding = &reading.binding;
    let read = |i: usize| -> Option<i32> {
        let host = &bank.list()[i].0;
        let at = host.device_address(binding).ok()?.value();
        let data: Option<Vec<u8>> = (0..host.width as u32)
            .map(|k| reading.bytes.get(&(at + k)).copied())
            .collect();
        let value = host.decode(&data?)?;
        Some(value.clamp(host.min, host.max))
    };
    let mut plain: HashMap<usize, i32> = HashMap::new();
    for i in 0..bank.len() {
        if bank.list()[i].0.effect.is_none()
            && bank.placed(i, binding)
            && let Some(value) = read(i)
        {
            plain.insert(i, value);
        }
    }
    let types: Vec<(usize, i32)> = plain
        .iter()
        .filter(|(i, _)| bank.is_holder(**i))
        .map(|(i, v)| (*i, *v))
        .collect();
    if !reading.apply {
        return Read {
            types,
            changes: Vec::new(),
        };
    }
    let mut changes: Vec<(usize, i32)> = plain
        .iter()
        .map(|(i, v)| (*i, *v))
        .filter(|(i, v)| bank.value(*i) != *v)
        .collect();
    for i in 0..bank.len() {
        let Some(effect) = bank.list()[i].0.effect else {
            continue;
        };
        let Some(holder) = bank.type_slot(i) else {
            continue;
        };
        let active = plain
            .get(&holder)
            .copied()
            .or_else(|| bank.device_type(holder));
        if active == Some(i32::from(effect.type_id))
            && bank.placed(i, binding)
            && let Some(value) = read(i)
            && bank.value(i) != value
        {
            changes.push((i, value));
        }
    }
    changes.sort_unstable();
    Read { types, changes }
}

/// The outcome of a read.
struct Read {
    /// The effect types read: (holder position, type).
    types: Vec<(usize, i32)>,
    /// The values that differ from the host's: (position, value).
    changes: Vec<(usize, i32)>,
}

#[cfg(test)]
mod tests {
    use super::super::notice::Notice;
    use super::*;
    use integral_core::params::ToneType;
    use integral_core::tone_banks::default_tone;
    use integral_device_service::SessionId;
    use integral_device_service::protocol::ToneSelect;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeWire {
        calls: RefCell<Vec<String>>,
        raws: RefCell<Vec<Vec<u8>>>,
        tones: RefCell<Vec<(u8, u8, u8, u8)>>,
        baselines: RefCell<Vec<u8>>,
    }

    impl Wire for FakeWire {
        fn subscribe(&self, b: Binding) {
            self.calls.borrow_mut().push(format!("subscribe {b:?}"));
        }
        fn unsubscribe(&self, b: Binding) {
            self.calls.borrow_mut().push(format!("unsubscribe {b:?}"));
        }
        fn receive_raw(&self, on: bool) {
            self.calls.borrow_mut().push(format!("raw {on}"));
        }
        fn raw(&self, bytes: Vec<u8>) {
            self.raws.borrow_mut().push(bytes);
        }
        fn change_tone(&self, part: u8, msb: u8, lsb: u8, pc: u8) {
            self.tones.borrow_mut().push((part, msb, lsb, pc));
        }
        fn baseline(&self, part: u8) {
            self.baselines.borrow_mut().push(part);
        }
    }

    /// The device's answer to the follower's question which tone `part`
    /// holds: a tone of `holds`.
    fn holds(part: u8, holds: ToneType) -> ClientEvent {
        let tone = default_tone(holds).expect("a default tone");
        ClientEvent::Message(Message::Sysex {
            bytes: sysex::build_dt1(
                sysex::DEFAULT_DEVICE_ID,
                &params::tone_select_block(part).0,
                &[tone.msb, tone.lsb, tone.pc],
            ),
        })
    }

    /// A bulk event of `part`'s tone that did not select one (parameters).
    fn bulk(part: u8) -> ClientEvent {
        ClientEvent::Message(Message::Bulk {
            writer: ClientId(9),
            kind: BulkKind::Tone,
            binding: Binding::of_part(part),
            selected: false,
            tone: None,
        })
    }

    /// A bulk event announcing that `part` was given a tone of `now`, or, with
    /// `None`, that its tone was selected and the service does not know which.
    fn selected(part: u8, now: Option<ToneType>) -> ClientEvent {
        ClientEvent::Message(Message::Bulk {
            writer: ClientId(9),
            kind: BulkKind::Tone,
            binding: Binding::of_part(part),
            selected: true,
            tone: now.map(|t| {
                let tone = default_tone(t).expect("a default tone");
                ToneSelect {
                    msb: tone.msb,
                    lsb: tone.lsb,
                    pc: tone.pc,
                }
            }),
        })
    }

    fn requests_at(wire: &FakeWire, at: Address) -> usize {
        wire.raws
            .borrow()
            .iter()
            .filter(|m| m[6] == sysex::CMD_RQ1 && m[7..11] == *at.as_bytes())
            .count()
    }

    /// How many requests for a block of the tone (not the select) were made.
    fn block_reads(wire: &FakeWire, part: u8) -> usize {
        let rq1s = wire
            .raws
            .borrow()
            .iter()
            .filter(|m| m[6] == sysex::CMD_RQ1)
            .count();
        rq1s - selects_asked(wire, part)
    }

    fn selects_asked(wire: &FakeWire, part: u8) -> usize {
        requests_at(wire, params::tone_select_block(part).0)
    }

    fn connected() -> ClientEvent {
        ClientEvent::Connected {
            client: ClientId(7),
            session: Some(SessionId(1)),
        }
    }

    fn instance(kind: PluginKind, binding: Binding) -> Arc<Instance> {
        let instance = Arc::new(Instance::new(kind));
        instance.binding.set(Some(binding));
        instance.settle();
        instance
    }

    #[test]
    fn a_binding_subscribes_by_part_and_a_new_one_replaces_it() {
        let instance = instance(PluginKind::PcmDrum, Binding::of_part(2).with_key(40));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        assert_eq!(
            wire.calls.borrow()[0],
            format!("subscribe {:?}", Binding::of_part(2))
        );
        instance.binding.set(Some(Binding::of_part(5)));
        follower.step(None, now, &wire);
        let calls = wire.calls.borrow();
        assert!(calls.contains(&format!("unsubscribe {:?}", Binding::of_part(2))));
        assert!(calls.contains(&format!("subscribe {:?}", Binding::of_part(5))));
    }

    #[test]
    fn writing_starts_with_the_first_read_and_a_parameter_bulk_changes_nothing_of_it() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        assert!(!instance.is_armed(), "the Part's tone type is being read");
        assert_eq!(selects_asked(&wire, 0), 1);
        assert_eq!(block_reads(&wire, 0), 0, "nothing of the tone before that");
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        assert_eq!(instance.fit(), ToneFit::Fits);
        assert!(!instance.is_armed(), "the read is under way");
        assert!(block_reads(&wire, 0) > 0, "blocks were requested");
        // Nothing arrives: the device goes quiet and the read gives up.
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert!(instance.is_armed());

        // Many parameters changed (a bulk that selected nothing): read again,
        // and neither stop writing nor ask the device which tone it holds.
        let asked = block_reads(&wire, 0);
        follower.step(Some(&bulk(0)), now, &wire);
        assert!(instance.is_armed());
        assert_eq!(selects_asked(&wire, 0), 1);
        assert!(block_reads(&wire, 0) > asked, "read again");
        follower.step(None, now + Duration::from_secs(4), &wire);

        // An MFX bulk is no change of type either.
        let mfx = ClientEvent::Message(Message::Bulk {
            writer: ClientId(9),
            kind: BulkKind::Mfx,
            binding: Binding::of_part(0),
            selected: false,
            tone: None,
        });
        let asked = block_reads(&wire, 0);
        follower.step(Some(&mfx), now + Duration::from_secs(5), &wire);
        assert!(instance.is_armed());
        assert!(block_reads(&wire, 0) > asked);
        follower.step(None, now + Duration::from_secs(8), &wire);

        let asked = wire.raws.borrow().len();
        follower.step(Some(&selected(1, None)), now, &wire);
        assert_eq!(
            wire.raws.borrow().len(),
            asked,
            "another Part's tone is not ours"
        );

        // A lost link does stop it.
        follower.step(Some(&ClientEvent::Message(Message::LinkDown)), now, &wire);
        assert!(!instance.is_armed());
    }

    #[test]
    fn a_selection_the_service_names_sets_the_fit_with_no_probe_and_one_it_does_not_is_asked() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert!(instance.is_armed());

        // Another type is selected, and the service says which: inert at once.
        let t = now + Duration::from_secs(3);
        follower.step(Some(&selected(0, Some(ToneType::SnDrum))), t, &wire);
        assert_eq!(instance.fit(), ToneFit::Mismatch);
        assert!(!instance.is_armed());
        assert_eq!(selects_asked(&wire, 0), 1, "no probe");

        // And back: the fit is ours, the read follows, nothing was asked.
        let reads = block_reads(&wire, 0);
        follower.step(Some(&selected(0, Some(ToneType::PcmSynth))), t, &wire);
        assert_eq!(instance.fit(), ToneFit::Fits);
        assert_eq!(selects_asked(&wire, 0), 1, "no probe");
        assert!(block_reads(&wire, 0) > reads);
        follower.step(None, t + Duration::from_secs(2), &wire);
        assert!(instance.is_armed());

        // A selection the service cannot name stops writing until the
        // service has read it and says which. The device is not asked
        // meanwhile: only if the service says nothing for a long while.
        let t = t + Duration::from_secs(5);
        follower.step(Some(&selected(0, None)), t, &wire);
        assert_eq!(instance.fit(), ToneFit::Unknown);
        assert!(!instance.is_armed());
        follower.step(None, t + Duration::from_secs(5), &wire);
        assert_eq!(selects_asked(&wire, 0), 1, "the service is reading it");
        follower.step(Some(&selected(0, Some(ToneType::PcmSynth))), t, &wire);
        assert_eq!(instance.fit(), ToneFit::Fits);
        follower.step(None, t + Duration::from_secs(30), &wire);
        assert_eq!(selects_asked(&wire, 0), 1, "it said, so nothing is asked");

        // If it never says, the device is asked after all.
        let t = t + Duration::from_secs(60);
        follower.step(Some(&selected(0, None)), t, &wire);
        follower.step(None, t + SETTLE_WAIT + Duration::from_secs(1), &wire);
        assert_eq!(selects_asked(&wire, 0), 2);
    }

    #[test]
    fn an_answer_to_a_probe_sent_before_a_selection_the_service_could_not_name_is_not_taken() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        // Connected: the device is asked which tone the Part holds.
        follower.step(Some(&connected()), now, &wire);
        assert_eq!(selects_asked(&wire, 0), 1);

        // A Studio Set was switched meanwhile; the service will read the
        // Parts and say.
        follower.step(Some(&selected(0, None)), now, &wire);
        // The answer to the first question comes now, from before the switch:
        // it says an SN Synth, which the Part no longer holds.
        follower.step(Some(&holds(0, ToneType::SnSynth)), now, &wire);
        assert_eq!(instance.fit(), ToneFit::Unknown, "that answer is old");
        assert!(!instance.is_armed());
        assert_eq!(instance.notices.get(), []);

        // The service's word is what counts.
        follower.step(Some(&selected(0, Some(ToneType::PcmSynth))), now, &wire);
        assert_eq!(instance.fit(), ToneFit::Fits);
    }

    #[test]
    fn a_part_of_another_type_makes_the_instance_inert_until_the_type_matches_again() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();

        // Match.
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert_eq!((instance.fit(), instance.is_armed()), (ToneFit::Fits, true));
        assert_eq!(instance.notices.get(), []);

        // Mismatch: the tone became an SN Synth's (a selection nobody could
        // name, so the device is asked).
        follower.step(
            Some(&selected(0, None)),
            now + Duration::from_secs(3),
            &wire,
        );
        assert_eq!(instance.fit(), ToneFit::Unknown);
        follower.step(
            Some(&selected(0, Some(ToneType::SnSynth))),
            now + Duration::from_secs(3),
            &wire,
        );
        assert_eq!(instance.fit(), ToneFit::Mismatch);
        assert!(!instance.is_armed());
        assert!(instance.is_inert());
        assert_eq!(instance.notices.get(), [Notice::TypeMismatch]);
        assert!(!instance.bank.is_empty(), "its parameter list stays");

        // It reads nothing and asks nothing more while inert, however long.
        let asked = wire.raws.borrow().len();
        follower.step(None, now + Duration::from_secs(60), &wire);
        assert_eq!(wire.raws.borrow().len(), asked);
        // A bulk of parameters does not start a read either.
        follower.step(Some(&bulk(0)), now + Duration::from_secs(61), &wire);
        assert_eq!(wire.raws.borrow().len(), asked);

        // Match again: the Part holds a PCM Synth tone, announced as a bulk.
        let later = now + Duration::from_secs(62);
        follower.step(Some(&selected(0, None)), later, &wire);
        assert_eq!(instance.fit(), ToneFit::Mismatch, "still, until it knows");
        follower.step(Some(&selected(0, Some(ToneType::PcmSynth))), later, &wire);
        assert_eq!(instance.fit(), ToneFit::Fits);
        assert_eq!(instance.notices.get(), []);
        assert!(block_reads(&wire, 0) > 0, "it reads the tone it now has");
        follower.step(None, later + Duration::from_secs(2), &wire);
        assert!(instance.is_armed());
    }

    #[test]
    fn an_inert_instance_ignores_changes_from_the_device() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let log = Arc::new(Log {
            instance: Arc::downgrade(&instance),
            replaced: Default::default(),
        });
        instance.bank.set_edits(Some(log));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(0, ToneType::SnDrum)), now, &wire);
        assert!(instance.is_inert());

        let bank = &instance.bank;
        let i = bank.position("pcms/partial1/0x0F").expect("a parameter");
        let host = bank.list()[i].0.clone();
        let new = if host.default == host.max {
            host.min
        } else {
            host.max
        };
        let changed = ClientEvent::Message(Message::Changed {
            writer: ClientId(9),
            param: ParamId {
                plugin: PluginKind::PcmSynth,
                id: host.id.clone(),
            },
            binding: Binding::of_part(0),
            value: new,
        });
        follower.step(Some(&changed), now, &wire);
        assert_ne!(bank.value(i), new, "no gesture, no value");
        assert_eq!(follower.gestures.len(), 0);
    }

    #[test]
    fn a_device_that_is_slow_to_say_what_the_part_holds_leaves_the_instance_unsure_and_its_late_word_counts()
     {
        let instance = instance(PluginKind::SnSynth, Binding::of_part(0));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        assert_eq!(selects_asked(&wire, 0), 1);

        // Past the wait: still unsure, nothing read or written, asked again
        // after a pause that grows.
        follower.step(None, now + Duration::from_millis(1500), &wire);
        assert_eq!(instance.fit(), ToneFit::Unknown);
        assert_eq!(block_reads(&wire, 0), 0);
        assert_eq!(selects_asked(&wire, 0), 1);
        follower.step(None, now + Duration::from_millis(2600), &wire);
        assert_eq!(selects_asked(&wire, 0), 2);
        follower.step(None, now + Duration::from_millis(4000), &wire);
        follower.step(None, now + Duration::from_millis(4700), &wire);
        assert_eq!(selects_asked(&wire, 0), 2, "the pause doubled");
        follower.step(None, now + Duration::from_secs(8), &wire);
        assert_eq!(selects_asked(&wire, 0), 3);
        assert_eq!(instance.fit(), ToneFit::Unknown);

        // The first answer comes at last, from a busy queue: another type.
        follower.step(
            Some(&holds(0, ToneType::PcmDrum)),
            now + Duration::from_secs(9),
            &wire,
        );
        assert_eq!(instance.fit(), ToneFit::Mismatch);
        assert_eq!(block_reads(&wire, 0), 0);
        // Nothing is asked after that.
        follower.step(None, now + Duration::from_secs(60), &wire);
        assert_eq!(selects_asked(&wire, 0), 3);
    }

    #[test]
    fn loading_the_default_tone_selects_the_types_first_preset_tone_only_while_inert() {
        let instance = instance(PluginKind::SnAcoustic, Binding::of_part(4));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(4, ToneType::SnAcoustic)), now, &wire);

        // The Part fits: there is nothing to repair.
        assert_eq!(instance.fit(), ToneFit::Fits);
        assert!(instance.request_default_tone().is_err());
        follower.step(None, now, &wire);
        assert_eq!(*wire.tones.borrow(), []);

        // A selection nobody could name makes it ask again; the answer is a
        // mismatch.
        follower.step(Some(&selected(4, None)), now, &wire);
        follower.step(Some(&selected(4, Some(ToneType::PcmSynth))), now, &wire);
        assert!(instance.is_inert());
        instance.request_default_tone().expect("inert and bound");
        assert!(instance.default_tone_pending());
        assert!(instance.request_default_tone().is_err(), "once at a time");
        follower.step(None, now, &wire);
        let tone = default_tone(ToneType::SnAcoustic).unwrap();
        assert_eq!(*wire.tones.borrow(), [(4, tone.msb, tone.lsb, tone.pc)]);
        // Once.
        follower.step(None, now, &wire);
        assert_eq!(wire.tones.borrow().len(), 1);
        assert!(instance.default_tone_pending(), "until the Part holds it");

        // The service announces the tone it selected for us: the Part fits,
        // and instead of a read behind the service's own, the baseline is
        // asked for. It is the read's result.
        let mut announce = selected(4, Some(ToneType::SnAcoustic));
        if let ClientEvent::Message(Message::Bulk { writer, .. }) = &mut announce {
            *writer = ClientId(7);
        }
        let reads = block_reads(&wire, 4);
        follower.step(Some(&announce), now, &wire);
        assert_eq!(instance.fit(), ToneFit::Fits);
        assert!(!instance.default_tone_pending());
        assert_eq!(*wire.baselines.borrow(), [4]);
        assert_eq!(block_reads(&wire, 4), reads, "no read of ours");
        let baseline = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(4),
            tone: Some(ToneSelect {
                msb: tone.msb,
                lsb: tone.lsb,
                pc: tone.pc,
            }),
            runs: vec![],
        });
        follower.step(Some(&baseline), now, &wire);
        assert!(instance.is_armed(), "the baseline is the first read");
        assert_eq!(block_reads(&wire, 4), reads);
    }

    #[test]
    fn a_default_tone_request_that_cannot_be_served_is_dropped_and_said() {
        let instance = instance(PluginKind::SnAcoustic, Binding::of_part(4));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(4, ToneType::PcmSynth)), now, &wire);
        instance.request_default_tone().expect("inert and bound");
        follower.step(Some(&ClientEvent::Message(Message::LinkDown)), now, &wire);
        // The link went down with the request pending: it is over.
        assert!(!instance.default_tone_pending());
        assert_eq!(*wire.tones.borrow(), []);

        follower.step(
            Some(&ClientEvent::Message(Message::LinkUp {
                session: SessionId(3),
            })),
            now,
            &wire,
        );
        follower.step(Some(&holds(4, ToneType::PcmSynth)), now, &wire);
        instance.request_default_tone().expect("inert and bound");
        // The Part got the type meanwhile (another client): nothing to load.
        instance.set_fit(ToneFit::Fits);
        assert!(!instance.default_tone_pending());
        instance.set_fit(ToneFit::Mismatch);
        instance.request_default_tone().expect("inert and bound");
        follower.step(Some(&ClientEvent::Message(Message::LinkDown)), now, &wire);
        assert!(!instance.default_tone_pending());
        // Asked while the link is down: dropped, and the editor is told.
        instance.set_fit(ToneFit::Mismatch);
        instance.request_default_tone().expect("inert and bound");
        follower.step(None, now, &wire);
        assert!(!instance.default_tone_pending());
        assert!(instance.take_default_tone_dropped());
        assert_eq!(*wire.tones.borrow(), []);
    }

    #[test]
    fn nothing_is_read_before_the_host_has_settled_or_while_the_link_is_down() {
        let instance = Arc::new(Instance::new(PluginKind::SnSynth));
        instance.binding.set(Some(Binding::of_part(0)));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        assert!(wire.raws.borrow().is_empty(), "not settled");
        instance.settle();
        follower.step(Some(&ClientEvent::Message(Message::LinkDown)), now, &wire);
        assert!(wire.raws.borrow().is_empty(), "link down");
        follower.step(
            Some(&ClientEvent::Message(Message::LinkUp {
                session: SessionId(2),
            })),
            now,
            &wire,
        );
        assert_eq!(selects_asked(&wire, 0), 1);
    }

    #[test]
    fn the_plan_holds_only_blocks_with_parameters_and_a_key_has_one_run() {
        for kind in PluginKind::ALL {
            let binding = match kind {
                PluginKind::StudioSet => Binding::ANY,
                _ => Binding::of_part(3),
            };
            let blocks = plan(&instance(kind, binding), &binding, false);
            assert!(!blocks.is_empty(), "{kind:?}");
            assert!(blocks.len() < 50, "{kind:?}: {} blocks", blocks.len());
            assert!(blocks.iter().all(|b| b.len <= 0x7F * 3), "{kind:?}");
        }
        let key = Binding::of_part(3).with_key(40);
        assert_eq!(
            plan(&instance(PluginKind::PcmDrum, key), &key, false).len(),
            1
        );
    }

    /// Applies a replace to the bank, as the wrapper does, and logs it.
    struct Log {
        instance: std::sync::Weak<Instance>,
        replaced: std::sync::Mutex<Vec<(String, i32)>>,
    }

    impl super::super::host::HostEdits for Log {
        fn begin(&self, _param: &nih_plug::prelude::IntParam) {}
        fn perform(&self, param: &nih_plug::prelude::IntParam, value: i32) {
            let instance = self.instance.upgrade().unwrap();
            let i = instance
                .bank
                .list()
                .iter()
                .position(|(_, p)| std::ptr::eq(p, param))
                .unwrap();
            instance.bank.host_changed(i, value);
        }
        fn end(&self, _param: &nih_plug::prelude::IntParam) {}
        fn replace(&self, changes: &[super::super::host::Replace<'_>]) -> bool {
            let instance = self.instance.upgrade().unwrap();
            for change in changes {
                let i = instance.bank.position(&change.host.id).unwrap();
                instance.bank.host_changed(i, change.value);
                self.replaced
                    .lock()
                    .unwrap()
                    .push((change.host.id.clone(), change.value));
            }
            true
        }
    }

    #[test]
    fn a_change_by_another_writer_during_the_read_is_not_undone_by_older_bytes() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let log = Arc::new(Log {
            instance: Arc::downgrade(&instance),
            replaced: Default::default(),
        });
        instance.bank.set_edits(Some(log.clone()));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        assert!(block_reads(&wire, 0) > 0, "a full read is under way");

        let bank = &instance.bank;
        let i = bank.position("pcms/partial1/0x0F").expect("a parameter");
        let host = bank.list()[i].0.clone();
        let old = host.default;
        let new = if host.default == host.max {
            host.min
        } else {
            host.max
        };
        let changed = ClientEvent::Message(Message::Changed {
            writer: ClientId(9),
            param: ParamId {
                plugin: PluginKind::PcmSynth,
                id: host.id.clone(),
            },
            binding: Binding::of_part(0),
            value: new,
        });
        follower.step(Some(&changed), now, &wire);
        assert_eq!(bank.value(i), new, "the gesture set it");

        // The reply, made before the change, holds the old value.
        let at = host.device_address(&Binding::of_part(0)).unwrap();
        let reply = ClientEvent::Message(Message::Sysex {
            bytes: sysex::build_dt1(0x10, &at, &host.encode(old)),
        });
        follower.step(Some(&reply), now, &wire);
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert!(instance.is_armed());
        assert_eq!(bank.value(i), new);
        assert!(
            log.replaced
                .lock()
                .unwrap()
                .iter()
                .all(|(id, _)| *id != host.id),
            "the old bytes were not applied: {:?}",
            log.replaced.lock().unwrap()
        );
    }
}
