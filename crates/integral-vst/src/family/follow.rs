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
//! read the device the first time, and not while the link is down. A bulk
//! event, an editor opening or a new Binding do not stop it: what the device's
//! side gives the host is marked ([`ParamBank::expect`]) so that it is not
//! written back. A host edit made while writing is off is kept and written
//! when it is back on: the host wins.
//!
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
use integral_core::sysex;
use integral_device_service::protocol::{BulkKind, ClientId, Message, ParamId};
use integral_device_service::{Client, ClientEvent};

use super::host::HostTask;
use super::instance::Instance;

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
        self.read_if_due(now, wire);
        self.finish_read(now, wire);
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
                self.out_of_date(wire);
            }
            ClientEvent::Disconnected | ClientEvent::Incompatible { .. } => {
                self.me = None;
                self.connected = false;
                self.link_up = false;
                self.instance.disarm();
                self.out_of_date(wire);
            }
            ClientEvent::Overflowed => self.out_of_date(wire),
            ClientEvent::Message(Message::LinkUp { .. }) => {
                self.link_up = true;
                self.out_of_date(wire);
            }
            ClientEvent::Message(Message::LinkDown) => {
                self.link_up = false;
                self.instance.disarm();
                self.out_of_date(wire);
            }
            ClientEvent::Message(Message::Changed {
                writer,
                param,
                binding,
                value,
            }) => self.on_changed(*writer, param, binding, *value, now),
            ClientEvent::Message(Message::Bulk { kind, binding, .. })
                if self.bulk_concerns_us(*kind, binding) =>
            {
                self.out_of_date(wire);
            }
            ClientEvent::Message(Message::Sysex { bytes }) => {
                if let Some(reading) = &mut self.reading
                    && let Ok(dt1) = sysex::parse_dt1(bytes)
                {
                    reading.take(dt1.address.value(), &dt1.data);
                    reading.deadline = now + READ_QUIET;
                }
            }
            _ => {}
        }
    }

    /// The values are out of date: drop a read under way and read again when
    /// possible.
    fn out_of_date(&mut self, wire: &dyn Wire) {
        // What the effects' types were says nothing of what they are now:
        // writing a slot meanwhile could land on a type just loaded.
        self.instance.bank.forget_types();
        self.stale = true;
        if self.reading.take().is_some() {
            wire.receive_raw(false);
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
        self.out_of_date(wire);
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
        if param.plugin != bank.kind() {
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
            || !self.connected
            || !self.link_up
            || !self.instance.is_settled()
            || !self.instance.kind().accepts(&binding)
        {
            return;
        }
        self.stale = false;
        // The project's values stand on a restored instance's first read.
        let apply = self.instance.bank.edits().is_some()
            && !(self.first_read && self.instance.was_restored());
        let blocks = plan(&self.instance, &binding, !apply);
        if blocks.is_empty() {
            self.first_read = false;
            self.instance.arm();
            return;
        }
        wire.receive_raw(true);
        for block in &blocks {
            let (Some(address), Some(size)) = (
                Address::from_value(block.start),
                DataSize::from_value(block.len),
            ) else {
                continue;
            };
            wire.raw(sysex::build_rq1(self.device_id, &address, &size));
        }
        self.reading = Some(Reading {
            binding,
            blocks,
            bytes: HashMap::new(),
            patched: Default::default(),
            apply,
            deadline: now + READ_QUIET,
            last_chance: now + READ_AT_MOST,
        });
    }

    /// Apply the read once it is complete or the device has gone quiet.
    fn finish_read(&mut self, now: Instant, wire: &dyn Wire) {
        if !self
            .reading
            .as_ref()
            .is_some_and(|r| r.complete() || r.deadline <= now || r.last_chance <= now)
        {
            return;
        }
        let reading = self.reading.take().expect("checked");
        wire.receive_raw(false);
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
    use super::*;
    use integral_device_service::SessionId;
    use std::cell::RefCell;

    #[derive(Default)]
    struct FakeWire {
        calls: RefCell<Vec<String>>,
        raws: RefCell<Vec<Vec<u8>>>,
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
    fn writing_starts_when_the_first_read_is_done_and_a_bulk_does_not_stop_it() {
        let instance = instance(PluginKind::PcmSynth, Binding::of_part(0));
        let wire = FakeWire::default();
        let mut follower = Follower::new(Arc::clone(&instance), sysex::DEFAULT_DEVICE_ID);
        let now = Instant::now();
        follower.step(Some(&connected()), now, &wire);
        assert!(!instance.is_armed(), "the read is under way");
        assert!(!wire.raws.borrow().is_empty(), "blocks were requested");
        // Nothing arrives: the device goes quiet and the read gives up.
        follower.step(None, now + Duration::from_secs(2), &wire);
        assert!(instance.is_armed());

        let bulk = ClientEvent::Message(Message::Bulk {
            writer: ClientId(9),
            kind: BulkKind::Tone,
            binding: Binding::of_part(0),
        });
        let asked = wire.raws.borrow().len();
        follower.step(Some(&bulk), now, &wire);
        assert!(instance.is_armed(), "still writing during the re-read");
        assert!(wire.raws.borrow().len() > asked, "read again");
        let elsewhere = ClientEvent::Message(Message::Bulk {
            writer: ClientId(9),
            kind: BulkKind::Tone,
            binding: Binding::of_part(1),
        });
        follower.step(None, now + Duration::from_secs(4), &wire);
        let asked = wire.raws.borrow().len();
        follower.step(Some(&elsewhere), now, &wire);
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
        assert!(!wire.raws.borrow().is_empty());
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
        assert!(!wire.raws.borrow().is_empty(), "a full read is under way");

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
