//! Bridge command: expose the INTEGRA-7 Wireless link as a virtual MIDI port.
//!
//! Spec: `docs/specs/wireless-transport.md`. The protocol lives sans-IO in
//! `integral_core::wireless`; this module owns the sockets and the port.

use std::collections::BTreeMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use integral_core::wireless::{self, DeviceInfo, Event, Session};
use midir::os::unix::{VirtualInput, VirtualOutput};
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

/// How long discovery keeps probing before giving up.
const DISCOVERY_WINDOW: Duration = Duration::from_secs(5);
/// Interval between discovery probes, and between reconnect attempts.
const RETRY_INTERVAL: Duration = Duration::from_secs(1);
/// Failed attempts on a discovered address before rediscovering it.
const REDISCOVER_AFTER: u32 = 3;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Run the Bridge forever (until Ctrl-C, whose process exit closes the TCP
/// connection and removes the port).
///
/// With `host`, connect there and never rediscover; otherwise find the
/// device by RDDP and rediscover after [`REDISCOVER_AFTER`] failed attempts.
pub fn bridge(host: Option<IpAddr>) -> Result<()> {
    let mut addr = match host {
        Some(ip) => ip,
        None => discover()?,
    };
    let start = Instant::now();
    let mut failures = 0;
    loop {
        let mut went_up = false;
        if let Err(e) = run_session(addr, start, &mut went_up) {
            eprintln!("Session with {addr} failed: {e:#}");
        }
        failures = if went_up { 0 } else { failures + 1 };
        if host.is_none() && failures >= REDISCOVER_AFTER {
            failures = 0;
            match discover() {
                Ok(ip) => addr = ip,
                Err(e) => eprintln!("Rediscovery failed, keeping {addr}: {e:#}"),
            }
        }
        thread::sleep(RETRY_INTERVAL);
    }
}

/// Find the one INTEGRA-7 on the LAN by RDDP broadcast.
fn discover() -> Result<IpAddr> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).context("RDDP listener")?;
    listener.set_nonblocking(true)?;
    let probe = wireless::rddp_probe(listener.local_addr()?.port());
    let udp = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).context("RDDP socket")?;
    udp.set_broadcast(true)?;

    eprintln!("Discovering INTEGRA-7 on the LAN...");
    let start = Instant::now();
    let mut next_probe = start;
    let mut found: BTreeMap<String, (IpAddr, DeviceInfo)> = BTreeMap::new();
    while start.elapsed() < DISCOVERY_WINDOW {
        if Instant::now() >= next_probe {
            // Stop at the end of the first probe round that got answers.
            if !found.is_empty() {
                break;
            }
            udp.send_to(&probe, (Ipv4Addr::BROADCAST, wireless::DISCOVERY_PORT))
                .context("RDDP probe")?;
            next_probe += RETRY_INTERVAL;
        }
        match listener.accept() {
            Ok((stream, peer)) => match read_rddp_reply(stream) {
                Ok(info) => {
                    found.insert(info.wireless_id.clone(), (peer.ip(), info));
                }
                Err(e) => eprintln!("Ignoring RDDP reply from {}: {e:#}", peer.ip()),
            },
            Err(e) if e.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(e).context("RDDP accept"),
        }
    }

    let mut devices = found.into_values();
    match (devices.next(), devices.len()) {
        (None, _) => bail!("no INTEGRA-7 answered discovery; try --host <ip>"),
        (Some((ip, info)), 0) => {
            eprintln!(
                "Found INTEGRA-7 {} at {ip} (system {})",
                info.wireless_id, info.version
            );
            Ok(ip)
        }
        (Some(first), _) => {
            eprintln!("Several INTEGRA-7s answered:");
            for (ip, info) in std::iter::once(first).chain(devices) {
                eprintln!("  {}  {ip}  system {}", info.wireless_id, info.version);
            }
            bail!("pick one with --host <ip>")
        }
    }
}

/// Read one RDDP reply from a device's callback connection.
fn read_rddp_reply(stream: TcpStream) -> Result<DeviceInfo> {
    // Accepted sockets inherit the listener's non-blocking mode on BSDs.
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(RETRY_INTERVAL))?;
    let mut reply = Vec::new();
    stream.take(256).read_to_end(&mut reply)?;
    Ok(wireless::parse_rddp_reply(&reply)?)
}

/// The virtual port pair, alive only while the session is up.
type Port = (MidiInputConnection<()>, MidiOutputConnection);

/// Hold one TCP session until it dies. Sets `went_up` once greeted.
fn run_session(addr: IpAddr, start: Instant, went_up: &mut bool) -> Result<()> {
    let now = || start.elapsed().as_millis() as u64;
    let target = SocketAddr::new(addr, wireless::SESSION_PORT);
    let mut reader = TcpStream::connect_timeout(&target, CONNECT_TIMEOUT)
        .with_context(|| format!("connect to {target}"))?;
    reader.set_nodelay(true)?;
    let writer = Arc::new(Mutex::new(reader.try_clone()?));
    eprintln!("Connected to {target}, waiting for greeting...");

    let mut session = Session::new(now());
    let mut port: Option<Port> = None;
    let mut buf = [0; 4096];
    loop {
        let mut events = session.tick(now());
        if !events.contains(&Event::Dead) {
            let wait = session.next_deadline().saturating_sub(now()).max(1);
            reader.set_read_timeout(Some(Duration::from_millis(wait)))?;
            match reader.read(&mut buf) {
                Ok(0) => bail!("closed by the device"),
                Ok(n) => events.extend(session.receive(&buf[..n], now())),
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) => return Err(e).context("read"),
            }
        }
        for event in events {
            match event {
                Event::Transmit(bytes) => send(&writer, &bytes)?,
                Event::Up { uptime_ms } => {
                    *went_up = true;
                    port = Some(open_port(Arc::clone(&writer))?);
                    eprintln!(
                        "Session up (device uptime {} s); port '{}' ready",
                        uptime_ms / 1000,
                        wireless::port_name()
                    );
                }
                Event::InUse => {
                    eprintln!("INTEGRA-7 in use by another editor; waiting for it to leave...")
                }
                Event::Midi(msg) => {
                    if let Some((_, out)) = port.as_mut() {
                        out.send(&msg).context("forward to port")?;
                    }
                }
                Event::Skipped { kind, len } => {
                    eprintln!("Skipped unknown frame type {kind:02X} ({len} bytes)")
                }
                Event::Dead => bail!("no traffic for {} ms", wireless::LIVENESS_TIMEOUT_MS),
            }
        }
    }
}

fn send(writer: &Mutex<TcpStream>, bytes: &[u8]) -> Result<()> {
    writer
        .lock()
        .map_err(|_| anyhow!("writer lock poisoned"))?
        .write_all(bytes)
        .context("write")
}

/// Create the virtual port pair; MIDI sent to it goes out as `02` frames.
fn open_port(writer: Arc<Mutex<TcpStream>>) -> Result<Port> {
    let name = wireless::port_name();
    let input = MidiInput::new("integral-bridge-in")
        .context("MIDI input")?
        .create_virtual(
            &name,
            move |_ts, msg, _| match wireless::encode_midi(msg) {
                Some(frame) => {
                    if let Err(e) = send(&writer, &frame) {
                        eprintln!("Dropping MIDI to device: {e:#}");
                    }
                }
                None => eprintln!("Dropping malformed MIDI: {msg:02X?}"),
            },
            (),
        )
        .map_err(|e| anyhow!("create virtual input: {e}"))?;
    let output = MidiOutput::new("integral-bridge-out")
        .context("MIDI output")?
        .create_virtual(&name)
        .map_err(|e| anyhow!("create virtual output: {e}"))?;
    Ok((input, output))
}
