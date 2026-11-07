//! The Device service as a transport: `--service` makes the CLI a raw SysEx
//! client of the service ([ADR 0006](../../../docs/adr/0006-device-service-owns-the-link.md))
//! instead of opening a MIDI port.
//!
//! [`connect`] gives the same [`Connection`] as a port, so no command knows
//! which transport it runs on: [`ServiceOutput`] sends through the service's
//! paced queue, and a thread turns the SysEx the service relays into the
//! receiver's messages.
//!
//! A CLI run is one-shot, so it fails like a port would: when the connection
//! drops, the service refuses the protocol version or its link to the
//! device is down, the receiver disconnects and every later send fails.

use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use integral_device_service::protocol::{Message, Reject};
use integral_device_service::{Client, ClientEvent, Events, default_socket_path};

use crate::midi::{Connection, Output};

/// How long to wait for the service to greet us.
const GREETING_TIMEOUT: Duration = Duration::from_secs(2);

/// How long dropping the connection waits for queued messages to reach the
/// socket.
const FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

/// How often the forwarding thread looks at its stop flag.
const STOP_POLL: Duration = Duration::from_millis(50);

/// The socket `explicit` names, or the default one the plugins look for.
pub fn socket_path(explicit: Option<&Path>) -> Result<PathBuf> {
    match explicit {
        Some(path) => Ok(path.to_path_buf()),
        None => default_socket_path().context("find the default socket path"),
    }
}

/// What the sending side and the forwarding thread share.
struct State {
    stop: AtomicBool,
    /// Why the connection is unusable, once it is.
    broken: Mutex<Option<String>>,
}

impl State {
    /// Record the first reason the connection is unusable.
    fn break_with(&self, why: &str) {
        if let Ok(mut broken) = self.broken.lock() {
            broken.get_or_insert_with(|| why.to_string());
        }
    }

    /// Why the connection is unusable, if it is.
    fn broken(&self) -> Option<String> {
        self.broken.lock().ok().and_then(|broken| broken.clone())
    }
}

/// The sending half of a connection to the service. Dropping it waits for
/// what was sent to reach the socket (a `Client` discards what is still
/// queued when dropped), then ends the forwarding thread.
pub struct ServiceOutput {
    client: Client,
    state: Arc<State>,
    forwarder: Option<JoinHandle<()>>,
}

impl ServiceOutput {
    /// Queue one SysEx message for the service.
    pub fn send(&mut self, message: &[u8]) -> Result<()> {
        if let Some(why) = self.state.broken() {
            bail!("the Device service connection is unusable: {why}");
        }
        if !self.client.raw(message.to_vec()) {
            bail!("the Device service client's send queue is full");
        }
        Ok(())
    }
}

impl Drop for ServiceOutput {
    fn drop(&mut self) {
        if self.state.broken().is_none() && !self.client.flush(FLUSH_TIMEOUT) {
            eprintln!("Warning: the last messages may not have reached the Device service.");
        }
        self.state.stop.store(true, Ordering::SeqCst);
        if let Some(forwarder) = self.forwarder.take() {
            let _ = forwarder.join();
        }
    }
}

/// Connect to the service on `path` (the default socket when `None`) and
/// receive its raw SysEx.
///
/// Fails at once when nothing listens there, and after
/// [`GREETING_TIMEOUT`] when something does not greet; also when the
/// service speaks another protocol version or its link to the INTEGRA-7 is
/// down.
pub fn connect(path: Option<&Path>) -> Result<Connection> {
    let path = socket_path(path)?;
    // The client retries in the background; look once to fail fast.
    UnixStream::connect(&path).with_context(|| {
        format!(
            "no Device service at {} (start one with `integral bridge`)",
            path.display()
        )
    })?;
    let (client, events) = Client::connect(&path);
    // Remembered by the client and sent with its greeting.
    client.receive_raw(true);
    await_greeting(&events, &path)?;
    eprintln!("Using Device service: {}", path.display());

    let (tx, rx) = mpsc::channel();
    let state = Arc::new(State {
        stop: AtomicBool::new(false),
        broken: Mutex::new(None),
    });
    let forwarder = {
        let state = Arc::clone(&state);
        thread::spawn(move || forward(&events, &tx, &state))
    };
    Ok(Connection {
        output: Output::Service(ServiceOutput {
            client,
            state,
            forwarder: Some(forwarder),
        }),
        rx,
    })
}

/// Wait for the greeting; the link to the device must be up.
fn await_greeting(events: &Events, path: &Path) -> Result<()> {
    let deadline = Instant::now() + GREETING_TIMEOUT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match events.wait_event(left) {
            Some(ClientEvent::Connected { session, .. }) => {
                if session.is_none() {
                    bail!("the Device service's link to the INTEGRA-7 is down");
                }
                return Ok(());
            }
            Some(ClientEvent::Incompatible { server }) => bail!(
                "the Device service at {} speaks protocol {server}",
                path.display()
            ),
            Some(_) => {}
            None => bail!("the Device service at {} did not answer", path.display()),
        }
    }
}

/// Pass the SysEx the service relays on to `tx` until told to stop. Ends,
/// and so disconnects `tx`'s receiver, when the connection is lost, the
/// protocol is refused or the link goes down.
fn forward(events: &Events, tx: &mpsc::Sender<Vec<u8>>, state: &State) {
    while !state.stop.load(Ordering::SeqCst) {
        match events.wait_event(STOP_POLL) {
            Some(ClientEvent::Message(Message::Sysex { bytes })) => {
                if tx.send(bytes).is_err() {
                    return;
                }
            }
            Some(ClientEvent::Message(Message::Rejected {
                reason: Reject::LinkDown,
                ..
            })) => return state.break_with("the service's link to the INTEGRA-7 is down"),
            Some(ClientEvent::Message(Message::Rejected { reason, .. })) => {
                eprintln!("Device service refused a request: {reason:?}");
            }
            Some(ClientEvent::Disconnected) => return state.break_with("the connection was lost"),
            // The service dropped what was pending when the session ended
            // without rejecting it: a one-shot run cannot trust its replies.
            Some(ClientEvent::Message(Message::LinkDown)) => {
                return state.break_with("the service's link to the INTEGRA-7 went down");
            }
            Some(ClientEvent::Message(Message::LinkUp { .. })) => {
                return state.break_with("the service's link to the INTEGRA-7 restarted");
            }
            Some(ClientEvent::Incompatible { server }) => {
                return state.break_with(&format!("the service speaks protocol {server}"));
            }
            _ => {}
        }
    }
}
