//! The editor's [`DeviceLink`]: a raw SysEx and note client of the Device
//! service (ADR 0006), so the plugin opens no MIDI port. A SysEx message goes
//! through the service's paced queue; anything else is asked to be played,
//! which the service does ahead of that queue if it is whole channel voice
//! messages, and refuses otherwise ([`Request::midi`]).
//!
//! The page sees one port, [`PORT`], while the instance's own connection
//! ([`Connection`](crate::family::link::Connection)) says the service and the
//! device are up and the editor's client, if open, is connected; otherwise
//! [`ports`](DeviceLink::ports) fails with a status text, which the bridge
//! tells the page once.
//!
//! **A second client, not the instance's.** [`Events`] has one owner, and the
//! instance's worker owns it to track status and Binding. The raw setting is
//! also per client: asking the instance's client for raw SysEx would have the
//! service relay all of the device's SysEx to every instance for as long as
//! it lives, though only an open editor wants it. So [`open`](DeviceLink::open)
//! connects a client of its own, that listens to raw SysEx, and
//! [`close`](DeviceLink::close) drops it. It never binds, so it takes no
//! target. The socket is the instance's ([`Instance::socket`]) and its
//! status the one source of availability. The client's ID is published on
//! the instance ([`Instance::editor_client`]), because the service names it
//! as the writer of what the page changes.
//!
//! **Opening waits for the connection.** A client discards what it sends
//! while it is not connected, so the link reports [`Incoming::Opened`] only
//! once the client's first greeting has arrived (or [`Incoming::OpenFailed`]
//! after [`OPEN_WAIT`]), and [`send`](DeviceLink::send) fails while the client
//! is disconnected (after a service restart, until it is back). What the
//! service refuses is passed on to the page as [`Incoming::Error`].
//!
//! Nothing here runs on the audio thread: the link is driven from the editor's
//! UI thread, and the forwarding thread is its own. Sends go through the
//! service's paced queue and do not wait for the device. A client discards
//! what is still queued when dropped, so closing first waits up to
//! [`FLUSH_TIMEOUT`] for the client to write it to the socket (the service
//! then sends it on, even though the client has gone).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use integral_device_service::protocol::{Message, Request};
use integral_device_service::{Client, ClientEvent, Events};

use super::link::{DeviceLink, Inbound, Incoming, LinkError};
use crate::family::{Instance, Status};

/// The one port the page is offered: the name of its Device service pair,
/// which is also what the page sends its `open` and `send` to.
pub(crate) const PORT: &str = integral_core::device_service::PAIR_NAME;

/// How often the forwarding thread looks at its stop flag, at most.
const STOP_POLL: Duration = Duration::from_millis(50);

/// How long an open waits for the client's first greeting: less than the
/// page waits for its answer.
const OPEN_WAIT: Duration = Duration::from_millis(1500);

/// How long closing waits for queued messages to reach the socket. The UI
/// thread waits, never a real-time one.
const FLUSH_TIMEOUT: Duration = Duration::from_millis(250);

/// A [`DeviceLink`] through the Device service.
pub(crate) struct ServiceLink {
    /// The instance the editor belongs to: its status says whether the
    /// service is usable, and its socket is the one it connects to.
    instance: Arc<Instance>,
    open: Option<Open>,
}

/// The open port: the client and the thread forwarding its events.
struct Open {
    client: Option<Client>,
    stop: Arc<AtomicBool>,
    /// The client is connected to the service (set by the forwarder).
    connected: Arc<AtomicBool>,
    forwarder: Option<JoinHandle<()>>,
    instance: Arc<Instance>,
}

impl Drop for Open {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            // A client discards what it has not written when dropped.
            client.flush(FLUSH_TIMEOUT);
            self.stop.store(true, Ordering::Release);
            // Dropping the last client ends its event stream, which wakes
            // the forwarder.
            drop(client);
        }
        if let Some(forwarder) = self.forwarder.take() {
            let _ = forwarder.join();
        }
        self.instance.set_editor_client(None);
    }
}

impl ServiceLink {
    /// A link for `instance`'s editor to the service the instance connects to.
    pub(crate) fn new(instance: Arc<Instance>) -> Self {
        Self {
            instance,
            open: None,
        }
    }

    /// The instance's status, if it is [`Status::Connected`]; else its text.
    fn require_connected(&self) -> Result<(), &'static str> {
        match self.instance.status.get() {
            Status::Connected => Ok(()),
            status => Err(status.text()),
        }
    }

    /// Whether the open client, if there is one, is connected.
    fn client_connected(&self) -> bool {
        self.open
            .as_ref()
            .is_none_or(|open| open.connected.load(Ordering::Acquire))
    }
}

/// Tell `inbound` what the client's events mean until told to stop (or the
/// client is gone). Keeps `connected` and the instance's editor client ID
/// current.
fn forward(
    events: &Events,
    mut inbound: Inbound,
    stop: &AtomicBool,
    connected: &AtomicBool,
    instance: &Instance,
) {
    let started = Instant::now();
    // Whether `Opened` or `OpenFailed` has been said.
    let mut answered = false;
    while !stop.load(Ordering::Acquire) {
        let waiting = Instant::now();
        match events.wait_event(STOP_POLL) {
            Some(ClientEvent::Connected { client, .. }) => {
                instance.set_editor_client(Some(client));
                connected.store(true, Ordering::Release);
                if !std::mem::replace(&mut answered, true) {
                    inbound(Incoming::Opened);
                }
            }
            Some(ClientEvent::Disconnected) => {
                connected.store(false, Ordering::Release);
                instance.set_editor_client(None);
            }
            Some(ClientEvent::Incompatible { server }) => {
                connected.store(false, Ordering::Release);
                instance.set_editor_client(None);
                let why = format!("the Device service speaks protocol {server}");
                if std::mem::replace(&mut answered, true) {
                    inbound(Incoming::Error(why));
                } else {
                    inbound(Incoming::OpenFailed(why));
                }
                // Final: the client has stopped, and so has its event stream.
                return;
            }
            Some(ClientEvent::Message(Message::Sysex { bytes })) => {
                inbound(Incoming::Receive(&bytes));
            }
            Some(ClientEvent::Message(Message::Rejected { reason, .. })) => {
                inbound(Incoming::Error(format!(
                    "the Device service refused a request: {reason:?}"
                )));
            }
            Some(_) => {}
            // A closed event stream answers at once instead of after the
            // wait: never spin on it while the stop flag is unset.
            None => thread::sleep(STOP_POLL.saturating_sub(waiting.elapsed())),
        }
        if !answered && started.elapsed() >= OPEN_WAIT {
            answered = true;
            inbound(Incoming::OpenFailed(
                "the Device service did not answer".into(),
            ));
        }
    }
}

impl DeviceLink for ServiceLink {
    fn ports(&mut self) -> Result<Vec<String>, LinkError> {
        self.require_connected()
            .map_err(|why| LinkError::Unavailable(why.into()))?;
        if !self.client_connected() {
            return Err(LinkError::Unavailable(
                Status::ServiceNotRunning.text().into(),
            ));
        }
        Ok(vec![PORT.into()])
    }

    fn open(&mut self, port: &str, inbound: Inbound) -> Result<(), LinkError> {
        if port != PORT {
            return Err(LinkError::NoSuchPort(port.into()));
        }
        self.close();
        let connect = |reason: &str| LinkError::Connect {
            port: port.into(),
            reason: reason.into(),
        };
        self.require_connected().map_err(connect)?;
        let socket = self
            .instance
            .socket()
            .ok_or_else(|| connect("the instance has no Device service socket"))?;
        let (client, events) = Client::connect(socket);
        // Remembered by the client and sent with every greeting.
        if !client.receive_raw(true) {
            return Err(connect("the client's queue is full"));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let connected = Arc::new(AtomicBool::new(false));
        let forwarder = {
            let (stop, connected, instance) = (
                Arc::clone(&stop),
                Arc::clone(&connected),
                Arc::clone(&self.instance),
            );
            thread::Builder::new()
                .name("integral-editor-link".into())
                .spawn(move || forward(&events, inbound, &stop, &connected, &instance))
                .map_err(|e| connect(&e.to_string()))?
        };
        self.open = Some(Open {
            client: Some(client),
            stop,
            connected,
            forwarder: Some(forwarder),
            instance: Arc::clone(&self.instance),
        });
        Ok(())
    }

    fn send(&mut self, data: &[u8]) -> Result<(), LinkError> {
        let open = self.open.as_ref().ok_or(LinkError::NotOpen)?;
        // A client discards what it sends while disconnected: say so instead.
        if !self.client_connected() {
            return Err(LinkError::Send(
                "not connected to the Device service".into(),
            ));
        }
        self.require_connected()
            .map_err(|why| LinkError::Send(why.into()))?;
        // SysEx goes through the paced queue; anything else is asked to be
        // played, ahead of it. The service checks either and says what it
        // refuses.
        let queued = open
            .client
            .as_ref()
            .is_some_and(|c| c.send(Request::midi(data.to_vec())));
        if queued {
            Ok(())
        } else {
            Err(LinkError::Send("the send queue is full".into()))
        }
    }

    fn close(&mut self) {
        self.open = None;
    }
}

/// The service fixture the integration tests share.
#[cfg(test)]
#[path = "../../tests/common/mod.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::super::bridge::Bridge;
    use super::super::ipc::PluginMessage;
    use super::super::link::Incoming;
    use super::*;
    use crate::family::link::Connection;
    use integral_core::address::DataSize;
    use integral_core::host_params::PluginKind;
    use integral_core::params::{self, part};
    use integral_core::sysex;
    use std::time::Instant;

    use super::common::{TempDir, TestService, WAIT, wait_for};

    const DEVICE_ID: u8 = 0x10;

    /// What the editor of a connected instance is built from: a service on
    /// a fake link with the link up, an instance connected to it, and a bridge
    /// over a [`ServiceLink`].
    struct Editor {
        // Declared so that the bridge and connection go before the service.
        bridge: Bridge,
        instance: Arc<Instance>,
        _connection: Connection,
        service: Option<TestService>,
        _dir: TempDir,
    }

    impl Editor {
        fn new() -> Self {
            let dir = TempDir::new();
            let service = TestService::start(&dir.socket());
            service.link.bring_up(None);
            let instance = Arc::new(Instance::new(PluginKind::StudioSet));
            let connection = Connection::start(dir.socket(), Arc::clone(&instance));
            wait_for("the instance to connect", || {
                instance.status.get() == Status::Connected
            });
            let link = ServiceLink::new(Arc::clone(&instance));
            Self {
                bridge: Bridge::new(Box::new(link), Arc::clone(&instance)),
                instance,
                _connection: connection,
                service: Some(service),
                _dir: dir,
            }
        }

        /// Close the editor (its bridge and link), keeping the service and
        /// the directory of its socket, which the service needs to stop.
        fn close(self) -> (TestService, TempDir) {
            (self.service.expect("running"), self._dir)
        }

        fn service(&self) -> &TestService {
            self.service.as_ref().expect("running")
        }

        /// Stop the service, as a crash would.
        fn stop_service(&mut self) {
            self.service = None;
        }

        /// A new service on the same socket, with its link up.
        fn start_service(&mut self) {
            let service = TestService::start(&self._dir.socket());
            service.link.bring_up(None);
            self.service = Some(service);
        }

        /// The page posts `text`.
        fn page_posts(&mut self, text: &str) {
            self.bridge.handle(text);
        }

        /// Messages for the page until one satisfies `pick`.
        fn page_hears<T>(&self, what: &str, pick: impl Fn(&PluginMessage) -> Option<T>) -> T {
            let deadline = Instant::now() + WAIT;
            loop {
                for message in self.bridge.take_outgoing() {
                    if let Some(found) = pick(&message) {
                        return found;
                    }
                }
                assert!(Instant::now() < deadline, "timed out waiting for {what}");
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    fn rq1(part_index: u8) -> Vec<u8> {
        let size = DataSize::from_value(1).expect("one byte");
        sysex::build_rq1(
            DEVICE_ID,
            &params::part_address(part_index, part::LEVEL),
            &size,
        )
    }

    fn open_message() -> String {
        format!(r#"{{"type":"open","port":"{PORT}"}}"#)
    }

    fn send_message(data: &[u8]) -> String {
        format!(r#"{{"type":"send","port":"{PORT}","data":{data:?}}}"#)
    }

    #[test]
    fn the_page_is_offered_the_service_and_nothing_else() {
        let mut editor = Editor::new();
        editor.page_posts(r#"{"type":"ready"}"#);
        let ports = editor.page_hears("the ports", |m| match m {
            PluginMessage::Ports { ports } => Some(ports.clone()),
            _ => None,
        });
        assert_eq!(ports, [PORT]);
    }

    #[test]
    fn a_burst_of_part_level_requests_reaches_the_fake_link_through_the_services_queue() {
        let mut editor = Editor::new();
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_posts(&open_message());
        editor.page_hears("the port to open", |m| {
            matches!(m, PluginMessage::Opened { .. }).then_some(())
        });

        // A burst the way the page makes it: one RQ1 for each Part's level.
        let load: Vec<Vec<u8>> = (0..16).map(rq1).collect();
        for request in &load {
            editor.page_posts(&send_message(request));
        }
        wait_for("the whole load at the link", || {
            editor.service().link.sent().len() >= load.len()
        });
        assert_eq!(editor.service().link.sent(), load);

        // The device's answer comes back to the page, on the port.
        let reply = sysex::build_dt1(DEVICE_ID, &params::part_address(3, part::LEVEL), &[91]);
        editor.service().link.device_says(&reply);
        let heard = editor.page_hears("the reply", |m| match m {
            PluginMessage::Receive { port, data } if port == PORT => Some(data.clone()),
            _ => None,
        });
        assert_eq!(heard, reply);
    }

    #[test]
    fn the_last_message_reaches_the_link_after_the_editor_closes() {
        let mut editor = Editor::new();
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_posts(&open_message());
        editor.page_hears("the port to open", |m| {
            matches!(m, PluginMessage::Opened { .. }).then_some(())
        });
        let load: Vec<Vec<u8>> = (0..8).map(rq1).collect();
        for request in &load {
            editor.page_posts(&send_message(request));
        }
        // Closing the editor right away: the page and its link go.
        // (A tuple drops the service before the directory it needs to stop.)
        let closed = editor.close();
        let service = &closed.0;
        wait_for("the whole load at the link", || {
            service.link.sent().len() >= load.len()
        });
        assert_eq!(service.link.sent(), load);
    }

    #[test]
    fn a_note_from_the_page_reaches_the_fake_link_before_queued_sysex() {
        const NOTE_ON: [u8; 3] = [0x99, 0x24, 0x64];
        const NOTE_OFF: [u8; 3] = [0x89, 0x24, 0x00];
        let mut editor = Editor::new();
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_posts(&open_message());
        editor.page_hears("the port to open", |m| {
            matches!(m, PluginMessage::Opened { .. }).then_some(())
        });
        // SysEx waits as it would behind the throttle; notes do not.
        editor.service().link.hold_sysex(Duration::from_secs(60));

        let request = rq1(0);
        editor.page_posts(&send_message(&request));
        editor.page_posts(&send_message(&NOTE_ON));
        wait_for("the note at the link", || {
            editor.service().link.sent().contains(&NOTE_ON.to_vec())
        });
        assert!(
            !editor.service().link.sent().contains(&request),
            "the RQ1 was held"
        );

        editor.service().link.hold_sysex(Duration::ZERO);
        // Another note wakes the service if it sleeps, and it sends what it held.
        editor.page_posts(&send_message(&NOTE_OFF));
        wait_for("the RQ1 at the link", || {
            editor.service().link.sent().contains(&request)
        });
        let sent = editor.service().link.sent();
        let at = |m: &[u8]| sent.iter().position(|s| s == m).expect("sent");
        assert!(at(&NOTE_ON) < at(&request));
        assert!(
            !editor
                .bridge
                .take_outgoing()
                .iter()
                .any(|m| matches!(m, PluginMessage::Error { .. })),
            "nothing refused"
        );
    }

    #[test]
    fn without_the_service_the_page_is_told_the_status_and_offered_nothing() {
        let instance = Arc::new(Instance::new(PluginKind::StudioSet));
        let mut link = ServiceLink::new(instance);
        let error = link.ports().expect_err("no service");
        assert_eq!(error.to_string(), Status::ServiceNotRunning.text());
        assert!(matches!(
            link.open(PORT, Box::new(|_| {})),
            Err(LinkError::Connect { .. })
        ));
    }

    #[test]
    fn the_status_follows_the_device() {
        let mut editor = Editor::new();
        editor.service().link.bring_down();
        wait_for("the status to follow", || {
            editor.instance.status.get() == Status::DeviceOffline
        });
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_hears("no ports", |m| {
            matches!(m, PluginMessage::Ports { ports } if ports.is_empty()).then_some(())
        });
    }

    #[test]
    fn the_editors_client_is_published_while_it_is_open() {
        let mut editor = Editor::new();
        assert_eq!(editor.instance.editor_client(), None);
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_posts(&open_message());
        editor.page_hears("the port to open", |m| {
            matches!(m, PluginMessage::Opened { .. }).then_some(())
        });
        // The open is answered once the client is greeted, ID known.
        assert!(editor.instance.editor_client().is_some(), "published");
        let instance = Arc::clone(&editor.instance);
        let _closed = editor.close();
        assert_eq!(instance.editor_client(), None);
    }

    #[test]
    fn a_service_restart_under_an_open_editor_is_survived() {
        let mut editor = Editor::new();
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_posts(&open_message());
        editor.page_hears("the port to open", |m| {
            matches!(m, PluginMessage::Opened { .. }).then_some(())
        });

        editor.stop_service();
        wait_for("the editor's client to notice", || {
            editor.instance.editor_client().is_none()
        });
        // While the service is away nothing is offered and a send says so.
        editor.bridge.refresh_ports();
        editor.page_hears("no ports", |m| {
            matches!(m, PluginMessage::Ports { ports } if ports.is_empty()).then_some(())
        });
        editor.page_posts(&send_message(&rq1(0)));
        let port = editor.page_hears("the failed send", |m| match m {
            PluginMessage::Error { port, .. } => Some(port.clone()),
            _ => None,
        });
        assert_eq!(port.as_deref(), Some(PORT));

        editor.start_service();
        wait_for("the editor's client to be back", || {
            editor.instance.status.get() == Status::Connected
                && editor.instance.editor_client().is_some()
        });
        editor.bridge.refresh_ports();
        editor.page_hears("the port again", |m| {
            matches!(m, PluginMessage::Ports { ports } if ports == &[PORT]).then_some(())
        });
        editor.page_posts(&send_message(&rq1(1)));
        wait_for("the send at the new link", || {
            editor.service().link.sent() == [rq1(1)]
        });
    }

    #[test]
    fn what_the_service_refuses_reaches_the_page() {
        let mut editor = Editor::new();
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_posts(&open_message());
        editor.page_hears("the port to open", |m| {
            matches!(m, PluginMessage::Opened { .. }).then_some(())
        });
        // A note cut short: the service plays nothing and says why.
        editor.page_posts(&send_message(&[0x90, 0x3C]));
        let port = editor.page_hears("the refusal", |m| match m {
            PluginMessage::Error { port, .. } => Some(port.clone()),
            _ => None,
        });
        assert_eq!(port.as_deref(), Some(PORT));
        assert!(editor.service().link.sent().is_empty());
    }

    #[test]
    fn an_open_with_no_service_behind_it_fails_instead_of_dropping_sends() {
        let dir = TempDir::new();
        let instance = Arc::new(Instance::new(PluginKind::StudioSet));
        // The instance thinks all is well, but nothing listens at the socket.
        let _connection = Connection::start(dir.socket(), Arc::clone(&instance));
        instance.status.set(Status::Connected);
        let mut link = ServiceLink::new(instance);
        let (tx, rx) = std::sync::mpsc::channel();
        link.open(
            PORT,
            Box::new(move |incoming| {
                if let Incoming::OpenFailed(why) = incoming {
                    let _ = tx.send(why);
                }
            }),
        )
        .expect("begins");
        assert!(matches!(link.send(&[0xF0, 0xF7]), Err(LinkError::Send(_))));
        assert!(link.ports().is_err());
        let why = rx.recv_timeout(WAIT).expect("gives up");
        assert!(why.contains("did not answer"), "{why}");
    }

    #[test]
    fn another_port_does_not_exist_and_a_closed_link_does_not_send() {
        let mut editor = Editor::new();
        editor.page_posts(r#"{"type":"ready"}"#);
        editor.page_posts(r#"{"type":"open","port":"INTEGRA-7"}"#);
        editor.page_hears("the refusal", |m| {
            matches!(m, PluginMessage::OpenFailed { .. }).then_some(())
        });
        let mut link = ServiceLink::new(Arc::clone(&editor.instance));
        assert!(matches!(link.send(&[0xF0, 0xF7]), Err(LinkError::NotOpen)));
    }
}
