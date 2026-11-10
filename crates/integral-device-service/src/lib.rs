//! The Device service: one process that owns the link to the INTEGRA-7 and
//! the one [`DeviceState`](integral_core::device::DeviceState).
//!
//! See ADR 0006 (`docs/adr/0006-device-service-owns-the-link.md`).
//!
//! - The link is a [`DeviceLink`] trait object given to [`Service::new`]: a
//!   [`WirelessLink`], or a [`MidirLink`] over an RTP-MIDI or USB port. A
//!   `MidirLink` reports up only when the device answers an Identity Request,
//!   and takes the SysEx device ID from the reply. Tests use a `FakeLink`
//!   (feature `test-util`).
//! - The note port is a [`NotePort`], by default the virtual destination
//!   [`NOTE_PORT_NAME`]. Notes reach the link at once, between SysEx messages
//!   and never inside one; SysEx arriving on it is dropped.
//! - A new [`SessionId`] begins each time the link comes up. A second
//!   [`Event::LinkUp`] with no [`Event::LinkDown`] between (the device ID
//!   changed, so the state was replaced) means the same as a first: the
//!   session is new, so re-read everything.
//!
//! Nothing here reads the environment or the working directory: every input
//! is a parameter.
//!
//! # The seam for the client protocol
//!
//! The service thread owns the state. Today another thread reaches it through
//! [`Handle::run`], which runs a closure there, and observes it through a
//! [`Listener`], which runs on the service thread and must not block.
//!
//! The client protocol is not built from closures that call `DeviceState`
//! directly. A client's request is data: an [`Input::Client`] carrying the
//! [`ClientId`](protocol::ClientId) and a [`Request`](protocol::Request),
//! checked when it arrives and queued in its client's own queue.
//! [`Service::pump`] moves queued items into the device's queue whenever that
//! is empty, one client's turn after another's, so the client queues are the
//! real ones: the writer's ID is known where a change is queued for sending
//! and announced, and that is where the cache rules apply (a write the
//! session knows the device already has is dropped, except in a push). A
//! session is one span of one link; when it ends, everything queued and
//! everything known is dropped. Subscribers hear changes as
//! [`Message`](protocol::Message)s. A raw SysEx client's DT1 goes the same
//! way as a typed write; the device's SysEx goes to every raw client.
//!
//! After a Preset or Expansion tone is selected the service reads the tone
//! back and keeps it as the Part's [`ToneBaseline`], which a client asks for
//! with [`Request::Baseline`](protocol::Request::Baseline). A Program Change
//! on the note port invalidates the tone of the Parts it reaches and has them
//! read again.
//!
//! The wire is JSON frames described in [`protocol`], over a Unix socket
//! ([`SocketServer`] for the service, [`Client`] for a plugin instance). The
//! socket and the note port are separate from the core, like the link, so a
//! Windows port adds implementations and changes none of this.

mod baseline;
mod clients;
mod link;
mod midir_link;
mod note;
mod probe;
pub mod protocol;
mod queues;
mod runtime;
mod rx;
mod service;
mod session;
mod tcp;
mod websocket;
mod wireless_link;

#[cfg(unix)]
mod client;
#[cfg(unix)]
mod sock;
#[cfg(unix)]
mod socket;

#[cfg(feature = "test-util")]
mod fake;
#[cfg(unix)]
mod virtual_port;

pub use baseline::ToneBaseline;
#[cfg(unix)]
pub use client::{Client, ClientEvent, Events, Sender};
#[doc(hidden)]
pub use clients::Outbox;
#[cfg(feature = "test-util")]
pub use fake::{FakeLink, FakeMemory, is_rq1};
pub use link::{DeviceLink, LinkError, LinkEvent, LinkSink, Log, stderr_log};
pub use midir_link::{MidirLink, PortMatch, matching_pairs, port_named};
pub use note::{NOTE_PORT_NAME, NotePort, NoteSink, PortError};
pub use probe::Timing as ProbeTiming;
pub use runtime::{Handle, Running, StartError, spawn};
pub use service::{Config, Event, Input, Listener, Service};
#[cfg(feature = "test-util")]
pub use session::SequentialSessionIds;
pub use session::{RandomSessionIds, SessionId, SessionIdSource};
#[cfg(unix)]
pub use socket::{
    SOCKET_FILE_NAME, SocketClaim, SocketServer, check_default_path_length, default_socket_dir,
    default_socket_path, ensure_private_dir, socket_dir_in, socket_path_in, user_temp_dir,
};
#[cfg(unix)]
pub use virtual_port::VirtualNotePort;
pub use websocket::{MAX_CLIENTS, WebSocketServer};
pub use wireless_link::WirelessLink;
