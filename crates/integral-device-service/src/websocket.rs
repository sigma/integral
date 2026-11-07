//! The service's WebSocket endpoint: how a browser, which cannot open a Unix
//! socket, is a raw SysEx client of the service (ADR 0006).
//!
//! # Framing
//!
//! One binary WebSocket frame is one whole SysEx message, `F0` to `F7`, in
//! both directions, and nothing else is carried:
//!
//! - Browser to service: a frame goes through the service's paced queue like
//!   a [`Request::Raw`] on the socket. A frame that is not one SysEx message
//!   is refused by the service and not answered; there is no reply channel
//!   for it, because a shared MIDI port has none either.
//! - Service to browser: every SysEx message the device sends, as on a shared
//!   MIDI port. The endpoint says [`Request::Hello`] with raw on for the
//!   client when it connects, so there is nothing to negotiate. Link state
//!   is told by the socket itself (see below); whether a device answers is
//!   asked with an Identity Request, as on any MIDI port.
//! - Text frames are ignored. Pings are answered.
//!
//! # The device link
//!
//! The endpoint follows the service's link to the INTEGRA-7, so a page's
//! existing reconnect covers it. When the link goes down, every connected
//! page is closed with code 1013 (try again later) and the reason "INTEGRA-7
//! link down", and a page that connects while it is down is accepted and
//! closed the same way at once: a browser sees a refused handshake only as a
//! generic failure, but it reads the close code. A page that sees the socket
//! close retries until the link is back.
//!
//! # Who may connect
//!
//! A web page can open a WebSocket to `127.0.0.1` from any origin, and
//! WebSockets are exempt from the same-origin policy, so the endpoint checks
//! the `Origin` header itself: the handshake succeeds only if it names an
//! entry of the allow-list. Both are parsed as `scheme://host[:port]` and
//! compared in lowercase; `null`, a path and a repeated `Origin` header are
//! refused, and so is a request with no `Origin`. Each refused origin is
//! reported through the [`Log`] given to [`WebSocketServer::bind`].
//!
//! This stops another page in the user's browser driving the device. It does
//! not stop a local program, which can write any header, and which can
//! reach the Unix socket anyway; nor another user of this machine, whose
//! program can connect to the loopback port (accepted: the device is not a
//! secret between the users of one machine, and the Unix socket is the
//! private path). An origin has no path, so the deployed app's
//! `https://sigma.github.io` admits every GitHub Pages site of that user,
//! not only `/integral/`.
//!
//! # Threads
//!
//! One accept thread, and per client a reader thread (frames in, as
//! [`Input`]s to the service) and a writer thread (frames out of a bounded
//! queue), like the Unix socket's. The service thread only ever puts a
//! message on that queue, so a slow or stuck page cannot stall pacing; a page
//! whose queue fills is dropped and reconnects. The service owns the queue's
//! sender through the client's outbox, so dropping the client ends the writer,
//! which closes the socket.

use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use tungstenite::handshake::server::{Callback, ErrorResponse, Request as Handshake, Response};
use tungstenite::http::{StatusCode, header::ORIGIN};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Role, WebSocketConfig};
use tungstenite::{Message as Frame, WebSocket};

use crate::clients::Outbox;
use crate::link::Log;
use crate::protocol::{ClientId, Message, PROTOCOL_VERSION, Request};
use crate::runtime::Handle;
use crate::service::Input;
use integral_core::device_service::WEBSOCKET_HOST;
use integral_core::sysex::MAX_MESSAGE_LEN;

/// Frames a client's outgoing queue holds before the service gives up on it.
const OUTBOX_FRAMES: usize = 4096;

/// Most clients served at once; others are closed unanswered.
pub const MAX_CLIENTS: usize = 32;

/// How long a connection has, in all, to finish its handshake.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a write to a client may stall before the client is dropped.
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// Longest wait after a failed `accept`.
const LONGEST_ACCEPT_RETRY: Duration = Duration::from_secs(1);

/// The close reason sent to pages when the device link goes down.
const LINK_DOWN_REASON: &str = "INTEGRA-7 link down";

/// An origin as the allow-list and the `Origin` header write it:
/// `scheme://host[:port]`, lowercased. `None` for anything else (`null`, a
/// path, userinfo, a bad port).
fn normalize_origin(origin: &str) -> Option<String> {
    let (scheme, authority) = origin.split_once("://")?;
    let mut chars = scheme.chars();
    let scheme_ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        // An IPv6 literal: the brackets hold hex digits and colons only.
        let (literal, after) = rest.split_once(']')?;
        if literal.is_empty() || !literal.chars().all(|c| c.is_ascii_hexdigit() || c == ':') {
            return None;
        }
        let port = if after.is_empty() {
            None
        } else {
            Some(after.strip_prefix(':')?)
        };
        (&authority[..literal.len() + 2], port)
    } else {
        match authority.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (authority, None),
        }
    };
    let host_ok = host.starts_with('[')
        || (!host.is_empty()
            && host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')));
    let port_ok = port.is_none_or(|p| {
        !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u16>().is_ok()
    });
    (scheme_ok && host_ok && port_ok).then(|| origin.to_ascii_lowercase())
}

/// A server accepting browsers as raw SysEx clients of a running service.
///
/// Dropping it stops accepting. Clients already connected stay until the
/// service stops or the link goes down.
pub struct WebSocketServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl WebSocketServer {
    /// Listen on `port` of the loopback address (0 for any free port) and
    /// serve the service behind `handle` to pages whose `Origin` is one of
    /// `origins` (see the module docs). Each refused origin goes to `log`.
    ///
    /// # Errors
    ///
    /// `InvalidInput` if an origin is not `scheme://host[:port]`, else the
    /// OS error (`AddrInUse` if something holds the port), or `BrokenPipe`
    /// if the service has stopped.
    pub fn bind(port: u16, origins: &[String], handle: Handle, log: Log) -> io::Result<Self> {
        let allowed = origins
            .iter()
            .map(|origin| {
                normalize_origin(origin).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("'{origin}' is not an origin (scheme://host[:port], no path)"),
                    )
                })
            })
            .collect::<io::Result<Vec<_>>>()?;
        let listener = TcpListener::bind((WEBSOCKET_HOST, port))?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            let gate = Arc::new(Gate { allowed, log });
            thread::Builder::new()
                .name("device-service-ws-accept".into())
                .spawn(move || accept_loop(&listener, &gate, &handle, &stop))?
        };
        Ok(Self {
            addr,
            stop,
            thread: Some(thread),
        })
    }

    /// The address the server listens on.
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for WebSocketServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocked accept.
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Who may connect.
struct Gate {
    allowed: Vec<String>,
    log: Log,
}

fn accept_loop(listener: &TcpListener, gate: &Arc<Gate>, handle: &Handle, stop: &AtomicBool) {
    let served = Arc::new(AtomicUsize::new(0));
    let mut retry = Duration::from_millis(10);
    loop {
        let accepted = listener.accept();
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let stream = match accepted {
            Ok((stream, _)) => stream,
            Err(_) => {
                // Out of descriptors, say: wait rather than spin on it.
                thread::sleep(retry);
                retry = (retry * 2).min(LONGEST_ACCEPT_RETRY);
                continue;
            }
        };
        retry = Duration::from_millis(10);
        if served.fetch_add(1, Ordering::SeqCst) >= MAX_CLIENTS {
            served.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let client = handle.new_client_id();
        let (gate, handle, served) = (Arc::clone(gate), handle.clone(), Arc::clone(&served));
        let spawned = thread::Builder::new()
            .name(format!("device-service-ws-client-{client}"))
            .spawn({
                let served = Arc::clone(&served);
                move || {
                    let _ = serve(stream, &gate, client, &handle);
                    served.fetch_sub(1, Ordering::SeqCst);
                }
            });
        if spawned.is_err() {
            // The closure, and its decrement, never ran.
            served.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// Accepts a handshake only from an allowed `Origin`.
struct Admit<'a>(&'a Gate);

impl Admit<'_> {
    /// Whether the request names exactly one allowed origin.
    fn origin_allowed(&self, request: &Handshake) -> bool {
        let mut origins = request.headers().get_all(ORIGIN).iter();
        match (origins.next(), origins.next()) {
            (Some(origin), None) => origin
                .to_str()
                .ok()
                .and_then(normalize_origin)
                .is_some_and(|origin| self.0.allowed.contains(&origin)),
            _ => false,
        }
    }

    /// The origin to report for a refused request.
    fn describe(request: &Handshake) -> String {
        let origins: Vec<_> = request
            .headers()
            .get_all(ORIGIN)
            .iter()
            .map(|origin| String::from_utf8_lossy(origin.as_bytes()).into_owned())
            .collect();
        match origins.as_slice() {
            [] => "no Origin header".to_string(),
            [one] => format!("Origin '{one}'"),
            many => format!("{} Origin headers: {}", many.len(), many.join(", ")),
        }
    }
}

fn refusal(status: StatusCode, body: &str) -> ErrorResponse {
    let mut refusal = ErrorResponse::new(Some(body.to_string()));
    *refusal.status_mut() = status;
    refusal
}

impl Callback for Admit<'_> {
    fn on_request(
        self,
        request: &Handshake,
        response: Response,
    ) -> Result<Response, ErrorResponse> {
        if !self.origin_allowed(request) {
            (self.0.log)(&format!(
                "WebSocket handshake refused: {}; allowed: {}",
                Self::describe(request),
                self.0.allowed.join(", ")
            ));
            return Err(refusal(StatusCode::FORBIDDEN, "Origin not allowed"));
        }
        Ok(response)
    }
}

/// What the writer thread sends.
enum Out {
    /// A SysEx message from the device.
    Sysex(Vec<u8>),
    /// The answer to a client's ping.
    Pong(Vec<u8>),
    /// The last frame: close the connection, saying why.
    Close(CloseFrame),
}

/// The sending end of a client's queue. The service's outbox owns it: when
/// the outbox goes, the sender is dropped, the writer ends and the socket is
/// closed. The reader borrows it only to answer pings.
type Lane = Arc<Mutex<Option<SyncSender<Out>>>>;

/// Queue `out` without blocking. Returns whether the lane took it.
fn put(lane: &Lane, out: Out) -> bool {
    lane.lock()
        .ok()
        .and_then(|lane| lane.as_ref().map(|queue| queue.try_send(out).is_ok()))
        .unwrap_or(false)
}

/// The reading half of a connection: it reads the client's stream, and what
/// its WebSocket would write (a pong, a close) is dropped, because the
/// writer thread owns the writing. A ping is answered through the writer.
struct ReadHalf(TcpStream);

impl Read for ReadHalf {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for ReadHalf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A stream whose reads all end by `end`: the handshake gets one deadline,
/// not one per read.
struct Deadline {
    stream: TcpStream,
    end: Instant,
}

impl Read for Deadline {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self
            .end
            .checked_duration_since(Instant::now())
            .filter(|left| !left.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "handshake too slow"))?;
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(buf)
    }
}

impl Write for Deadline {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Serve one client until it or the service goes away.
fn serve(stream: TcpStream, gate: &Gate, client: ClientId, handle: &Handle) -> io::Result<()> {
    stream.set_nodelay(true)?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT))?;
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_LEN))
        .max_frame_size(Some(MAX_MESSAGE_LEN));
    let timed = Deadline {
        stream,
        end: Instant::now() + HANDSHAKE_TIMEOUT,
    };
    // Nothing a browser sends precedes the answer to its handshake, so the
    // stream holds no frame the handshake read ahead.
    let socket = tungstenite::accept_hdr_with_config(timed, Admit(gate), Some(config))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let stream = socket.into_inner().stream;
    stream.set_read_timeout(None)?;

    let mut reader =
        WebSocket::from_raw_socket(ReadHalf(stream.try_clone()?), Role::Server, Some(config));
    let (frames, queue) = mpsc::sync_channel(OUTBOX_FRAMES);
    let lane: Lane = Arc::new(Mutex::new(Some(frames)));
    thread::Builder::new()
        .name(format!("device-service-ws-client-{client}-out"))
        .spawn(move || write_loop(stream, &queue))?;

    let connect = Input::Connect {
        client,
        outbox: Box::new(WebSocketOutbox(Arc::clone(&lane))),
    };
    if handle.client_input(connect) {
        let hello = Request::Hello {
            version: PROTOCOL_VERSION,
            subscriptions: Vec::new(),
            raw: true,
            bound: None,
        };
        let mut open = handle.client_input(Input::Client {
            client,
            request: hello,
        });
        while open {
            open = match reader.read() {
                Ok(Frame::Binary(bytes)) => handle.client_input(Input::Client {
                    client,
                    request: Request::Raw {
                        bytes: bytes.to_vec(),
                    },
                }),
                Ok(Frame::Ping(payload)) => {
                    // A full queue means the writer is behind; the pong is
                    // not worth a wait.
                    put(&lane, Out::Pong(payload.to_vec()));
                    true
                }
                Ok(Frame::Close(_)) | Err(_) => false,
                Ok(_) => true,
            };
        }
        handle.client_input(Input::Disconnect { client });
    }
    Ok(())
}

/// Write queued frames to the client until the queue closes (the service
/// dropped the client) or the socket fails or a close is sent, then close
/// the socket, which also ends the reader.
fn write_loop(stream: TcpStream, queue: &Receiver<Out>) {
    let mut socket = WebSocket::from_raw_socket(&stream, Role::Server, None);
    for out in queue {
        let frame = match out {
            Out::Sysex(bytes) => Frame::binary(bytes),
            Out::Pong(payload) => Frame::Pong(payload.into()),
            Out::Close(frame) => {
                let _ = socket.close(Some(frame));
                break;
            }
        };
        if socket.send(frame).is_err() {
            break;
        }
    }
    let _ = socket.close(None);
    let _ = stream.shutdown(Shutdown::Both);
}

/// The service's [`Outbox`] for a page: the device's SysEx becomes a frame
/// queued without blocking, and the link going down closes the page; the
/// rest of what the service says has no place on a MIDI port and is dropped.
struct WebSocketOutbox(Lane);

impl Drop for WebSocketOutbox {
    fn drop(&mut self) {
        if let Ok(mut lane) = self.0.lock() {
            *lane = None;
        }
    }
}

impl Outbox for WebSocketOutbox {
    fn deliver(&mut self, message: &Message) -> bool {
        match message {
            Message::Sysex { bytes } => put(&self.0, Out::Sysex(bytes.clone())),
            // A link already down at the greeting, or going down: tell the
            // page to try again later, and drop it.
            Message::Welcome { session: None, .. } | Message::LinkDown => {
                put(
                    &self.0,
                    Out::Close(CloseFrame {
                        code: CloseCode::Again,
                        reason: LINK_DOWN_REASON.into(),
                    }),
                );
                false
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_origin_is_scheme_host_and_optional_port_in_lowercase() {
        for (given, normal) in [
            ("http://localhost:5181", "http://localhost:5181"),
            ("HTTPS://Sigma.GitHub.io", "https://sigma.github.io"),
            ("http://[::1]:8080", "http://[::1]:8080"),
            ("http://[::1]", "http://[::1]"),
            ("integral://localhost", "integral://localhost"),
        ] {
            assert_eq!(normalize_origin(given).as_deref(), Some(normal), "{given}");
        }
    }

    #[test]
    fn anything_else_is_not_an_origin() {
        for given in [
            "null",
            "",
            "localhost:5181",
            "http://",
            "http://host/",
            "http://host/path",
            "http://user@host",
            "http://host:",
            "http://host:99999",
            "http://host:80x",
            "http://ho st",
            "http://[]",
            "http://[::1]x",
            "://host",
            "1http://host",
        ] {
            assert_eq!(normalize_origin(given), None, "{given}");
        }
    }
}
