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
use integral_core::host_params::{Binding, ParamHandle, ParamIndex, PluginKind};

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
    /// The device did not answer a [`Request::Read`].
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
    /// A SysEx message the device sent, to clients that asked
    /// ([`Request::ReceiveRaw`]).
    Sysex {
        /// The whole message, `F0` to `F7`.
        bytes: Vec<u8>,
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
                raw: false
            }
        );
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
