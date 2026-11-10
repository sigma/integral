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
//! take the device's values on its first read, nor on the first of a session
//! that has not had its push (ADR 0009): the project's are the push's to
//! send. The [`Recall`] starts once that read has settled the instance, and
//! sends with `Push`, which bypasses the service's cache; no read applies
//! while it is under way. Later reads take the device's values.
//!
//! A read that applies also learns the tone the Part holds, for the instance
//! to save ([`recall::capture`]): the Part's tone-select bytes are read with
//! its blocks.
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
    BaselineRun, BulkKind, ClientId, Message, ParamId, Reject, Request, ToneSelect,
};
use integral_device_service::{Client, ClientEvent, SessionId};

use super::host::HostTask;
use super::instance::{Instance, ToneFit};
use super::recall::{self, Context, Recall};

/// How long a parameter goes without a change by another writer before the
/// gesture on it ends.
pub const GESTURE_END: Duration = Duration::from_millis(300);

/// How long after the host refused a bulk replace the follower tries again.
const REPLACE_RETRY: Duration = Duration::from_millis(250);

/// How long after the host's queue refused a gesture's task (it was full, or
/// posting was off) the follower wakes to post it again.
const POST_RETRY: Duration = Duration::from_millis(50);

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
    /// Queue `requests` as one push (`Request::Push`).
    fn push(&self, requests: Vec<Request>);
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

    fn push(&self, requests: Vec<Request>) {
        Client::send(self, Request::Push { requests });
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
///
/// What the host's queue refuses (it is full) is owed, and posted again at
/// the next step: the `Begin`, then only the latest value (an older one is
/// overtaken, not queued), then the `End`. So the host ends up with the
/// device's value once the queue drains.
struct Gesture {
    index: usize,
    deadline: Instant,
    /// The host's queue took the `Begin`.
    begun: bool,
    /// The latest value whose `Perform` the queue has not taken. It is
    /// expected ([`ParamBank::expect`]) once, when it comes in, not at each
    /// retry: a retry would push out the value queued before it.
    owed: Option<i32>,
    /// When to post again what the queue refused.
    retry_at: Option<Instant>,
}

impl Gesture {
    /// A gesture on parameter `index`, with nothing posted yet.
    fn new(index: usize, now: Instant) -> Self {
        Self {
            index,
            deadline: now + GESTURE_END,
            begun: false,
            owed: None,
            retry_at: None,
        }
    }

    /// When the follower should next look at the gesture.
    fn wake(&self) -> Instant {
        self.retry_at.unwrap_or(self.deadline)
    }

    /// Post what the host's queue has not taken yet, in order, and the `End`
    /// once the deadline has passed. Returns whether the gesture is over.
    fn advance(&mut self, instance: &Instance, now: Instant) -> bool {
        self.retry_at = None;
        let param = self.index as u32;
        if !self.begun && !instance.post(HostTask::Begin(param)) {
            self.retry_at = Some(now + POST_RETRY);
            return false;
        }
        self.begun = true;
        if let Some(value) = self.owed {
            if !instance.post(HostTask::Perform(param, value)) {
                self.retry_at = Some(now + POST_RETRY);
                return false;
            }
            self.owed = None;
        }
        if self.deadline > now {
            return false;
        }
        if !instance.post(HostTask::End(param)) {
            self.retry_at = Some(now + POST_RETRY);
            return false;
        }
        true
    }
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
    /// Whether the instance takes what the device holds as its own state
    /// (the tone it saves), as opposed to a restored project's.
    adopt: bool,
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
    /// How many restores the follower has seen the instance have: a new one
    /// is a new state to push.
    seen_restores: u64,
    /// A restore came with the Binding change the next step sees: the new
    /// Binding is the project's, not the user's.
    restored_now: bool,
    /// The device session the link is in, while it is up.
    session: Option<SessionId>,
    /// The push of the project's state to the device.
    recall: Recall,
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
            seen_restores: 0,
            restored_now: false,
            session: None,
            recall: Recall::default(),
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
            .map(Gesture::wake)
            .chain(self.reading.as_ref().map(|r| r.deadline.min(r.last_chance)))
            .chain(self.probe.as_ref().and_then(|p| p.deadline))
            .chain(self.probe_retry)
            .min()
            .map(|deadline| deadline.saturating_duration_since(now))
            .into_iter()
            .chain(self.recall.next_wake(now))
            .min()
    }

    /// Take the next event, if there was one, and carry out what follows
    /// from it and from the time `now`.
    pub fn step(&mut self, event: Option<&ClientEvent>, now: Instant, wire: &dyn Wire) {
        if let Some(event) = event {
            self.on_event(event, now, wire);
        }
        // The host gave the instance a state (a preset load, a revert): the
        // Binding it sets with it is the project's, and the state is owed a
        // push, after a read that keeps its values.
        let restores = self.instance.restores();
        if restores != self.seen_restores {
            self.seen_restores = restores;
            self.restored_now = true;
            self.recall.abort(&self.instance);
            self.stale = true;
        }
        self.follow_binding(wire);
        self.restored_now = false;
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
        self.recall_if_due(now, wire);
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
        for gesture in self.gestures.drain(..).filter(|g| g.begun) {
            self.instance.post(HostTask::End(gesture.index as u32));
        }
    }

    fn on_event(&mut self, event: &ClientEvent, now: Instant, wire: &dyn Wire) {
        match event {
            ClientEvent::Connected { client, session } => {
                self.me = Some(*client);
                self.connected = true;
                self.link_up = session.is_some();
                self.session = *session;
                self.recall.abort(&self.instance);
                self.out_of_date(true);
            }
            ClientEvent::Disconnected | ClientEvent::Incompatible { .. } => {
                self.me = None;
                self.connected = false;
                self.link_up = false;
                self.recall.abort(&self.instance);
                self.instance.disarm();
                self.forget_fit();
                self.out_of_date(true);
            }
            ClientEvent::Overflowed => {
                // `Pushed` or the baseline may be among what was lost.
                self.recall.abort_lost(&self.instance);
                self.out_of_date(true);
            }
            ClientEvent::Message(Message::LinkUp { session }) => {
                self.link_up = true;
                self.session = Some(*session);
                self.recall.abort(&self.instance);
                self.out_of_date(true);
            }
            ClientEvent::Message(Message::LinkDown) => {
                self.link_up = false;
                self.session = None;
                self.recall.abort(&self.instance);
                self.instance.disarm();
                self.forget_fit();
                self.out_of_date(true);
            }
            ClientEvent::Message(message @ Message::Pushed) => {
                if let Some(binding) = self.bound {
                    let cx = Context {
                        instance: &self.instance,
                        binding,
                        device_id: self.device_id,
                        wire,
                    };
                    self.recall.on_message(&cx, message, now);
                }
            }
            ClientEvent::Message(Message::Changed {
                writer,
                param,
                binding,
                value,
            }) => self.on_changed(*writer, param, binding, *value, now),
            // What the instance's own push does to the device is not news: the
            // values the host holds are what it sent.
            ClientEvent::Message(Message::Bulk { writer, .. })
                if self.recall.active() && Some(*writer) == self.me => {}
            ClientEvent::Message(Message::Bulk {
                writer,
                kind,
                binding,
                selected,
                tone,
            }) if self.bulk_concerns_us(*kind, binding) => {
                // The whole kit's selection reloads every key under a key's
                // plugin.
                if *kind == BulkKind::Tone
                    && *selected
                    && Some(*writer) != self.instance.editor_client()
                    && let Some(mine) = self.bound
                {
                    self.recall
                        .kit_reloaded(&self.instance, &mine, self.session);
                }
                // The editor's pick reached the Part during the push: the
                // push's values may now land on the pick's tone. The push
                // starts over, its selection first, after the read it waits
                // for. Its answers still to come are not the new push's.
                if *kind == BulkKind::Tone
                    && *selected
                    && self.recall.active()
                    && Some(*writer) == self.instance.editor_client()
                    && self.bound.is_some_and(|b| b.key.is_none())
                {
                    self.recall.abort_lost(&self.instance);
                }
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
            ClientEvent::Message(message @ Message::Baseline { .. }) => {
                if let Some(binding) = self.bound {
                    let cx = Context {
                        instance: &self.instance,
                        binding,
                        device_id: self.device_id,
                        wire,
                    };
                    self.recall.on_message(&cx, message, now);
                }
            }
            ClientEvent::Message(Message::Rejected {
                reason,
                param: None,
            }) if self.recall.active() && !matches!(reason, Reject::LinkDown) => {
                self.recall.refused(&self.instance);
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
                        if let [msb, lsb, pc, ..] = dt1.data[..] {
                            self.learn_tone((msb, lsb, pc), false);
                        }
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
        if let Some(tone) = tone
            && selected
            && kind == BulkKind::Tone
        {
            let by_editor = Some(writer) == self.instance.editor_client();
            self.learn_tone((tone.msb, tone.lsb, tone.pc), by_editor);
        }
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

    /// The device says which tone the Part holds: the instance saves it as its
    /// own unless a push of the project's is owed or under way, which the
    /// device's tone must not replace. The editor offers no tone pick in that
    /// window ([`Instance::recall_owed`]), since the push would select the
    /// saved tone over it.
    ///
    /// The page learns of the window late, so a pick can still reach the
    /// device in it (`by_editor`,
    /// [#504](https://github.com/sigma/integral/issues/504)). If the push is
    /// owed only because the device session is new, the pick is the user's
    /// latest word and stands:
    /// it is saved and adopted, so nothing is pushed over it in the session,
    /// and the read that follows takes its values. If a restore made the push
    /// owed, the restore is the later act and the project's: it wins, and the
    /// push selects its tone over the pick before writing that tone's values.
    fn learn_tone(&mut self, tone: (u8, u8, u8), by_editor: bool) {
        let owed = self.recall.owes(&self.instance, self.session);
        // A key's plugin selects nothing and its editor is not held back.
        let pick_stands = by_editor
            && self.bound.is_some_and(|b| b.key.is_none())
            && self
                .recall
                .owes_only_the_session(&self.instance, self.session);
        if !self.recall.active() && (!owed || pick_stands) {
            recall::save_tone(&self.instance, tone, None);
            // A tone the instance takes from the device is not one to push
            // back in the session it came from.
            self.recall.adopted(&self.instance, self.session);
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
            adopt: !self.keeps_project(),
            deadline: now,
            last_chance: now,
        };
        self.conclude(reading, now);
    }

    /// Whether the project's values stand: a push of them is owed (a restored
    /// state, or one the instance adopted, in a session it has not been pushed
    /// in), and is to bring them to the device.
    fn keeps_project(&self) -> bool {
        self.recall.owes(&self.instance, self.session)
    }

    /// Whether a read's values go to the host: it can be told, and the
    /// project's do not stand.
    fn applies(&self) -> bool {
        self.instance.bank.edits().is_some() && !self.keeps_project()
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
        // The project's state was not saved for another target: a user who
        // binds the instance elsewhere gets no push of it there. The Binding a
        // restore sets is the project's own.
        if self.bound.is_some() && !self.restored_now {
            self.recall.abandon(&self.instance, self.session);
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
        if self.recall.active() {
            // Another writer's change meets the push: the device may end up
            // with either, so read it when the push is out.
            self.stale = true;
        }
        if !bank.is_active(i) || bank.edits().is_none() || bank.value(i) == value {
            return;
        }
        let slot = match self.gestures.iter().position(|g| g.index == i) {
            Some(slot) => slot,
            None => {
                self.gestures.push(Gesture::new(i, now));
                self.gestures.len() - 1
            }
        };
        let gesture = &mut self.gestures[slot];
        gesture.deadline = now + GESTURE_END;
        gesture.owed = Some(value);
        bank.expect(i, value);
        gesture.advance(&self.instance, now);
    }

    /// Post what the host's queue refused before, and end the gestures that
    /// have been quiet long enough.
    fn end_gestures(&mut self, now: Instant) {
        let instance = &self.instance;
        self.gestures
            .retain_mut(|gesture| !gesture.advance(instance, now));
    }

    /// Post again a replace the host handed back or its queue refused, once
    /// its time has come.
    fn retry_replace(&mut self, now: Instant) {
        if self.instance.bank.take_retry() {
            self.retry_at = Some(now + REPLACE_RETRY);
        }
        if self.retry_at.is_some_and(|at| at <= now) {
            self.retry_at = None;
            self.post_replace(now);
        }
    }

    /// Post a replace of the values the bank has queued. One the host's queue
    /// refuses keeps them queued and is posted again after [`REPLACE_RETRY`].
    fn post_replace(&mut self, now: Instant) {
        if !self.instance.post(HostTask::Replace) {
            self.retry_at = Some(now + REPLACE_RETRY);
        }
    }

    /// Start the read if the values are out of date and it can be done.
    fn read_if_due(&mut self, now: Instant, wire: &dyn Wire) {
        let Some(binding) = self.bound else { return };
        if !self.stale
            || self.reading.is_some()
            || self.instance.fit() != ToneFit::Fits
            // Whatever the device holds meanwhile is the preset's, or the old
            // project's: no read applies, and the push is the one to say.
            || self.recall.active()
            || !self.connected
            || !self.link_up
            || !self.instance.is_settled()
            || !self.instance.kind().accepts(&binding)
        {
            return;
        }
        self.stale = false;
        // The project's values stand while a push of them is owed (a restored
        // state, or one the instance adopted, in a session it has not been
        // pushed in): the push is to bring them to the device.
        let keeps_project = self.keeps_project();
        let apply = self.applies();
        let mut blocks = plan(&self.instance, &binding, !apply);
        let checking = self.recall.checking();
        if !keeps_project || checking {
            blocks.extend(tone_blocks(&self.instance, &binding, !apply, checking));
        }
        if blocks.is_empty() {
            if !keeps_project {
                self.recall.adopted(&self.instance, self.session);
            }
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
            adopt: !keeps_project,
            deadline: now + READ_QUIET,
            last_chance: now + READ_AT_MOST,
        });
        // Raw replies on before the requests leave.
        self.sync_raw(wire);
        for rq1 in requests {
            wire.raw(rq1);
        }
    }

    /// Start the push of the project's state when it is owed and the instance
    /// can tell where it stands.
    ///
    /// - On a Part that holds the plugin's type, once the first read of the
    ///   session has kept the project's values.
    /// - On a Part known to hold another type, at once: the push's selection
    ///   of the saved tone is what fixes the mismatch. A Tone or Kit plugin
    ///   bound to a key selects nothing, so it waits for the Part to fit.
    /// - Not while the Part's tone is unknown: a probe is under way, and its
    ///   answer, from before the selection, would undo the fit the selection
    ///   sets.
    fn recall_if_due(&mut self, now: Instant, wire: &dyn Wire) {
        let was_active = self.recall.active();
        // A default tone is not offered while a push of the saved one is owed.
        // (A key's plugin selects nothing, so its Mismatch is no recall's to
        // fix.)
        let selects = self.bound.is_some_and(|b| b.key.is_none());
        self.instance.set_recall_owed(
            selects && (was_active || self.recall.owes(&self.instance, self.session)),
        );
        let Some(binding) = self.bound else { return };
        let cx = Context {
            instance: &self.instance,
            binding,
            device_id: self.device_id,
            wire,
        };
        self.recall.tick(&cx, now);
        let can_select = binding.key.is_none() && self.checks_tone();
        let placed = match self.instance.fit() {
            ToneFit::Fits => !self.stale && self.reading.is_none(),
            ToneFit::Mismatch => can_select,
            ToneFit::Unknown => false,
        };
        if placed && self.connected && self.link_up && self.instance.is_settled() {
            self.recall.begin(&cx, self.session, now);
        }
        // The push is over and the instance is not armed (it was not on a
        // Part that fits when it ended): a read arms it.
        if was_active && !self.recall.active() && !self.instance.is_armed() {
            self.stale = true;
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
        self.conclude(reading, now);
    }

    /// Give the host what a finished read found, and arm the instance.
    fn conclude(&mut self, reading: Reading, now: Instant) {
        let byte = |at: u32| reading.bytes.get(&at).copied();
        if reading.adopt {
            recall::capture(&self.instance, &reading.binding, &byte);
            self.recall.adopted(&self.instance, self.session);
        } else if self.recall.checking()
            && !self
                .recall
                .select_matches(&self.instance, &reading.binding, self.session, &byte)
        {
            // Someone else's kit is on the Part: the instance takes it.
            self.stale = true;
        }
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
            self.post_replace(now);
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

/// The blocks that tell the instance what to save of the tone a Part holds:
/// the tone's name, if the instance reads only effect types (`types_only`;
/// the full read has it with the tone's Common block), and with `select` the
/// tone-select bytes too. Those are otherwise the probe's and the service's to
/// tell ([`Follower::learn_tone`]).
fn tone_blocks(
    instance: &Instance,
    binding: &Binding,
    types_only: bool,
    select: bool,
) -> Vec<Block> {
    let (Some(part), Some(_)) = (binding.part, instance.kind().tone_type()) else {
        return Vec::new();
    };
    let mut blocks = Vec::new();
    if select {
        let (address, size) = params::tone_select_block(part);
        blocks.push(Block {
            start: address.value(),
            len: size.value(),
        });
    }
    if types_only && let Some((start, len)) = recall::name_block(instance.kind(), binding) {
        blocks.push(Block { start, len });
    }
    blocks
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
        /// The tones the pushes selected, as (MSB, LSB, PC).
        pushed_tones: RefCell<Vec<(u8, u8, u8)>>,
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
        fn push(&self, requests: Vec<Request>) {
            self.calls
                .borrow_mut()
                .push(format!("push {}", requests.len()));
            for request in &requests {
                if let Request::ChangeTone { msb, lsb, pc, .. } = request {
                    self.pushed_tones.borrow_mut().push((*msb, *lsb, *pc));
                }
            }
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
    fn a_tone_selected_for_the_bound_part_becomes_a_fresh_instances_saved_tone() {
        let instance = instance(PluginKind::SnSynth, Binding::of_part(1));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(1, ToneType::SnSynth)), now, &wire);
        let tone = ToneSelect {
            msb: default_tone(ToneType::SnSynth).expect("a default tone").msb,
            lsb: 0,
            pc: 42,
        };
        let own = ClientEvent::Message(Message::Bulk {
            writer: ClientId(7),
            kind: BulkKind::Tone,
            binding: Binding::of_part(1),
            selected: true,
            tone: Some(tone),
        });
        follower.step(Some(&own), now, &wire);
        let saved = instance.tone.get().expect("a saved tone");
        assert_eq!((saved.msb, saved.lsb, saved.pc), (tone.msb, 0, 42));
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

    #[test]
    fn the_recall_waits_for_the_first_read_ignores_its_own_bulk_and_holds_reads_until_it_is_out() {
        use integral_device_service::protocol::ToneSelect;
        let instance = Arc::new(Instance::new(PluginKind::PcmSynth));
        instance.binding.set(Some(Binding::of_part(0)));
        instance
            .tone
            .set(Some(super::super::tone::SavedTone::new(87, 64, 3, "")));
        instance.mark_restored();
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        // The device says the Part holds a tone of the plugin's type.
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        let pushes = |wire: &FakeWire| {
            wire.calls
                .borrow()
                .iter()
                .filter(|c| c.starts_with("push"))
                .count()
        };
        assert_eq!(pushes(&wire), 0, "the first read comes first");
        assert!(instance.notices.get().is_empty());

        // The read gives up (the device is quiet): the selection and a
        // request for the baseline go out, and the instance says so.
        let now = now + Duration::from_secs(2);
        follower.step(None, now, &wire);
        assert_eq!(pushes(&wire), 1);
        assert_eq!(wire.baselines.borrow().len(), 1);
        assert_eq!(
            instance.notices.get(),
            [super::super::notice::Notice::Recalling]
        );
        assert!(!instance.is_armed());

        // Its own select announces a bulk: no news. Another writer's is
        // news, but no read applies while the push is under way.
        let bulk = |writer| {
            ClientEvent::Message(Message::Bulk {
                writer: ClientId(writer),
                kind: BulkKind::Tone,
                binding: Binding::of_part(0),
                selected: false,
                tone: None,
            })
        };
        let asked = wire.raws.borrow().len();
        follower.step(Some(&bulk(7)), now, &wire);
        follower.step(Some(&bulk(9)), now, &wire);
        assert_eq!(wire.raws.borrow().len(), asked, "nothing is read");

        // The baseline arrives: everything differs from none, so the fields go
        // as one more push, and the instance is done at its `Pushed`.
        let baseline = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: Some(ToneSelect {
                msb: 87,
                lsb: 64,
                pc: 3,
            }),
            runs: Vec::new(),
        });
        follower.step(Some(&baseline), now, &wire);
        assert_eq!(pushes(&wire), 2);
        follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        assert!(!instance.notices.get().is_empty(), "one push to go");
        follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        assert!(instance.notices.get().is_empty());
        assert!(instance.is_armed());

        // The other writer's bulk is read now, and the session has had its
        // push: the same session pushes nothing more, a new one does.
        assert!(wire.raws.borrow().len() > asked, "read after the push");
        follower.step(Some(&ClientEvent::Disconnected), now, &wire);
        follower.step(Some(&connected()), now, &wire);
        // The device says the Part holds a tone of the plugin's type.
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        follower.step(None, now + Duration::from_secs(40), &wire);
        assert_eq!(pushes(&wire), 2, "the same session");
        let up = ClientEvent::Message(Message::LinkUp {
            session: SessionId(2),
        });
        follower.step(Some(&up), now, &wire);
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        follower.step(None, now + Duration::from_secs(80), &wire);
        assert_eq!(pushes(&wire), 3, "a new session");
    }

    #[test]
    fn a_tone_selected_after_the_recall_replaces_the_restored_saved_tone() {
        let (instance, mut follower, wire, now) = recalling();
        let saved = |instance: &Instance| {
            let t = instance.tone.get().expect("a saved tone");
            (t.msb, t.lsb, t.pc)
        };
        assert_eq!(saved(&instance), (87, 64, 3), "tone A, as restored");
        // The baseline, then the pushes, complete the recall.
        let baseline = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: None,
            runs: Vec::new(),
        });
        follower.step(Some(&baseline), now, &wire);
        for _ in 0..3 {
            follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        }
        assert!(instance.notices.get().is_empty(), "the recall is out");
        // A selection of tone B for the Part, from any writer, becomes the saved tone.
        let b = ToneSelect {
            msb: 87,
            lsb: 64,
            pc: 9,
        };
        let selection = ClientEvent::Message(Message::Bulk {
            writer: ClientId(9),
            kind: BulkKind::Tone,
            binding: Binding::of_part(0),
            selected: true,
            tone: Some(b),
        });
        follower.step(Some(&selection), now, &wire);
        assert_eq!(saved(&instance), (87, 64, 9), "tone B");
    }

    #[test]
    fn the_editor_cannot_pick_a_tone_while_the_recall_is_owed_or_under_way() {
        // A pick then would not be saved, and the push would write the saved
        // tone over it: the instance says so, and the editor's tone button is
        // off until the push is out. (`recalling` checks it is owed before
        // the push begins.)
        let (instance, mut follower, wire, now) = recalling();
        assert!(instance.recall_owed(), "under way");
        let baseline = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: None,
            runs: Vec::new(),
        });
        follower.step(Some(&baseline), now, &wire);
        for _ in 0..3 {
            follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        }
        assert!(instance.notices.get().is_empty(), "the recall is out");
        follower.step(None, now, &wire);
        assert!(!instance.recall_owed(), "a pick can be made again");
    }

    /// The editor's client, as the service names it in a bulk event.
    const EDITOR: ClientId = ClientId(8);

    /// A restored PCM Synth instance on Part 0 whose recall of tone A
    /// (87, 64, 3) is out in session 1, with an editor open.
    fn recalled() -> (Arc<Instance>, Follower, FakeWire, Instant) {
        let (instance, mut follower, wire, now) = recalling();
        let baseline = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: None,
            runs: Vec::new(),
        });
        follower.step(Some(&baseline), now, &wire);
        for _ in 0..3 {
            follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        }
        follower.step(None, now, &wire);
        assert!(!instance.recall_owed(), "the recall is out");
        instance.set_editor_client(Some(EDITOR));
        (instance, follower, wire, now)
    }

    /// The service's word that the editor selected `(msb, lsb, pc)` on Part 0.
    fn editor_pick(tone: (u8, u8, u8)) -> ClientEvent {
        ClientEvent::Message(Message::Bulk {
            writer: EDITOR,
            kind: BulkKind::Tone,
            binding: Binding::of_part(0),
            selected: true,
            tone: Some(ToneSelect {
                msb: tone.0,
                lsb: tone.1,
                pc: tone.2,
            }),
        })
    }

    /// The saved tone's (MSB, LSB, PC).
    fn saved(instance: &Instance) -> (u8, u8, u8) {
        let t = instance.tone.get().expect("a saved tone");
        (t.msb, t.lsb, t.pc)
    }

    #[test]
    fn an_editor_pick_that_reaches_the_device_in_a_new_session_is_saved_and_not_pushed_over() {
        let (instance, mut follower, wire, now) = recalled();
        let pushes = calls(&wire, "push");
        // The editor picks tone B; the link drops and comes back (a new
        // session, a push owed) before the service announces the pick.
        follower.step(Some(&ClientEvent::Message(Message::LinkDown)), now, &wire);
        let up = ClientEvent::Message(Message::LinkUp {
            session: SessionId(2),
        });
        follower.step(Some(&up), now, &wire);
        assert!(instance.recall_owed(), "owed in the new session");
        let reads = block_reads(&wire, 0);
        follower.step(Some(&editor_pick((87, 64, 9))), now, &wire);
        assert_eq!(saved(&instance), (87, 64, 9), "the pick is the state");
        assert!(!instance.recall_owed(), "nothing is owed in this session");
        assert!(block_reads(&wire, 0) > reads, "the pick's values are read");
        // The read ends; no push of tone A follows.
        follower.step(None, now + Duration::from_secs(2), &wire);
        follower.step(None, now + Duration::from_secs(4), &wire);
        assert_eq!(calls(&wire, "push"), pushes, "{:?}", wire.calls.borrow());
        assert_eq!(saved(&instance), (87, 64, 9));
        assert!(instance.is_armed());
    }

    #[test]
    fn an_editor_pick_that_reaches_the_part_during_the_push_makes_it_start_over_with_its_selection()
    {
        // The pick would take the push's values onto its tone: the push
        // selects the saved tone again before any of them, so nothing ends
        // up half one tone and half the other.
        let (instance, mut follower, wire, now) = recalling();
        instance.set_editor_client(Some(EDITOR));
        assert_eq!(wire.pushed_tones.borrow().as_slice(), [(87, 64, 3)]);
        follower.step(Some(&editor_pick((87, 64, 9))), now, &wire);
        assert!(!instance.notices.get().contains(&Notice::Recalling));
        assert!(instance.recall_owed(), "still owed");
        assert_eq!(saved(&instance), (87, 64, 3), "the pick is not saved");
        // The select push's answer is the old push's, not the new one's.
        follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert_eq!(
            wire.pushed_tones.borrow().as_slice(),
            [(87, 64, 3), (87, 64, 3)],
            "selected again"
        );
        assert!(instance.notices.get().contains(&Notice::Recalling));
    }

    #[test]
    fn a_restore_just_after_an_editor_pick_wins_and_its_push_selects_the_restored_tone() {
        // The restore is the later act, and the project's (ADR 0009): the
        // pick is not saved, and the push selects the restored tone over it
        // before it writes that tone's values, so what the editor shows (the
        // device's tone) and the instance hold the restored tone, whole.
        let (instance, mut follower, wire, now) = recalled();
        // The editor picks tone B; the host restores tone C before the
        // service announces the pick.
        instance
            .tone
            .set(Some(super::super::tone::SavedTone::new(87, 0, 1, "")));
        instance.mark_restored();
        follower.step(Some(&editor_pick((87, 64, 9))), now, &wire);
        assert_eq!(saved(&instance), (87, 0, 1), "the restored tone stands");
        assert!(instance.recall_owed());
        // The read keeps the project's values and the push goes.
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert_eq!(wire.pushed_tones.borrow().last(), Some(&(87, 0, 1)));
        let baseline = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: Some(ToneSelect {
                msb: 87,
                lsb: 0,
                pc: 1,
            }),
            runs: Vec::new(),
        });
        follower.step(Some(&baseline), now, &wire);
        for _ in 0..2 {
            follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        }
        follower.step(None, now + Duration::from_secs(4), &wire);
        assert!(!instance.recall_owed(), "the push is out");
        assert_eq!(saved(&instance), (87, 0, 1));
    }

    /// A restored PCM Synth instance on Part 0 with a Preset tone, whose
    /// recall has begun: the selection is pushed and the baseline asked for.
    fn recalling() -> (Arc<Instance>, Follower, FakeWire, Instant) {
        let instance = Arc::new(Instance::new(PluginKind::PcmSynth));
        instance.binding.set(Some(Binding::of_part(0)));
        instance
            .tone
            .set(Some(super::super::tone::SavedTone::new(87, 64, 3, "")));
        instance.mark_restored();
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        assert!(instance.recall_owed(), "owed, before the push begins");
        // The device says the Part holds a tone of the plugin's type.
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        let now = now + Duration::from_secs(2);
        follower.step(None, now, &wire);
        assert!(instance.notices.get().contains(&Notice::Recalling));
        (instance, follower, wire, now)
    }

    fn calls(wire: &FakeWire, prefix: &str) -> usize {
        wire.calls
            .borrow()
            .iter()
            .filter(|c| c.starts_with(prefix))
            .count()
    }

    #[test]
    fn a_refused_push_clears_the_status_is_tried_again_and_is_given_up_on_after_a_few() {
        let (instance, mut follower, wire, now) = recalling();
        let refused = ClientEvent::Message(Message::Rejected {
            reason: Reject::Invalid {
                message: "no".into(),
            },
            param: None,
        });
        follower.step(Some(&refused), now, &wire);
        // The push begins again at once.
        assert_eq!(calls(&wire, "push"), 2);
        follower.step(Some(&refused), now, &wire);
        assert_eq!(calls(&wire, "push"), 3);
        follower.step(Some(&refused), now, &wire);
        assert_eq!(calls(&wire, "push"), 3, "given up on the session");
        assert!(instance.notices.get().is_empty());
        assert!(instance.is_armed());
    }

    #[test]
    fn lost_events_do_not_let_an_old_pushed_complete_the_next_push() {
        let (instance, mut follower, wire, now) = recalling();
        // The select push is out; its `Pushed` is among the lost events, or
        // late. A new push starts.
        follower.step(Some(&ClientEvent::Overflowed), now, &wire);
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert_eq!(calls(&wire, "push"), 2);
        // The old push's answer is not the new one's.
        follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        assert!(!instance.notices.get().is_empty(), "still recalling");
        // The new push's `Pushed` is not enough either: its select was one and
        // its fields are to come with the baseline.
        let baseline = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: None,
            runs: Vec::new(),
        });
        follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        follower.step(Some(&baseline), now, &wire);
        follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        assert!(instance.notices.get().is_empty());
    }

    #[test]
    fn a_baseline_of_another_tone_selects_ours_again_and_a_missing_one_pushes_in_full() {
        use integral_device_service::protocol::ToneSelect;
        let (_instance, mut follower, wire, now) = recalling();
        let other = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: Some(ToneSelect {
                msb: 87,
                lsb: 64,
                pc: 9,
            }),
            runs: Vec::new(),
        });
        follower.step(Some(&other), now, &wire);
        assert_eq!(calls(&wire, "push"), 2, "selected again");
        assert_eq!(wire.baselines.borrow().len(), 2, "and asked again");
        let none = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: None,
            runs: Vec::new(),
        });
        follower.step(Some(&none), now, &wire);
        assert_eq!(calls(&wire, "push"), 3, "the fields");
    }

    #[test]
    fn another_writers_change_during_the_recall_is_read_when_it_is_out() {
        let (instance, mut follower, wire, now) = recalling();
        let host = instance.bank.list()[5].0.clone();
        let changed = ClientEvent::Message(Message::Changed {
            writer: ClientId(9),
            param: ParamId {
                plugin: PluginKind::PcmSynth,
                id: host.id.clone(),
            },
            binding: Binding::of_part(0),
            value: host.max,
        });
        follower.step(Some(&changed), now, &wire);
        let asked = wire.raws.borrow().len();
        for _ in 0..2 {
            follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        }
        // (The baseline never came: the first `Pushed` was the select's.)
        let none = ClientEvent::Message(Message::Baseline {
            binding: Binding::of_part(0),
            tone: None,
            runs: Vec::new(),
        });
        follower.step(Some(&none), now, &wire);
        follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        assert!(instance.notices.get().is_empty());
        follower.step(None, now, &wire);
        assert!(wire.raws.borrow().len() > asked, "read after the push");
    }

    #[test]
    fn a_rebind_before_the_push_pushes_nothing_to_the_new_part() {
        let instance = Arc::new(Instance::new(PluginKind::PcmSynth));
        instance.binding.set(Some(Binding::of_part(0)));
        instance
            .tone
            .set(Some(super::super::tone::SavedTone::new(87, 64, 3, "")));
        instance.mark_restored();
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        // The device says the Part holds a tone of the plugin's type.
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        // The user picks another Part while the first read is under way.
        instance.binding.set(Some(Binding::of_part(5)));
        follower.step(None, now + Duration::from_secs(2), &wire);
        follower.step(None, now + Duration::from_secs(4), &wire);
        assert_eq!(calls(&wire, "push"), 0);
        assert!(instance.notices.get().is_empty());
    }

    #[test]
    fn a_restore_that_changes_the_binding_is_the_projects_and_is_pushed() {
        let (instance, mut follower, wire, now) = recalling();
        for _ in 0..2 {
            follower.step(Some(&ClientEvent::Message(Message::Pushed)), now, &wire);
        }
        // A DAW preset: another Part, another tone.
        instance.binding.set(Some(Binding::of_part(4)));
        instance
            .tone
            .set(Some(super::super::tone::SavedTone::new(87, 0, 1, "")));
        instance.mark_restored();
        follower.step(None, now, &wire);
        follower.step(Some(&holds(4, ToneType::PcmSynth)), now, &wire);
        follower.step(None, now + Duration::from_secs(2), &wire);
        follower.step(None, now + Duration::from_secs(4), &wire);
        assert!(calls(&wire, "push") >= 2, "{:?}", wire.calls.borrow());
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
        // The device says the Part holds a tone of the plugin's type.
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
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

    #[test]
    fn a_gesture_a_full_queue_refused_catches_up_with_the_latest_value_once_it_drains() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let log = Arc::new(Log {
            instance: Arc::downgrade(&instance),
            replaced: Default::default(),
        });
        instance.bank.set_edits(Some(log.clone()));
        let queue = super::super::instance::test_queue::Queue::for_instance(&instance, usize::MAX);
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        follower.step(Some(&holds(0, ToneType::PcmSynth)), now, &wire);
        queue.drain(&instance);

        let bank = &instance.bank;
        let i = bank.position("pcms/partial1/0x0F").expect("a parameter");
        let host = bank.list()[i].0.clone();
        let old = bank.value(i);
        // Two values, neither the one the host holds.
        let (first, last) = (host.min + 1, host.min + 2);
        assert!(old != first && old != last && last <= host.max);
        let changed = |value: i32| {
            ClientEvent::Message(Message::Changed {
                writer: ClientId(9),
                param: ParamId {
                    plugin: PluginKind::PcmSynth,
                    id: host.id.clone(),
                },
                binding: Binding::of_part(0),
                value,
            })
        };

        // The queue is full: the executor reports the drop.
        queue.make_room(0);
        assert!(
            !instance.post(HostTask::End(i as u32)),
            "the drop is reported"
        );
        follower.step(Some(&changed(first)), now, &wire);
        assert!(
            follower.next_wake(now) <= Some(POST_RETRY),
            "a refused Begin is retried soon, not at the gesture's end"
        );
        follower.step(Some(&changed(last)), now, &wire);
        follower.step(None, now + GESTURE_END * 2, &wire);
        assert_eq!(bank.value(i), old, "nothing reached the host");

        // Room for the Begin only: the value is still owed.
        queue.make_room(1);
        let later = now + GESTURE_END * 3;
        follower.step(None, later, &wire);
        assert_eq!(queue.drain(&instance), [HostTask::Begin(i as u32)]);

        // The queue drains: the latest value goes, then the gesture ends.
        queue.make_room(usize::MAX);
        follower.step(None, later, &wire);
        assert_eq!(
            queue.drain(&instance),
            [HostTask::Perform(i as u32, last), HostTask::End(i as u32)]
        );
        assert_eq!(bank.value(i), last, "the mirror caught up");
        assert!(follower.gestures.is_empty());
    }

    #[test]
    fn a_gesture_waits_after_a_refusal_instead_of_spinning() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        instance.bank.set_edits(Some(Arc::new(Log {
            instance: Arc::downgrade(&instance),
            replaced: Default::default(),
        })));
        let _queue = super::super::instance::test_queue::Queue::for_instance(&instance, 0);
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        let mut gesture = Gesture::new(0, now);
        gesture.deadline = now;
        gesture.begun = true;
        follower.gestures.push(gesture);
        follower.end_gestures(now);
        assert_eq!(follower.gestures.len(), 1, "the refused End is owed");
        assert_eq!(follower.next_wake(now), Some(POST_RETRY));
    }

    #[test]
    fn a_replace_a_full_queue_refused_is_posted_again_and_the_mirror_catches_up() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let log = Arc::new(Log {
            instance: Arc::downgrade(&instance),
            replaced: Default::default(),
        });
        instance.bank.set_edits(Some(log.clone()));
        let queue = super::super::instance::test_queue::Queue::for_instance(&instance, 0);
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let bank = &instance.bank;
        let i = bank.position("pcms/partial1/0x0F").expect("a parameter");
        let host = bank.list()[i].0.clone();
        let value = if bank.value(i) == host.max {
            host.min
        } else {
            host.max
        };
        let now = Instant::now();
        bank.expect(i, value);
        bank.queue_replace([(i, value)]);
        follower.post_replace(now);
        assert_eq!(queue.drain(&instance), []);

        queue.make_room(usize::MAX);
        follower.retry_replace(now + REPLACE_RETRY / 2);
        assert_eq!(queue.drain(&instance), [], "not before its time");
        follower.retry_replace(now + REPLACE_RETRY);
        assert_eq!(queue.drain(&instance), [HostTask::Replace]);
        assert_eq!(bank.value(i), value, "the mirror caught up");
    }
}
