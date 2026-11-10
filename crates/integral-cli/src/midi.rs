//! MIDI connection helpers for the INTEGRA-7.

use std::ops::Range;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use integral_core::address::{Address, DataSize};
use integral_core::librarian::reader::{BlockData, BlockReader};
use integral_core::sysex;
use integral_device_service::{PortMatch, matching_pairs, port_named};
use midir::{MidiInput, MidiOutput};

/// Default MIDI port name pattern for the INTEGRA-7.
pub const DEFAULT_PORT_PATTERN: &str = "Integra";

/// Default SysEx device ID (broadcast for the INTEGRA-7).
pub use integral_core::sysex::DEFAULT_DEVICE_ID;

/// How long to wait for an Identity Reply when choosing between ports.
const PORT_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// Where a command reaches the device: a MIDI port pair opened with
/// `midir`, or the Device service's socket (`--service`).
#[derive(Clone, Copy, Debug)]
pub enum Endpoint<'a> {
    /// The port pair whose name contains this pattern.
    Port(&'a str),
    /// The Device service listening on this Unix socket, or on the default
    /// one when `None`.
    Service(Option<&'a Path>),
}

impl<'a> Endpoint<'a> {
    /// The service if `--service` was given (with its optional socket),
    /// else the port matching `port`.
    pub fn select(service: Option<Option<&'a Path>>, port: &'a str) -> Self {
        service.map_or(Self::Port(port), Self::Service)
    }
}

/// The `--timeout`: `explicit` if given, else `default_secs`, plus
/// [`SERVICE_EXTRA_TIMEOUT`] through the service, whose queue paces and may
/// hold other clients' messages before ours. Under `--service` a timeout runs
/// from the CLI's send, so it covers that wait too.
pub fn timeout(explicit: Option<f64>, default_secs: f64, service: bool) -> Duration {
    explicit.map_or_else(
        || {
            let default = Duration::from_secs_f64(default_secs);
            if service {
                default + SERVICE_EXTRA_TIMEOUT
            } else {
                default
            }
        },
        Duration::from_secs_f64,
    )
}

/// What the service adds to a default `--timeout`.
pub const SERVICE_EXTRA_TIMEOUT: Duration = Duration::from_secs(3);

/// The sending half of a connection: SysEx goes to the device through a
/// MIDI port or through the Device service. It owns what keeps the
/// connection alive, so dropping it closes the connection.
pub enum Output {
    /// A `midir` output port, with the input connection feeding the
    /// receiver.
    Midi {
        /// The output.
        out: midir::MidiOutputConnection,
        /// Keeps the input callback running.
        _input: midir::MidiInputConnection<()>,
    },
    /// A client of the Device service, which paces and forwards raw SysEx.
    #[cfg(unix)]
    Service(crate::service::ServiceOutput),
}

impl Output {
    /// Send one whole MIDI message. Through the service only SysEx is
    /// accepted: it owns the note path.
    pub fn send(&mut self, message: &[u8]) -> Result<()> {
        match self {
            Self::Midi { out, .. } => out.send(message).map_err(Into::into),
            #[cfg(unix)]
            Self::Service(service) => service.send(message),
        }
    }

    /// The Device service connection, if that is the transport.
    #[cfg(unix)]
    pub fn service(&mut self) -> Option<&mut crate::service::ServiceOutput> {
        match self {
            Self::Service(service) => Some(service),
            Self::Midi { .. } => None,
        }
    }
}

/// An open connection to the device.
pub struct Connection {
    /// Sends to the device; dropped first.
    pub output: Output,
    /// The messages from the device. Disconnected when the connection is
    /// lost.
    pub rx: mpsc::Receiver<Vec<u8>>,
}

/// Open the connection `endpoint` names.
pub fn open_midi(endpoint: Endpoint<'_>) -> Result<Connection> {
    match endpoint {
        Endpoint::Port(pattern) => open_port(pattern),
        #[cfg(unix)]
        Endpoint::Service(path) => crate::service::connect(path),
        #[cfg(not(unix))]
        Endpoint::Service(_) => bail!("--service needs a Unix socket: not supported here"),
    }
}

/// Open MIDI input+output connections on the ports matching `port_pattern`.
///
/// When several ports match `port_pattern` (e.g. a dead network session
/// next to the wireless Bridge), use the first that answers an Identity
/// Request, falling back to the first match.
fn open_port(port_pattern: &str) -> Result<Connection> {
    // Only a name that is both an input and an output is a device; the
    // Device service's note port ("Integral Device Service", an input only)
    // contains "Integra" but never matches.
    let names = matching_pairs(&PortMatch::Contains(port_pattern.to_string()))
        .map_err(|e| anyhow::anyhow!("failed to list MIDI ports: {e}"))?;
    if names.len() > 1 {
        for name in &names {
            let Ok((conn_in, mut conn_out, rx)) = connect(name) else {
                continue;
            };
            conn_out
                .send(&sysex::identity_request())
                .context("failed to send identity request")?;
            if await_identity(&rx, PORT_PROBE_TIMEOUT)?.is_some() {
                eprintln!("Using MIDI port: {name}");
                return Ok(midi_connection(conn_in, conn_out, rx));
            }
        }
        eprintln!("No port matching '{port_pattern}' answered; trying the first.");
    }
    let name = names
        .first()
        .with_context(|| format!("no MIDI port pair matching '{port_pattern}'"))?;
    let (conn_in, conn_out, rx) = connect(name)?;
    eprintln!("Using MIDI port: {name}");
    Ok(midi_connection(conn_in, conn_out, rx))
}

/// The [`Connection`] over a port pair.
fn midi_connection(
    input: midir::MidiInputConnection<()>,
    out: midir::MidiOutputConnection,
    rx: mpsc::Receiver<Vec<u8>>,
) -> Connection {
    Connection {
        output: Output::Midi { out, _input: input },
        rx,
    }
}

/// Connect to the output port named `name` and the input port matching it.
fn connect(
    name: &str,
) -> Result<(
    midir::MidiInputConnection<()>,
    midir::MidiOutputConnection,
    mpsc::Receiver<Vec<u8>>,
)> {
    let midi_out = MidiOutput::new("integral-out").context("failed to create MIDI output")?;
    let midi_in = MidiInput::new("integral-in").context("failed to create MIDI input")?;

    let out_port = port_named(&midi_out, name)
        .with_context(|| format!("no MIDI output port named '{name}'"))?;
    let in_port =
        port_named(&midi_in, name).with_context(|| format!("no MIDI input port named '{name}'"))?;

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

/// Read `size` bytes at `address`: send one RQ1 and collect every DT1
/// packet of the reply, placed by address, until the block is whole or
/// `timeout` elapses. Logs the traffic to stderr when `verbose`.
pub fn read_block(
    conn_out: &mut Output,
    rx: &mpsc::Receiver<Vec<u8>>,
    device_id: u8,
    address: &Address,
    size: &DataSize,
    timeout: Duration,
    verbose: bool,
) -> Result<BlockData> {
    let mut reader = BlockReader::new(device_id, [(*address, *size)]);
    let rq1 = reader.next_request().context("nothing to read")?;
    if verbose {
        eprintln!(
            "Sending RQ1: addr={address}, size={size} ({} bytes)",
            size.value()
        );
    }
    conn_out.send(&rq1).context("failed to send RQ1")?;
    collect_reply(reader, rx, timeout, verbose)
}

/// Like [`read_block`], but fails unless every byte of the block arrived.
pub fn request_data(
    conn_out: &mut Output,
    rx: &mpsc::Receiver<Vec<u8>>,
    device_id: u8,
    address: &Address,
    size: &DataSize,
    timeout: Duration,
    verbose: bool,
) -> Result<Vec<u8>> {
    let data = read_block(conn_out, rx, device_id, address, size, timeout, verbose)?;
    complete(address, data)
}

/// Feed the DT1 packets from `rx` to `reader`, whose one RQ1 has been
/// sent, until its block is whole or `timeout` elapses.
fn collect_reply(
    mut reader: BlockReader,
    rx: &mpsc::Receiver<Vec<u8>>,
    timeout: Duration,
    verbose: bool,
) -> Result<BlockData> {
    let deadline = Instant::now() + timeout;
    while reader.is_waiting() {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(msg) => {
                if let Ok(dt1) = sysex::parse_dt1(&msg)
                    && reader.on_dt1(&dt1.address, &dt1.data)
                    && verbose
                {
                    eprintln!("  DT1 at {} ({} bytes)", dt1.address, dt1.data.len());
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => reader.on_timeout(),
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("MIDI input disconnected"),
        }
    }
    let read = reader.result().pop().context("nothing was read")?;
    Ok(read.data)
}

/// The bytes of a block read at `address`, or an error naming what is
/// missing.
fn complete(address: &Address, data: BlockData) -> Result<Vec<u8>> {
    match data {
        BlockData::Complete(data) => Ok(data),
        BlockData::Gap { missing, .. } => bail!(
            "incomplete reply at {address}: byte ranges {} missing",
            format_ranges(&missing)
        ),
        BlockData::Timeout => bail!("RQ1 timeout waiting for response at address {address}"),
    }
}

/// Format byte ranges as `start..end`, comma-separated.
pub fn format_ranges(ranges: &[Range<usize>]) -> String {
    let ranges: Vec<String> = ranges
        .iter()
        .map(|r| format!("{}..{}", r.start, r.end))
        .collect();
    ranges.join(", ")
}

/// Detect the device ID by sending an Identity Request and parsing the reply.
///
/// Returns the device ID from the first valid Identity Reply, or the default
/// if no reply is received within the timeout.
pub fn detect_device_id(
    conn_out: &mut Output,
    rx: &mpsc::Receiver<Vec<u8>>,
    timeout: Duration,
) -> Result<u8> {
    conn_out
        .send(&sysex::identity_request())
        .context("failed to send identity request")?;
    match await_identity(rx, timeout)? {
        Some(identity) => {
            eprintln!("Detected device ID: 0x{:02X}", identity.device_id);
            Ok(identity.device_id)
        }
        None => {
            eprintln!(
                "Warning: no identity reply received, using default device ID 0x{:02X}",
                DEFAULT_DEVICE_ID
            );
            Ok(DEFAULT_DEVICE_ID)
        }
    }
}

/// Wait up to `timeout` for an Identity Reply on `rx`, skipping other
/// messages. `None` when none arrives.
fn await_identity(
    rx: &mpsc::Receiver<Vec<u8>>,
    timeout: Duration,
) -> Result<Option<sysex::DeviceIdentity>> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(data) => {
                if let Ok(identity) = sysex::parse_identity_reply(&data) {
                    return Ok(Some(identity));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => return Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => bail!("MIDI input disconnected"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)] // `missing` is a list of ranges
mod tests {
    use super::*;

    const MFX: Address = Address::new(0x50, 0x00, 0x02, 0x00);
    const TIMEOUT: Duration = Duration::from_millis(20);

    /// A reader for `size` bytes at [`MFX`] whose RQ1 has been sent, and a
    /// receiver holding `replies` (address offset, data) as raw DT1s, and
    /// its sender, which keeps the receiver connected.
    fn scripted(
        size: u32,
        replies: &[(u8, &[u8])],
    ) -> (BlockReader, mpsc::Receiver<Vec<u8>>, mpsc::Sender<Vec<u8>>) {
        let mut reader = BlockReader::new(0x10, [(MFX, DataSize::from_value(size).unwrap())]);
        reader.next_request().unwrap();
        let (tx, rx) = mpsc::channel();
        tx.send(sysex::identity_request().to_vec()).unwrap();
        for &(offset, data) in replies {
            let addr = MFX.offset([0, 0, 0, offset]);
            tx.send(sysex::build_dt1(0x10, &addr, data)).unwrap();
        }
        (reader, rx, tx)
    }

    #[test]
    fn collects_every_packet_of_a_split_reply() {
        let (reader, rx, _tx) = scripted(6, &[(4, &[5, 6]), (0, &[1, 2, 3, 4])]);
        let data = collect_reply(reader, &rx, TIMEOUT, false).unwrap();
        assert_eq!(complete(&MFX, data).unwrap(), [1, 2, 3, 4, 5, 6]);
    }

    /// The device stops a reply at the last whole parameter.
    #[test]
    fn short_reply_is_an_error() {
        let (reader, rx, _tx) = scripted(6, &[(0, &[1, 2, 3, 4])]);
        let data = collect_reply(reader, &rx, TIMEOUT, false).unwrap();
        assert_eq!(
            data,
            BlockData::Gap {
                data: vec![1, 2, 3, 4, 0, 0],
                missing: vec![4..6],
            }
        );
        let err = complete(&MFX, data).unwrap_err().to_string();
        assert!(err.contains("4..6 missing"), "{err}");
    }

    #[test]
    fn awaits_identity_past_other_messages() {
        let reply = [
            0xF0, 0x7E, 0x10, 0x06, 0x02, 0x41, 0x64, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xF7,
        ];
        let (tx, rx) = mpsc::channel();
        tx.send(vec![0x90, 0x3C, 0x64]).unwrap();
        tx.send(reply.to_vec()).unwrap();
        let identity = await_identity(&rx, TIMEOUT).unwrap().unwrap();
        assert_eq!(identity.device_id, 0x10);
        assert!(await_identity(&rx, TIMEOUT).unwrap().is_none());
    }

    #[test]
    fn no_reply_is_an_error() {
        let (reader, rx, _tx) = scripted(6, &[]);
        let data = collect_reply(reader, &rx, TIMEOUT, false).unwrap();
        assert_eq!(data, BlockData::Timeout);
        assert!(complete(&MFX, data).is_err());
    }
}
