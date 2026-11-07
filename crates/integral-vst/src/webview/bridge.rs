//! The plugin's half of the plugin transport: page messages in, link calls
//! out, and what the page should hear queued for delivery.
//!
//! The bridge does not throttle: the page paces its sends with core's
//! `DeviceState`, as on the web (ADR 0004).
//!
//! It also carries the page's half of the instance: the page binds the
//! instance (`bind`), hears what it is and how it is doing (`instance`), and
//! edits host parameters in gestures (`beginEdit`, `performEdit`, `endEdit`),
//! and asks for the plugin's default tone when the Part holds another type
//! (`loadDefaultTone`).
//! A gesture the page leaves open when it reloads or goes away is ended, so
//! the host never waits on one for ever.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::gestures::{self, Gesture};
use super::ipc::{self, NoticeView, PageMessage, PluginMessage};
use super::link::{DeviceLink, Incoming, LinkError};
use crate::family::Instance;
use integral_core::host_params::Binding;

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
    /// The instance the page is the editor of.
    instance: Arc<Instance>,
    /// The gestures the page has begun and not ended, in order.
    editing: Vec<Gesture>,
    /// The IDs whose begin was refused, until the page ends them: the rest of
    /// that gesture is dropped without a word, the refusal having been said.
    refused: Vec<String>,
    /// The `instance` message the page last heard.
    reported: Option<PluginMessage>,
}

impl Bridge {
    /// A bridge over `link` for `instance`, with nothing open. The page's host
    /// gestures go to the instance's [`HostEdits`](crate::family::host::HostEdits).
    pub(crate) fn new(link: Box<dyn DeviceLink>, instance: Arc<Instance>) -> Self {
        Self {
            link,
            outbox: Arc::default(),
            ready: false,
            open: Arc::default(),
            listed: None,
            told: None,
            resize: None,
            instance,
            editing: Vec::new(),
            refused: Vec::new(),
            reported: None,
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
                self.reported = None;
                self.end_edits();
                self.refused.clear();
                self.refresh_ports();
                self.refresh_instance();
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
            PageMessage::Bind { part, key } => match self.instance.bind(Binding { part, key }) {
                Ok(()) => self.refresh_instance(),
                Err(e) => self.fail(e.to_string()),
            },
            PageMessage::LoadDefaultTone => {
                if let Err(e) = self.instance.request_default_tone() {
                    self.fail(e.to_string());
                }
            }
            PageMessage::BeginEdit { id } => self.begin_edit(id),
            PageMessage::PerformEdit { id, value } => {
                match self.editing.iter_mut().find(|g| g.id == id) {
                    Some(gesture) => gestures::perform(&self.instance, gesture, value),
                    None if self.refused.contains(&id) => {}
                    None => self.fail(format!("no edit of {id:?} is begun")),
                }
            }
            PageMessage::EndEdit { id } => {
                let Some(at) = self.editing.iter().position(|g| g.id == id) else {
                    if let Some(at) = self.refused.iter().position(|r| *r == id) {
                        self.refused.remove(at);
                        return;
                    }
                    return self.fail(format!("no edit of {id:?} is begun"));
                };
                gestures::end(&self.instance, self.editing.remove(at));
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

    /// Tell the page the plugin refused a request about the instance.
    fn fail(&self, message: String) {
        self.outbox.push(PluginMessage::Refused { message });
    }

    /// Begin the edit of `id`; one already begun stays as it is.
    fn begin_edit(&mut self, id: String) {
        if self.editing.iter().any(|g| g.id == id) || self.refused.contains(&id) {
            return;
        }
        match gestures::begin(&self.instance, &id) {
            Ok(gesture) => self.editing.push(gesture),
            Err(e) => {
                self.refused.push(id);
                self.fail(e.to_string());
            }
        }
    }

    /// End every edit the page left begun.
    fn end_edits(&mut self) {
        for gesture in std::mem::take(&mut self.editing) {
            gestures::end(&self.instance, gesture);
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

    /// Tell a ready page what the instance is and how it is doing, if that
    /// changed since it last heard.
    pub(crate) fn refresh_instance(&mut self) {
        if !self.ready {
            return;
        }
        let status = self.instance.status.get();
        let message = PluginMessage::Instance {
            plugin: self.instance.kind(),
            binding: self.instance.binding.get(),
            status,
            status_text: status.text(),
            host_edits: self.instance.bank.edits().is_some(),
            notices: self
                .instance
                .notices
                .get()
                .into_iter()
                .map(|notice| NoticeView {
                    notice,
                    text: notice.text(),
                })
                .collect(),
            default_tone: self.instance.default_tone().map(|tone| tone.name),
            default_tone_pending: self.instance.default_tone_pending(),
        };
        if self.instance.take_default_tone_dropped() {
            self.fail(
                "The default tone could not be loaded: the plugin has no connection to the device"
                    .into(),
            );
        }
        if self.reported.as_ref() != Some(&message) {
            self.reported = Some(message.clone());
            self.outbox.push(message);
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
        // The page is gone with the editor; so are its edits and its link.
        self.end_edits();
        self.link.close();
    }
}

#[cfg(test)]
mod tests {
    use super::super::link::{Inbound, Incoming, LinkError};
    use super::*;
    use crate::family::host::{HostEdits, Replace};
    use crate::family::{Notice, Status};
    use integral_core::host_params::PluginKind;
    use std::sync::Mutex;

    /// What the page's gestures asked of the host, in order.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Edit {
        Begin(String),
        Perform(String, i32),
        End(String),
    }

    /// The host, as the instance's [`HostEdits`] reach it: records what it is
    /// asked.
    struct Host {
        instance: std::sync::Weak<Instance>,
        edits: Arc<Mutex<Vec<Edit>>>,
    }

    impl Host {
        fn id_of(&self, param: &nih_plug::prelude::IntParam) -> String {
            let instance = self.instance.upgrade().expect("the instance");
            let (host, _) = instance
                .bank
                .list()
                .iter()
                .find(|(_, p)| std::ptr::eq(p, param))
                .expect("one of the instance's parameters");
            host.id.clone()
        }
    }

    impl HostEdits for Host {
        fn begin(&self, param: &nih_plug::prelude::IntParam) {
            let id = self.id_of(param);
            self.edits.lock().unwrap().push(Edit::Begin(id));
        }
        fn perform(&self, param: &nih_plug::prelude::IntParam, value: i32) {
            let id = self.id_of(param);
            self.edits.lock().unwrap().push(Edit::Perform(id, value));
        }
        fn end(&self, param: &nih_plug::prelude::IntParam) {
            let id = self.id_of(param);
            self.edits.lock().unwrap().push(Edit::End(id));
        }
        fn replace(&self, _: &[Replace<'_>]) -> bool {
            true
        }
    }

    /// A [`Bridge`] under test, with the sides it talks to.
    struct Editor {
        bridge: Bridge,
        instance: Arc<Instance>,
        device: Arc<Mutex<Device>>,
        edits: Arc<Mutex<Vec<Edit>>>,
    }

    /// A bridge for an instance of `kind` over a link to a device with
    /// `ports`.
    fn editor(kind: PluginKind, ports: &[&str]) -> Editor {
        let device = Arc::new(Mutex::new(Device {
            ports: ports.iter().map(|p| p.to_string()).collect(),
            ..Device::default()
        }));
        let instance = Arc::new(Instance::new(kind));
        let edits = Arc::new(Mutex::new(Vec::new()));
        instance.bank.set_edits(Some(Arc::new(Host {
            instance: Arc::downgrade(&instance),
            edits: Arc::clone(&edits),
        })));
        let bridge = Bridge::new(Box::new(FakeLink(device.clone())), Arc::clone(&instance));
        Editor {
            bridge,
            instance,
            device,
            edits,
        }
    }

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
        let Editor { bridge, device, .. } = editor(PluginKind::StudioSet, ports);
        (bridge, device)
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
        assert_eq!(outgoing(&b), vec![ports(&["INTEGRA-7"])]);
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
        let out = outgoing(&b);
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
            outgoing(&b),
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
        assert_eq!(outgoing(&b), vec![ports(&["A"])]);
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

    /// What `bridge` has for the page, less the `instance` messages.
    fn outgoing(bridge: &Bridge) -> Vec<PluginMessage> {
        bridge
            .take_outgoing()
            .into_iter()
            .filter(|m| !matches!(m, PluginMessage::Instance { .. }))
            .collect()
    }

    /// The messages `bridge` has for the page, less the port lists.
    fn heard(bridge: &Bridge) -> Vec<PluginMessage> {
        bridge
            .take_outgoing()
            .into_iter()
            .filter(|m| !matches!(m, PluginMessage::Ports { .. }))
            .collect()
    }

    fn is_error(message: &PluginMessage) -> bool {
        matches!(message, PluginMessage::Refused { .. })
    }

    fn begin(id: &str) -> String {
        format!(r#"{{"type":"beginEdit","id":"{id}"}}"#)
    }

    fn perform(id: &str, value: i32) -> String {
        format!(r#"{{"type":"performEdit","id":"{id}","value":{value}}}"#)
    }

    fn end(id: &str) -> String {
        format!(r#"{{"type":"endEdit","id":"{id}"}}"#)
    }

    #[test]
    fn a_control_drag_reaches_the_host_as_one_gesture_in_order() {
        let Editor {
            bridge: mut b,
            edits,
            ..
        } = editor(PluginKind::StudioSet, &[]);
        b.handle(r#"{"type":"ready"}"#);
        for text in [
            begin("part07/0x09"),
            perform("part07/0x09", 10),
            perform("part07/0x09", 11),
            perform("part07/0x09", 12),
            end("part07/0x09"),
        ] {
            b.handle(&text);
        }
        let id = || "part07/0x09".to_string();
        assert_eq!(
            *edits.lock().unwrap(),
            [
                Edit::Begin(id()),
                Edit::Perform(id(), 10),
                Edit::Perform(id(), 11),
                Edit::Perform(id(), 12),
                Edit::End(id()),
            ]
        );
        assert!(heard(&b).iter().all(|m| !is_error(m)));
    }

    #[test]
    fn gestures_on_two_parameters_interleave_as_sent() {
        let Editor {
            bridge: mut b,
            edits,
            ..
        } = editor(PluginKind::StudioSet, &[]);
        b.handle(r#"{"type":"ready"}"#);
        for text in [
            begin("part07/0x09"),
            begin("part07/0x0A"),
            perform("part07/0x0A", 1),
            end("part07/0x09"),
            end("part07/0x0A"),
        ] {
            b.handle(&text);
        }
        let (a, c) = ("part07/0x09".to_string(), "part07/0x0A".to_string());
        assert_eq!(
            *edits.lock().unwrap(),
            [
                Edit::Begin(a.clone()),
                Edit::Begin(c.clone()),
                Edit::Perform(c.clone(), 1),
                Edit::End(a),
                Edit::End(c),
            ]
        );
    }

    #[test]
    fn a_gesture_out_of_order_or_on_an_unknown_parameter_is_an_error_and_not_forwarded() {
        let Editor {
            bridge: mut b,
            edits,
            ..
        } = editor(PluginKind::StudioSet, &[]);
        b.handle(r#"{"type":"ready"}"#);
        heard(&b);
        b.handle(&perform("part07/0x09", 1));
        b.handle(&end("part07/0x09"));
        b.handle(&begin("nope/0x00"));
        b.handle(&begin("part07/0x09"));
        b.handle(&end("part07/0x09"));
        b.handle(&perform("part07/0x09", 2));
        assert_eq!(
            *edits.lock().unwrap(),
            [
                Edit::Begin("part07/0x09".into()),
                Edit::End("part07/0x09".into())
            ]
        );
        assert_eq!(heard(&b).iter().filter(|m| is_error(m)).count(), 4);
    }

    #[test]
    fn beginning_a_begun_gesture_does_nothing() {
        let Editor {
            bridge: mut b,
            edits,
            ..
        } = editor(PluginKind::StudioSet, &[]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(&begin("part07/0x09"));
        b.handle(&begin("part07/0x09"));
        b.handle(&end("part07/0x09"));
        assert_eq!(edits.lock().unwrap().len(), 2);
    }

    #[test]
    fn a_gesture_the_page_leaves_open_is_ended_when_it_reloads_or_goes() {
        let Editor {
            bridge: mut b,
            edits,
            ..
        } = editor(PluginKind::StudioSet, &[]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(&begin("part07/0x09"));
        b.handle(&perform("part07/0x09", 5));
        b.handle(r#"{"type":"ready"}"#);
        let id = || "part07/0x09".to_string();
        assert_eq!(
            edits.lock().unwrap().last(),
            Some(&Edit::End(id())),
            "a reload ends it"
        );
        // The ended gesture is not open any more.
        b.handle(&perform("part07/0x09", 6));
        b.handle(&begin("part07/0x0A"));
        drop(b);
        assert_eq!(
            *edits.lock().unwrap(),
            [
                Edit::Begin(id()),
                Edit::Perform(id(), 5),
                Edit::End(id()),
                Edit::Begin("part07/0x0A".into()),
                Edit::End("part07/0x0A".into()),
            ],
            "going away ends it"
        );
    }

    #[test]
    fn ready_tells_the_page_what_the_instance_is_and_changes_are_sent_once() {
        let Editor {
            bridge: mut b,
            instance,
            ..
        } = editor(PluginKind::PcmDrum, &[]);
        let instance_message = |status, binding, notices: &[Notice]| PluginMessage::Instance {
            plugin: PluginKind::PcmDrum,
            binding,
            status,
            status_text: Status::text(status),
            host_edits: true,
            notices: notices
                .iter()
                .map(|&notice| NoticeView {
                    notice,
                    text: notice.text(),
                })
                .collect(),
            default_tone: instance.default_tone().map(|tone| tone.name),
            default_tone_pending: false,
        };
        b.refresh_instance();
        assert_eq!(b.take_outgoing(), [], "nothing before ready");

        b.handle(r#"{"type":"ready"}"#);
        assert_eq!(
            heard(&b),
            [instance_message(Status::ServiceNotRunning, None, &[])]
        );
        b.refresh_instance();
        assert_eq!(heard(&b), [], "unchanged");

        instance.status.set(Status::Connected);
        instance.notices.set(Notice::Recalling, true);
        b.refresh_instance();
        b.refresh_instance();
        assert_eq!(
            heard(&b),
            [instance_message(
                Status::Connected,
                None,
                &[Notice::Recalling]
            )]
        );

        // A reloaded page hears it all again.
        b.handle(r#"{"type":"ready"}"#);
        assert_eq!(heard(&b).len(), 1);
    }

    #[test]
    fn the_page_is_told_the_default_tone_by_name_and_the_mismatch_as_a_notice() {
        let Editor {
            bridge: mut b,
            instance,
            ..
        } = editor(PluginKind::SnSynth, &[]);
        b.handle(r#"{"type":"ready"}"#);
        let tone = |out: &[PluginMessage]| match out {
            [
                PluginMessage::Instance {
                    default_tone,
                    notices,
                    ..
                },
            ] => (
                *default_tone,
                notices.iter().map(|n| n.notice).collect::<Vec<_>>(),
            ),
            other => panic!("{other:?}"),
        };
        assert_eq!(tone(&heard(&b)), (Some("JP8 Strings1"), vec![]));
        instance.set_fit(crate::family::ToneFit::Mismatch);
        b.refresh_instance();
        assert_eq!(
            tone(&heard(&b)),
            (Some("JP8 Strings1"), vec![Notice::TypeMismatch])
        );
    }

    #[test]
    fn the_default_tone_is_requested_only_for_a_bound_instance_on_another_type() {
        let Editor {
            bridge: mut b,
            instance,
            ..
        } = editor(PluginKind::SnSynth, &[]);
        b.handle(r#"{"type":"ready"}"#);
        heard(&b);
        let load = r#"{"type":"loadDefaultTone"}"#;

        // Unbound, then bound to a Part that fits: nothing to load.
        b.handle(load);
        instance.bind(Binding::of_part(1)).unwrap();
        instance.set_fit(crate::family::ToneFit::Fits);
        b.handle(load);
        let refusals = heard(&b);
        assert_eq!(refusals.len(), 2, "{refusals:?}");
        assert!(
            refusals
                .iter()
                .all(|m| matches!(m, PluginMessage::Refused { .. })),
            "{refusals:?}"
        );
        assert!(!instance.take_default_tone_request());

        instance.set_fit(crate::family::ToneFit::Mismatch);
        b.handle(load);
        assert_eq!(heard(&b), []);

        // Pending until the Part holds the tone: the page hears it, and a
        // second request is refused.
        b.refresh_instance();
        let pending = |out: &[PluginMessage]| {
            out.iter().any(|m| {
                matches!(
                    m,
                    PluginMessage::Instance {
                        default_tone_pending: true,
                        ..
                    }
                )
            })
        };
        assert!(pending(&heard(&b)));
        b.handle(load);
        assert!(matches!(&heard(&b)[..], [PluginMessage::Refused { .. }]));
        assert!(instance.take_default_tone_request());

        // A request the worker could not serve is said to the page, and the
        // button comes back.
        instance.drop_default_tone();
        b.refresh_instance();
        let out = heard(&b);
        assert!(
            out.iter()
                .any(|m| matches!(m, PluginMessage::Refused { .. })),
            "{out:?}"
        );
        assert!(!pending(&out));
    }

    #[test]
    fn the_page_is_told_whether_the_instance_can_tell_the_host() {
        let Editor {
            bridge: mut b,
            instance,
            ..
        } = editor(PluginKind::StudioSet, &[]);
        let host_edits = |out: &[PluginMessage]| match out {
            [PluginMessage::Instance { host_edits, .. }] => *host_edits,
            other => panic!("{other:?}"),
        };
        b.handle(r#"{"type":"ready"}"#);
        assert!(host_edits(&heard(&b)));
        // The host's context goes away (or never was): said once, on change.
        instance.bank.set_edits(None);
        b.refresh_instance();
        b.refresh_instance();
        assert!(!host_edits(&heard(&b)));
    }

    #[test]
    fn a_refused_begin_is_said_once_and_the_rest_of_its_gesture_is_dropped() {
        let Editor {
            bridge: mut b,
            instance,
            edits,
            ..
        } = editor(PluginKind::StudioSet, &[]);
        instance.bank.set_edits(None);
        b.handle(r#"{"type":"ready"}"#);
        heard(&b);
        b.handle(&begin("part07/0x09"));
        for value in 0..50 {
            b.handle(&perform("part07/0x09", value));
        }
        b.handle(&end("part07/0x09"));
        let out = heard(&b);
        assert!(
            matches!(&out[..], [PluginMessage::Refused { .. }]),
            "{out:?}"
        );
        assert_eq!(*edits.lock().unwrap(), []);

        // The next gesture is asked again, and works once the host is there.
        let host = Arc::new(Host {
            instance: Arc::downgrade(&instance),
            edits: Arc::clone(&edits),
        });
        instance.bank.set_edits(Some(host));
        b.handle(&begin("part07/0x09"));
        b.handle(&end("part07/0x09"));
        assert_eq!(edits.lock().unwrap().len(), 2);
        assert!(heard(&b).iter().all(|m| !is_error(m)));
    }

    #[test]
    fn binding_sets_the_instances_binding_and_the_page_hears_it() {
        let Editor {
            bridge: mut b,
            instance,
            ..
        } = editor(PluginKind::SnDrum, &[]);
        b.handle(r#"{"type":"ready"}"#);
        heard(&b);
        b.handle(r#"{"type":"bind","part":4,"key":38}"#);
        assert_eq!(
            instance.binding.get(),
            Some(Binding::of_part(4).with_key(38))
        );
        let out = heard(&b);
        assert!(
            matches!(
                &out[..],
                [PluginMessage::Instance { binding: Some(got), .. }]
                    if *got == Binding::of_part(4).with_key(38)
            ),
            "{out:?}"
        );
    }

    #[test]
    fn binding_while_a_gesture_is_open_leaves_the_gesture_to_end_as_begun() {
        let Editor {
            bridge: mut b,
            instance,
            edits,
            ..
        } = editor(PluginKind::SnSynth, &[]);
        let tone_id = instance.bank.list()[0].0.id.clone();
        b.handle(r#"{"type":"ready"}"#);
        b.handle(r#"{"type":"bind","part":1}"#);
        b.handle(&begin(&tone_id));
        b.handle(&perform(&tone_id, 3));
        b.handle(r#"{"type":"bind","part":2}"#);
        assert_eq!(instance.binding.get(), Some(Binding::of_part(2)));
        b.handle(&perform(&tone_id, 4));
        b.handle(&end(&tone_id));
        let id = || tone_id.clone();
        assert_eq!(
            *edits.lock().unwrap(),
            [
                Edit::Begin(id()),
                Edit::Perform(id(), 3),
                Edit::Perform(id(), 4),
                Edit::End(id()),
            ]
        );
        assert!(heard(&b).iter().all(|m| !is_error(m)));
    }

    #[test]
    fn a_binding_the_kind_cannot_have_is_refused_and_the_old_one_stays() {
        let Editor {
            bridge: mut b,
            instance,
            ..
        } = editor(PluginKind::SnSynth, &[]);
        b.handle(r#"{"type":"ready"}"#);
        b.handle(r#"{"type":"bind","part":2}"#);
        heard(&b);
        for refused in [
            r#"{"type":"bind","part":2,"key":36}"#,
            r#"{"type":"bind","part":16}"#,
            r#"{"type":"bind"}"#,
        ] {
            b.handle(refused);
            let out = heard(&b);
            assert!(
                matches!(&out[..], [PluginMessage::Refused { .. }]),
                "{out:?}"
            );
            assert_eq!(instance.binding.get(), Some(Binding::of_part(2)));
        }
    }

    #[test]
    fn dropping_the_bridge_closes_the_link() {
        let (b, device) = bridge(&["A"]);
        drop(b);
        assert_eq!(device.lock().unwrap().closed, 1);
    }
}
