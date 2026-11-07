//! The device link: the seam between the service and whatever carries MIDI to
//! the INTEGRA-7 (a Wireless session, an RTP-MIDI port or a USB port).

use std::sync::Arc;

use thiserror::Error;

/// Where a link writes its progress messages (reconnects, failures).
pub type Log = Arc<dyn Fn(&str) + Send + Sync>;

/// A [`Log`] that writes lines to standard error.
pub fn stderr_log() -> Log {
    Arc::new(|line| eprintln!("{line}"))
}

/// A link failure.
#[derive(Debug, Error)]
pub enum LinkError {
    /// The link is not up, so nothing can be sent.
    #[error("the link is down")]
    Down,
    /// The MIDI system or the network refused the operation.
    #[error("{0}")]
    Io(String),
}

/// What a link reports to the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkEvent {
    /// The link came up: the device can be reached. `device_id` is the ID
    /// the device gave in an Identity Reply, if the link learnt it.
    Up {
        /// The device ID from the Identity Reply, if the link saw one.
        device_id: Option<u8>,
    },
    /// The link went down: a drop, or the device left.
    Down,
    /// One MIDI message the device sent (a whole SysEx message, or one
    /// channel message).
    Midi(Vec<u8>),
}

/// Where a link reports [`LinkEvent`]s. Cheap to clone and callable from any
/// thread; the link uses it from its own threads.
#[derive(Clone)]
pub struct LinkSink(Arc<dyn Fn(LinkEvent) + Send + Sync>);

impl LinkSink {
    /// A sink that hands every event to `deliver`.
    pub fn new(deliver: impl Fn(LinkEvent) + Send + Sync + 'static) -> Self {
        Self(Arc::new(deliver))
    }

    /// Report that the link came up, with the device ID if it is known.
    pub fn up(&self, device_id: Option<u8>) {
        (self.0)(LinkEvent::Up { device_id });
    }

    /// Report that the link went down.
    pub fn down(&self) {
        (self.0)(LinkEvent::Down);
    }

    /// Report one MIDI message from the device.
    pub fn midi(&self, message: &[u8]) {
        (self.0)(LinkEvent::Midi(message.to_vec()));
    }
}

/// A link to the device.
///
/// The service owns exactly one, as a trait object. One [`send`](Self::send)
/// sends one whole MIDI message, so the service orders notes and SysEx at
/// message granularity; a link must never interleave anything inside a
/// message. A link may send its own liveness probes (an Identity Request)
/// between messages.
pub trait DeviceLink: Send {
    /// Start bringing the link up and keep it up, reconnecting as needed.
    /// Report through `events`: [`LinkSink::up`] once the device is known to
    /// answer, [`LinkSink::down`] when it cannot, and every message the
    /// device sends. Called once, before the first [`send`](Self::send).
    ///
    /// # Errors
    ///
    /// Fails if the link cannot even try (the MIDI system is unavailable).
    fn start(&mut self, events: LinkSink) -> Result<(), LinkError>;

    /// How long the service should hold SysEx back as of now, because the
    /// link itself just sent some (a liveness probe). The service must not
    /// block on it: it sends notes meanwhile and wakes when the gap ends.
    fn sysex_gap(&self) -> std::time::Duration {
        std::time::Duration::ZERO
    }

    /// Send one whole MIDI message to the device.
    ///
    /// # Errors
    ///
    /// [`LinkError::Down`] while the link is down, [`LinkError::Io`] if the
    /// transport refuses the message.
    fn send(&mut self, message: &[u8]) -> Result<(), LinkError>;
}
