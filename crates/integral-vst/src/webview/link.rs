//! The plugin's device link: the seam behind the plugin transport.
//!
//! The page's port interface reaches the device through a [`DeviceLink`].
//! The plugin opens no MIDI port (Live drops plugin SysEx, #328, and one
//! process must own the device's port): the link is a raw SysEx client of the
//! Device service ([`service_link`](super::service_link), ADR 0006).

use thiserror::Error;

/// What a link tells the page, from the link's own thread.
pub(crate) enum Incoming<'a> {
    /// The port is open: sends will be carried. Once per
    /// [`open`](DeviceLink::open) that returned `Ok`, unless
    /// [`OpenFailed`](Self::OpenFailed) comes instead. It may come before
    /// `open` returns, or later when the link has to connect first.
    Opened,
    /// The port could not be opened after all, and why.
    OpenFailed(String),
    /// One message from the device.
    Receive(&'a [u8]),
    /// A request the link carried was refused, and why.
    Error(String),
}

/// Where a link hands what it has to tell the page.
pub(crate) type Inbound = Box<dyn FnMut(Incoming<'_>) + Send + 'static>;

/// A link failure.
#[derive(Debug, Error)]
pub(crate) enum LinkError {
    /// The link is unavailable, and why.
    #[error("{0}")]
    Unavailable(String),
    /// No pair by that name exists.
    #[error("no port named {0:?}")]
    NoSuchPort(String),
    /// The pair exists but would not connect.
    #[error("could not connect to {port:?}: {reason}")]
    Connect {
        /// The pair's name.
        port: String,
        /// The reason.
        reason: String,
    },
    /// Nothing is open to send to.
    #[error("no port is open")]
    NotOpen,
    /// The link refused a message.
    #[error("send failed: {0}")]
    Send(String),
}

/// A link to the device: lists port pairs, opens one, sends to it, and
/// hands what it receives to an [`Inbound`].
pub(crate) trait DeviceLink {
    /// The pairs (an input and an output of one name) that can be opened.
    fn ports(&mut self) -> Result<Vec<String>, LinkError>;
    /// Open the pair `port`, closing any other. `Ok` says the open has begun:
    /// `inbound` hears [`Incoming::Opened`] or [`Incoming::OpenFailed`], and
    /// then what the device sends, until the next `open` or `close`.
    fn open(&mut self, port: &str, inbound: Inbound) -> Result<(), LinkError>;
    /// Send one MIDI message to the open pair.
    fn send(&mut self, data: &[u8]) -> Result<(), LinkError>;
    /// Close the open pair, if any.
    fn close(&mut self);
}
