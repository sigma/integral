//! The note port on CoreMIDI and ALSA: a midir virtual destination.

use midir::os::unix::VirtualInput;
use midir::{Ignore, MidiInput, MidiInputConnection};

use crate::note::{NOTE_PORT_NAME, NotePort, NoteSink, PortError};

/// The "Integral Device Service" virtual destination, made with midir.
#[derive(Default)]
pub struct VirtualNotePort {
    connection: Option<MidiInputConnection<()>>,
}

impl VirtualNotePort {
    /// A port not yet open.
    pub fn new() -> Self {
        Self::default()
    }
}

impl NotePort for VirtualNotePort {
    fn open(&mut self, notes: NoteSink) -> Result<(), PortError> {
        let mut input = MidiInput::new(NOTE_PORT_NAME).map_err(|e| PortError(e.to_string()))?;
        // Nothing is filtered by the driver: the service drops SysEx and
        // passes real-time messages through.
        input.ignore(Ignore::None);
        let connection = input
            .create_virtual(NOTE_PORT_NAME, move |_, msg, ()| notes.message(msg), ())
            .map_err(|e| PortError(e.to_string()))?;
        self.connection = Some(connection);
        Ok(())
    }
}
