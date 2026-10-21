//! The plugin's device link: the seam behind the plugin transport.
//!
//! The page's port interface reaches the device through a [`DeviceLink`].
//! Today that is [`MidirLink`], which opens the device's MIDI port pair
//! itself, because Live drops plugin SysEx (#328). ADR 0006 moves the link
//! into an out-of-process Device service; a client of that service
//! implements [`DeviceLink`] and replaces [`MidirLink`] without the bridge or
//! the page changing.

use thiserror::Error;

/// Where a link hands what the device sends: called on the link's own
/// thread, once per MIDI message.
pub(crate) type Inbound = Box<dyn FnMut(&[u8]) + Send + 'static>;

/// A link failure.
#[derive(Debug, Error)]
pub(crate) enum LinkError {
    /// The MIDI system is unavailable.
    #[error("MIDI unavailable: {0}")]
    Unavailable(String),
    /// No pair by that name exists.
    #[error("no MIDI port pair named {0:?}")]
    NoSuchPort(String),
    /// The pair exists but would not connect.
    #[error("could not connect to {port:?}: {reason}")]
    Connect {
        /// The pair's name.
        port: String,
        /// The MIDI system's reason.
        reason: String,
    },
    /// Nothing is open to send to.
    #[error("no MIDI port pair is open")]
    NotOpen,
    /// The MIDI system refused a message.
    #[error("send failed: {0}")]
    Send(String),
}

/// A link to the device: lists port pairs, opens one, sends to it, and
/// hands what it receives to an [`Inbound`].
pub(crate) trait DeviceLink {
    /// The pairs (an input and an output of one name) that can be opened.
    fn ports(&mut self) -> Result<Vec<String>, LinkError>;
    /// Open the pair `port`, closing any other. Received messages go to
    /// `inbound` until the next `open` or `close`.
    fn open(&mut self, port: &str, inbound: Inbound) -> Result<(), LinkError>;
    /// Send one MIDI message to the open pair.
    fn send(&mut self, data: &[u8]) -> Result<(), LinkError>;
    /// Close the open pair, if any.
    fn close(&mut self);
}

/// The MIDI client name the link shows the system.
const CLIENT: &str = "Integral";

/// A [`DeviceLink`] that opens the device's MIDI ports directly with midir.
#[derive(Default)]
pub(crate) struct MidirLink {
    /// Clients kept for listing ports, made on first use.
    scan: Option<(midir::MidiInput, midir::MidiOutput)>,
    input: Option<midir::MidiInputConnection<()>>,
    output: Option<midir::MidiOutputConnection>,
}

/// Names in both `inputs` and `outputs`, in `inputs` order, once each.
fn paired(inputs: Vec<String>, outputs: &[String]) -> Vec<String> {
    let mut pairs: Vec<String> = Vec::new();
    for name in inputs {
        if outputs.contains(&name) && !pairs.contains(&name) {
            pairs.push(name);
        }
    }
    pairs
}

fn unavailable(e: impl std::fmt::Display) -> LinkError {
    LinkError::Unavailable(e.to_string())
}

impl DeviceLink for MidirLink {
    fn ports(&mut self) -> Result<Vec<String>, LinkError> {
        if self.scan.is_none() {
            self.scan = Some((
                midir::MidiInput::new(CLIENT).map_err(unavailable)?,
                midir::MidiOutput::new(CLIENT).map_err(unavailable)?,
            ));
        }
        let (input, output) = self.scan.as_ref().expect("made above");
        let inputs = input
            .ports()
            .iter()
            .filter_map(|p| input.port_name(p).ok())
            .collect();
        let outputs: Vec<String> = output
            .ports()
            .iter()
            .filter_map(|p| output.port_name(p).ok())
            .collect();
        Ok(paired(inputs, &outputs))
    }

    fn open(&mut self, port: &str, mut inbound: Inbound) -> Result<(), LinkError> {
        self.close();
        let mut input = midir::MidiInput::new(CLIENT).map_err(unavailable)?;
        // Receive SysEx, timing and active sensing: the page filters.
        input.ignore(midir::Ignore::None);
        let output = midir::MidiOutput::new(CLIENT).map_err(unavailable)?;
        let in_port = input
            .ports()
            .into_iter()
            .find(|p| input.port_name(p).is_ok_and(|n| n == port))
            .ok_or_else(|| LinkError::NoSuchPort(port.into()))?;
        let out_port = output
            .ports()
            .into_iter()
            .find(|p| output.port_name(p).is_ok_and(|n| n == port))
            .ok_or_else(|| LinkError::NoSuchPort(port.into()))?;
        let connect = |reason: String| LinkError::Connect {
            port: port.into(),
            reason,
        };
        let output = output
            .connect(&out_port, "Integral out")
            .map_err(|e| connect(e.to_string()))?;
        let input = input
            .connect(&in_port, "Integral in", move |_, msg, _| inbound(msg), ())
            .map_err(|e| connect(e.to_string()))?;
        self.output = Some(output);
        self.input = Some(input);
        Ok(())
    }

    fn send(&mut self, data: &[u8]) -> Result<(), LinkError> {
        let output = self.output.as_mut().ok_or(LinkError::NotOpen)?;
        output
            .send(data)
            .map_err(|e| LinkError::Send(e.to_string()))
    }

    fn close(&mut self) {
        if let Some(input) = self.input.take() {
            input.close();
        }
        if let Some(output) = self.output.take() {
            output.close();
        }
    }
}

impl Drop for MidirLink {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn pairs_are_names_with_both_halves() {
        assert_eq!(
            paired(
                names(&["INTEGRA-7", "Keyboard", "Network Integra", "INTEGRA-7"]),
                &names(&["Network Integra", "INTEGRA-7", "Synth"]),
            ),
            names(&["INTEGRA-7", "Network Integra"]),
        );
    }
}
