//! The wire protocol between the Device service and its clients.
//!
//! # Framing
//!
//! A stream of frames over a Unix domain socket. A frame is a 4-byte
//! big-endian length `N` then `N` bytes of UTF-8 JSON, one [`Request`]
//! (client to service) or one [`Message`] (service to client). `N` is at
//! most [`MAX_FRAME_LEN`]; a longer frame ends the connection, since the
//! stream can no longer be trusted. A frame whose body does not decode is
//! only that frame's loss: the length prefix keeps the stream aligned, so the
//! service answers [`Reject::Malformed`] and carries on.
//!
//! JSON because `serde_json` is already a dependency of the workspace, the
//! traffic is a few hundred small messages a second at most, and a frame can
//! be read by eye. Every object has a `"type"` naming its variant.
//!
//! # Parameters
//!
//! A parameter is named as the host parameter lists name it (ADR 0008): a
//! [`PluginKind`] and the ID that plugin's list gives it, as
//! `{"plugin":"pcm_synth","id":"pcms/partial2/0x2A"}`. The ID is relative to
//! the [`Binding`], which is sent beside it: `{"part":2}`, and `"key"` too
//! for a drum key's parameter. A Studio Set ID holds its Part and needs no
//! binding. Values are the raw `i32` the list's `min..=max` gives. The
//! service resolves the address, the encoding and the range from core's
//! [`ParamIndex`]; a client never builds an address.
//!
//! # Versioning and compatibility
//!
//! The first frame a client sends is [`Request::Hello`] with its
//! [`PROTOCOL_VERSION`], and the subscriptions and raw setting it wants, which
//! are in force before the answer. The service answers [`Message::Welcome`]
//! if it speaks that version, else [`Message::Rejected`] with
//! [`Reject::Version`] and closes; that is final, and a client does not try
//! again. Any other request before `Hello` is rejected the same way.
//!
//! Within a version, peers ignore what they do not know: a new [`Message`]
//! type decodes as [`Message::Unknown`] and a new [`Reject`] kind as
//! [`Reject::Unknown`], which a receiver skips; a new request type is
//! answered [`Reject::Malformed`]. A change an old peer would misread (a field
//! changing meaning) bumps the version.
//!
//! `Bind`, `Claim`, the `Hello`'s `bound`, `Push`, `Baseline`, `Pushed`,
//! `Claimed` and the `Baseline` answer are part of version 1: no earlier peer
//! shipped, so nothing had to be added compatibly. From here on, a request
//! type added within a version is answered [`Reject::Malformed`] by a peer
//! that predates it, and a message type is skipped. A plugin instance that
//! gets such a refusal to its `Bind` or `Claim` treats the service as
//! incompatible. [`Message::Shared`] is of this kind: a client that predates
//! it skips it and never warns of a duplicate.
//!
//! [`Request::LoadExpansion`] and its answer [`Message::ExpansionLoadDone`] are
//! of this kind too. A service that predates them answers the request
//! [`Reject::Malformed`], and nothing is loaded: a client takes that to mean
//! this service cannot load Expansions. Only the requester is sent the
//! answer, so a client that predates it never sees one; the load's end
//! reaches every subscriber as a [`BulkKind::StudioSet`] event, which every
//! client knows.
//!
//! [`Request::Note`] is of this kind as well, and has no answer. A service
//! that predates it answers [`Reject::Malformed`] to each and plays nothing;
//! the plugin editor passes that on to its page as an error, as it does any
//! refusal.
//!
//! # Pacing and cache
//!
//! Requests of one client run in the order it sent them. The service takes
//! one item at a time from the client served least recently, so sixteen
//! instances that each push a tone fill in together, and a client that has
//! been idle (a fader that starts to move) is served at the next turn. A
//! [`Request::Write`] that covers an earlier write still queued, whoever's,
//! replaces it (the survivor is announced as the last writer's), so a fader
//! dragged faster than the device takes writes costs one message per slot. A
//! write whose value the service knows the device already has is dropped, but
//! only against what this session has read or written: every field is unknown
//! when a session begins, and a [`Request::Push`] is never dropped from. Two
//! pushes that touch the same Part interleave item by item; the later item
//! wins as each reaches the device.
//!
//! When a session ends (the link drops, or the device turns out to be
//! another) everything outstanding is refused with [`Reject::LinkDown`]: each
//! queued request, a read, a baseline, a push not yet handed over. The
//! service forgets the device (the cache, the queue, the baselines of Expansion
//! tones) and does not replay anything to the next one.
//!
//! # Tone baselines and recall
//!
//! After a client selects a Preset or Expansion tone on a Part
//! ([`Request::ChangeTone`], or a DT1 on the select), the service reads the
//! tone back, a block at a time, in that client's turns and ahead of the
//! client's later requests. What it read is the Part's baseline
//! ([`Request::Baseline`]). A tone read once is kept for every Part for the
//! life of the service (Expansion tones for the session), so a repeat recall
//! reads nothing.
//!
//! A recall that selects a Preset or Expansion tone asks for the baseline
//! after the selection and sends what differs from it in a second push: the
//! service never drops from a push.
//!
//! # A session
//!
//! ```text
//! client  {"type":"hello","version":1,"subscriptions":[{"part":2}]}
//! service {"type":"welcome","version":1,"client":3,"session":"7"}
//! client  {"type":"write","param":{"plugin":"studio_set","id":"part03/0x18002009"},
//!          "binding":{},"value":100}
//! service {"type":"changed","writer":3,"param":{..},"binding":{},"value":100}
//! service {"type":"link_down"}
//! service {"type":"link_up","session":"8"}
//! ```
//!
//! A client ID is assigned per connection and is meaningful only to the
//! service process that gave it; [`ClientId::DEVICE`] (0) is the device
//! itself. A [`SessionId`] (a decimal string, so a JavaScript client does not
//! round it) says which span of one link a message is from: a client that
//! sees it change (or sees [`Message::LinkUp`]) must re-read what it cares
//! about.

use std::io::{self, Read, Write};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::session::SessionId;
#[cfg(doc)]
use integral_core::expansion_slots::ExpansionSlots;
use integral_core::expansion_slots::SLOT_COUNT;
use integral_core::host_params::{Binding, ParamHandle, ParamIndex, PluginKind};
use integral_core::sysex;

/// The protocol version this crate speaks.
pub const PROTOCOL_VERSION: u32 = 1;

/// Most bytes in one frame's body.
pub const MAX_FRAME_LEN: usize = 64 * 1024;

/// Who made a change: a connected client, or the device itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ClientId(pub u64);

impl ClientId {
    /// The device: the writer of a change that came from it.
    pub const DEVICE: Self = Self(0);
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A parameter on the wire: a plugin and the ID its list gives it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ParamId {
    /// The plugin whose list the ID is from.
    pub plugin: PluginKind,
    /// The ID (see the module docs).
    pub id: String,
}

impl ParamId {
    /// The wire name of the parameter `handle` names in `index`.
    pub fn of(index: &ParamIndex, handle: ParamHandle) -> Self {
        Self {
            plugin: handle.plugin,
            id: index.get(handle).id.clone(),
        }
    }

    /// The handle of the parameter in `index`, if it has one.
    pub fn handle(&self, index: &ParamIndex) -> Option<ParamHandle> {
        index.find(self.plugin, &self.id)
    }
}

/// A plugin instance's identity: its plugin and the target it is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bound {
    /// The plugin.
    pub plugin: PluginKind,
    /// Its Binding: [`Binding::ANY`] for the Studio Set plugin.
    pub binding: Binding,
}

impl Bound {
    /// Whether the two instances are duplicates: the same plugin, and (but
    /// for the Studio Set plugin, which has one target) the same Binding, a
    /// Part and for a key-bound Kit plugin the key. Two plugins of different
    /// kinds on one Part are not: they cannot both match its tone type.
    pub fn shares_target(&self, other: &Self) -> bool {
        self.plugin == other.plugin
            && (self.plugin == PluginKind::StudioSet || self.binding == other.binding)
    }
}

/// What a client asks of the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    /// The first request: the protocol version the client speaks, and what
    /// it wants in force before the [`Message::Welcome`].
    Hello {
        /// The client's [`PROTOCOL_VERSION`].
        version: u32,
        /// As [`Request::Subscribe`], for each.
        #[serde(default)]
        subscriptions: Vec<Binding>,
        /// As [`Request::ReceiveRaw`].
        #[serde(default)]
        raw: bool,
        /// As [`Request::Bind`], so that a reconnect binds before anything
        /// else. An older service ignores it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bound: Option<Bound>,
    },
    /// Receive [`Message::Changed`] for changes in `binding` (a filter: see
    /// [`Binding::matches`]) and [`Message::Bulk`] events. Several
    /// subscriptions add up.
    Subscribe {
        /// What to receive.
        binding: Binding,
    },
    /// Drop a subscription made with the same binding.
    Unsubscribe {
        /// The binding given to [`Request::Subscribe`].
        binding: Binding,
    },
    /// Turn on or off receiving every SysEx message the device sends, as on a
    /// shared MIDI port ([`Message::Sysex`]).
    ReceiveRaw {
        /// Whether to receive them.
        on: bool,
    },
    /// Say what this client is: the plugin it is an instance of and the
    /// Binding it acts on, or `None` to be no instance. For a restored
    /// Binding or one the user picked. The service refuses a Binding the
    /// plugin cannot have ([`PluginKind::accepts`]) with [`Reject::Invalid`].
    /// It lasts as long as the connection (the client library says it again
    /// in its `Hello` after a reconnect).
    Bind {
        /// The plugin, if any, with its Binding.
        bound: Option<Bound>,
    },
    /// Take the first target of `plugin` that is free, and bind this client
    /// to it, in one step so that two instances never get the same one. A
    /// Part is taken if any Tone plugin, or a Kit plugin bound to the whole
    /// kit, is bound to it (of any kind); a key-bound Kit plugin takes
    /// nothing. Among the free Parts the service prefers one that holds a tone
    /// of the plugin's type, if it knows, else the first. The answer is
    /// [`Message::Claimed`], to this client only.
    Claim {
        /// The plugin kind a new instance of which is looking for a target.
        plugin: PluginKind,
    },
    /// Set a parameter. Goes through the service's paced queue.
    Write {
        /// The parameter.
        param: ParamId,
        /// The Part (and key) it is for.
        binding: Binding,
        /// The raw value.
        value: i32,
    },
    /// Ask the device for a parameter's current value. The answer is
    /// [`Message::Value`], to this client only, or [`Reject::Timeout`] if the
    /// device does not reply.
    Read {
        /// The parameter.
        param: ParamId,
        /// The Part (and key) it is for.
        binding: Binding,
    },
    /// Send one whole SysEx message through the same paced queue. A DT1 must
    /// have a good checksum; it is also parsed into the cache and announced
    /// to subscribers with this client's ID. The device ID in a Roland
    /// message is replaced with the service's.
    Raw {
        /// The message, `F0` to `F7`, at most 256 bytes.
        bytes: Vec<u8>,
    },
    /// Select a tone on a Part by Bank Select and Program Change. Announced
    /// as [`BulkKind::Tone`].
    ChangeTone {
        /// The Part (0-based).
        part: u8,
        /// Bank Select MSB.
        msb: u8,
        /// Bank Select LSB.
        lsb: u8,
        /// Program Change number.
        pc: u8,
    },
    /// Select a Studio Set by Program Change. Announced as
    /// [`BulkKind::StudioSet`].
    SwitchStudioSet {
        /// Program Change number.
        pc: u8,
    },
    /// Read the whole mixer from the device. When the primary blocks have
    /// arrived the service announces [`BulkKind::StudioSet`].
    LoadMixer,
    /// Recall: push saved state to the device (ADR 0009). The requests run
    /// in order, each in the client's turn of the round-robin like any other,
    /// and the project wins: unlike [`Request::Write`] outside a push, a write
    /// is sent even if the service believes the device already has the value.
    /// A write the client queues after the push is applied after all of it.
    /// The service answers [`Message::Pushed`] when the last has been handed
    /// to the device's queue (not when it has left: the queue paces them).
    ///
    /// Only [`Request::Write`], [`Request::Raw`], [`Request::ChangeTone`] and
    /// [`Request::SwitchStudioSet`] may be in it; if any request is not valid
    /// the whole push is refused ([`Reject::Invalid`]) and none is queued.
    Push {
        /// What to send, in order.
        requests: Vec<Request>,
    },
    /// Ask for the baseline of a Part's tone: the tone as the device reported
    /// it right after a Preset or Expansion tone was selected, for recall to
    /// diff against. The answer is [`Message::Baseline`], once the service
    /// has read the tone back if it is doing so.
    Baseline {
        /// The Part ([`Binding::part`]).
        binding: Binding,
    },
    /// Load Expansions into the Expansion slots. In the client's turn, the
    /// service reads the slots first if it does not know them, then sends
    /// the load and holds every client's SysEx until the load is over: the
    /// device cuts all sound and ignores requests for seconds per slot. The
    /// answer is [`Message::ExpansionLoadDone`], to this client only, once
    /// the load is over, however it ended. Before anything is sent the
    /// request is refused with [`Reject::Timeout`] if the device does not
    /// answer the slot read, or [`Reject::Invalid`] if another load is
    /// running. When a load that changed anything ends, every subscriber is
    /// sent a [`BulkKind::StudioSet`] event with this client as the writer.
    LoadExpansion {
        /// The slot values to load, slot A first, as the device's load
        /// request carries them ([`ExpansionSlots::values`]): `0` for OFF.
        /// The service refuses values that name no Expansion
        /// ([`ExpansionSlots::from_values`]) with [`Reject::Invalid`].
        slots: [u8; SLOT_COUNT],
    },
    /// Play notes: one or more whole channel voice messages (`8n` to `En`,
    /// each with its own status byte), sent to the device as note-port
    /// traffic is, at once and in order, between SysEx messages and ahead of
    /// the paced queue. A Program Change makes the service re-read the tone
    /// of the Parts on its channel. Not answered; anything else (running
    /// status, SysEx, a system or real-time message, a message cut short) is
    /// refused with [`Reject::Invalid`] and nothing is played. Dropped while
    /// the link is down, as a note port's traffic is. For a client with no
    /// MIDI port of its own, such as the plugin editor's page.
    Note {
        /// The messages, back to back.
        bytes: Vec<u8>,
    },
}

impl Request {
    /// The request that sends one MIDI message as a shared MIDI port would:
    /// a SysEx message (starting `F0`) as [`Request::Raw`], through the paced
    /// queue, and anything else as [`Request::Note`], played ahead of it. The
    /// service checks either and refuses what it cannot send.
    pub fn midi(bytes: Vec<u8>) -> Self {
        if bytes.first() == Some(&sysex::SYSEX_START) {
            Self::Raw { bytes }
        } else {
            Self::Note { bytes }
        }
    }
}

/// What a bulk event stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BulkKind {
    /// A Part's tone was replaced, or changed in too many parameters (or in
    /// a way, such as an effect's slots, that names no one parameter) to
    /// announce one by one. Sent to subscriptions that match its binding
    /// (the Part).
    Tone,
    /// The Studio Set was replaced, the mixer was read in full, or the
    /// Studio Set changed in many parameters at once. Sent to every
    /// subscriber.
    StudioSet,
    /// A tone's MFX changed in a slot that no one parameter is named by (the
    /// service does not know the MFX type). Sent as [`BulkKind::Tone`] is.
    Mfx,
    /// The Chorus changed in a slot that no one parameter is named by. Sent
    /// as [`BulkKind::StudioSet`] is.
    Chorus,
    /// The Reverb changed in a slot that no one parameter is named by. Sent
    /// as [`BulkKind::StudioSet`] is.
    Reverb,
}

/// How an Expansion load ended ([`Message::ExpansionLoadDone`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionLoadResult {
    /// The slots already held the target: nothing was sent.
    Unchanged,
    /// The slots now hold the target.
    Loaded,
    /// The device read back other slots than the target.
    Failed,
    /// The device stopped answering; the slots are unknown.
    TimedOut,
}

/// A tone as the device selects it: Bank Select and Program Change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ToneSelect {
    /// Bank Select MSB.
    pub msb: u8,
    /// Bank Select LSB.
    pub lsb: u8,
    /// Program Change number.
    pub pc: u8,
}

/// A run of consecutive bytes of a Part's Temporary Tone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineRun {
    /// The device address of the first byte, as in a DT1.
    pub address: [u8; 4],
    /// The bytes from there, as the device holds them.
    pub bytes: Vec<u8>,
}

/// Why a request was refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reject {
    /// The client sent something before [`Request::Hello`].
    NotGreeted,
    /// The service does not speak the client's version. Final.
    Version {
        /// The version the service speaks.
        server: u32,
    },
    /// The link is down, so the request would go nowhere.
    LinkDown,
    /// The request is well formed but not valid, for the reason given.
    Invalid {
        /// Why.
        message: String,
    },
    /// The frame did not decode into a request (or not into one this service
    /// knows). The connection stays.
    Malformed {
        /// What the decoder said.
        message: String,
    },
    /// The device did not answer a [`Request::Read`], or the slot read a
    /// [`Request::LoadExpansion`] begins with (nothing was loaded).
    Timeout,
    /// A refusal this peer does not know; skip it.
    #[serde(other)]
    Unknown,
}

/// What the service tells a client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// The answer to [`Request::Hello`].
    Welcome {
        /// The version the service speaks.
        version: u32,
        /// The ID given to this connection.
        client: ClientId,
        /// The current session, or none while the link is down.
        session: Option<SessionId>,
    },
    /// The link came up and a new session began: re-read everything.
    LinkUp {
        /// The new session.
        session: SessionId,
    },
    /// The link went down.
    LinkDown,
    /// A parameter changed. Sent to subscribers whose binding matches,
    /// including the writer: it ignores changes carrying its own
    /// [`ClientId`]. A change the device reports is sent only if the value
    /// differs from the last the service knew, and never as the answer to a
    /// read.
    Changed {
        /// Who changed it ([`ClientId::DEVICE`] if the device reported it).
        writer: ClientId,
        /// The parameter.
        param: ParamId,
        /// The Part (and key) it is in; only what the parameter uses.
        binding: Binding,
        /// The new raw value.
        value: i32,
    },
    /// Many parameters changed (or may have): re-read, don't expect one
    /// [`Message::Changed`] each.
    Bulk {
        /// Who caused it.
        writer: ClientId,
        /// What happened.
        kind: BulkKind,
        /// The Part, for [`BulkKind::Tone`]; empty otherwise.
        binding: Binding,
        /// The event may have changed the Part's tone selection (or, for a
        /// Studio Set event, every Part's): a tone or Studio Set select, or a
        /// mixer load. `false` for a bulk of parameters, which leaves the
        /// selection as it was. Absent (`false`) from a service that predates
        /// it.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        selected: bool,
        /// With `selected` on a [`BulkKind::Tone`] event: the Part's tone
        /// selection now, if the service knows it (it was selected through
        /// the service, or written whole, or has been read). Else absent, and
        /// the service reads the Part's select itself (after a Studio Set
        /// switch, a Program Change on the note port, a partial select write,
        /// a mixer load) and announces it with another such event, with the
        /// tone, or without it again if the read fails. A client therefore
        /// does not ask the device itself after one: its answer could be of
        /// the selection before.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<ToneSelect>,
    },
    /// The answer to a [`Request::Read`].
    Value {
        /// The parameter.
        param: ParamId,
        /// The Part (and key) it was read for.
        binding: Binding,
        /// The raw value the device gave.
        value: i32,
    },
    /// The answer to [`Request::Claim`].
    Claimed {
        /// The plugin kind asked about.
        plugin: PluginKind,
        /// The target now bound to the client, or none if every one is
        /// taken (and nothing changed).
        binding: Option<Binding>,
    },
    /// How many other clients are bound to the same target as this one
    /// ([`Bound::shares_target`]), sent to a client whenever that number
    /// changes: another instance took its Binding or left it, or this client
    /// bound. Never sent as `0` before a first non-zero count; a client that
    /// is unsure it has the latest (it lost events, or bound again) sends
    /// [`Request::Bind`], which has the service tell it again if the count is
    /// not zero. Only clients that said what they are bound to hear it.
    Shared {
        /// The other clients on the same target.
        others: u32,
    },
    /// A SysEx message the device sent, to clients that asked
    /// ([`Request::ReceiveRaw`]).
    Sysex {
        /// The whole message, `F0` to `F7`.
        bytes: Vec<u8>,
    },
    /// The last request of a [`Request::Push`] has been handed to the device's
    /// queue; it leaves at the device's pace. One per push, in the order they
    /// were made.
    Pushed,
    /// The answer to [`Request::Baseline`].
    Baseline {
        /// The Part asked about.
        binding: Binding,
        /// The tone the baseline is of, or `None` if the Part has no
        /// baseline: its tone is a User tone (which can be overwritten), is
        /// not one the device reads back (GM2, ExPCM), was not selected
        /// through the service, changed again while the tone was being read,
        /// or the device did not answer in time.
        tone: Option<ToneSelect>,
        /// The tone's bytes the service read: every Part-wide parameter of
        /// the tone (a drum kit's per-key parameters are not in it), as
        /// consecutive runs in address order. Empty if `tone` is `None`. A
        /// field no run covers has no baseline and is pushed in full.
        runs: Vec<BaselineRun>,
    },
    /// The answer to [`Request::LoadExpansion`]: the load is over and the
    /// queue runs again.
    ExpansionLoadDone {
        /// How it ended.
        result: ExpansionLoadResult,
        /// The slot values the device holds now, slot A first
        /// ([`ExpansionSlots::values`]), or absent if they are unknown
        /// ([`ExpansionLoadResult::TimedOut`]).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slots: Option<[u8; SLOT_COUNT]>,
    },
    /// A request was refused.
    Rejected {
        /// Why.
        reason: Reject,
        /// The parameter of the refused write or read, if it had one, to
        /// tell which request it was.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        param: Option<ParamId>,
    },
    /// A message this peer does not know; skip it.
    #[serde(other)]
    Unknown,
}

/// Why a frame could not be read or written.
#[derive(Debug, Error)]
pub enum FrameError {
    /// The stream failed or ended.
    #[error("socket: {0}")]
    Io(#[from] io::Error),
    /// The frame's length is above [`MAX_FRAME_LEN`].
    #[error("frame of {0} bytes is too long")]
    TooLong(usize),
    /// The frame is not a valid message.
    #[error("malformed frame: {0}")]
    Malformed(#[from] serde_json::Error),
}

/// A frame holding `value`: length prefix and JSON body.
///
/// # Errors
///
/// [`FrameError::TooLong`] if the body is over [`MAX_FRAME_LEN`].
#[doc(hidden)]
pub fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, FrameError> {
    let body = serde_json::to_vec(value)?;
    if body.len() > MAX_FRAME_LEN {
        return Err(FrameError::TooLong(body.len()));
    }
    let length = u32::try_from(body.len()).expect("bounded by MAX_FRAME_LEN");
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&body);
    Ok(frame)
}

/// Read one frame from `reader` and decode it.
///
/// # Errors
///
/// [`FrameError`] if the stream fails or ends (`UnexpectedEof` at a frame
/// boundary is a closed connection), the frame is too long, or it does not
/// decode.
#[doc(hidden)]
pub fn read_frame<T: for<'de> Deserialize<'de>>(reader: &mut impl Read) -> Result<T, FrameError> {
    let mut length = [0u8; 4];
    reader.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_FRAME_LEN {
        return Err(FrameError::TooLong(length));
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body)?)
}

/// Write one frame holding `value` to `writer`.
///
/// # Errors
///
/// [`FrameError`] if the value is too long or the stream fails.
#[doc(hidden)]
pub fn write_frame<T: Serialize>(writer: &mut impl Write, value: &T) -> Result<(), FrameError> {
    writer.write_all(&encode_frame(value)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level() -> ParamId {
        ParamId {
            plugin: PluginKind::StudioSet,
            id: "part03/0x18002009".to_string(),
        }
    }

    fn frame(body: &[u8]) -> Vec<u8> {
        let mut frame = (body.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(body);
        frame
    }

    #[test]
    fn a_frame_round_trips() {
        let request = Request::Write {
            param: level(),
            binding: Binding::of_part(2),
            value: -5,
        };
        let mut buffer = Vec::new();
        write_frame(&mut buffer, &request).unwrap();
        assert_eq!(
            read_frame::<Request>(&mut buffer.as_slice()).unwrap(),
            request
        );
    }

    #[test]
    fn the_documented_hello_decodes() {
        let frame = frame(br#"{"type":"hello","version":1,"subscriptions":[{"part":2}]}"#);
        assert_eq!(
            read_frame::<Request>(&mut frame.as_slice()).unwrap(),
            Request::Hello {
                version: PROTOCOL_VERSION,
                subscriptions: vec![Binding::of_part(2)],
                raw: false,
                bound: None
            }
        );
    }

    #[test]
    fn bind_and_claim_have_a_readable_wire_form() {
        let bind = Request::Bind {
            bound: Some(Bound {
                plugin: PluginKind::PcmSynth,
                binding: Binding::of_part(2),
            }),
        };
        assert_eq!(
            serde_json::to_string(&bind).unwrap(),
            r#"{"type":"bind","bound":{"plugin":"pcm_synth","binding":{"part":2}}}"#
        );
        let claimed = Message::Claimed {
            plugin: PluginKind::StudioSet,
            binding: None,
        };
        assert_eq!(
            serde_json::to_string(&claimed).unwrap(),
            r#"{"type":"claimed","plugin":"studio_set","binding":null}"#
        );
        assert_eq!(
            serde_json::to_string(&Request::Claim {
                plugin: PluginKind::SnDrum
            })
            .unwrap(),
            r#"{"type":"claim","plugin":"sn_drum"}"#
        );
    }

    #[test]
    fn shared_is_plain_json_and_only_instances_on_one_target_share_it() {
        assert_eq!(
            serde_json::to_string(&Message::Shared { others: 2 }).unwrap(),
            r#"{"type":"shared","others":2}"#
        );
        let on = |plugin, binding| Bound { plugin, binding };
        let part = Binding::of_part(3);
        let tone = on(PluginKind::PcmSynth, part);
        assert!(tone.shares_target(&on(PluginKind::PcmSynth, part)));
        assert!(!tone.shares_target(&on(PluginKind::SnSynth, part)));
        assert!(!tone.shares_target(&on(PluginKind::PcmSynth, Binding::of_part(4))));
        // A key is not the whole kit.
        let key = on(PluginKind::PcmDrum, part.with_key(36));
        assert!(!key.shares_target(&on(PluginKind::PcmDrum, part)));
        assert!(key.shares_target(&on(PluginKind::PcmDrum, part.with_key(36))));
        let studio = on(PluginKind::StudioSet, Binding::ANY);
        assert!(studio.shares_target(&studio));
        assert!(!studio.shares_target(&tone));
    }

    #[test]
    fn a_session_is_a_string_on_the_wire() {
        let json = serde_json::to_string(&Message::LinkUp {
            session: SessionId(u64::MAX),
        })
        .unwrap();
        assert_eq!(
            json,
            r#"{"type":"link_up","session":"18446744073709551615"}"#
        );
    }

    #[test]
    fn an_overlong_frame_is_refused_before_it_is_read() {
        let frame = (MAX_FRAME_LEN as u32 + 1).to_be_bytes();
        let error = read_frame::<Request>(&mut frame.as_slice()).unwrap_err();
        assert!(matches!(error, FrameError::TooLong(_)));
    }

    #[test]
    fn a_bad_body_costs_one_frame_and_keeps_the_stream_aligned() {
        let mut bytes = frame(b"{x}");
        bytes.extend(frame(br#"{"type":"load_mixer"}"#));
        let mut reader = bytes.as_slice();
        assert!(matches!(
            read_frame::<Request>(&mut reader),
            Err(FrameError::Malformed(_))
        ));
        assert_eq!(
            read_frame::<Request>(&mut reader).unwrap(),
            Request::LoadMixer
        );
    }

    #[test]
    fn a_push_and_a_baseline_round_trip_and_are_plain_json() {
        let push = Request::Push {
            requests: vec![
                Request::ChangeTone {
                    part: 1,
                    msb: 87,
                    lsb: 64,
                    pc: 0,
                },
                Request::Write {
                    param: level(),
                    binding: Binding::ANY,
                    value: 3,
                },
            ],
        };
        let json = serde_json::to_string(&push).unwrap();
        assert!(json.starts_with(r#"{"type":"push","requests":[{"type":"change_tone""#));
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), push);

        let ask = Request::Baseline {
            binding: Binding::of_part(2),
        };
        let json = serde_json::to_string(&ask).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), ask);

        let answer = Message::Baseline {
            binding: Binding::of_part(2),
            tone: Some(ToneSelect {
                msb: 87,
                lsb: 64,
                pc: 5,
            }),
            runs: vec![BaselineRun {
                address: [0x19, 0x40, 0x00, 0x00],
                bytes: vec![1, 2, 3],
            }],
        };
        let json = serde_json::to_string(&answer).unwrap();
        assert_eq!(serde_json::from_str::<Message>(&json).unwrap(), answer);
        assert_eq!(
            serde_json::to_string(&Message::Pushed).unwrap(),
            r#"{"type":"pushed"}"#
        );
    }

    #[test]
    fn an_expansion_load_and_its_answer_are_plain_json() {
        let load = Request::LoadExpansion {
            slots: [0x03, 0x00, 0x07, 0x08],
        };
        let json = serde_json::to_string(&load).unwrap();
        assert_eq!(json, r#"{"type":"load_expansion","slots":[3,0,7,8]}"#);
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), load);

        let answer = Message::ExpansionLoadDone {
            result: ExpansionLoadResult::Loaded,
            slots: Some([0x03, 0x00, 0x07, 0x08]),
        };
        let json = serde_json::to_string(&answer).unwrap();
        assert_eq!(
            json,
            r#"{"type":"expansion_load_done","result":"loaded","slots":[3,0,7,8]}"#
        );
        assert_eq!(serde_json::from_str::<Message>(&json).unwrap(), answer);
        let timed_out = Message::ExpansionLoadDone {
            result: ExpansionLoadResult::TimedOut,
            slots: None,
        };
        let json = serde_json::to_string(&timed_out).unwrap();
        assert_eq!(
            json,
            r#"{"type":"expansion_load_done","result":"timed_out"}"#
        );

        // Four slots, no more.
        let five = r#"{"type":"load_expansion","slots":[3,0,7,8,9]}"#;
        assert!(serde_json::from_str::<Request>(five).is_err());
    }

    #[test]
    fn a_midi_message_is_raw_if_sysex_else_a_note() {
        assert_eq!(
            Request::midi(vec![0xF0, 0x7E, 0xF7]),
            Request::Raw {
                bytes: vec![0xF0, 0x7E, 0xF7]
            }
        );
        for other in [vec![0x90, 60, 100], vec![0xF8], vec![]] {
            assert_eq!(Request::midi(other.clone()), Request::Note { bytes: other });
        }
    }

    #[test]
    fn a_note_is_plain_json() {
        let note = Request::Note {
            bytes: vec![0x90, 60, 100],
        };
        let json = serde_json::to_string(&note).unwrap();
        assert_eq!(json, r#"{"type":"note","bytes":[144,60,100]}"#);
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), note);
    }

    #[test]
    fn unknown_messages_and_refusals_decode_as_unknown() {
        let message: Message = serde_json::from_str(r#"{"type":"from_the_future","x":1}"#).unwrap();
        assert_eq!(message, Message::Unknown);
        let message: Message =
            serde_json::from_str(r#"{"type":"rejected","reason":{"kind":"new_thing"}}"#).unwrap();
        assert_eq!(
            message,
            Message::Rejected {
                reason: Reject::Unknown,
                param: None
            }
        );
        assert!(serde_json::from_str::<Request>(r#"{"type":"from_the_future"}"#).is_err());
    }
}
