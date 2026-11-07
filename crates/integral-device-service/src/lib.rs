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
//! The client protocol (a socket server beside this core) will not be built
//! from closures that call `DeviceState` directly. It needs what the core
//! does not have yet: a writer client ID on every change, per-client queues
//! served round-robin, unchanged-write filtering, and raw DT1s from a client
//! parsed into the cache. Those arrive as data-shaped [`Input`] variants,
//! handled inside [`Service::handle`], with the matching [`Event`]s for
//! subscribers. The socket and the note port are traits like the link, so a
//! Windows port adds implementations and changes none of this.

mod link;
mod midir_link;
mod note;
mod probe;
mod runtime;
mod service;
mod session;
mod wireless_link;

#[cfg(feature = "test-util")]
mod fake;
#[cfg(unix)]
mod virtual_port;

#[cfg(feature = "test-util")]
pub use fake::FakeLink;
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
pub use virtual_port::VirtualNotePort;
pub use wireless_link::WirelessLink;
