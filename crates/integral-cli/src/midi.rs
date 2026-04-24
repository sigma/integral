//! MIDI connection helpers for the INTEGRA-7.

use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use integral_core::address::{Address, DataSize};
use integral_core::sysex;
use midir::{MidiInput, MidiOutput};

/// Default MIDI port name pattern for the INTEGRA-7.
pub const DEFAULT_PORT_PATTERN: &str = "Integra";

/// Default SysEx device ID (broadcast for the INTEGRA-7).
pub const DEFAULT_DEVICE_ID: u8 = 0x10;

/// Find a MIDI port whose name contains `pattern` (case-insensitive).
pub fn find_port_by_name<T: midir::MidiIO>(midi_io: &T, pattern: &str) -> Option<T::Port> {
    let pattern_lower = pattern.to_lowercase();
    midi_io.ports().into_iter().find(|port| {
        midi_io
            .port_name(port)
            .is_ok_and(|name| name.to_lowercase().contains(&pattern_lower))
    })
}

/// Open MIDI input+output connections and return (conn_in_guard, conn_out, rx).
pub fn open_midi(
    port_pattern: &str,
) -> Result<(
    midir::MidiInputConnection<()>,
    midir::MidiOutputConnection,
    mpsc::Receiver<Vec<u8>>,
)> {
    let midi_out = MidiOutput::new("integral-out").context("failed to create MIDI output")?;
    let midi_in = MidiInput::new("integral-in").context("failed to create MIDI input")?;

    let out_port = find_port_by_name(&midi_out, port_pattern)
        .with_context(|| format!("no MIDI output port matching '{port_pattern}'"))?;
    let in_port = find_port_by_name(&midi_in, port_pattern)
        .with_context(|| format!("no MIDI input port matching '{port_pattern}'"))?;

    let port_name = midi_out
        .port_name(&out_port)
        .unwrap_or_else(|_| port_pattern.to_string());
    eprintln!("Using MIDI port: {port_name}");

    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let conn_in = midi_in
        .connect(
            &in_port,
            "integral-in",
            move |_timestamp, message, _| {
                let _ = tx.send(message.to_vec());
            },
            (),
        )
        .map_err(|e| anyhow::anyhow!("failed to connect MIDI input: {e}"))?;

    let conn_out = midi_out
        .connect(&out_port, "integral-out")
        .map_err(|e| anyhow::anyhow!("failed to connect MIDI output: {e}"))?;

    Ok((conn_in, conn_out, rx))
}

/// Send an RQ1 and wait for the DT1 response.
pub fn request_data(
    conn_out: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
    device_id: u8,
    address: &Address,
    size: &DataSize,
    timeout: Duration,
) -> Result<Vec<u8>> {
    let rq1 = sysex::build_rq1(device_id, address, size);
    eprintln!("Sending RQ1: addr={}, size={}", address, size);
    eprintln!(
        "  Raw: {}",
        rq1.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    );
    conn_out.send(&rq1).context("failed to send RQ1")?;

    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            bail!("RQ1 timeout waiting for response at address {address}");
        }
        match rx.recv_timeout(remaining) {
            Ok(data) => {
                // Print every incoming message for debugging
                eprintln!(
                    "  Received ({} bytes): {}",
                    data.len(),
                    data.iter()
                        .take(20)
                        .map(|b| format!("{:02X}", b))
                        .collect::<Vec<_>>()
                        .join(" ")
                );

                match sysex::parse_dt1(&data) {
                    Ok(dt1) => {
                        eprintln!(
                            "  Parsed DT1: addr={}, {} data bytes",
                            dt1.address,
                            dt1.data.len()
                        );
                        if dt1.address == *address {
                            return Ok(dt1.data);
                        }
                        eprintln!("  (address mismatch, continuing...)");
                    }
                    Err(e) => {
                        eprintln!("  (not a DT1: {e})");
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                bail!("RQ1 timeout waiting for response at address {address}");
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!("MIDI input disconnected");
            }
        }
    }
}

/// Detect the device ID by sending an Identity Request and parsing the reply.
///
/// Returns the device ID from the first valid Identity Reply, or the default
/// if no reply is received within the timeout.
pub fn detect_device_id(
    conn_out: &mut midir::MidiOutputConnection,
    rx: &mpsc::Receiver<Vec<u8>>,
    timeout: Duration,
) -> Result<u8> {
    conn_out
        .send(&sysex::identity_request())
        .context("failed to send identity request")?;

    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            eprintln!(
                "Warning: no identity reply received, using default device ID 0x{:02X}",
                DEFAULT_DEVICE_ID
            );
            return Ok(DEFAULT_DEVICE_ID);
        }
        match rx.recv_timeout(remaining) {
            Ok(data) => {
                if let Ok(identity) = sysex::parse_identity_reply(&data) {
                    eprintln!("Detected device ID: 0x{:02X}", identity.device_id);
                    return Ok(identity.device_id);
                }
                // Not an identity reply, keep waiting.
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                eprintln!(
                    "Warning: no identity reply received, using default device ID 0x{:02X}",
                    DEFAULT_DEVICE_ID
                );
                return Ok(DEFAULT_DEVICE_ID);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!("MIDI input disconnected");
            }
        }
    }
}
