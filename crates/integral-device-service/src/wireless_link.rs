//! The Wireless link: the INTEGRA-7's wireless LAN session over TCP.
//!
//! The protocol is sans-IO in `integral_core::wireless`; this module owns the
//! sockets. With a shared port enabled (Unix only) it also exposes the session
//! as the virtual MIDI port `INTEGRA-7 (Wireless)`, so the web UI and the CLI
//! keep reaching a device that accepts only one TCP session. Writes through
//! that port bypass the service's queue, the shared-port convention of
//! ADR 0006 (`docs/adr/0006-device-service-owns-the-link.md`).

use std::collections::BTreeMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use integral_core::wireless::{self, DeviceInfo, Event, Reconnect, Session};

use crate::link::{DeviceLink, LinkError, LinkSink, Log};
use crate::tcp;

const PROBE_INTERVAL: Duration = Duration::from_millis(wireless::PROBE_INTERVAL_MS);
const DISCOVERY_WINDOW: Duration = Duration::from_millis(wireless::DISCOVERY_WINDOW_MS);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// The TCP stream's write half, shared by the session, the service and the
/// shared port. `None` while no session is up.
type Writer = Arc<Mutex<Option<TcpStream>>>;

/// A [`DeviceLink`] over the INTEGRA-7's Wireless session.
pub struct WirelessLink {
    host: Option<IpAddr>,
    share_port: bool,
    writer: Writer,
    stop: Arc<AtomicBool>,
    log: Log,
}

impl WirelessLink {
    /// A link to the device at `host`, never rediscovered; or, with `None`,
    /// to the one found on the LAN by RDDP broadcast and rediscovered when it
    /// stops answering.
    pub fn new(host: Option<IpAddr>, log: Log) -> Self {
        Self {
            host,
            share_port: false,
            writer: Arc::default(),
            stop: Arc::default(),
            log,
        }
    }

    /// Also expose the session as the virtual MIDI port
    /// `wireless::port_name()` while it is up. Unix only; ignored elsewhere.
    #[must_use]
    pub fn with_shared_port(mut self, share: bool) -> Self {
        self.share_port = share;
        self
    }
}

impl DeviceLink for WirelessLink {
    fn start(&mut self, events: LinkSink) -> Result<(), LinkError> {
        let mut worker = Worker {
            host: self.host,
            found: self.host,
            share_port: self.share_port,
            writer: Arc::clone(&self.writer),
            stop: Arc::clone(&self.stop),
            log: Arc::clone(&self.log),
            events,
        };
        if worker.host.is_none() {
            // Fail now, not forever, when the choice is the user's to make.
            match worker.discover() {
                Ok(ip) => worker.found = Some(ip),
                Err(Discovery::Several(why)) => return Err(LinkError::Io(why)),
                Err(Discovery::Failed(why)) => (self.log)(&why),
            }
        }
        thread::Builder::new()
            .name("wireless-link".into())
            .spawn(move || worker.run())
            .map_err(|e| LinkError::Io(e.to_string()))?;
        Ok(())
    }

    fn send(&mut self, message: &[u8]) -> Result<(), LinkError> {
        let frame = wireless::encode_midi(message)
            .ok_or_else(|| LinkError::Io(format!("malformed MIDI: {message:02X?}")))?;
        write_frame(&self.writer, &frame)
    }
}

impl Drop for WirelessLink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// How long a write may stall before the session is given up: one liveness
/// interval, so a stalled TCP window cannot freeze the service thread.
const WRITE_TIMEOUT: Duration = Duration::from_millis(wireless::LIVENESS_TIMEOUT_MS);

fn write_frame(writer: &Writer, bytes: &[u8]) -> Result<(), LinkError> {
    let mut guard = writer.lock().expect("writer lock");
    let stream = guard.as_mut().ok_or(LinkError::Down)?;
    stream.write_all(bytes).map_err(|e| {
        // A partly written frame desyncs the stream: end the session, which
        // the reader notices and reconnects.
        let _ = stream.shutdown(Shutdown::Both);
        LinkError::Io(format!("write: {e}"))
    })
}

/// Why discovery found no device to use.
enum Discovery {
    /// Nothing answered, or the broadcast failed: worth trying again.
    Failed(String),
    /// Several devices answered: the user must choose, so retrying is futile.
    Several(String),
}

impl std::fmt::Display for Discovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (Self::Failed(why) | Self::Several(why)) = self;
        f.write_str(why)
    }
}

impl From<String> for Discovery {
    fn from(why: String) -> Self {
        Self::Failed(why)
    }
}

/// What the link's thread owns.
struct Worker {
    /// The address the user gave; never rediscovered.
    host: Option<IpAddr>,
    /// The address to start with: `host`, or the first discovery's.
    found: Option<IpAddr>,
    share_port: bool,
    writer: Writer,
    stop: Arc<AtomicBool>,
    log: Log,
    events: LinkSink,
}

impl Worker {
    /// Connect, hold the session until it dies, reconnect per [`Reconnect`],
    /// until the link is dropped.
    fn run(self) {
        let start = Instant::now();
        let now = || start.elapsed().as_millis() as u64;
        let mut reconnect = Reconnect::new(self.host.is_none());
        let mut addr = self.found;
        if addr.is_none() {
            // The synchronous discovery in `start` found nothing: pause
            // before the next try.
            thread::sleep(Duration::from_millis(wireless::RECONNECT_INTERVAL_MS));
        }
        while !self.stop.load(Ordering::Relaxed) {
            let target = match addr {
                Some(ip) => ip,
                None => match self.discover() {
                    Ok(ip) => {
                        addr = Some(ip);
                        ip
                    }
                    Err(e) => {
                        (self.log)(&e.to_string());
                        thread::sleep(Duration::from_millis(wireless::RECONNECT_INTERVAL_MS));
                        continue;
                    }
                },
            };
            let mut went_up = false;
            if let Err(e) = self.run_session(target, start, &mut went_up) {
                (self.log)(&format!("Session with {target} failed: {e}"));
            }
            *self.writer.lock().expect("writer lock") = None;
            if went_up {
                self.events.down();
            }
            let next = reconnect.after_attempt(went_up, now());
            thread::sleep(Duration::from_millis(next.at.saturating_sub(now())));
            if next.rediscover {
                match self.discover() {
                    Ok(ip) => addr = Some(ip),
                    Err(e) => (self.log)(&format!("Rediscovery failed, keeping {target}: {e}")),
                }
            }
        }
    }

    /// Find the one INTEGRA-7 on the LAN by RDDP broadcast.
    fn discover(&self) -> Result<IpAddr, Discovery> {
        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0))
            .map_err(|e| format!("RDDP listener: {e}"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let probe = wireless::rddp_probe(port);
        let udp =
            UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(|e| format!("RDDP socket: {e}"))?;
        udp.set_broadcast(true).map_err(|e| e.to_string())?;

        (self.log)("Discovering INTEGRA-7 on the LAN...");
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
                    .map_err(|e| format!("RDDP probe: {e}"))?;
                next_probe += PROBE_INTERVAL;
            }
            match listener.accept() {
                Ok((stream, peer)) => match read_rddp_reply(stream) {
                    Ok(info) => {
                        found.insert(info.wireless_id.clone(), (peer.ip(), info));
                    }
                    Err(e) => (self.log)(&format!("Ignoring RDDP reply from {}: {e}", peer.ip())),
                },
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => return Err(format!("RDDP accept: {e}").into()),
            }
        }

        let mut devices = found.into_values();
        match (devices.next(), devices.len()) {
            (None, _) => Err("no INTEGRA-7 answered discovery; try --host <ip>"
                .to_string()
                .into()),
            (Some((ip, info)), 0) => {
                (self.log)(&format!(
                    "Found INTEGRA-7 {} at {ip} (system {})",
                    info.wireless_id, info.version
                ));
                Ok(ip)
            }
            (Some(first), _) => {
                let list: Vec<String> = std::iter::once(first)
                    .chain(devices)
                    .map(|(ip, info)| format!("{} {ip} system {}", info.wireless_id, info.version))
                    .collect();
                Err(Discovery::Several(format!(
                    "several INTEGRA-7s answered ({}); pick one with --host <ip>",
                    list.join("; ")
                )))
            }
        }
    }

    /// Hold one TCP session until it dies. Sets `went_up` once greeted.
    fn run_session(&self, addr: IpAddr, start: Instant, went_up: &mut bool) -> Result<(), String> {
        let now = || start.elapsed().as_millis() as u64;
        let target = SocketAddr::new(addr, wireless::SESSION_PORT);
        let mut reader = TcpStream::connect_timeout(&target, CONNECT_TIMEOUT)
            .map_err(|e| format!("connect to {target}: {e}"))?;
        reader.set_nodelay(true).map_err(|e| e.to_string())?;
        let write_half = reader.try_clone().map_err(|e| e.to_string())?;
        tcp::set_write_timeout(&write_half, Some(WRITE_TIMEOUT)).map_err(|e| e.to_string())?;
        // The greeting handshake writes before the session is up.
        *self.writer.lock().expect("writer lock") = Some(write_half);
        (self.log)(&format!("Connected to {target}, waiting for greeting..."));

        let mut session = Session::new(now());
        let mut port: Option<shared::Port> = None;
        let mut buf = [0; 4096];
        while !self.stop.load(Ordering::Relaxed) {
            let mut events = session.tick(now());
            if !events.contains(&Event::Dead) {
                let wait = session.next_deadline().saturating_sub(now()).max(1);
                tcp::set_read_timeout(&reader, Some(Duration::from_millis(wait)))
                    .map_err(|e| e.to_string())?;
                match reader.read(&mut buf) {
                    Ok(0) => return Err("closed by the device".into()),
                    Ok(n) => events.extend(session.receive(&buf[..n], now())),
                    Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                    Err(e) => return Err(format!("read: {e}")),
                }
            }
            for event in events {
                match event {
                    Event::Transmit(bytes) => {
                        write_frame(&self.writer, &bytes).map_err(|e| e.to_string())?;
                    }
                    Event::Up { uptime_ms } => {
                        if self.share_port {
                            port = Some(shared::open(Arc::clone(&self.writer), &self.log)?);
                        }
                        *went_up = true;
                        (self.log)(&format!(
                            "Session up (device uptime {} s)",
                            uptime_ms / 1000
                        ));
                        self.events.up(None);
                    }
                    Event::InUse => {
                        (self.log)("INTEGRA-7 in use by another editor; waiting for it to leave...")
                    }
                    Event::Midi(msg) => {
                        shared::forward(&mut port, &msg)?;
                        self.events.midi(&msg);
                    }
                    Event::Skipped { kind, len } => {
                        (self.log)(&format!(
                            "Skipped unknown frame type {kind:02X} ({len} bytes)"
                        ));
                    }
                    Event::Dead => {
                        return Err(format!(
                            "no traffic for {} ms",
                            wireless::LIVENESS_TIMEOUT_MS
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Read one RDDP reply from a device's callback connection.
fn read_rddp_reply(stream: TcpStream) -> Result<DeviceInfo, String> {
    // Accepted sockets inherit the listener's non-blocking mode on BSDs.
    stream.set_nonblocking(false).map_err(|e| e.to_string())?;
    tcp::set_read_timeout(&stream, Some(PROBE_INTERVAL)).map_err(|e| e.to_string())?;
    let mut reply = Vec::new();
    if let Err(e) = stream.take(256).read_to_end(&mut reply) {
        // A reset right behind the reply is the device hanging up: the bytes
        // already read are the answer, so they stand. A reset with nothing
        // read is the connection's own failure, and is reported as that.
        if e.kind() != ErrorKind::ConnectionReset || reply.is_empty() {
            return Err(e.to_string());
        }
    }
    wireless::parse_rddp_reply(&reply).map_err(|e| e.to_string())
}

/// The shared virtual port, where the platform has virtual ports.
#[cfg(unix)]
mod shared {
    use super::{Writer, write_frame};
    use crate::link::Log;
    use integral_core::wireless;
    use midir::os::unix::{VirtualInput, VirtualOutput};
    use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

    /// The virtual port pair, alive only while the session is up.
    pub type Port = (MidiInputConnection<()>, MidiOutputConnection);

    /// Create the pair; MIDI sent to it goes out as `02` frames.
    pub fn open(writer: Writer, log: &Log) -> Result<Port, String> {
        let name = wireless::port_name();
        let on_drop = log.clone();
        let mut input = MidiInput::new("integral-device-service-in").map_err(|e| e.to_string())?;
        // Transparent: clock and active sensing pass, as on a plain port.
        input.ignore(Ignore::None);
        let input = input
            .create_virtual(
                &name,
                move |_ts, msg, ()| match wireless::encode_midi(msg) {
                    Some(frame) => {
                        if let Err(e) = write_frame(&writer, &frame) {
                            on_drop(&format!("Dropping MIDI to device: {e}"));
                        }
                    }
                    None => on_drop(&format!("Dropping malformed MIDI: {msg:02X?}")),
                },
                (),
            )
            .map_err(|e| format!("create virtual input: {e}"))?;
        let output = MidiOutput::new("integral-device-service-out")
            .map_err(|e| e.to_string())?
            .create_virtual(&name)
            .map_err(|e| format!("create virtual output: {e}"))?;
        log(&format!("Shared port '{name}' ready"));
        Ok((input, output))
    }

    /// Pass a device message to the shared port, if open.
    pub fn forward(port: &mut Option<Port>, msg: &[u8]) -> Result<(), String> {
        if let Some((_, out)) = port.as_mut() {
            out.send(msg).map_err(|e| format!("forward to port: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(not(unix))]
mod shared {
    use super::Writer;
    use crate::link::Log;

    pub type Port = ();

    pub fn open(_writer: Writer, _log: &Log) -> Result<Port, String> {
        Ok(())
    }

    pub fn forward(_port: &mut Option<Port>, _msg: &[u8]) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 84-byte RDDP reply of an INTEGRA-7, as
    /// [`wireless::parse_rddp_reply`] reads it.
    fn rddp_reply() -> Vec<u8> {
        let mut reply = vec![0; 84];
        reply[..6].copy_from_slice(b"RDDPv1");
        reply[20..27].copy_from_slice(b"Roland\0");
        reply[36..42].copy_from_slice(b"41288\0");
        reply[44..49].copy_from_slice(b"1.21\0");
        reply[52..62].copy_from_slice(b"INTEGRA-7\0");
        reply
    }

    /// A device that answers discovery and then resets its callback
    /// connection is still read: the reply is the answer, whether the reset
    /// arrives before the timeout is set (macOS answers `EINVAL`) or behind
    /// the buffered reply.
    #[cfg(unix)]
    #[test]
    fn a_reply_the_device_follows_with_a_reset_is_still_read() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
        let addr = listener.local_addr().expect("addr");
        let device = thread::spawn(move || {
            let mut stream = TcpStream::connect(addr).expect("connect");
            stream.write_all(&rddp_reply()).expect("reply");
            reset(&stream);
            drop(stream);
        });
        let (stream, _) = listener.accept().expect("accept");
        let info = read_rddp_reply(stream).expect("the reply stands");
        assert_eq!(info.wireless_id, "41288");
        device.join().expect("device");
    }

    /// Close `stream` with a reset rather than a FIN, as a device dropping
    /// its side abruptly does.
    #[cfg(unix)]
    fn reset(stream: &TcpStream) {
        use std::os::fd::AsRawFd;
        let linger = libc::linger {
            l_onoff: 1,
            l_linger: 0,
        };
        // Safety: a plain setsockopt with a valid option and buffer.
        unsafe {
            assert_eq!(
                libc::setsockopt(
                    stream.as_raw_fd(),
                    libc::SOL_SOCKET,
                    libc::SO_LINGER,
                    &linger as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::linger>() as libc::socklen_t,
                ),
                0
            );
        }
    }
}
