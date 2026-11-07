//! The WebSocket endpoint against a fake link: who may connect, how it
//! follows the device link, and that a page behaves as a shared MIDI port.

use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use integral_core::address::Address;
use integral_core::device_service::default_origins;
use integral_core::params::{self, part};
use integral_core::sysex;
use integral_device_service::{
    Config, FakeLink, MAX_CLIENTS, Running, SequentialSessionIds, Service, WebSocketServer, spawn,
};
use tungstenite::client::IntoClientRequest;
use tungstenite::http::header::ORIGIN;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Error, Message, WebSocket};

const DEVICE_ID: u8 = 0x10;
const WAIT: Duration = Duration::from_secs(5);
const ALLOWED: &str = "https://app.example";

type Page = WebSocket<MaybeTlsStream<TcpStream>>;

/// A service on a fake link, with its WebSocket endpoint on a free port.
struct Fixture {
    link: FakeLink,
    running: Option<Running>,
    server: Option<WebSocketServer>,
    url: String,
    log: Arc<Mutex<Vec<String>>>,
}

impl Fixture {
    /// Start with the link up, and wait until the endpoint knows it.
    fn start() -> Self {
        let link = FakeLink::new();
        let config = Config {
            device_id: Some(DEVICE_ID),
            session_ids: Box::new(SequentialSessionIds(1)),
        };
        let running = spawn(Service::new(Box::new(link.clone()), config), None).expect("start");
        link.bring_up(None);
        let log = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let server = WebSocketServer::bind(
            0,
            &[ALLOWED.to_string()],
            running.handle(),
            Arc::new(move |line| sink.lock().expect("log").push(line.to_string())),
        )
        .expect("bind");
        let url = format!("ws://{}", server.local_addr());
        let service = Self {
            link,
            running: Some(running),
            server: Some(server),
            url,
            log,
        };
        service.connect_soon(ALLOWED);
        service
    }

    /// Connect, sending `origin` if there is one.
    fn connect(&self, origin: Option<&str>) -> Result<Page, Error> {
        self.connect_with(origin.into_iter().collect())
    }

    /// Connect, sending an `Origin` header for each of `origins`.
    fn connect_with(&self, origins: Vec<&str>) -> Result<Page, Error> {
        let mut request = self.url.as_str().into_client_request().expect("request");
        for origin in origins {
            request
                .headers_mut()
                .append(ORIGIN, origin.parse().expect("header value"));
        }
        tungstenite::connect(request).map(|(socket, _)| socket)
    }

    /// Connect from `origin`, trying again for as long as it takes the
    /// endpoint to learn the link is up or a closed page to be forgotten.
    fn connect_soon(&self, origin: &str) -> Page {
        let deadline = Instant::now() + WAIT;
        loop {
            match self.connect(Some(origin)) {
                Ok(page) => return page,
                Err(e) => {
                    assert!(Instant::now() < deadline, "could not connect: {e}");
                    std::thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    /// The status a handshake from `origin` is refused with, once it is.
    fn refused_with(&self, origin: Option<&str>) -> u16 {
        refused_with(self.connect(origin))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.take();
        if let Some(running) = self.running.take() {
            running.stop();
        }
    }
}

fn wait_for(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn level_address() -> Address {
    params::part_address(2, part::LEVEL)
}

fn dt1(value: u8) -> Vec<u8> {
    sysex::build_dt1(DEVICE_ID, &level_address(), &[value])
}

fn refused_with(result: Result<Page, Error>) -> u16 {
    match result {
        Err(Error::Http(response)) => response.status().as_u16(),
        Err(other) => panic!("expected an HTTP refusal, got {other}"),
        Ok(_) => panic!("the handshake was accepted"),
    }
}

/// Read until the page is closed; returns the close frame, if it had one.
fn read_until_closed(page: &mut Page) -> Option<(CloseCode, String)> {
    let deadline = Instant::now() + WAIT;
    loop {
        assert!(Instant::now() < deadline, "the page was not closed");
        match page.read() {
            Ok(Message::Close(frame)) => {
                return frame.map(|f| (f.code, f.reason.to_string()));
            }
            Ok(Message::Binary(_)) => {}
            Ok(other) => panic!("unexpected {other:?}"),
            Err(_) => return None,
        }
    }
}

#[test]
fn a_handshake_from_an_origin_off_the_list_is_refused() {
    let service = Fixture::start();

    assert_eq!(service.refused_with(Some("https://evil.example")), 403);
    // A path, a scheme or a port makes another origin.
    assert_eq!(service.refused_with(Some("https://app.example/")), 403);
    assert_eq!(service.refused_with(Some("http://app.example")), 403);
    assert_eq!(service.refused_with(Some("https://app.example:8443")), 403);
    assert_eq!(service.refused_with(Some("null")), 403);
}

#[test]
fn a_handshake_with_no_origin_or_two_is_refused() {
    let service = Fixture::start();

    assert_eq!(service.refused_with(None), 403);
    let both = service.connect_with(vec![ALLOWED, ALLOWED]);
    assert_eq!(refused_with(both), 403);
    let one_bad = service.connect_with(vec![ALLOWED, "https://evil.example"]);
    assert_eq!(refused_with(one_bad), 403);
}

#[test]
fn a_handshake_from_an_origin_on_the_list_is_accepted_whatever_its_case() {
    let service = Fixture::start();

    assert!(service.connect(Some(ALLOWED)).is_ok());
    assert!(service.connect(Some("HTTPS://App.Example")).is_ok());
}

#[test]
fn a_refused_origin_is_logged() {
    let service = Fixture::start();

    service.refused_with(Some("https://evil.example"));
    service.refused_with(None);

    let log = service.log.lock().expect("log").join("\n");
    assert!(log.contains("Origin 'https://evil.example'"), "{log}");
    assert!(log.contains("no Origin header"), "{log}");
    assert!(log.contains(ALLOWED), "{log}");
}

#[test]
fn an_origin_that_is_not_one_cannot_be_allowed() {
    let link = FakeLink::new();
    let running = spawn(
        Service::new(Box::new(link), Config::new(Some(DEVICE_ID))),
        None,
    )
    .expect("start");

    for bad in ["null", "https://app.example/path", "app.example"] {
        let result =
            WebSocketServer::bind(0, &[bad.to_string()], running.handle(), Arc::new(|_| {}));
        let error = result.err().expect("refused");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput, "{bad}");
    }
    running.stop();
}

#[test]
fn the_default_origins_are_all_well_formed() {
    let link = FakeLink::new();
    let running = spawn(
        Service::new(Box::new(link), Config::new(Some(DEVICE_ID))),
        None,
    )
    .expect("start");

    let server = WebSocketServer::bind(0, &default_origins(), running.handle(), Arc::new(|_| {}));

    assert!(server.is_ok());
    drop(server);
    running.stop();
}

#[test]
fn a_binary_frame_is_sent_to_the_device_as_one_sysex_message() {
    let service = Fixture::start();
    let mut page = service.connect_soon(ALLOWED);

    let message = dt1(77);
    page.send(Message::binary(message.clone())).expect("send");

    wait_for("the DT1 on the link", || {
        service.link.sent().contains(&message)
    });
}

#[test]
fn the_devices_sysex_arrives_as_one_binary_frame_each() {
    let service = Fixture::start();
    let mut first = service.connect_soon(ALLOWED);
    let mut second = service.connect_soon(ALLOWED);
    // The endpoint says Hello for a page before anything the page sends, so
    // a message of each on the link proves both are listening.
    let request = sysex::identity_request().to_vec();
    first.send(Message::binary(request.clone())).expect("send");
    let write = dt1(1);
    second.send(Message::binary(write.clone())).expect("send");
    wait_for("both messages on the link", || {
        let sent = service.link.sent();
        sent.contains(&request) && sent.contains(&write)
    });

    let reply = dt1(5);
    service.link.device_says(&reply);

    for page in [&mut first, &mut second] {
        assert_eq!(page.read().expect("frame"), Message::binary(reply.clone()));
    }
}

#[test]
fn what_is_not_one_sysex_message_does_not_reach_the_device() {
    let service = Fixture::start();
    let mut page = service.connect_soon(ALLOWED);

    page.send(Message::binary(vec![0x90, 0x3C, 0x7F]))
        .expect("send");
    page.send(Message::text("hello")).expect("send");
    // Malformed SysEx: a status byte inside, and no checksum.
    page.send(Message::binary(vec![0xF0, 0x90, 0xF7]))
        .expect("send");
    page.send(Message::binary(vec![0xF0, 0x41, 0x10, 0xF7]))
        .expect("send");
    // Two messages in one frame.
    let two = [dt1(1), dt1(2)].concat();
    page.send(Message::binary(two.clone())).expect("send");
    let good = dt1(9);
    page.send(Message::binary(good.clone())).expect("send");

    wait_for("the DT1 on the link", || {
        service.link.sent().contains(&good)
    });
    let sent = service.link.sent();
    assert!(!sent.contains(&two));
    assert!(!sent.contains(&vec![0xF0, 0x90, 0xF7]));
    assert!(sent.iter().all(|m| m.first() == Some(&sysex::SYSEX_START)));
}

#[test]
fn a_frame_over_the_sysex_limit_closes_the_page() {
    let service = Fixture::start();
    let mut page = service.connect_soon(ALLOWED);

    let mut long = vec![0xF0];
    long.resize(sysex::MAX_MESSAGE_LEN + 44, 0x01);
    long.push(0xF7);
    page.send(Message::binary(long.clone())).expect("send");

    read_until_closed(&mut page);
    assert!(!service.link.sent().contains(&long));
}

#[test]
fn the_link_going_down_closes_every_page_and_the_next_one_at_once() {
    let service = Fixture::start();
    let mut first = service.connect_soon(ALLOWED);
    let mut second = service.connect_soon(ALLOWED);

    service.link.bring_down();

    for page in [&mut first, &mut second] {
        assert_eq!(read_until_closed(page), Some(link_down()));
    }
    // A page that comes while it is down is accepted, then closed the same
    // way: a browser tells a refused handshake from nothing else.
    let mut late = service.connect_soon(ALLOWED);
    assert_eq!(read_until_closed(&mut late), Some(link_down()));
    // The origin check comes first: a stranger is refused, not told.
    assert_eq!(service.refused_with(Some("https://evil.example")), 403);

    service.link.bring_up(None);
    let mut page = service.connect_soon(ALLOWED);
    let reply = dt1(3);
    page.send(Message::binary(dt1(4))).expect("send");
    wait_for("the page greeted", || service.link.sent().contains(&dt1(4)));
    service.link.device_says(&reply);
    assert_eq!(page.read().expect("frame"), Message::binary(reply));
}

fn link_down() -> (CloseCode, String) {
    (CloseCode::Again, "INTEGRA-7 link down".to_string())
}

#[test]
fn a_page_the_service_drops_without_a_close_frame_is_closed() {
    let service = Fixture::start();
    let mut page = service.connect_soon(ALLOWED);
    // The page never reads, so its queue and the socket's buffers fill, and
    // the service drops it as too far behind to serve: no close frame says so.
    let big = [vec![0xF0, 0x7E], vec![0x01; 250], vec![0xF7]].concat();
    for _ in 0..30_000 {
        service.link.device_says(&big);
    }

    // Reading drains what was queued; then the socket must end.
    read_until_closed(&mut page);
}

#[test]
fn no_more_than_the_most_clients_are_served() {
    let service = Fixture::start();
    let pages: Vec<Page> = (0..MAX_CLIENTS)
        .map(|_| service.connect_soon(ALLOWED))
        .collect();

    assert!(service.connect(Some(ALLOWED)).is_err());
    drop(pages);
}
