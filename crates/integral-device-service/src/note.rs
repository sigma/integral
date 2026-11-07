//! The note port: a virtual MIDI destination DAW tracks send notes to.

use std::sync::Arc;

use thiserror::Error;

/// Name of the virtual MIDI destination the service exposes.
pub const NOTE_PORT_NAME: &str = "Integral Device Service";

/// A note port failure.
#[derive(Debug, Error)]
#[error("note port: {0}")]
pub struct PortError(pub String);

/// Where a note port hands what it receives. Cheap to clone and callable from
/// any thread.
#[derive(Clone)]
pub struct NoteSink(Arc<NoteFn>);

type NoteFn = dyn Fn(&[u8]) + Send + Sync;

impl NoteSink {
    /// A sink that hands every message to `deliver`.
    pub fn new(deliver: impl Fn(&[u8]) + Send + Sync + 'static) -> Self {
        Self(Arc::new(deliver))
    }

    /// Hand over one MIDI message received on the port.
    pub fn message(&self, message: &[u8]) {
        (self.0)(message);
    }
}

/// The virtual destination notes arrive on. A trait so another platform's
/// MIDI system (Windows MIDI Services) is an additional implementation.
pub trait NotePort: Send {
    /// Create the port and keep it for as long as `self` lives, handing every
    /// received message to `notes`.
    ///
    /// # Errors
    ///
    /// Fails if the MIDI system cannot create a virtual destination.
    fn open(&mut self, notes: NoteSink) -> Result<(), PortError>;
}

/// Whether a message from the note port may go to the device.
///
/// Everything passes except SysEx (`F0`..`F7`), which is dropped, never
/// echoed: a monitored Live track feeds the device's own replies back to this
/// port. Real-time messages (clock, start, stop, active sensing) pass, and so
/// does every channel message. Anything that does not start with a status
/// byte is malformed and dropped too.
pub(crate) fn is_note_traffic(message: &[u8]) -> bool {
    message
        .first()
        .is_some_and(|status| *status >= 0x80 && !matches!(status, 0xF0 | 0xF7))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysex_is_dropped_and_everything_else_passes() {
        assert!(is_note_traffic(&[0x90, 60, 100]));
        assert!(is_note_traffic(&[0xB3, 7, 90]));
        assert!(is_note_traffic(&[0xE0, 0, 64]));
        for realtime in [0xF8, 0xFA, 0xFB, 0xFC, 0xFE] {
            assert!(is_note_traffic(&[realtime]), "{realtime:02X}");
        }
        assert!(!is_note_traffic(&[0xF0, 0x41, 0x10, 0xF7]));
        assert!(!is_note_traffic(&[0xF7]));
        assert!(!is_note_traffic(&[0x40, 0x20]));
        assert!(!is_note_traffic(&[]));
    }
}
