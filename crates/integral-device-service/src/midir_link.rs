//! A link over a MIDI port pair: an RTP-MIDI session or a USB port.
//!
//! Both look the same to the service, a named input and output port, so one
//! type serves them and a [`PortMatch`] says which to open. A pair that
//! exists is not a device that answers, so the link reports up only on an
//! Identity Reply and down when probes go unanswered (see [`crate::probe`]).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use integral_core::device::THROTTLE_MS;
use integral_core::sysex;
use integral_core::wireless::RECONNECT_INTERVAL_MS;
use midir::{Ignore, MidiIO, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};

use crate::link::{DeviceLink, LinkError, LinkSink, Log};
use crate::probe::{Action, Liveness, Timing};

/// MIDI client name the link shows the system.
const CLIENT: &str = "Integral";

/// Which port pair a [`MidirLink`] opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortMatch {
    /// A port with exactly this name (a USB port).
    Exact(String),
    /// A port whose name contains this, ignoring case (an RTP-MIDI session,
    /// whose name the user chose).
    Contains(String),
}

impl PortMatch {
    /// Whether a port called `name` matches.
    pub fn matches(&self, name: &str) -> bool {
        match self {
            Self::Exact(wanted) => name == wanted,
            Self::Contains(part) => name.to_lowercase().contains(&part.to_lowercase()),
        }
    }
}

/// Names of the ports of one direction.
fn names<T: MidiIO>(io: &T) -> Vec<String> {
    io.ports()
        .iter()
        .filter_map(|p| io.port_name(p).ok())
        .collect()
}

/// The port called exactly `name`.
pub fn port_named<T: MidiIO>(io: &T, name: &str) -> Option<T::Port> {
    io.ports()
        .into_iter()
        .find(|p| io.port_name(p).is_ok_and(|n| n == name))
}

/// Names matching `wanted` that exist as both an input and an output port,
/// in input order, once each. A device is a pair: the service's own note
/// port is an input only, so it never matches.
///
/// # Errors
///
/// The MIDI system's reason, if it is unavailable.
pub fn matching_pairs(wanted: &PortMatch) -> Result<Vec<String>, String> {
    let input = MidiInput::new(CLIENT).map_err(|e| e.to_string())?;
    let output = MidiOutput::new(CLIENT).map_err(|e| e.to_string())?;
    let outputs = names(&output);
    let mut pairs: Vec<String> = Vec::new();
    for name in names(&input) {
        if wanted.matches(&name) && outputs.contains(&name) && !pairs.contains(&name) {
            pairs.push(name);
        }
    }
    Ok(pairs)
}

/// Keeps SysEx the core's throttle apart whoever sends it: the service, or
/// the link's own probes. The service paces its own messages; this covers a
/// probe next to one of them, in both orders, without ever blocking: a probe
/// is skipped after a service SysEx, and the service is told to hold its
/// SysEx after a probe ([`DeviceLink::sysex_gap`]). One lock covers the
/// check, the record and the write, so the two cannot interleave.
#[derive(Default)]
struct Pace(Mutex<Marks>);

#[derive(Default)]
struct Marks {
    service: Option<Instant>,
    probe: Option<Instant>,
}

impl Pace {
    fn spacing() -> Duration {
        Duration::from_secs_f64(THROTTLE_MS / 1000.0)
    }

    /// How much longer to leave the wire alone as of `now`, after a SysEx
    /// sent at `last`.
    fn left(last: Option<Instant>, now: Instant) -> Duration {
        last.map_or(Duration::ZERO, |at| {
            (at + Self::spacing()).saturating_duration_since(now)
        })
    }

    fn marks(&self) -> std::sync::MutexGuard<'_, Marks> {
        self.0.lock().expect("pace lock")
    }

    /// How long a probe waits after the service's last SysEx.
    fn after_service(&self, now: Instant) -> Duration {
        Self::left(self.marks().service, now)
    }

    /// How long the service waits after the last probe.
    fn after_probe(&self, now: Instant) -> Duration {
        Self::left(self.marks().probe, now)
    }
}

#[derive(Clone)]
struct Wire {
    output: Arc<Mutex<Option<MidiOutputConnection>>>,
    pace: Arc<Pace>,
}

impl Wire {
    fn new() -> Self {
        Self {
            output: Arc::default(),
            pace: Arc::default(),
        }
    }

    fn write(&self, message: &[u8]) -> Result<(), LinkError> {
        let mut output = self.output.lock().expect("output lock");
        let out = output.as_mut().ok_or(LinkError::Down)?;
        out.send(message).map_err(|e| LinkError::Io(e.to_string()))
    }

    /// Send one of the service's messages.
    fn send(&self, message: &[u8]) -> Result<(), LinkError> {
        if message.first() == Some(&0xF0) {
            let mut marks = self.pace.marks();
            marks.service = Some(Instant::now());
            return self.write(message);
        }
        self.write(message)
    }

    /// Send a probe unless a service SysEx went out within the spacing: the
    /// check, the record and the write happen under one lock. Returns
    /// whether the probe was sent.
    fn probe(&self) -> Result<bool, LinkError> {
        let mut marks = self.pace.marks();
        let now = Instant::now();
        if !Pace::left(marks.service, now).is_zero() {
            return Ok(false);
        }
        marks.probe = Some(now);
        self.write(&sysex::identity_request())?;
        Ok(true)
    }
}

/// A [`DeviceLink`] over a MIDI input and output port of a matching name.
pub struct MidirLink {
    wanted: PortMatch,
    timing: Timing,
    wire: Wire,
    stop: Arc<AtomicBool>,
    log: Log,
}

impl MidirLink {
    /// A link that opens the first port pair matching `wanted`.
    pub fn new(wanted: PortMatch, log: Log) -> Self {
        Self {
            wanted,
            timing: Timing::default(),
            wire: Wire::new(),
            stop: Arc::default(),
            log,
        }
    }

    /// Probe with other timing than the default.
    #[must_use]
    pub fn with_timing(mut self, timing: Timing) -> Self {
        self.timing = timing;
        self
    }
}

/// An open pair: the input connection stays open for its callback.
struct Open {
    name: String,
    _input: MidiInputConnection<()>,
    messages: mpsc::Receiver<Vec<u8>>,
}

/// Open the first pair matching `wanted`, storing its output in `wire`.
fn open(wanted: &PortMatch, wire: &Wire) -> Result<Open, String> {
    let name = matching_pairs(wanted)?
        .into_iter()
        .next()
        .ok_or("no matching port pair")?;
    let mut input = MidiInput::new(CLIENT).map_err(|e| e.to_string())?;
    // Pass everything through: raw clients see what a shared port shows.
    input.ignore(Ignore::None);
    let out = MidiOutput::new(CLIENT).map_err(|e| e.to_string())?;
    let in_port = port_named(&input, &name).ok_or("the input port vanished")?;
    let out_port = port_named(&out, &name).ok_or("the output port vanished")?;
    let out = out
        .connect(&out_port, "Integral Device Service out")
        .map_err(|e| e.to_string())?;
    let (tx, messages) = mpsc::channel();
    let input = input
        .connect(
            &in_port,
            "Integral Device Service in",
            move |_, msg, ()| {
                let _ = tx.send(msg.to_vec());
            },
            (),
        )
        .map_err(|e| e.to_string())?;
    *wire.output.lock().map_err(|_| "output lock poisoned")? = Some(out);
    Ok(Open {
        name,
        _input: input,
        messages,
    })
}

/// What the link's thread owns.
struct Worker {
    wanted: PortMatch,
    timing: Timing,
    wire: Wire,
    stop: Arc<AtomicBool>,
    log: Log,
    events: LinkSink,
}

impl Worker {
    /// Open the pair, probe it, and forward what the device says, until the
    /// link is dropped.
    fn run(self) {
        let start = Instant::now();
        let now = || start.elapsed().as_secs_f64() * 1000.0;
        let port_check = Duration::from_millis(RECONNECT_INTERVAL_MS);
        let mut waiting_logged = false;
        while !self.stop.load(Ordering::Relaxed) {
            let open = match open(&self.wanted, &self.wire) {
                Ok(open) => open,
                Err(reason) => {
                    if !waiting_logged {
                        (self.log)(&format!("Waiting for the MIDI port ({reason})"));
                        waiting_logged = true;
                    }
                    thread::sleep(port_check);
                    continue;
                }
            };
            waiting_logged = false;
            (self.log)(&format!(
                "MIDI port '{}' open; probing the device",
                open.name
            ));
            let was_up = self.serve(&open, Liveness::new(self.timing, now()), port_check, &now);
            *self.wire.output.lock().expect("output lock") = None;
            if was_up {
                self.events.down();
            }
        }
    }

    /// Serve one open pair until it vanishes, a send fails or the link is
    /// dropped. Returns whether the link was up at the end.
    fn serve(
        &self,
        open: &Open,
        mut liveness: Liveness,
        port_check: Duration,
        now: &dyn Fn() -> f64,
    ) -> bool {
        let port_check_ms = port_check.as_secs_f64() * 1000.0;
        let mut next_port_check = now() + port_check_ms;
        while !self.stop.load(Ordering::Relaxed) {
            // A probe due right after a SysEx waits out the spacing.
            let gap = self.wire.pace.after_service(Instant::now());
            let probe_due = liveness.next_deadline() <= now();
            let wait = if probe_due && !gap.is_zero() {
                gap.as_secs_f64() * 1000.0
            } else {
                for action in liveness.tick(now()) {
                    match action {
                        Action::Probe => {
                            if let Err(e) = self.wire.probe() {
                                (self.log)(&format!(
                                    "MIDI port '{}': probe failed: {e}",
                                    open.name
                                ));
                                return liveness.is_up();
                            }
                        }
                        Action::Down => {
                            (self.log)("The device stopped answering");
                            self.events.down();
                        }
                        Action::Up(_) => unreachable!("only a message brings the link up"),
                    }
                }
                liveness.next_deadline().min(next_port_check) - now()
            };
            match open
                .messages
                .recv_timeout(Duration::from_secs_f64(wait.max(1.0) / 1000.0))
            {
                Ok(message) => {
                    let action = liveness.on_message(&message, now());
                    if let Some(Action::Up(device_id)) = action {
                        (self.log)(&format!("Device {device_id:02X} answered"));
                        self.events.up(Some(device_id));
                    }
                    if liveness.is_up() {
                        self.events.midi(&message);
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return liveness.is_up(),
            }
            if now() >= next_port_check {
                next_port_check = now() + port_check_ms;
                let still_there =
                    MidiOutput::new(CLIENT).is_ok_and(|out| names(&out).contains(&open.name));
                if !still_there {
                    (self.log)(&format!("MIDI port '{}' is gone", open.name));
                    return liveness.is_up();
                }
            }
        }
        liveness.is_up()
    }
}

impl DeviceLink for MidirLink {
    fn start(&mut self, events: LinkSink) -> Result<(), LinkError> {
        // Fail now, rather than retrying forever, if there is no MIDI system.
        MidiInput::new(CLIENT).map_err(|e| LinkError::Io(e.to_string()))?;
        let worker = Worker {
            wanted: self.wanted.clone(),
            timing: self.timing,
            wire: self.wire.clone(),
            stop: Arc::clone(&self.stop),
            log: Arc::clone(&self.log),
            events,
        };
        thread::Builder::new()
            .name("midir-link".into())
            .spawn(move || worker.run())
            .map_err(|e| LinkError::Io(e.to_string()))?;
        Ok(())
    }

    fn send(&mut self, message: &[u8]) -> Result<(), LinkError> {
        self.wire.send(message)
    }

    fn sysex_gap(&self) -> Duration {
        self.wire.pace.after_probe(Instant::now())
    }
}

impl Drop for MidirLink {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_matches_only_the_whole_name() {
        let wanted = PortMatch::Exact("INTEGRA-7".into());
        assert!(wanted.matches("INTEGRA-7"));
        assert!(!wanted.matches("INTEGRA-7 (Wireless)"));
        assert!(!wanted.matches("integra-7"));
    }

    #[test]
    fn each_side_waits_out_the_other() {
        let pace = Pace::default();
        let t0 = Instant::now();
        assert_eq!(pace.after_probe(t0), Duration::ZERO, "nothing sent yet");
        pace.marks().probe = Some(t0);
        assert_eq!(pace.after_probe(t0), Pace::spacing());
        assert!(pace.after_probe(t0 + Duration::from_millis(10)) > Duration::ZERO);
        assert_eq!(pace.after_probe(t0 + Pace::spacing()), Duration::ZERO);
        assert_eq!(
            pace.after_service(t0),
            Duration::ZERO,
            "probes don't hold probes"
        );
        pace.marks().service = Some(t0);
        assert_eq!(pace.after_service(t0), Pace::spacing());
    }

    #[test]
    fn contains_ignores_case() {
        let wanted = PortMatch::Contains("network integra".into());
        assert!(wanted.matches("Network Integra"));
        assert!(!wanted.matches("Network Other"));
    }
}
