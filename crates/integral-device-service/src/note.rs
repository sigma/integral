//! The note port: a virtual MIDI destination DAW tracks send notes to.

use std::sync::Arc;

use integral_core::wireless::message_len;
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

/// The status bytes of channel voice messages: Note Off (`8n`) to Pitch Bend
/// (`En`).
const CHANNEL_VOICE: std::ops::RangeInclusive<u8> = 0x80..=0xEF;

/// The channel voice messages `frame` is made of, in order, or `None` unless
/// it is one or more of them, each whole and with its own status byte.
///
/// This is what a WebSocket client may play: no running status, no SysEx, no
/// system or real-time message, nothing cut short.
pub(crate) fn channel_voice_messages(frame: &[u8]) -> Option<Vec<&[u8]>> {
    let mut messages = Vec::new();
    let mut rest = frame;
    while let Some(&status) = rest.first() {
        if !CHANNEL_VOICE.contains(&status) {
            return None;
        }
        let (message, tail) = rest.split_at_checked(message_len(status)?)?;
        if message[1..].iter().any(|b| *b >= 0x80) {
            return None;
        }
        messages.push(message);
        rest = tail;
    }
    (!messages.is_empty()).then_some(messages)
}

/// Status nibble of a Program Change.
const PROGRAM_CHANGE: u8 = 0xC0;

/// The MIDI channel (0-based) of a Program Change. The device applies it to
/// every Part that receives on the channel, behind anything the service has
/// cached. (A Bank Select only sets the bank the next Program Change uses.)
pub(crate) fn program_change_channel(message: &[u8]) -> Option<u8> {
    let status = *message.first()?;
    (status & 0xF0 == PROGRAM_CHANGE).then_some(status & 0x0F)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_program_change_chooses_a_tone() {
        assert_eq!(program_change_channel(&[0xC3, 5]), Some(3));
        assert_eq!(program_change_channel(&[0xB0, 0, 87]), None, "bank select");
        assert_eq!(program_change_channel(&[0xB2, 7, 90]), None, "volume");
        assert_eq!(program_change_channel(&[0x92, 60, 100]), None, "note");
        assert_eq!(program_change_channel(&[0xF8]), None);
        assert_eq!(program_change_channel(&[]), None);
    }

    #[test]
    fn a_note_frame_is_whole_channel_voice_messages() {
        assert_eq!(
            channel_voice_messages(&[0x90, 60, 100]),
            Some(vec![&[0x90, 60, 100][..]])
        );
        assert_eq!(
            channel_voice_messages(&[0xC3, 5, 0xD0, 0x10, 0xE1, 0, 64]),
            Some(vec![&[0xC3, 5][..], &[0xD0, 0x10], &[0xE1, 0, 64]])
        );
        for bad in [
            &[][..],
            &[0x90, 60],
            &[0x90, 60, 100, 61],
            &[0x90, 60, 0x80],
            &[0x90, 60, 100, 0xF8],
            &[0xF0, 0x41, 0xF7],
            &[0x90, 60, 100, 0xF0, 0xF7],
            &[0xF2, 0, 0],
            &[0xFE],
            &[0x40, 0x20],
        ] {
            assert_eq!(channel_voice_messages(bad), None, "{bad:02X?}");
        }
    }

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
