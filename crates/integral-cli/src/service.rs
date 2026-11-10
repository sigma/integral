//! The Device service as a transport: `--service` makes the CLI a raw SysEx
//! client of the service ([ADR 0006](../../../docs/adr/0006-device-service-owns-the-link.md))
//! instead of opening a MIDI port.
//!
//! [`connect`] gives the same [`Connection`] as a port, so no command knows
//! which transport it runs on: [`ServiceOutput`] sends through the service's
//! paced queue, and a thread turns the SysEx the service relays into the
//! receiver's messages. An Expansion load is the exception: the service runs
//! it, holding its queue for every client meanwhile
//! ([`ServiceOutput::load_expansion`]).
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
use integral_core::expansion_load::LoadOutcome;
use integral_core::expansion_slots::ExpansionSlots;
use integral_device_service::protocol::{ExpansionLoadResult, Message, Reject, Request};
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
    /// An Expansion load waits for its answer: refusals are its.
    loading: AtomicBool,
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
    /// The answers to an Expansion load, from the forwarding thread.
    answers: mpsc::Receiver<Message>,
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

    /// Have the service load `target` into the Expansion slots
    /// ([`Request::LoadExpansion`]) and wait up to `wait` for the outcome.
    ///
    /// # Errors
    ///
    /// When the service gives no outcome: the connection is unusable or is
    /// lost before the answer, the service predates the request, or it
    /// refused it (the slots could not be read, another load runs). Only a
    /// lost connection leaves it unknown whether the load was sent.
    pub fn load_expansion(
        &mut self,
        target: &ExpansionSlots,
        wait: Duration,
    ) -> Result<LoadOutcome> {
        if let Some(why) = self.state.broken() {
            bail!("the Device service connection is unusable: {why}");
        }
        self.state.loading.store(true, Ordering::SeqCst);
        let outcome = self.await_load(target, wait);
        self.state.loading.store(false, Ordering::SeqCst);
        outcome
    }

    fn await_load(&mut self, target: &ExpansionSlots, wait: Duration) -> Result<LoadOutcome> {
        let slots = target.values();
        if !self.client.send(Request::LoadExpansion { slots }) {
            bail!("the Device service client's send queue is full");
        }
        let deadline = Instant::now() + wait;
        loop {
            match self.answers.recv_timeout(STOP_POLL) {
                Ok(Message::ExpansionLoadDone { result, slots }) => {
                    let slots = slots
                        .map(|slots| ExpansionSlots::from_values(&slots))
                        .transpose()
                        .context("the Device service answered unknown Expansion slots")?;
                    return match (result, slots) {
                        (ExpansionLoadResult::Unchanged, _) => Ok(LoadOutcome::NoOp),
                        (ExpansionLoadResult::Loaded, Some(slots)) => {
                            Ok(LoadOutcome::Loaded(slots))
                        }
                        (ExpansionLoadResult::Failed, Some(slots)) => {
                            Ok(LoadOutcome::Failed(slots))
                        }
                        (ExpansionLoadResult::TimedOut, _) => Ok(LoadOutcome::TimedOut),
                        (_, None) => bail!("the Device service answered no Expansion slots"),
                    };
                }
                Ok(Message::Rejected {
                    reason: Reject::Timeout,
                    ..
                }) => bail!("the device did not answer the Expansion slot read"),
                Ok(Message::Rejected {
                    reason: Reject::Malformed { .. },
                    ..
                }) => bail!(
                    "this Device service cannot load Expansions; update it, or run \
                     `expansion load` without --service while it is stopped"
                ),
                Ok(Message::Rejected { reason, .. }) => {
                    bail!("the Device service refused the load: {reason:?}")
                }
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // The forwarding thread ended: the connection broke.
                    let why = self.state.broken().unwrap_or_default();
                    bail!("{why} before the load's answer; the Expansion slots are unknown");
                }
            }
            if Instant::now() >= deadline {
                return Ok(LoadOutcome::TimedOut);
            }
        }
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
    let (answer_tx, answers) = mpsc::channel();
    let state = Arc::new(State {
        stop: AtomicBool::new(false),
        loading: AtomicBool::new(false),
        broken: Mutex::new(None),
    });
    let forwarder = {
        let state = Arc::clone(&state);
        thread::spawn(move || forward(&events, &tx, &answer_tx, &state))
    };
    Ok(Connection {
        output: Output::Service(ServiceOutput {
            client,
            state,
            answers,
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

/// Pass the SysEx the service relays on to `tx`, and the answers to an
/// Expansion load to `answers`, until told to stop. Ends, and so disconnects
/// both receivers, when the connection is lost, the protocol is refused or
/// the link goes down.
fn forward(
    events: &Events,
    tx: &mpsc::Sender<Vec<u8>>,
    answers: &mpsc::Sender<Message>,
    state: &State,
) {
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
            Some(ClientEvent::Message(
                answer @ (Message::ExpansionLoadDone { .. } | Message::Rejected { .. }),
            )) if state.loading.load(Ordering::SeqCst) => {
                let _ = answers.send(answer);
            }
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
