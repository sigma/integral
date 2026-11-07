//! The service core: one `DeviceState`, one link, one note queue.
//!
//! [`Service`] is sans-IO. Everything that happens to it arrives as an
//! [`Input`] with a timestamp, and [`Service::pump`] is the only place it
//! sends to the link, so the order on the wire is decided in one spot:
//! notes first, then at most one paced SysEx message.

use std::collections::VecDeque;
use std::time::Duration;

use integral_core::device::DeviceState;
use integral_core::expansion_load::LoadAction;
use integral_core::sysex::{self, DEFAULT_DEVICE_ID};

use crate::link::{DeviceLink, LinkError, LinkEvent};
use crate::note::is_note_traffic;
use crate::session::{RandomSessionIds, SessionId, SessionIdSource};

/// How long the service waits for the Identity Reply that settles the device
/// ID before it carries on with the ID it has (ms).
const ID_WAIT_MS: f64 = 1000.0;

/// Key of the queued Identity Request.
const IDENTITY_KEY: &str = "identity";

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
    listener: Box<dyn Listener>,
}

impl Service {
    /// A service owning `link`. The link starts when the service is
    /// [spawned](crate::spawn); a test drives [`handle`](Self::handle) and
    /// [`pump`](Self::pump) itself.
    pub fn new(link: Box<dyn DeviceLink>, config: Config) -> Self {
        Self {
            link,
            device: DeviceState::new(config.device_id.unwrap_or(DEFAULT_DEVICE_ID)),
            id_fixed: config.device_id.is_some(),
            ids: config.session_ids,
            session: None,
            id_wait: IdWait::Idle,
            down_deferred: false,
            notes: VecDeque::new(),
            listener: Box::new(NoListener),
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
            Input::Run(work) => work(self),
            Input::Stop => return false,
        }
        true
    }

    fn begin_session(&mut self) {
        let id = self.ids.next_id();
        self.session = Some(id);
        self.listener.event(Event::LinkUp(id));
    }

    fn end_session(&mut self) {
        self.session = None;
        self.id_wait = IdWait::Idle;
        self.down_deferred = false;
        // Notes are for now; they are worthless late.
        self.notes.clear();
        self.listener.event(Event::LinkDown);
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
        self.session?;
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
        let mut due = match self.id_wait {
            // Nothing drains until then; wake for the fallback, or for an
            // Expansion load step if that is sooner.
            IdWait::Sent { until } => {
                Some(self.device.next_deadline().map_or(until, |d| d.min(until)))
            }
            _ => self.device.next_deadline(),
        };
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
