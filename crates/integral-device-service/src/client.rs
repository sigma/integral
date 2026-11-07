//! The client library: what a plugin instance (or any program) uses to talk
//! to a Device service over its socket.
//!
//! Built for a host that cannot be blocked or killed:
//!
//! - **A real-time thread has its own way in.** [`Client::sender`] makes a
//!   [`Sender`] for one thread. Its [`write`](Sender::write) and
//!   [`read`](Sender::read) take `Copy` arguments (a
//!   [`ParamHandle`](integral_core::host_params::ParamHandle), a binding and
//!   a value) and push them through a wait-free single-producer ring (`rtrb`):
//!   no allocation, lock or syscall. The client thread polls the rings every
//!   [`POLL`] (2 ms) while there is activity, and every [`IDLE_POLL`] (20 ms)
//!   after 200 ms without any: a write waits 2 ms at most in a busy moment,
//!   and 20 ms when it is the first in a while.
//!   Every other method of [`Client`] takes a lock and allocates: use them
//!   from a GUI or worker thread.
//! - **Events come out separately.** [`Client::connect`] returns the
//!   [`Client`] (cloneable, to send) and an [`Events`] (one owner, to
//!   receive). The event queue is bounded: when the caller falls behind, the
//!   newest events are dropped and the next one it reads is
//!   [`ClientEvent::Overflowed`], meaning "re-read what you care about".
//! - **No `SIGPIPE`.** The socket is set `SO_NOSIGPIPE` (or writes pass
//!   `MSG_NOSIGNAL`), so a dead service is an error, never a signal.
//! - **It reconnects.** When the service goes away the client reports
//!   [`ClientEvent::Disconnected`], retries with a growing delay and, once it
//!   is back, says `Hello` again with its subscriptions and raw setting, so
//!   they are in force before [`ClientEvent::Connected`] is reported (and
//!   so is its [`Client::bind`], carried in the `Hello`). The new
//!   connection has a new [`ClientId`] and, with a restarted service, a new
//!   session: re-read what you care about. A service that refuses the
//!   protocol version ([`ClientEvent::Incompatible`]) is not retried.
//! - **It only talks to this user's service**: a socket whose other end runs
//!   as another user is not used.
//!
//! # Sends while disconnected are dropped
//!
//! A write to a device you cannot reach is stale by the time you could send
//! it. Anything sent while the client is not connected is discarded (the
//! methods still return `true` if the queue took it); only subscriptions, the
//! raw setting and the bind are remembered, and replayed on reconnect. After
//! [`ClientEvent::Connected`] the instance pushes its state again.
//!
//! Dropping the last [`Client`] stops the client threads and waits for them,
//! which is quick (they are told to stop, not asked); do it off the audio
//! thread.

use std::io::BufReader;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use integral_core::host_params::{Binding, ParamHandle, ParamIndex, PluginKind};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::protocol::{
    Bound, ClientId, Message, PROTOCOL_VERSION, ParamId, Reject, Request, read_frame, write_frame,
};
use crate::service::shared_params;
use crate::session::SessionId;
use crate::sock::{Quiet, no_sigpipe, peer_is_me};

/// Requests that may wait for the client thread (non-real-time).
const CONTROL_QUEUE: usize = 1024;

/// Writes and reads one [`Sender`] may have waiting.
const RING_LEN: usize = 1024;

/// Events that may wait for the caller.
const EVENT_QUEUE: usize = 1024;

/// First wait before retrying a connection.
const FIRST_RETRY: Duration = Duration::from_millis(20);

/// Longest wait between connection attempts.
const LONGEST_RETRY: Duration = Duration::from_millis(500);

/// Time allowed for a connection attempt and its `Hello` on top of the retry
/// delay and the polling that follows it.
const CONNECT_MARGIN: Duration = Duration::from_millis(100);

/// How long, after a service starts, the clients that were connected to its
/// predecessor take to be back with their `Hello`: the longest retry delay,
/// the polling that follows it, and [`CONNECT_MARGIN`]. The service holds
/// [`Request::Claim`] answers for this long.
pub const RECONNECT_WINDOW: Duration = Duration::from_millis(
    (LONGEST_RETRY.as_millis() + IDLE_POLL.as_millis() + CONNECT_MARGIN.as_millis()) as u64,
);

/// How often the client thread looks at its queues while there is
/// activity.
pub const POLL: Duration = Duration::from_millis(2);

/// How often it looks once nothing has been sent for [`IDLE_AFTER`] looks.
pub const IDLE_POLL: Duration = Duration::from_millis(20);

/// Looks without anything to send before the client thread slows down.
const IDLE_AFTER: u32 = 100;

/// What a [`Client`] reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientEvent {
    /// Connected and greeted, with subscriptions in force. `session` is the
    /// link's current session, or none if the link is down.
    Connected {
        /// This connection's ID: what the service puts in the `writer` of
        /// this client's changes.
        client: ClientId,
        /// The current session.
        session: Option<SessionId>,
    },
    /// The connection is gone; the client is retrying.
    Disconnected,
    /// Events were dropped because the caller did not read them: re-read
    /// what you care about.
    Overflowed,
    /// The service speaks another protocol version. Final: the client has
    /// stopped.
    Incompatible {
        /// The version the service speaks.
        server: u32,
    },
    /// A message from the service (other than the welcome, and messages
    /// this client does not know).
    Message(Message),
}

/// What a real-time thread may send: `Copy`, so no allocation.
#[derive(Clone, Copy)]
enum Realtime {
    Write {
        param: ParamHandle,
        binding: Binding,
        value: i32,
    },
    Read {
        param: ParamHandle,
        binding: Binding,
    },
}

/// What the caller hands to the client thread.
enum Command {
    Request(Request),
    Ring(Consumer<Realtime>),
}

/// What the client and its thread share.
struct Shared {
    stop: AtomicBool,
    /// The current connection, so a stop can cut a blocked read.
    stream: Mutex<Option<UnixStream>>,
}

/// Stops and joins the client thread when the last [`Client`] goes.
struct Guard {
    shared: Arc<Shared>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Ok(stream) = self.shared.stream.lock()
            && let Some(stream) = stream.as_ref()
        {
            let _ = stream.shutdown(Shutdown::Both);
        }
        if let Ok(mut thread) = self.thread.lock()
            && let Some(thread) = thread.take()
        {
            let _ = thread.join();
        }
    }
}

/// A connection to a Device service that keeps itself up. Cheap to clone;
/// the connection ends when the last clone is dropped.
#[derive(Clone)]
pub struct Client {
    control: SyncSender<Command>,
    _guard: Arc<Guard>,
}

/// The events of a [`Client`]. One owner.
pub struct Events {
    rx: Receiver<ClientEvent>,
    /// Shared with the [`Sink`]: events were dropped.
    lost: Arc<AtomicBool>,
}

/// A real-time thread's way to write and read (see the module docs). One
/// per thread; not `Clone`.
pub struct Sender {
    ring: Producer<Realtime>,
}

impl Client {
    /// Start a client of the service listening on `path`. Returns at once;
    /// the connection is made (and remade) in the background.
    pub fn connect(path: impl Into<PathBuf>) -> (Self, Events) {
        let (control, queue) = mpsc::sync_channel(CONTROL_QUEUE);
        let (events_out, events) = mpsc::sync_channel(EVENT_QUEUE);
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            stream: Mutex::new(None),
        });
        let path = path.into();
        let lost = Arc::new(AtomicBool::new(false));
        let sink = Arc::new(Sink {
            tx: events_out,
            lost: Arc::clone(&lost),
        });
        let thread = {
            let shared = Arc::clone(&shared);
            let sink = Arc::clone(&sink);
            // A thread that cannot be spawned is a client that never connects.
            thread::Builder::new()
                .name("device-service-client".into())
                .spawn(move || run(&path, &queue, &shared, &sink))
                .ok()
        };
        let guard = Arc::new(Guard {
            shared,
            thread: Mutex::new(thread),
        });
        (
            Self {
                control,
                _guard: guard,
            },
            Events { rx: events, lost },
        )
    }

    /// A [`Sender`] for one real-time thread, or `None` if the client has
    /// stopped. Call it off that thread.
    pub fn sender(&self) -> Option<Sender> {
        let (ring, consumer) = RingBuffer::new(RING_LEN);
        self.control.try_send(Command::Ring(consumer)).ok()?;
        Some(Sender { ring })
    }

    /// Queue a request. Returns `false` for a [`Request::Hello`] (the client
    /// says its own), if the queue is full, or if the client has stopped;
    /// never blocks. `true` means queued: it is discarded if the client is
    /// not connected when its turn comes (see the module docs).
    pub fn send(&self, request: Request) -> bool {
        !matches!(request, Request::Hello { .. })
            && self.control.try_send(Command::Request(request)).is_ok()
    }

    /// Set `param` for `binding` to the raw `value`.
    pub fn write(&self, param: ParamId, binding: Binding, value: i32) -> bool {
        self.send(Request::Write {
            param,
            binding,
            value,
        })
    }

    /// Ask the device for `param`; the answer is a [`Message::Value`].
    pub fn read(&self, param: ParamId, binding: Binding) -> bool {
        self.send(Request::Read { param, binding })
    }

    /// Receive changes in `binding` (see [`Request::Subscribe`]), now and
    /// after every reconnect.
    pub fn subscribe(&self, binding: Binding) -> bool {
        self.send(Request::Subscribe { binding })
    }

    /// Stop receiving changes in the exact `binding` given to
    /// [`subscribe`](Self::subscribe).
    pub fn unsubscribe(&self, binding: Binding) -> bool {
        self.send(Request::Unsubscribe { binding })
    }

    /// Turn on or off receiving every SysEx message the device sends, now
    /// and after every reconnect.
    pub fn receive_raw(&self, on: bool) -> bool {
        self.send(Request::ReceiveRaw { on })
    }

    /// Say which plugin instance this client is and what it is bound to
    /// (`None`: none), now and after every reconnect.
    pub fn bind(&self, bound: Option<Bound>) -> bool {
        self.send(Request::Bind { bound })
    }

    /// Take the first free target of `plugin` (see [`Request::Claim`]); the
    /// answer is a [`Message::Claimed`]. Follow it with
    /// [`bind`](Self::bind) so that a reconnect remembers it.
    pub fn claim(&self, plugin: PluginKind) -> bool {
        self.send(Request::Claim { plugin })
    }

    /// Send one whole SysEx message through the service's paced queue.
    pub fn raw(&self, bytes: Vec<u8>) -> bool {
        self.send(Request::Raw { bytes })
    }
}

impl Sender {
    /// Set `param` for `binding` to the raw `value`. Real-time safe. Returns
    /// `false` if the ring is full or the client has stopped.
    pub fn write(&mut self, param: ParamHandle, binding: Binding, value: i32) -> bool {
        self.push(Realtime::Write {
            param,
            binding,
            value,
        })
    }

    /// Ask the device for `param`. Real-time safe; returns as
    /// [`write`](Self::write) does.
    pub fn read(&mut self, param: ParamHandle, binding: Binding) -> bool {
        self.push(Realtime::Read { param, binding })
    }

    fn push(&mut self, item: Realtime) -> bool {
        !self.ring.is_abandoned() && self.ring.push(item).is_ok()
    }
}

impl Events {
    /// The next event, if one is waiting. Never blocks.
    pub fn try_event(&self) -> Option<ClientEvent> {
        self.rx.try_recv().ok().or_else(|| self.overflowed())
    }

    /// The next event, waiting up to `timeout` for it. For a thread that has
    /// nothing else to do, such as a test; not for a host's real-time thread.
    pub fn wait_event(&self, timeout: Duration) -> Option<ClientEvent> {
        self.rx
            .recv_timeout(timeout)
            .ok()
            .or_else(|| self.overflowed())
    }

    /// [`ClientEvent::Overflowed`] if events were dropped and the queue has
    /// been read empty, so it follows the events that did get through.
    fn overflowed(&self) -> Option<ClientEvent> {
        self.lost
            .swap(false, Ordering::SeqCst)
            .then_some(ClientEvent::Overflowed)
    }
}

/// Where the client thread puts events: bounded, and it never waits.
struct Sink {
    tx: SyncSender<ClientEvent>,
    /// Events were dropped since the last one that got through.
    lost: Arc<AtomicBool>,
}

impl Sink {
    fn emit(&self, event: ClientEvent) {
        if self.lost.swap(false, Ordering::SeqCst)
            && self.tx.try_send(ClientEvent::Overflowed).is_err()
        {
            self.lost.store(true, Ordering::SeqCst);
            return;
        }
        if self.tx.try_send(event).is_err() {
            self.lost.store(true, Ordering::SeqCst);
        }
    }
}

/// What the client keeps across connections.
#[derive(Default)]
struct Remembered {
    subscriptions: Vec<Binding>,
    raw: bool,
    bound: Option<Bound>,
}

impl Remembered {
    /// Note a request that outlives a connection.
    fn note(&mut self, request: &Request) {
        match request {
            Request::Subscribe { binding } => {
                if !self.subscriptions.contains(binding) {
                    self.subscriptions.push(*binding);
                }
            }
            Request::Unsubscribe { binding } => self.subscriptions.retain(|s| s != binding),
            Request::ReceiveRaw { on } => self.raw = *on,
            Request::Bind { bound } => self.bound = *bound,
            _ => {}
        }
    }

    /// The `Hello` that restores it on a new connection.
    fn hello(&self) -> Request {
        Request::Hello {
            version: PROTOCOL_VERSION,
            subscriptions: self.subscriptions.clone(),
            raw: self.raw,
            bound: self.bound,
        }
    }
}

/// How a connection ended.
enum Ended {
    /// The caller dropped the client.
    Stopped,
    /// The service refused the protocol version: stop for good.
    Final,
    /// The connection failed; whether the service had welcomed us.
    Lost { welcomed: bool },
}

/// The client thread's queues and memory.
struct Io<'a> {
    control: &'a Receiver<Command>,
    rings: Vec<Consumer<Realtime>>,
    remembered: Remembered,
    params: &'static ParamIndex,
    /// Looks since anything was sent.
    idle: u32,
}

impl Io<'_> {
    /// Take everything the caller queued. With a connection, send it; with
    /// none, note what outlives a connection and drop the rest. Returns
    /// whether the control queue is still open, and `Err` if a send failed.
    fn pump(&mut self, out: Option<&UnixStream>) -> Result<bool, ()> {
        let mut open = true;
        let mut worked = false;
        loop {
            match self.control.try_recv() {
                Ok(Command::Request(request)) => {
                    worked = true;
                    self.remembered.note(&request);
                    if let Some(stream) = out {
                        write_frame(&mut Quiet(stream), &request).map_err(drop)?;
                    }
                }
                Ok(Command::Ring(consumer)) => self.rings.push(consumer),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    open = false;
                    break;
                }
            }
        }
        for ring in &mut self.rings {
            while let Ok(item) = ring.pop() {
                worked = true;
                let Some(stream) = out else { continue };
                let request = match item {
                    Realtime::Write {
                        param,
                        binding,
                        value,
                    } => Request::Write {
                        param: ParamId::of(self.params, param),
                        binding,
                        value,
                    },
                    Realtime::Read { param, binding } => Request::Read {
                        param: ParamId::of(self.params, param),
                        binding,
                    },
                };
                write_frame(&mut Quiet(stream), &request).map_err(drop)?;
            }
        }
        self.rings.retain(|r| !(r.is_abandoned() && r.is_empty()));
        self.idle = if worked {
            0
        } else {
            self.idle.saturating_add(1)
        };
        Ok(open)
    }

    /// How long to wait before looking again.
    fn nap(&self) -> Duration {
        if self.idle < IDLE_AFTER {
            POLL
        } else {
            IDLE_POLL
        }
    }
}

/// The client thread: connect, serve, and connect again.
fn run(path: &Path, control: &Receiver<Command>, shared: &Shared, sink: &Arc<Sink>) {
    let mut io = Io {
        control,
        rings: Vec::new(),
        remembered: Remembered::default(),
        params: shared_params(),
        idle: 0,
    };
    let mut delay = FIRST_RETRY;
    loop {
        // Anything asked for while away is stale; keep only what outlives it.
        if shared.stop.load(Ordering::SeqCst) || io.pump(None) == Ok(false) {
            return;
        }
        if let Ok(stream) = UnixStream::connect(path) {
            match serve(&stream, &mut io, shared, sink) {
                Ended::Stopped | Ended::Final => return,
                Ended::Lost { welcomed } => {
                    if welcomed {
                        delay = FIRST_RETRY;
                    }
                }
            }
        }
        // Wait out the delay, still taking requests that outlive it.
        let until = Instant::now() + delay;
        while Instant::now() < until {
            if shared.stop.load(Ordering::SeqCst) || io.pump(None) == Ok(false) {
                return;
            }
            thread::sleep(io.nap());
        }
        delay = (delay * 2).min(LONGEST_RETRY);
    }
}

/// Run one connection until it ends.
fn serve(stream: &UnixStream, io: &mut Io<'_>, shared: &Shared, sink: &Arc<Sink>) -> Ended {
    let lost = Ended::Lost { welcomed: false };
    if no_sigpipe(stream).is_err() || !peer_is_me(stream) {
        return lost;
    }
    let (Ok(read_half), Ok(cut)) = (stream.try_clone(), stream.try_clone()) else {
        return lost;
    };
    if let Ok(mut current) = shared.stream.lock() {
        *current = Some(cut);
    }
    let alive = Arc::new(AtomicBool::new(true));
    let welcomed = Arc::new(AtomicBool::new(false));
    let refused = Arc::new(AtomicBool::new(false));
    let reader = {
        let (alive, welcomed, refused) = (
            Arc::clone(&alive),
            Arc::clone(&welcomed),
            Arc::clone(&refused),
        );
        let sink = Arc::clone(sink);
        thread::Builder::new()
            .name("device-service-client-in".into())
            .spawn(move || read_loop(&read_half, &sink, &welcomed, &refused, &alive))
    };
    let Ok(reader) = reader else {
        return lost;
    };

    let mut stopped = false;
    let mut failed = write_frame(&mut Quiet(stream), &io.remembered.hello()).is_err();
    while !failed && alive.load(Ordering::SeqCst) {
        if shared.stop.load(Ordering::SeqCst) {
            stopped = true;
            break;
        }
        match io.pump(Some(stream)) {
            Ok(true) => {}
            Ok(false) => {
                stopped = true;
                break;
            }
            Err(()) => failed = true,
        }
        thread::sleep(io.nap());
    }
    let _ = stream.shutdown(Shutdown::Both);
    let _ = reader.join();
    if let Ok(mut current) = shared.stream.lock() {
        *current = None;
    }
    if stopped {
        return Ended::Stopped;
    }
    if refused.load(Ordering::SeqCst) {
        return Ended::Final;
    }
    let was_welcomed = welcomed.load(Ordering::SeqCst);
    if was_welcomed {
        sink.emit(ClientEvent::Disconnected);
    }
    Ended::Lost {
        welcomed: was_welcomed,
    }
}

/// Read messages from the service until the connection ends.
fn read_loop(
    stream: &UnixStream,
    sink: &Sink,
    welcomed: &AtomicBool,
    refused: &AtomicBool,
    alive: &AtomicBool,
) {
    let mut reader = BufReader::new(stream);
    // A frame that does not decode is skipped: the stream stays aligned.
    loop {
        let message = match read_frame::<Message>(&mut reader) {
            Ok(message) => message,
            Err(crate::protocol::FrameError::Malformed(_)) => continue,
            Err(_) => break,
        };
        match message {
            Message::Welcome {
                client, session, ..
            } => {
                welcomed.store(true, Ordering::SeqCst);
                sink.emit(ClientEvent::Connected { client, session });
            }
            Message::Unknown => {}
            Message::Rejected {
                reason: Reject::Version { server },
                ..
            } => {
                refused.store(true, Ordering::SeqCst);
                sink.emit(ClientEvent::Incompatible { server });
                break;
            }
            other => sink.emit(ClientEvent::Message(other)),
        }
    }
    alive.store(false, Ordering::SeqCst);
}
