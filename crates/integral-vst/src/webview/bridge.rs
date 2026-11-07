//! The plugin's half of the plugin transport: page messages in, link calls
//! out, and what the page should hear queued for delivery.
//!
//! The bridge does not throttle: the page paces its sends with core's
//! `DeviceState`, as on the web (ADR 0004).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::ipc::{self, PageMessage, PluginMessage};
use super::link::{DeviceLink, Incoming, LinkError};

/// Messages waiting for the page, beyond which the oldest are dropped: a
/// page that stops draining (a hidden window) must not grow memory forever.
const OUTBOX_LIMIT: usize = 8192;

/// The smallest and largest window side the page may ask for, logical px.
const WINDOW_SIDE: std::ops::RangeInclusive<u32> = 200..=16384;

/// What the page should hear, filled from the UI thread and the link's.
#[derive(Default)]
struct Outbox(Mutex<VecDeque<PluginMessage>>);

impl Outbox {
    fn push(&self, message: PluginMessage) {
        let mut queue = self.0.lock().expect("outbox lock");
        if queue.len() == OUTBOX_LIMIT {
            queue.pop_front();
        }
        queue.push_back(message);
    }

    fn take(&self) -> Vec<PluginMessage> {
        self.0.lock().expect("outbox lock").drain(..).collect()
    }
}

/// Where the page's `open` has got to.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum OpenState {
    /// Nothing is open.
    #[default]
    Closed,
    /// The link began to open the pair and has not said it is ready.
    Opening(String),
    /// The link said the pair is open.
    Open(String),
}

/// Connects one page to one [`DeviceLink`].
pub(crate) struct Bridge {
    link: Box<dyn DeviceLink>,
    outbox: Arc<Outbox>,
    /// Whether the page said `ready`; nothing is delivered before.
    ready: bool,
    /// The pair open on the link, and whether it has reported itself open.
    open: Arc<Mutex<OpenState>>,
    /// The port list the page last heard.
    listed: Option<Vec<String>>,
    /// Why the link was last unavailable, as the page heard it.
    told: Option<String>,
    /// The window size the page asked for and the editor hasn't applied.
    resize: Option<(u32, u32)>,
}

impl Bridge {
    /// A bridge over `link`, with nothing open.
    pub(crate) fn new(link: Box<dyn DeviceLink>) -> Self {
        Self {
            link,
            outbox: Arc::default(),
            ready: false,
            open: Arc::default(),
            listed: None,
            told: None,
            resize: None,
        }
    }

    /// Handle one message the page posted.
    pub(crate) fn handle(&mut self, text: &str) {
        match ipc::decode(text) {
            Ok(message) => self.dispatch(message),
            Err(e) => self.outbox.push(PluginMessage::Error {
                port: None,
                message: e.to_string(),
            }),
        }
    }

    fn dispatch(&mut self, message: PageMessage) {
        match message {
            PageMessage::Ready => {
                // A reloaded page starts over: it re-lists and re-opens, and
                // what the last page left unread is not its business.
                self.ready = true;
                self.outbox.take();
                self.listed = None;
                self.told = None;
                self.refresh_ports();
            }
            PageMessage::Open { port } => self.open(port),
            PageMessage::Send { port, data } => {
                let is_open =
                    *self.open.lock().expect("open lock") == OpenState::Open(port.clone());
                let result = if is_open {
                    self.link.send(&data)
                } else {
                    Err(LinkError::NotOpen)
                };
                if let Err(e) = result {
                    self.outbox.push(PluginMessage::Error {
                        port: Some(port),
                        message: e.to_string(),
                    });
                }
            }
            PageMessage::Resize { width, height } => {
                if WINDOW_SIDE.contains(&width) && WINDOW_SIDE.contains(&height) {
                    self.resize = Some((width, height));
                } else {
                    self.outbox.push(PluginMessage::Error {
                        port: None,
                        message: format!("window size {width}×{height} out of range"),
                    });
                }
            }
        }
    }

    fn open(&mut self, port: String) {
        {
            let state = self.open.lock().expect("open lock");
            match &*state {
                OpenState::Open(open) if *open == port => {
                    self.outbox.push(PluginMessage::Opened { port });
                    return;
                }
                // The answer is on its way.
                OpenState::Opening(opening) if *opening == port => return,
                _ => {}
            }
        }
        let (outbox, state, name) = (self.outbox.clone(), self.open.clone(), port.clone());
        let inbound = Box::new(move |incoming: Incoming<'_>| {
            let mut state = state.lock().expect("open lock");
            let message = match incoming {
                Incoming::Opened => {
                    *state = OpenState::Open(name.clone());
                    PluginMessage::Opened { port: name.clone() }
                }
                Incoming::OpenFailed(message) => {
                    *state = OpenState::Closed;
                    PluginMessage::OpenFailed {
                        port: name.clone(),
                        message,
                    }
                }
                Incoming::Receive(data) => PluginMessage::Receive {
                    port: name.clone(),
                    data: data.to_vec(),
                },
                Incoming::Error(message) => PluginMessage::Error {
                    port: Some(name.clone()),
                    message,
                },
            };
            outbox.push(message);
        });
        *self.open.lock().expect("open lock") = OpenState::Opening(port.clone());
        match self.link.open(&port, inbound) {
            Ok(()) => {}
            Err(e) => {
                *self.open.lock().expect("open lock") = OpenState::Closed;
                self.outbox.push(PluginMessage::OpenFailed {
                    port,
                    message: e.to_string(),
                });
            }
        }
    }

    /// Re-list the link's ports and tell a ready page if they changed.
    pub(crate) fn refresh_ports(&mut self) {
        if !self.ready {
            return;
        }
        let ports = match self.link.ports() {
            Ok(ports) => {
                self.told = None;
                ports
            }
            Err(e) => {
                // Said once per change of reason, not on every poll.
                let why = e.to_string();
                if self.told.as_ref() != Some(&why) {
                    self.told = Some(why.clone());
                    self.outbox.push(PluginMessage::Error {
                        port: None,
                        message: why,
                    });
                }
                Vec::new()
            }
        };
        if self.listed.as_ref() != Some(&ports) {
            self.listed = Some(ports.clone());
            self.outbox.push(PluginMessage::Ports { ports });
        }
    }

    /// What the page should hear now, in order; nothing before `ready`.
    pub(crate) fn take_outgoing(&self) -> Vec<PluginMessage> {
        if self.ready {
            self.outbox.take()
        } else {
            Vec::new()
        }
    }

    /// The window size the page asked for since the last call.
    pub(crate) fn take_resize(&mut self) -> Option<(u32, u32)> {
        self.resize.take()
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        // The page is gone with the editor; so is its link.
        self.link.close();
    }
}

#[cfg(test)]
mod tests {
    use super::super::link::{Inbound, Incoming, LinkError};
    use super::*;
    use std::sync::Mutex;

    /// What the fake link was asked to do, and its device side.
    #[derive(Default)]
    struct Device {
        ports: Vec<String>,
        unavailable: bool,
        refuse: bool,
        /// The link has not said the pair is open yet.
        slow: bool,
        opened: Vec<String>,
        sent: Vec<Vec<u8>>,
        closed: usize,
        inbound: Option<Inbound>,
    }

    /// A [`DeviceLink`] over a shared [`Device`].
    struct FakeLink(Arc<Mutex<Device>>);

    impl DeviceLink for FakeLink {
        fn ports(&mut self) -> Result<Vec<String>, LinkError> {
            let d = self.0.lock().unwrap();
            if d.unavailable {
                return Err(LinkError::Unavailable("no CoreMIDI".into()));
            }
            Ok(d.ports.clone())
        }
        fn open(&mut self, port: &str, inbound: Inbound) -> Result<(), LinkError> {
            let mut d = self.0.lock().unwrap();
            d.inbound = None;
            if d.refuse || !d.ports.iter().any(|p| p == port) {
                return Err(LinkError::NoSuchPort(port.into()));
            }
            d.opened.push(port.into());
            let mut inbound = inbound;
            if !d.slow {
                inbound(Incoming::Opened);
            }
            d.inbound = Some(inbound);
            Ok(())
        }
        fn send(&mut self, data: &[u8]) -> Result<(), LinkError> {
            self.0.lock().unwrap().sent.push(data.to_vec());
            Ok(())
        }
        fn close(&mut self) {
            let mut d = self.0.lock().unwrap();
            d.closed += 1;
            d.inbound = None;
        }
    }

    fn bridge(ports: &[&str]) -> (Bridge, Arc<Mutex<Device>>) {
        let device = Arc::new(Mutex::new(Device {
            ports: ports.iter().map(|p| p.to_string()).collect(),
            ..Device::default()
        }));
        (Bridge::new(Box::new(FakeLink(device.clone()))), device)
    }

    /// The device sends `data` on the open pair.
    fn device_sends(device: &Arc<Mutex<Device>>, data: &[u8]) {
        let mut inbound = device.lock().unwrap().inbound.take().expect("open");
        inbound(Incoming::Receive(data));
        device.lock().unwrap().inbound = Some(inbound);
    }

    fn ports(names: &[&str]) -> PluginMessage {
        PluginMessage::Ports {
            ports: names.iter().map(|n| n.to_string()).collect(),
        }
    }

    #[test]
    fn ready_lists_the_ports() {
        let (mut b, _) = bridge(&["INTEGRA-7"]);
        b.handle(r#"{"type":"ready"}"#);
        assert_eq!(b.take_outgoing(), vec![ports(&["INTEGRA-7"])]);
    }

    #[test]
    fn nothing_is_delivered_before_ready() {
        let (mut b, _) = bridge(&["A"]);
        b.handle(r#"{"type":"open","port":"A"}"#);
        b.refresh_ports();
        assert_eq!(b.take_outgoing(), vec![]);
    }

    #[test]
    fn a_changed_port_list_is_sent_once() {
        let (mut b, device) = bridge(&["A"]);
        b.handle(r#"{"type":"ready"}"#);
        b.take_outgoing();
        b.refresh_ports();
        assert_eq!(b.take_outgoing(), vec![]);
        device.lock().unwrap().ports.push("B".into());
        b.refresh_ports();
        b.refresh_ports();
        assert_eq!(b.take_outgoing(), vec![ports(&["A", "B"])]);
    }

    #[test]
    fn an_unavailable_link_lists_no_ports_and_says_why_once() {
        let (mut b, device) = bridge(&["A"]);
        device.lock().unwrap().unavailable = true;
        b.handle(r#"{"type":"ready"}"#);
        b.refresh_ports();
        let out = b.take_outgoing();
        assert_eq!(out.len(), 2);
        assert!(matches!(&out[0], PluginMessage::Error { port: None, .. }));
        assert_eq!(out[1], ports(&[]));
    }

    #[test]
    fn opens_sends_and_receives() {
        let (mut b, device) = bridge(&["A"]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(r#"{"type":"open","port":"A"}"#);
        b.handle(r#"{"type":"send","port":"A","data":[240,65,247]}"#);
        device_sends(&device, &[0xF0, 0x7E, 0xF7]);
        assert_eq!(
            b.take_outgoing(),
            vec![
                ports(&["A"]),
                PluginMessage::Opened { port: "A".into() },
                PluginMessage::Receive {
                    port: "A".into(),
                    data: vec![0xF0, 0x7E, 0xF7]
                },
            ]
        );
        let d = device.lock().unwrap();
        assert_eq!(d.opened, vec!["A".to_string()]);
        assert_eq!(d.sent, vec![vec![0xF0, 0x41, 0xF7]]);
    }

    #[test]
    fn reopening_the_open_pair_does_not_reconnect() {
        let (mut b, device) = bridge(&["A"]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(r#"{"type":"open","port":"A"}"#);
        b.handle(r#"{"type":"open","port":"A"}"#);
        assert_eq!(device.lock().unwrap().opened.len(), 1);
        let opened = b
            .take_outgoing()
            .into_iter()
            .filter(|m| matches!(m, PluginMessage::Opened { .. }))
            .count();
        assert_eq!(opened, 2);
    }

    /// The link says, from its own thread, what became of the open.
    fn link_says(device: &Arc<Mutex<Device>>, incoming: Incoming<'_>) {
        let mut inbound = device.lock().unwrap().inbound.take().expect("open");
        inbound(incoming);
        device.lock().unwrap().inbound = Some(inbound);
    }

    #[test]
    fn a_slow_open_is_answered_when_the_link_is_ready_and_sends_wait_for_it() {
        let (mut b, device) = bridge(&["A"]);
        device.lock().unwrap().slow = true;
        b.handle(r#"{"type":"ready"}"#);
        b.take_outgoing();
        b.handle(r#"{"type":"open","port":"A"}"#);
        // A second open while waiting asks the link nothing.
        b.handle(r#"{"type":"open","port":"A"}"#);
        b.handle(r#"{"type":"send","port":"A","data":[1]}"#);
        assert_eq!(device.lock().unwrap().opened.len(), 1);
        assert!(device.lock().unwrap().sent.is_empty());
        let heard = b.take_outgoing();
        assert!(
            matches!(heard.as_slice(), [PluginMessage::Error { port: Some(p), .. }] if p == "A"),
            "{heard:?}"
        );

        link_says(&device, Incoming::Opened);
        assert_eq!(
            b.take_outgoing(),
            vec![PluginMessage::Opened { port: "A".into() }]
        );
        b.handle(r#"{"type":"send","port":"A","data":[1]}"#);
        assert_eq!(device.lock().unwrap().sent, vec![vec![1]]);
    }

    #[test]
    fn an_open_the_link_gives_up_on_can_be_asked_again() {
        let (mut b, device) = bridge(&["A"]);
        device.lock().unwrap().slow = true;
        b.handle(r#"{"type":"ready"}"#);
        b.take_outgoing();
        b.handle(r#"{"type":"open","port":"A"}"#);
        link_says(&device, Incoming::OpenFailed("no answer".into()));
        assert_eq!(
            b.take_outgoing(),
            vec![PluginMessage::OpenFailed {
                port: "A".into(),
                message: "no answer".into()
            }]
        );
        b.handle(r#"{"type":"open","port":"A"}"#);
        assert_eq!(device.lock().unwrap().opened.len(), 2);
    }

    #[test]
    fn what_the_link_reports_while_open_reaches_the_page() {
        let (mut b, device) = bridge(&["A"]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(r#"{"type":"open","port":"A"}"#);
        b.take_outgoing();
        link_says(&device, Incoming::Error("refused".into()));
        assert_eq!(
            b.take_outgoing(),
            vec![PluginMessage::Error {
                port: Some("A".into()),
                message: "refused".into()
            }]
        );
    }

    #[test]
    fn a_refused_open_fails_that_open() {
        let (mut b, device) = bridge(&["A"]);
        device.lock().unwrap().refuse = true;
        b.handle(r#"{"type":"ready"}"#);
        b.take_outgoing();
        b.handle(r#"{"type":"open","port":"A"}"#);
        let out = b.take_outgoing();
        assert!(
            matches!(&out[..], [PluginMessage::OpenFailed { port, .. }] if port == "A"),
            "{out:?}"
        );
    }

    #[test]
    fn a_send_to_a_pair_not_open_is_an_error() {
        let (mut b, device) = bridge(&["A", "B"]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(r#"{"type":"open","port":"A"}"#);
        b.take_outgoing();
        b.handle(r#"{"type":"send","port":"B","data":[248]}"#);
        assert!(device.lock().unwrap().sent.is_empty());
        let out = b.take_outgoing();
        assert!(
            matches!(&out[..], [PluginMessage::Error { port: Some(p), .. }] if p == "B"),
            "{out:?}"
        );
    }

    #[test]
    fn a_malformed_message_is_an_error() {
        let (mut b, _) = bridge(&[]);
        b.handle(r#"{"type":"ready"}"#);
        b.take_outgoing();
        b.handle("{");
        let out = b.take_outgoing();
        assert!(
            matches!(&out[..], [PluginMessage::Error { port: None, .. }]),
            "{out:?}"
        );
    }

    #[test]
    fn resize_is_held_for_the_editor_within_bounds() {
        let (mut b, _) = bridge(&[]);
        b.handle(r#"{"type":"ready"}"#);
        b.take_outgoing();
        b.handle(r#"{"type":"resize","width":1920,"height":912}"#);
        assert_eq!(b.take_resize(), Some((1920, 912)));
        assert_eq!(b.take_resize(), None);
        b.handle(r#"{"type":"resize","width":10,"height":912}"#);
        assert_eq!(b.take_resize(), None);
        assert_eq!(b.take_outgoing().len(), 1);
    }

    #[test]
    fn a_reloaded_page_hears_the_port_list_again() {
        let (mut b, _) = bridge(&["A"]);
        b.handle(r#"{"type":"ready"}"#);
        b.take_outgoing();
        b.handle(r#"{"type":"ready"}"#);
        assert_eq!(b.take_outgoing(), vec![ports(&["A"])]);
    }

    #[test]
    fn the_outbox_drops_the_oldest_beyond_its_limit() {
        let (mut b, device) = bridge(&["A"]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(r#"{"type":"open","port":"A"}"#);
        b.take_outgoing();
        for i in 0..=OUTBOX_LIMIT {
            device_sends(&device, &[(i % 128) as u8]);
        }
        let out = b.take_outgoing();
        assert_eq!(out.len(), OUTBOX_LIMIT);
        assert_eq!(
            out[0],
            PluginMessage::Receive {
                port: "A".into(),
                data: vec![1]
            }
        );
    }

    #[test]
    fn dropping_the_bridge_closes_the_link() {
        let (b, device) = bridge(&["A"]);
        drop(b);
        assert_eq!(device.lock().unwrap().closed, 1);
    }
}
