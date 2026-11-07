//! The device side of the scenarios: the real Device service on a link of
//! the host's choosing, serving the socket the plugins look for.
//!
//! A plugin finds the service at [`default_socket_path`], which comes from
//! the operating system, not from the environment, so the host cannot point
//! a loaded plugin anywhere else. Instead the host *is* the service: it
//! takes that path itself (the socket lock refuses it if a live service
//! holds it) before it opens any link, and serves **only its own process**:
//! a DAW running beside it, whose plugins look at the same path, is turned
//! away before it can say hello.
//!
//! The link is either a [`FakeLink`] in front of a [`FakeMemory`] (a device
//! that answers reads and keeps writes), or a real INTEGRA-7 over a MIDI
//! port pair. Both sit behind a [`Tap`] that records what the service sent,
//! so a scenario reads the DT1s the same way on either.
//!
//! A scenario names the blocks of the device it touches. The rig reads them
//! once the link is up and can [`rewind`](Rig::rewind) the device to them;
//! on a real device it writes them back when it is dropped.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};
use integral_core::address::{Address, DataSize};
use integral_core::device_spec::INTEGRA7;
use integral_core::sysex::{self, DEFAULT_DEVICE_ID, MAX_DT1_DATA};
use integral_device_service::protocol::Message;
use integral_device_service::{
    Client, ClientEvent, Config, DeviceLink, Events, FakeLink, FakeMemory, LinkError, LinkSink,
    MidirLink, PortMatch, Running, SequentialSessionIds, Service, SocketServer,
    default_socket_path, spawn, stderr_log,
};
use serde_json::json;

use crate::report::Report;

/// How long to wait for the link to come up.
const SESSION_WAIT: Duration = Duration::from_secs(30);
/// How long to wait for the device to answer a read.
const READ_WAIT: Duration = Duration::from_secs(30);
/// How often to look at the link.
const POLL: Duration = Duration::from_millis(20);
/// Silence that ends a burst of the rig's own writes.
const WRITES_QUIET: Duration = Duration::from_secs(1);
/// How long the service may take to start sending the rig's own writes.
const WRITES_START: Duration = Duration::from_secs(4);
/// The longest the rig's own writes may take.
const WRITES_LIMIT: Duration = Duration::from_secs(60);
/// How long a flush of the rig's own client may take.
const FLUSH: Duration = Duration::from_secs(5);

/// What the service talks to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Device {
    /// A fake link in front of a memory that answers reads: nothing leaves
    /// the machine. The default.
    Fake,
    /// A real INTEGRA-7 over an RTP-MIDI session whose name contains this.
    Rtp(String),
    /// A real INTEGRA-7 over the USB port of exactly this name.
    Usb(String),
}

impl Device {
    /// Reads a `--device` value: `fake`, `rtp:<session name>`, `usb` or
    /// `usb:<port name>`.
    ///
    /// # Errors
    ///
    /// Says what was wrong with `text`.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (kind, name) = match text.split_once(':') {
            Some((kind, name)) => (kind, Some(name)),
            None => (text, None),
        };
        match (kind, name) {
            ("fake", None) => Ok(Self::Fake),
            ("rtp", Some(name)) if !name.is_empty() => Ok(Self::Rtp(name.to_owned())),
            ("usb", None) => Ok(Self::Usb(INTEGRA7.name.to_owned())),
            ("usb", Some(name)) if !name.is_empty() => Ok(Self::Usb(name.to_owned())),
            _ => Err(format!(
                "{text:?} is not fake, rtp:<session name>, usb or usb:<port name>"
            )),
        }
    }

    /// Whether this is the fake device.
    pub fn is_fake(&self) -> bool {
        *self == Self::Fake
    }
}

/// An address as the device's four bytes, in hex.
pub fn bytes_of_address(address: &Address) -> String {
    let [a, b, c, d] = *address.as_bytes();
    format!("{a:02x} {b:02x} {c:02x} {d:02x}")
}

/// One message the service sent, with when.
#[derive(Debug, Clone)]
pub struct Sent {
    /// Seconds since the rig started.
    pub at: f64,
    /// The whole MIDI message.
    pub message: Vec<u8>,
}

/// One DT1 the service sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    /// Milliseconds since the rig started.
    pub at_ms: u64,
    /// The first address.
    pub address: Address,
    /// The bytes written.
    pub data: Vec<u8>,
}

type Log = Arc<Mutex<Vec<Sent>>>;

/// A link that records what the service sends through the link it wraps.
struct Tap {
    inner: Box<dyn DeviceLink>,
    log: Log,
    started: Instant,
}

impl DeviceLink for Tap {
    fn start(&mut self, events: LinkSink) -> Result<(), LinkError> {
        self.inner.start(events)
    }

    fn sysex_gap(&self) -> Duration {
        self.inner.sysex_gap()
    }

    fn send(&mut self, message: &[u8]) -> Result<(), LinkError> {
        self.log.lock().expect("tap lock").push(Sent {
            at: self.started.elapsed().as_secs_f64(),
            message: message.to_vec(),
        });
        self.inner.send(message)
    }
}

/// The fake device: the link to bring up and the memory behind it.
struct Fake {
    link: FakeLink,
    memory: FakeMemory,
}

/// What a scenario changes on the device, and so what the rig reads when the
/// link comes up and writes back.
pub struct Touched {
    /// The blocks to read, the ones to write back first first.
    pub blocks: Vec<(Address, DataSize)>,
    /// Write back only these runs (a linear start and a length) of the
    /// blocks, nothing between them; `None` writes the blocks whole. A tone's
    /// blocks are written whole after its selection; the Studio Set's only
    /// where the plugin has parameters, since a write to a Part's tone select
    /// or to a byte the device ignores would do more than put a value back.
    pub only: Option<Vec<(u32, u32)>>,
}

impl Touched {
    /// Blocks written back whole.
    pub fn whole(blocks: Vec<(Address, DataSize)>) -> Self {
        Self { blocks, only: None }
    }
}

/// The runs of `saved` that lie in `only`, as the DT1s' start and bytes.
fn runs_within(saved: &[(Address, Vec<u8>)], only: &[(u32, u32)]) -> Vec<(Address, Vec<u8>)> {
    let mut out = Vec::new();
    for (block, data) in saved {
        let (start, end) = (block.value(), block.value() + data.len() as u32);
        for &(at, len) in only {
            let (from, to) = (at.max(start), (at + len).min(end));
            if from < to
                && let Some(address) = Address::from_value(from)
            {
                out.push((
                    address,
                    data[(from - start) as usize..(to - start) as usize].to_vec(),
                ));
            }
        }
    }
    out
}

/// The Device service running on the chosen link, listening where the
/// plugins look.
pub struct Rig {
    log: Log,
    fake: Option<Fake>,
    /// This process's own client of the service, for reading and writing
    /// the device as another writer would.
    client: Option<Client>,
    events: Events,
    // Dropped in this order: the socket first, then the service.
    server: Option<SocketServer>,
    running: Option<Running>,
    started: Instant,
    device_id: u8,
    /// The blocks the scenario touches, and what the device held in them
    /// when the link came up.
    touched: Touched,
    saved: Vec<(Address, Vec<u8>)>,
    /// Write `saved` back on drop: the device is a real one.
    restore_on_drop: bool,
}

impl Rig {
    /// Starts the service on `device` and listens at the default socket,
    /// where the plugins look for it, for this process alone; reports where
    /// as a `service` line. `touched` are the blocks of the device the
    /// scenario will change (the first ones first: they are written back in
    /// order).
    ///
    /// The socket is taken before any link opens, so a service that is
    /// already there is found out before the device is touched.
    ///
    /// # Errors
    ///
    /// The service or the socket could not start, among them because a
    /// service already holds the default socket.
    pub fn start(device: &Device, touched: Touched, report: &Report) -> Result<Self> {
        let started = Instant::now();
        let socket = default_socket_path().context("finding the default socket path")?;
        let claim = SocketServer::claim(&socket).with_context(|| {
            format!(
                "taking {}: stop the Device service that holds it",
                socket.display()
            )
        })?;
        let log = Log::default();
        let (inner, fake): (Box<dyn DeviceLink>, Option<Fake>) = match device {
            Device::Fake => {
                let link = FakeLink::new();
                let memory = FakeMemory::answer(&link);
                (Box::new(link.clone()), Some(Fake { link, memory }))
            }
            Device::Rtp(name) => (
                Box::new(MidirLink::new(
                    PortMatch::Contains(name.clone()),
                    stderr_log(),
                )),
                None,
            ),
            Device::Usb(name) => (
                Box::new(MidirLink::new(PortMatch::Exact(name.clone()), stderr_log())),
                None,
            ),
        };
        let tap = Tap {
            inner,
            log: Arc::clone(&log),
            started,
        };
        // A real device's ID is the one it gives and its sessions are random,
        // as in the service the plugins normally find.
        let config = if fake.is_some() {
            Config {
                device_id: Some(DEFAULT_DEVICE_ID),
                session_ids: Box::new(SequentialSessionIds(1)),
            }
        } else {
            Config::new(None)
        };
        let running = spawn(Service::new(Box::new(tap), config), None)
            .context("starting the Device service")?;
        let server = match claim.serve(running.handle(), Some(std::process::id())) {
            Ok(server) => server,
            Err(e) => {
                running.stop();
                return Err(e).with_context(|| format!("listening on {}", socket.display()));
            }
        };
        report.emit(
            "service",
            json!({ "socket": socket.display().to_string(), "device": format!("{device:?}"),
                    "peers": "this process only" }),
        );
        let (client, events) = Client::connect(&socket);
        Ok(Self {
            log,
            fake,
            client: Some(client),
            events,
            server: Some(server),
            running: Some(running),
            started,
            device_id: DEFAULT_DEVICE_ID,
            touched,
            saved: Vec::new(),
            restore_on_drop: !device.is_fake(),
        })
    }

    /// The fake device's memory, or `None` on a real device.
    pub fn memory(&self) -> Option<&FakeMemory> {
        self.fake.as_ref().map(|f| &f.memory)
    }

    /// Brings the link up (a fake one at once, a real one when the device
    /// answers), waits for the service to have a session, and reads the
    /// blocks the scenario touches.
    ///
    /// # Errors
    ///
    /// The link did not come up in time, or the device did not answer.
    pub fn bring_up(&mut self) -> Result<()> {
        if let Some(fake) = &self.fake {
            fake.link.bring_up(Some(DEFAULT_DEVICE_ID));
        }
        let deadline = Instant::now() + SESSION_WAIT;
        while !self.has_session()? {
            if Instant::now() >= deadline {
                bail!("the device did not come up");
            }
            sleep(POLL);
        }
        self.device_id = self.ask(|service| service.device().device_id())?;
        self.saved = self.read_blocks(&self.touched.blocks.clone())?;
        Ok(())
    }

    /// Asks the service something, on its thread.
    fn ask<T: Send + 'static>(
        &self,
        question: impl FnOnce(&mut Service) -> T + Send + 'static,
    ) -> Result<T> {
        let (tx, rx) = mpsc::channel();
        let running = self
            .running
            .as_ref()
            .ok_or_else(|| anyhow!("the service has stopped"))?;
        let sent = running.handle().run(move |service| {
            let _ = tx.send(question(service));
        });
        if !sent {
            bail!("the service has stopped");
        }
        Ok(rx.recv_timeout(SESSION_WAIT)?)
    }

    fn has_session(&self) -> Result<bool> {
        self.ask(|service| service.session().is_some())
    }

    fn client(&self) -> Result<&Client> {
        self.client
            .as_ref()
            .ok_or_else(|| anyhow!("the rig's client has stopped"))
    }

    /// Reads `blocks` from the device through the service, as a raw client.
    fn read_blocks(&self, blocks: &[(Address, DataSize)]) -> Result<Vec<(Address, Vec<u8>)>> {
        let client = self.client()?;
        self.wait_connected()?;
        client.receive_raw(true);
        let mut out = Vec::new();
        for (address, size) in blocks {
            let (start, len) = (address.value(), size.value() as usize);
            client.raw(sysex::build_rq1(self.device_id, address, size));
            let mut got: Vec<Option<u8>> = vec![None; len];
            let mut missing = len;
            let deadline = Instant::now() + READ_WAIT;
            while missing > 0 {
                let left = deadline.saturating_duration_since(Instant::now());
                let Some(event) = self.events.wait_event(left) else {
                    bail!(
                        "the device did not answer the read of {} ({missing} of {len} bytes missing)",
                        bytes_of_address(address)
                    );
                };
                let ClientEvent::Message(Message::Sysex { bytes }) = event else {
                    continue;
                };
                let Ok(dt1) = sysex::parse_dt1(&bytes) else {
                    continue;
                };
                for (k, byte) in dt1.data.iter().enumerate() {
                    let at = dt1.address.value() + k as u32;
                    if let Some(slot) = at.checked_sub(start).and_then(|i| got.get_mut(i as usize))
                        && slot.replace(*byte).is_none()
                    {
                        missing -= 1;
                    }
                }
            }
            out.push((*address, got.into_iter().flatten().collect()));
        }
        client.receive_raw(false);
        Ok(out)
    }

    fn wait_connected(&self) -> Result<()> {
        let deadline = Instant::now() + SESSION_WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.events.wait_event(left) {
                Some(ClientEvent::Connected { .. }) => return Ok(()),
                Some(_) => {}
                None => bail!("the rig's client did not connect"),
            }
        }
    }

    /// What the device held in `address`' block when the link came up.
    ///
    /// # Errors
    ///
    /// The scenario did not name that block.
    pub fn held(&self, address: Address, len: usize) -> Result<Vec<u8>> {
        self.saved
            .iter()
            .find(|(at, data)| *at == address && data.len() >= len)
            .map(|(_, data)| data[..len].to_vec())
            .ok_or_else(|| anyhow!("{} is not a block the rig read", bytes_of_address(&address)))
    }

    /// Writes `data` at `address` as another writer would (a raw client of
    /// the service), and waits for it to leave.
    ///
    /// # Errors
    ///
    /// The rig's client has stopped.
    pub fn write_as_other(&self, address: &Address, data: &[u8]) -> Result<()> {
        self.write_blocks(&[(*address, data.to_vec())])
    }

    /// Puts the device back as it was when the link came up, for the blocks
    /// the scenario touches.
    ///
    /// # Errors
    ///
    /// The rig's client has stopped.
    pub fn rewind(&self) -> Result<()> {
        match &self.touched.only {
            None => self.write_blocks(&self.saved),
            Some(only) => self.write_blocks(&runs_within(&self.saved, only)),
        }
    }

    fn write_blocks(&self, blocks: &[(Address, Vec<u8>)]) -> Result<()> {
        let client = self.client()?;
        let before = self.writes().len();
        let mut chunks = 0;
        for (address, data) in blocks {
            for (n, chunk) in data.chunks(MAX_DT1_DATA).enumerate() {
                let at = Address::from_value(address.value() + (n * MAX_DT1_DATA) as u32)
                    .ok_or_else(|| anyhow!("an address past the device's"))?;
                client.raw(sysex::build_dt1(self.device_id, &at, chunk));
                chunks += 1;
            }
        }
        if !client.flush(FLUSH) {
            bail!("the rig's writes did not reach the service");
        }
        // The service paces them out, and leaves out the ones the device is
        // known to hold: wait for all of them, or for the service to be
        // quiet once it has had time to start.
        let flushed = Instant::now();
        let deadline = flushed + WRITES_LIMIT;
        loop {
            let sent = self.sent();
            let idle = self.started.elapsed().as_secs_f64() - sent.last().map_or(0.0, |s| s.at);
            let done = self.writes().len() >= before + chunks;
            let settled = flushed.elapsed() >= WRITES_START && idle >= WRITES_QUIET.as_secs_f64();
            if done || settled {
                return Ok(());
            }
            if Instant::now() >= deadline {
                bail!("the service was still sending after {WRITES_LIMIT:?}");
            }
            sleep(POLL);
        }
    }

    /// Everything the service sent so far.
    pub fn sent(&self) -> Vec<Sent> {
        self.log.lock().expect("tap lock").clone()
    }

    /// The DT1s the service sent, in order.
    pub fn writes(&self) -> Vec<Write> {
        self.sent()
            .iter()
            .filter_map(|s| {
                let dt1 = sysex::parse_dt1(&s.message).ok()?;
                Some(Write {
                    at_ms: (s.at * 1000.0) as u64,
                    address: dt1.address,
                    data: dt1.data,
                })
            })
            .collect()
    }

    /// Forgets what was sent, and (on a fake device) what the fake link
    /// recorded.
    pub fn clear(&self) {
        self.log.lock().expect("tap lock").clear();
        if let Some(fake) = &self.fake {
            fake.link.clear();
        }
    }

    /// Waits until the service has sent at least `min_reads` RQ1s and
    /// `min_writes` DT1s and then nothing at all for `quiet`, for at most
    /// `limit`; returns the DT1s.
    ///
    /// # Errors
    ///
    /// Too little was sent within `limit`.
    pub fn wait_quiet(
        &self,
        (min_reads, min_writes): (usize, usize),
        quiet: Duration,
        limit: Duration,
    ) -> Result<Vec<Write>> {
        let deadline = Instant::now() + limit;
        loop {
            let sent = self.sent();
            let idle = self.started.elapsed().as_secs_f64() - sent.last().map_or(0.0, |s| s.at);
            let reads = sent
                .iter()
                .filter(|s| integral_device_service::is_rq1(&s.message))
                .count();
            let writes = self.writes();
            if reads >= min_reads && writes.len() >= min_writes && idle >= quiet.as_secs_f64() {
                return Ok(writes);
            }
            if Instant::now() >= deadline {
                bail!(
                    "{reads} of {min_reads} reads and {} of {min_writes} DT1s after {limit:?}",
                    writes.len(),
                );
            }
            sleep(POLL);
        }
    }

    /// Reports each DT1 as a `dt1` line, under `phase`.
    pub fn report_writes(&self, report: &Report, phase: &str, writes: &[Write]) {
        for w in writes {
            report.emit(
                "dt1",
                json!({
                    "phase": phase,
                    "t_ms": w.at_ms,
                    "address": bytes_of_address(&w.address),
                    "data": w.data,
                }),
            );
        }
    }
}

impl Drop for Rig {
    fn drop(&mut self) {
        // A real device gets back what it held.
        if self.restore_on_drop && !self.saved.is_empty() {
            let _ = self.rewind();
        }
        self.client.take();
        self.server.take();
        if let Some(running) = self.running.take() {
            running.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_is_read_from_its_flag() {
        assert_eq!(Device::parse("fake"), Ok(Device::Fake));
        assert_eq!(
            Device::parse("rtp:Network"),
            Ok(Device::Rtp("Network".into()))
        );
        assert_eq!(Device::parse("usb"), Ok(Device::Usb(INTEGRA7.name.into())));
        assert_eq!(Device::parse("usb:X"), Ok(Device::Usb("X".into())));
        assert!(Device::parse("rtp").is_err());
        assert!(Device::parse("rtp:").is_err());
        assert!(Device::parse("fake:1").is_err());
        assert!(Device::parse("midi").is_err());
    }

    #[test]
    fn only_the_runs_asked_for_are_written_back() {
        let at = |n| Address::from_value(n).unwrap();
        let saved = vec![(at(100), (0..10).collect::<Vec<u8>>())];
        // A run across the block's start, one inside, one outside.
        let only = [(98, 4), (106, 2), (200, 3)];
        assert_eq!(
            runs_within(&saved, &only),
            [(at(100), vec![0, 1]), (at(106), vec![6, 7])]
        );
    }

    #[test]
    fn an_address_prints_as_the_devices_bytes() {
        let address = Address::new(0x03, 0x00, 0x10, 0x06);
        assert_eq!(bytes_of_address(&address), "03 00 10 06");
    }
}
