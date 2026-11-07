//! The service core: one `DeviceState`, one link, one note queue.
//!
//! [`Service`] is sans-IO. Everything that happens to it arrives as an
//! [`Input`] with a timestamp, and [`Service::pump`] is the only place it
//! sends to the link, so the order on the wire is decided in one spot:
//! notes first, then at most one paced SysEx message.
//!
//! Clients reach it the same way: a request is an [`Input::Client`], checked
//! on arrival and queued with its client's ID. `pump` moves one queued item
//! into the device's own queue whenever that is empty, so the client queue is
//! the real one and an item's writer is known until the moment its DT1s are
//! queued for sending, which is when subscribers hear of it.

use std::collections::VecDeque;
use std::time::Duration;

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::OnceLock;

use integral_core::address::{Address, DataSize};
use integral_core::device::{Coalesce, DeviceState, coalesce, dt1_span, selects};
use integral_core::expansion_load::LoadAction;
use integral_core::host_params::{
    Anchor, Binding, EffectKind, Located, ParamHandle, ParamIndex, PluginKind, temporary_tone_part,
};
use integral_core::params;
use integral_core::sysex::{self, DEFAULT_DEVICE_ID};

use crate::clients::{Clients, Outbox};
use crate::link::{DeviceLink, LinkError, LinkEvent};
use crate::note::is_note_traffic;
use crate::protocol::{
    Bound, BulkKind, ClientId, Message, PROTOCOL_VERSION, ParamId, Reject, Request,
};
use crate::session::{RandomSessionIds, SessionId, SessionIdSource};

/// How long the service waits for the Identity Reply that settles the device
/// ID before it carries on with the ID it has (ms).
const ID_WAIT_MS: f64 = 1000.0;

/// Key of the queued Identity Request.
const IDENTITY_KEY: &str = "identity";

/// Most typed reads awaiting the device's reply. A reply that never comes
/// (the device ignored the request) is forgotten, oldest first.
const MAX_PENDING_READS: usize = 256;

/// How long the device has to answer a typed read (ms).
const READ_TIMEOUT_MS: f64 = 2000.0;

/// Most parameters one write or device message announces one by one; past
/// that, a bulk event.
const BULK_THRESHOLD: usize = 16;

/// Most addresses the service remembers a value for.
const MAX_KNOWN: usize = 1 << 16;

/// The index of every plugin's parameters, built once for the process: it
/// is the same for every service and client in it.
pub(crate) fn shared_params() -> &'static ParamIndex {
    static PARAMS: OnceLock<ParamIndex> = OnceLock::new();
    PARAMS.get_or_init(ParamIndex::new)
}

/// A checked request of a client waiting for the device's queue.
struct Pending {
    client: ClientId,
    request: Request,
    /// The addresses it writes, if it is a write (the rule that lets a later
    /// write replace it is [`coalesce`]).
    span: Option<Range<u32>>,
}

/// A typed read sent to the device and not yet answered.
struct PendingRead {
    client: ClientId,
    handle: ParamHandle,
    binding: Binding,
    address: Address,
    deadline: f64,
}

/// Whether a message is a Roland DT1 (valid or not).
fn is_dt1(message: &[u8]) -> bool {
    message.get(1) == Some(&sysex::ROLAND_ID) && message.get(6) == Some(&sysex::CMD_DT1)
}

/// Who an announcement is for, and how.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    /// A client's write: always announced.
    Client,
    /// The device's report: announced if the value differs from the last
    /// known.
    Device,
    /// The device's report during a mixer load, which ends in one bulk
    /// event: remembered only.
    Quiet,
}

/// Progress of the Identity Request sent when a link came up without a
/// device ID.
#[derive(Debug, Clone, Copy, PartialEq)]
enum IdWait {
    /// Not waiting.
    Idle,
    /// The request is queued first and has not left yet.
    Queued,
    /// The request left; SysEx is held until the reply or this time.
    Sent { until: f64 },
}

/// How long after the service starts (its clock's origin) a [`Request::Claim`]
/// waits for an answer: the time the clients that were bound to the service
/// before a restart take to reconnect, so that their `Hello`'s bind lands
/// before a new instance is given a Part one of them holds.
#[cfg(unix)]
const CLAIM_HOLD: Duration = crate::client::RECONNECT_WINDOW;
#[cfg(not(unix))]
const CLAIM_HOLD: Duration = Duration::ZERO;

/// [`CLAIM_HOLD`] in the milliseconds of the service's clock.
const CLAIM_HOLD_MS: f64 = CLAIM_HOLD.as_secs_f64() * 1000.0;

/// What the service is built with.
pub struct Config {
    /// Use this SysEx device ID instead of the one the device gives in its
    /// Identity Reply.
    pub device_id: Option<u8>,
    /// Where session IDs come from.
    pub session_ids: Box<dyn SessionIdSource>,
}

impl Config {
    /// A config with random session IDs. `device_id` overrides discovery.
    pub fn new(device_id: Option<u8>) -> Self {
        Self {
            device_id,
            session_ids: Box::new(RandomSessionIds::default()),
        }
    }
}

/// Something the service tells its [`Listener`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<'a> {
    /// The link came up and a new session began.
    LinkUp(SessionId),
    /// The link went down; the session ended.
    LinkDown,
    /// The device sent this MIDI message. Raw SysEx clients receive all of
    /// them, as they would on a shared MIDI port.
    FromDevice {
        /// The message.
        message: &'a [u8],
        /// Whether it changed the cache: a DT1 that updated a field or
        /// answered a tracked read or an Expansion load step.
        changed: bool,
    },
    /// The link refused a message, which is lost.
    SendFailed(&'a str),
}

/// Observes a [`Service`].
///
/// Called on the service's own thread, after the service has updated itself,
/// once per event. It must not block: a slow listener stalls pacing, notes
/// and every other client. Hand the event to a channel or a socket writer
/// with its own buffer.
pub trait Listener: Send {
    /// Called for each event.
    fn event(&mut self, event: Event<'_>);
}

struct NoListener;

impl Listener for NoListener {
    fn event(&mut self, _event: Event<'_>) {}
}

/// Something that happens to a [`Service`].
pub enum Input {
    /// The link reported something.
    Link(LinkEvent),
    /// A message arrived on the note port.
    Note(Vec<u8>),
    #[doc(hidden)]
    /// A client connected. It must send [`Request::Hello`] first. Its
    /// `client` ID is the connection's, unique within this service.
    Connect {
        /// The new client.
        client: ClientId,
        /// Where the service puts what it says to the client.
        outbox: Box<dyn Outbox>,
    },
    /// A client sent a request.
    Client {
        /// The sender.
        client: ClientId,
        /// What it asks.
        request: Request,
    },
    /// A client sent a frame that did not decode. The connection stays.
    Malformed {
        /// The sender.
        client: ClientId,
        /// What the decoder said.
        message: String,
    },
    /// A client went away.
    Disconnect {
        /// The client.
        client: ClientId,
    },
    /// Run this against the service on its thread.
    Run(Box<dyn FnOnce(&mut Service) + Send>),
    /// Stop the service.
    Stop,
}

/// The Device service's core.
pub struct Service {
    link: Box<dyn DeviceLink>,
    device: DeviceState,
    /// The device ID is the caller's, not the device's.
    id_fixed: bool,
    ids: Box<dyn SessionIdSource>,
    session: Option<SessionId>,
    id_wait: IdWait,
    /// The link went down during an Expansion load; applied when it ends.
    down_deferred: bool,
    notes: VecDeque<Vec<u8>>,
    /// The clock at the last input or pump: the service's time origin is
    /// its start (see [`CLAIM_HOLD_MS`]).
    now_ms: f64,
    /// Claims waiting for [`CLAIM_HOLD_MS`] to pass, in arrival order.
    held_claims: Vec<(ClientId, PluginKind)>,
    listener: Box<dyn Listener>,
    clients: Clients,
    /// Client work checked and waiting for the device's queue to empty, each
    /// item with its client.
    ///
    /// One FIFO for now. Each item keeps its [`ClientId`] until `feed_one`
    /// applies it, which is where per-client queues served round-robin, push
    /// requests and unchanged-write dropping slot in.
    pending: VecDeque<Pending>,
    /// Typed reads the device has not answered.
    reads: Vec<PendingRead>,
    /// The client whose `LoadMixer` is still being read.
    mixer_load: Option<ClientId>,
    /// Every parameter of every plugin, by ID and by address.
    params: &'static ParamIndex,
    /// What this session knows of the device: the last bytes written or
    /// heard at each parameter's address. An address not in it is unknown
    /// (its first sighting is silent). This is the cache #452's
    /// unchanged-write filter should read, not a second one.
    known: HashMap<Address, Vec<u8>>,
    /// The tone selects (a Part's, or `None` for the Studio Set's) whose
    /// bytes this session has seen whole.
    selects_known: HashSet<Option<u8>>,
}

impl Service {
    /// A service owning `link`. The link starts when the service is
    /// [spawned](crate::spawn); a test drives [`handle`](Self::handle) and
    /// [`pump`](Self::pump) itself.
    pub fn new(link: Box<dyn DeviceLink>, config: Config) -> Self {
        let device = DeviceState::new(config.device_id.unwrap_or(DEFAULT_DEVICE_ID));
        Self {
            link,
            params: shared_params(),
            known: HashMap::new(),
            selects_known: HashSet::new(),
            device,
            id_fixed: config.device_id.is_some(),
            ids: config.session_ids,
            session: None,
            id_wait: IdWait::Idle,
            down_deferred: false,
            notes: VecDeque::new(),
            now_ms: 0.0,
            held_claims: Vec::new(),
            listener: Box::new(NoListener),
            clients: Clients::default(),
            pending: VecDeque::new(),
            reads: Vec::new(),
            mixer_load: None,
        }
    }

    /// Replace the listener.
    #[must_use]
    pub fn with_listener(mut self, listener: Box<dyn Listener>) -> Self {
        self.listener = listener;
        self
    }

    /// The one device state, with its cache and queue.
    pub fn device(&self) -> &DeviceState {
        &self.device
    }

    /// The one device state, to queue sends on.
    pub fn device_mut(&mut self) -> &mut DeviceState {
        &mut self.device
    }

    /// The current session, or `None` while the link is down.
    pub fn session(&self) -> Option<SessionId> {
        self.session
    }

    pub(crate) fn link_mut(&mut self) -> &mut dyn DeviceLink {
        self.link.as_mut()
    }

    /// Apply one input at `now_ms` (a monotonic clock). Returns `false` once
    /// the service is told to stop.
    pub fn handle(&mut self, input: Input, now_ms: f64) -> bool {
        self.now_ms = now_ms;
        match input {
            Input::Link(LinkEvent::Up { device_id }) => {
                if self.down_deferred {
                    // The link bounced during an Expansion load: the session
                    // never ended.
                    self.down_deferred = false;
                    return true;
                }
                if let Some(id) = device_id {
                    self.adopt_device_id(id);
                }
                self.begin_session();
                if device_id.is_none() && !self.id_fixed {
                    // The link did not learn the ID. Ask the device, ahead
                    // of anything queued, and hold the rest until it answers.
                    self.device
                        .send_raw_first(IDENTITY_KEY, sysex::identity_request().to_vec());
                    self.id_wait = IdWait::Queued;
                }
            }
            Input::Link(LinkEvent::Down) => {
                if self.device.expansion_load_target().is_some() {
                    // The device goes quiet for seconds per slot during a
                    // load; that is not a lost link.
                    self.down_deferred = true;
                } else {
                    self.end_session();
                }
            }
            Input::Link(LinkEvent::Midi(message)) => {
                self.receive_from_device(&message, now_ms);
            }
            Input::Note(message) => {
                if self.session.is_some() && is_note_traffic(&message) {
                    self.notes.push_back(message);
                }
            }
            Input::Connect { client, outbox } => self.clients.connect(client, outbox),
            Input::Client { client, request } => self.client_request(client, request),
            Input::Malformed { client, message } => {
                self.reject(client, Reject::Malformed { message });
            }
            Input::Disconnect { client } => {
                self.held_claims.retain(|(held, _)| *held != client);
                self.clients.disconnect(client);
            }
            Input::Run(work) => work(self),
            Input::Stop => return false,
        }
        true
    }

    fn begin_session(&mut self) {
        let id = self.ids.next_id();
        self.session = Some(id);
        self.forget_client_work();
        self.listener.event(Event::LinkUp(id));
        self.clients.link_up(id);
    }

    fn end_session(&mut self) {
        self.session = None;
        self.id_wait = IdWait::Idle;
        self.down_deferred = false;
        // Notes are for now; they are worthless late.
        self.notes.clear();
        self.forget_client_work();
        self.listener.event(Event::LinkDown);
        self.clients.link_down();
    }

    /// Drop what clients asked for in the session that ended: it would go to
    /// a device that is not the one they asked, and what was known of the
    /// old one is not known of the new.
    fn forget_client_work(&mut self) {
        self.pending.clear();
        self.reads.clear();
        self.known.clear();
        self.selects_known.clear();
        self.mixer_load = None;
    }

    /// Refuse a client's request.
    fn reject(&mut self, client: ClientId, reason: Reject) {
        self.reject_for(client, reason, None);
    }

    /// Refuse a client's request about `param`, so it can tell which.
    fn reject_for(&mut self, client: ClientId, reason: Reject, param: Option<ParamId>) {
        self.clients
            .send(client, &Message::Rejected { reason, param });
    }

    /// Bind `client` to the first free target of `plugin` and tell it.
    fn answer_claim(&mut self, client: ClientId, plugin: PluginKind) {
        let state = self.device.state();
        let binding = self.clients.claim(client, plugin, |part| {
            plugin.tone_type().is_some()
                && params::tone_type_from_bank_msb(state.parts[usize::from(part)].tone_bank_msb)
                    == plugin.tone_type()
        });
        self.clients
            .send(client, &Message::Claimed { plugin, binding });
    }

    /// Answer the held claims once [`CLAIM_HOLD_MS`] has passed. Returns how
    /// long until then, if any are still held.
    fn pump_claims(&mut self, now_ms: f64) -> Option<Duration> {
        self.now_ms = now_ms;
        if self.held_claims.is_empty() {
            return None;
        }
        if now_ms < CLAIM_HOLD_MS {
            return Some(Duration::from_secs_f64((CLAIM_HOLD_MS - now_ms) / 1000.0));
        }
        for (client, plugin) in std::mem::take(&mut self.held_claims) {
            self.answer_claim(client, plugin);
        }
        None
    }

    /// Record what `client` is bound to, unless the plugin cannot have that
    /// Binding, which is refused.
    fn bind_client(&mut self, client: ClientId, bound: Option<Bound>) {
        match bound {
            Some(b) if !b.plugin.accepts(&b.binding) => {
                let message = format!("{:?} cannot be bound to {:?}", b.plugin, b.binding);
                self.reject_for(client, Reject::Invalid { message }, None);
            }
            _ => self.clients.bind(client, bound),
        }
    }

    /// Refuse a client's request and drop the client: what it sent cannot be
    /// recovered from.
    fn reject_and_drop(&mut self, client: ClientId, reason: Reject) {
        self.reject(client, reason);
        self.clients.disconnect(client);
    }

    /// Take a request from a client: answer or apply the ones that are about
    /// the client itself, check the others and queue them for `pump`.
    fn client_request(&mut self, client: ClientId, request: Request) {
        match request {
            Request::Hello { version, .. } if version != PROTOCOL_VERSION => {
                let reason = Reject::Version {
                    server: PROTOCOL_VERSION,
                };
                self.reject_and_drop(client, reason);
            }
            Request::Hello {
                subscriptions,
                raw,
                bound,
                ..
            } => {
                self.clients.greet(client);
                for binding in subscriptions {
                    self.clients.subscribe(client, binding);
                }
                self.clients.set_raw(client, raw);
                let welcome = Message::Welcome {
                    version: PROTOCOL_VERSION,
                    client,
                    session: self.session,
                };
                self.clients.send(client, &welcome);
                self.bind_client(client, bound);
            }
            _ if !self.clients.is_greeted(client) => {
                self.reject_and_drop(client, Reject::NotGreeted);
            }
            Request::Subscribe { binding } => self.clients.subscribe(client, binding),
            Request::Unsubscribe { binding } => self.clients.unsubscribe(client, &binding),
            Request::ReceiveRaw { on } => self.clients.set_raw(client, on),
            Request::Bind { bound } => self.bind_client(client, bound),
            Request::Claim { plugin } => {
                if self.now_ms < CLAIM_HOLD_MS {
                    self.held_claims.push((client, plugin));
                } else {
                    self.answer_claim(client, plugin);
                }
            }
            work => {
                let param = match &work {
                    Request::Write { param, .. } | Request::Read { param, .. } => {
                        Some(param.clone())
                    }
                    _ => None,
                };
                if self.session.is_none() {
                    self.reject_for(client, Reject::LinkDown, param);
                } else {
                    match self.check(&work) {
                        Ok(span) => self.enqueue(client, work, span),
                        Err(message) => {
                            self.reject_for(client, Reject::Invalid { message }, param);
                        }
                    }
                }
            }
        }
    }

    /// Queue a checked request. A write replaces the earlier write it
    /// covers, whoever wrote it, by the rule of the device's own queue (so a
    /// fader dragged faster than the device takes writes sends the latest
    /// value, not every one; the last writer wins, and the surviving item
    /// is announced as theirs): looking back, it passes unrelated items and
    /// stops at an overlap, a select, or a message that is not a write.
    fn enqueue(&mut self, client: ClientId, request: Request, span: Option<Range<u32>>) {
        if let Some(new) = &span {
            let mut replaced = Vec::new();
            for (i, item) in self.pending.iter().enumerate().rev() {
                match coalesce(new, item.span.as_ref()) {
                    Coalesce::Replace => replaced.push(i),
                    Coalesce::PassOver => {}
                    Coalesce::Stop => break,
                }
            }
            replaced.reverse();
            if let Some((&first, rest)) = replaced.split_first() {
                for &i in rest.iter().rev() {
                    self.pending.remove(i);
                }
                self.pending[first] = Pending {
                    client,
                    request,
                    span,
                };
                return;
            }
        }
        self.pending.push_back(Pending {
            client,
            request,
            span,
        });
    }

    /// The parameter a request names, where it is for the request's binding,
    /// and the parameter itself.
    fn resolve(
        &self,
        param: &ParamId,
        binding: &Binding,
    ) -> Result<(ParamHandle, Address), String> {
        let handle = param
            .handle(self.params)
            .ok_or_else(|| format!("no parameter {} in {:?}", param.id, param.plugin))?;
        let address = self
            .params
            .get(handle)
            .device_address(binding)
            .map_err(|e| e.to_string())?;
        Ok((handle, address))
    }

    /// Whether a request can be applied: its parameter exists and is in
    /// range, its binding has what it needs, and a SysEx message is one the
    /// device takes. Applying a checked request cannot fail or put a bad
    /// byte on the wire.
    fn check(&self, request: &Request) -> Result<Option<Range<u32>>, String> {
        let data_byte = |name: &str, value: u8| {
            if value < 0x80 {
                Ok(None)
            } else {
                Err(format!("{name} {value} is not a 7-bit value"))
            }
        };
        match request {
            Request::Write {
                param,
                binding,
                value,
            } => {
                let (handle, address) = self.resolve(param, binding)?;
                let def = self.params.get(handle);
                if (def.min..=def.max).contains(value) {
                    Ok(Some(dt1_span(&address, def.width)))
                } else {
                    Err(format!(
                        "value {value} is outside {}..={}",
                        def.min, def.max
                    ))
                }
            }
            Request::Read { param, binding } => self.resolve(param, binding).map(|_| None),
            Request::Raw { bytes } => {
                let framed = bytes.first() == Some(&sysex::SYSEX_START)
                    && bytes.last() == Some(&sysex::SYSEX_END);
                if !framed || bytes.len() > sysex::MAX_MESSAGE_LEN {
                    return Err(format!(
                        "not one SysEx message of at most {} bytes",
                        sysex::MAX_MESSAGE_LEN
                    ));
                }
                if let Some(b) = bytes[1..bytes.len() - 1].iter().find(|b| **b >= 0x80) {
                    return Err(format!("byte {b:#04X} inside a SysEx message"));
                }
                if is_dt1(bytes) {
                    let dt1 = sysex::parse_dt1(bytes).map_err(|e| format!("bad DT1: {e}"))?;
                    return Ok(Some(dt1_span(&dt1.address, dt1.data.len())));
                }
                Ok(None)
            }
            Request::ChangeTone { part, msb, lsb, pc } => {
                if *part >= DeviceState::NUM_PARTS {
                    return Err(format!("part {part} is out of range"));
                }
                data_byte("msb", *msb)?;
                data_byte("lsb", *lsb)?;
                data_byte("pc", *pc)
            }
            Request::SwitchStudioSet { pc } => data_byte("pc", *pc),
            _ => Ok(None),
        }
    }

    /// Move one item of client work into the device's queue if that queue is
    /// empty, so the per-client queue is the real one: the device's holds
    /// one item at a time (a write can be several DT1s) and an item's writer
    /// is still known when it leaves the line.
    fn feed_one(&mut self, now_ms: f64) {
        if self.device.has_pending() {
            return;
        }
        if let Some(Pending {
            client, request, ..
        }) = self.pending.pop_front()
        {
            self.apply(client, request, now_ms);
        }
    }

    /// Apply one checked request of `client`: queue it for the device and
    /// announce what it changes, with `client` as the writer.
    fn apply(&mut self, client: ClientId, request: Request, now_ms: f64) {
        match request {
            Request::Write {
                param,
                binding,
                value,
            } => {
                let Ok((handle, address)) = self.resolve(&param, &binding) else {
                    return;
                };
                let bytes = self.params.get(handle).encode(value);
                self.write_dt1(client, &address, &bytes, Some(handle));
            }
            Request::Read { param, binding } => {
                let Ok((handle, address)) = self.resolve(&param, &binding) else {
                    return;
                };
                let width = self.params.get(handle).width;
                let size = DataSize::from_value(width as u32).expect("a parameter is a few bytes");
                let rq1 = sysex::build_rq1(self.device.device_id(), &address, &size);
                self.device.send_raw(&format!("read:{address}"), rq1);
                if self.reads.len() >= MAX_PENDING_READS {
                    self.reads.remove(0);
                }
                self.reads.push(PendingRead {
                    client,
                    handle,
                    binding,
                    address,
                    deadline: now_ms + READ_TIMEOUT_MS,
                });
            }
            Request::Raw { mut bytes } => {
                if is_dt1(&bytes) {
                    let dt1 = sysex::parse_dt1(&bytes).expect("checked on arrival");
                    self.write_dt1(client, &dt1.address, &dt1.data, None);
                } else {
                    if bytes.get(1) == Some(&sysex::ROLAND_ID) && bytes.len() > 2 {
                        bytes[2] = self.device.device_id();
                    }
                    self.device.send_raw(&format!("raw:{client}"), bytes);
                }
            }
            Request::ChangeTone { part, msb, lsb, pc } => {
                self.device.change_part_tone(part, msb, lsb, pc);
                self.selects_known.insert(Some(part));
                self.announce_select(client, Some(part));
            }
            Request::SwitchStudioSet { pc } => {
                self.device.switch_studio_set(pc);
                self.selects_known.insert(None);
                self.announce_select(client, None);
            }
            Request::LoadMixer => {
                self.device.load_mixer();
                self.mixer_load = Some(client);
            }
            // Handled on arrival.
            Request::Hello { .. }
            | Request::Subscribe { .. }
            | Request::Unsubscribe { .. }
            | Request::ReceiveRaw { .. }
            | Request::Bind { .. }
            | Request::Claim { .. } => {}
        }
    }

    /// Write `data` at `address` for `client`: the cache takes it, the
    /// device gets it (a tone or Studio Set select through the state's own
    /// path), and the subscribers hear what it set. `exact` is the parameter
    /// a typed write meant, which settles an address that several
    /// parameters share (an effect's slot, one per type).
    fn write_dt1(
        &mut self,
        client: ClientId,
        address: &Address,
        data: &[u8],
        exact: Option<ParamHandle>,
    ) {
        let hits = selects(address, data.len());
        // A select written in part, with bytes the session never saw, cannot
        // be completed from the cache: its bytes go through as data, and the
        // change is still announced.
        let blind = hits
            .iter()
            .any(|h| !h.whole && !self.selects_known.contains(&h.part));
        if blind {
            self.device.write_data(address, data);
            for hit in &hits {
                self.announce_select(client, hit.part);
            }
        } else {
            let done = self.device.write_dt1(address, data);
            for part in done.tones {
                self.announce_select(client, Some(part));
            }
            if done.studio_set {
                self.announce_select(client, None);
            }
        }
        self.remember_selects(address, data.len());
        self.announce(client, address, data, exact, Source::Client);
    }

    /// Announce that a Part's tone (or, for `None`, the Studio Set) was
    /// selected.
    ///
    /// What the session knew of the old tone (its Temporary Tone area, or
    /// everything for a Studio Set) says nothing of the new, so it is
    /// forgotten here, where typed and raw selects meet: the reads that
    /// follow are first sightings, and quiet.
    fn announce_select(&mut self, writer: ClientId, part: Option<u8>) {
        match part {
            Some(part) => {
                self.known
                    .retain(|a, _| temporary_tone_part(a) != Some(part));
                self.clients
                    .bulk(writer, BulkKind::Tone, Binding::of_part(part));
            }
            None => {
                self.known.clear();
                self.clients.bulk(writer, BulkKind::StudioSet, Binding::ANY);
            }
        }
    }

    /// Note the selects a write of `len` bytes at `address` covers whole.
    fn remember_selects(&mut self, address: &Address, len: usize) {
        for hit in selects(address, len).into_iter().filter(|h| h.whole) {
            self.selects_known.insert(hit.part);
        }
    }

    /// Tell subscribers that `writer` set the parameters in `data` at
    /// `address`: one change each, or one bulk event past
    /// [`BULK_THRESHOLD`] parameters. A change from the device is announced
    /// only if its value differs from one this session knew; its first
    /// sighting is silent.
    ///
    /// The data is walked parameter by parameter from `address`: a byte no
    /// parameter starts at is skipped, so a write that begins inside a
    /// parameter, or ends inside one, names only the whole parameters in it.
    /// An address several parameters share is an effect's slot, one per
    /// type: `exact` (the parameter a typed write meant) or the effect's
    /// type, if the session knows it, picks the one; failing both, one bulk
    /// event for that effect.
    fn announce(
        &mut self,
        writer: ClientId,
        address: &Address,
        data: &[u8],
        exact: Option<ParamHandle>,
        source: Source,
    ) {
        let params = self.params;
        let bulk_of = |located: &Located| -> (BulkKind, Binding) {
            let param = params.get(located.handle);
            let part = Binding::of_part(located.binding.part.unwrap_or(0));
            match param.effect.map(|e| e.kind) {
                Some(EffectKind::Mfx) => (BulkKind::Mfx, part),
                Some(EffectKind::Chorus) => (BulkKind::Chorus, Binding::ANY),
                Some(EffectKind::Reverb) => (BulkKind::Reverb, Binding::ANY),
                None if param.anchor == Anchor::Absolute => (BulkKind::StudioSet, Binding::ANY),
                None => (BulkKind::Tone, part),
            }
        };
        let mut changes = Vec::new();
        let mut bulks: Vec<(BulkKind, Binding)> = Vec::new();
        let mut pos = 0;
        while pos < data.len() {
            let at = address.offset(*DataSize::from_value(pos as u32).expect("short").as_bytes());
            let fits: Vec<_> = params
                .locate(&at)
                .into_iter()
                .filter(|l| params.get(l.handle).width <= data.len() - pos)
                .collect();
            let chosen = match fits.as_slice() {
                [] => {
                    pos += 1;
                    continue;
                }
                [one] => Some(*one),
                many => many
                    .iter()
                    .find(|l| Some(l.handle) == exact)
                    .or_else(|| many.iter().find(|l| self.is_active_type(l)))
                    .copied(),
            };
            let width = params.get(chosen.unwrap_or(fits[0]).handle).width;
            let bytes = &data[pos..pos + width];
            let before = self.known.insert(at, bytes.to_vec());
            let differs = source == Source::Client || before.as_deref().is_some_and(|k| k != bytes);
            if differs {
                match chosen {
                    Some(located) => {
                        let value = params.get(located.handle).decode(bytes);
                        changes.push((located, value.expect("a whole parameter")));
                    }
                    None => {
                        let entry = bulk_of(&fits[0]);
                        if !bulks.contains(&entry) {
                            bulks.push(entry);
                        }
                    }
                }
            }
            pos += width;
        }
        if self.known.len() > MAX_KNOWN {
            self.known.clear();
        }
        if source == Source::Quiet {
            return;
        }
        if changes.len() > BULK_THRESHOLD {
            for (located, _) in changes.drain(..) {
                let entry = bulk_of(&located);
                if !bulks.contains(&entry) {
                    bulks.push(entry);
                }
            }
        }
        for (located, value) in changes {
            let param = ParamId::of(params, located.handle);
            self.clients.changed(writer, param, located.binding, value);
        }
        for (kind, binding) in bulks {
            self.clients.bulk(writer, kind, binding);
        }
    }

    /// Whether the effect type `located` is a parameter of is the one its
    /// effect has, as far as this session knows.
    fn is_active_type(&self, located: &Located) -> bool {
        let param = self.params.get(located.handle);
        let Some(effect) = param.effect else {
            return false;
        };
        let Some(Ok(type_at)) = param.effect_type_address(&located.binding) else {
            return false;
        };
        self.known.get(&type_at).and_then(|b| b.first()) == Some(&effect.type_id)
    }

    /// Announce the end of a mixer load a client asked for, once the device
    /// has answered (or the reads gave up).
    fn settle_mixer_load(&mut self) {
        if self.mixer_load.is_some()
            && !self.device.mixer_loading()
            && let Some(client) = self.mixer_load.take()
        {
            self.clients.bulk(client, BulkKind::StudioSet, Binding::ANY);
        }
    }

    /// Give a typed read the reply to it, if `address` and `data` are that.
    /// Returns whether they were.
    fn answer_read(&mut self, address: &Address, data: &[u8]) -> bool {
        let Some(i) = self.reads.iter().position(|r| {
            r.address == *address && self.params.get(r.handle).decode(data).is_some()
        }) else {
            return false;
        };
        let read = self.reads.remove(i);
        let value = self.params.get(read.handle).decode(data).expect("checked");
        self.known.insert(*address, data.to_vec());
        let message = Message::Value {
            param: ParamId::of(self.params, read.handle),
            binding: read.binding,
            value,
        };
        self.clients.send(read.client, &message);
        true
    }

    /// Refuse the reads the device has not answered by `now_ms`.
    fn expire_reads(&mut self, now_ms: f64) {
        let (late, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut self.reads)
            .into_iter()
            .partition(|r| r.deadline <= now_ms);
        self.reads = waiting;
        for read in late {
            let param = ParamId::of(self.params, read.handle);
            self.reject_for(read.client, Reject::Timeout, Some(param));
        }
    }

    /// Use the SysEx device ID the device gave, unless the caller fixed one.
    /// A different ID replaces the state, dropping the cache and queue built
    /// for the old one. Returns whether it did.
    fn adopt_device_id(&mut self, id: u8) -> bool {
        if self.id_fixed || id == self.device.device_id() {
            return false;
        }
        self.device = DeviceState::new(id);
        true
    }

    /// Parse what the device sent into the cache, then tell the listener.
    fn receive_from_device(&mut self, message: &[u8], now_ms: f64) {
        let mut changed = false;
        if let Ok(dt1) = sysex::parse_dt1(message) {
            changed = self
                .device
                .handle_expansion_dt1(&dt1.address, &dt1.data, now_ms);
            changed |= self.device.handle_dt1(&dt1.address, &dt1.data, now_ms);
            self.remember_selects(&dt1.address, dt1.data.len());
            if !self.answer_read(&dt1.address, &dt1.data) {
                let source = if self.mixer_load.is_some() {
                    Source::Quiet
                } else {
                    Source::Device
                };
                self.announce(ClientId::DEVICE, &dt1.address, &dt1.data, None, source);
            }
        } else if let Ok(identity) = sysex::parse_identity_reply(message)
            && identity.is_integra7()
        {
            self.id_wait = IdWait::Idle;
            if self.adopt_device_id(identity.device_id) && self.session.is_some() {
                // The device the session began with is not the one that
                // answered: a new session, for a new state.
                self.begin_session();
            }
        }
        if message.first() == Some(&sysex::SYSEX_START) {
            self.clients.sysex(message);
        }
        self.settle_mixer_load();
        self.listener.event(Event::FromDevice { message, changed });
    }

    /// Send what is due at `now_ms`: every queued note, at once, then the
    /// Expansion load's step if one is due, then at most one SysEx message if
    /// the throttle allows. Each is one whole `send`, so a note lands between
    /// SysEx messages and never inside one. Returns how long the caller may
    /// sleep before pumping again (the state's next deadline, at least 1 ms),
    /// or `None` to wait for the next input.
    ///
    /// While the Identity Request that settles the device ID is out, notes
    /// still go but no SysEx does, for up to a second: then the ID in use
    /// stands.
    pub fn pump(&mut self, now_ms: f64) -> Option<Duration> {
        let claims = self.pump_claims(now_ms);
        let device = self.pump_device(now_ms);
        match (device, claims) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// The device side of [`pump`](Self::pump).
    fn pump_device(&mut self, now_ms: f64) -> Option<Duration> {
        self.session?;
        self.expire_reads(now_ms);
        self.feed_one(now_ms);
        while let Some(note) = self.notes.pop_front() {
            self.send(&note, true);
        }
        let gap = self.link.sysex_gap();
        let held = !gap.is_zero();
        if !held
            && let Some(LoadAction::Send(bytes) | LoadAction::Poll(bytes)) =
                self.device.next_load_action(now_ms)
        {
            self.send(&bytes, false);
        }
        if self.down_deferred && self.device.expansion_load_target().is_none() {
            self.end_session();
            return None;
        }
        if !held {
            match self.id_wait {
                IdWait::Idle => self.drain_one(now_ms),
                IdWait::Queued => {
                    // Only the request itself may leave first.
                    if let Some(message) = self.device.drain(now_ms) {
                        self.send(&message, false);
                        self.id_wait = IdWait::Sent {
                            until: now_ms + ID_WAIT_MS,
                        };
                    }
                }
                IdWait::Sent { until } => {
                    if now_ms >= until {
                        self.id_wait = IdWait::Idle;
                        self.drain_one(now_ms);
                    }
                }
            }
        }
        // Refill the device's queue, so its deadline covers the next item.
        self.feed_one(now_ms);
        self.settle_mixer_load();
        let mut due = match self.id_wait {
            // Nothing drains until then; wake for the fallback, or for an
            // Expansion load step if that is sooner.
            IdWait::Sent { until } => {
                Some(self.device.next_deadline().map_or(until, |d| d.min(until)))
            }
            _ => self.device.next_deadline(),
        };
        if let Some(read) = self.reads.iter().map(|r| r.deadline).min_by(f64::total_cmp) {
            due = Some(due.map_or(read, |d| d.min(read)));
        }
        if held {
            // The link just sent SysEx of its own: come back when the gap
            // ends, rather than wait for it here.
            let gap_end = now_ms + gap.as_secs_f64() * 1000.0;
            due = Some(due.map_or(gap_end, |d| d.min(gap_end)));
        }
        let due = due?;
        Some(Duration::from_millis((due - now_ms).max(1.0).ceil() as u64))
    }

    fn drain_one(&mut self, now_ms: f64) {
        if let Some(message) = self.device.drain(now_ms) {
            self.send(&message, false);
        }
    }

    fn send(&mut self, message: &[u8], is_note: bool) {
        if let Err(e) = self.link.send(message)
            && !(is_note && matches!(e, LinkError::Down))
        {
            self.listener.event(Event::SendFailed(&e.to_string()));
        }
    }
}
