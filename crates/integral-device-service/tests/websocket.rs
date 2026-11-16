//! The WebSocket endpoint against a fake link: who may connect, how it
//! follows the device link, and that a page behaves as a shared MIDI port.

use std::io::{Read, Write};
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

    /// A page whose receive buffer is small, so it stalls the service's sends
    /// as soon as it stops reading.
    fn connect_soon_small_window(&self, origin: &str) -> Page {
        let addr = self.url.trim_start_matches("ws://");
        let stream = TcpStream::connect(addr).expect("connect");
        shrink_receive_buffer(&stream);
        let mut request = self.url.as_str().into_client_request().expect("request");
        request
            .headers_mut()
            .append(ORIGIN, origin.parse().expect("header value"));
        let (page, _) =
            tungstenite::client(request, MaybeTlsStream::Plain(stream)).expect("handshake");
        page
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

/// Cap the receive buffer of `stream` at about 64 KiB.
fn shrink_receive_buffer(stream: &TcpStream) {
    use std::os::fd::AsRawFd;
    let size: libc::c_int = 64 * 1024;
    // SAFETY: the descriptor is open while `stream` is; the value is a `c_int`.
    let rc = unsafe {
        libc::setsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_RCVBUF,
            (&raw const size).cast(),
            size_of::<libc::c_int>() as libc::socklen_t,
        )
    };
    assert_eq!(rc, 0, "SO_RCVBUF");
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

/// Bound each read of `page`, so a page nobody closes fails a test rather
/// than blocking it (and the CI job) for good.
fn read_timeout(page: &Page, timeout: Duration) {
    match page.get_ref() {
        MaybeTlsStream::Plain(stream) => stream.set_read_timeout(Some(timeout)).expect("timeout"),
        _ => unreachable!("tests connect over plain TCP"),
    }
}

/// Read until the page is closed; returns the close frame, if it had one.
fn read_until_closed(page: &mut Page) -> Option<(CloseCode, String)> {
    let deadline = Instant::now() + WAIT;
    read_timeout(page, WAIT);
    loop {
        assert!(Instant::now() < deadline, "the page was not closed");
        match page.read() {
            Err(Error::Io(e))
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                panic!("the page was not closed: no data or close within {WAIT:?}")
            }
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

/// A response to a plain HTTP request.
struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    /// The value of the header `name` (in any case), if there is one.
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Send `request` (a whole HTTP/1.1 request) as a plain HTTP client and read
/// the response until the server closes the connection.
fn http(service: &Fixture, request: &str) -> Response {
    let addr = service.server.as_ref().expect("server").local_addr();
    let mut stream = TcpStream::connect(addr).expect("connect");
    stream.set_read_timeout(Some(WAIT)).expect("timeout");
    stream.write_all(request.as_bytes()).expect("send");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read");
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut response = httparse::Response::new(&mut headers);
    let httparse::Status::Complete(end) = response.parse(&bytes).expect("HTTP") else {
        panic!("an incomplete response head");
    };
    Response {
        status: response.code.expect("a status"),
        headers: response
            .headers
            .iter()
            .map(|h| {
                let value = String::from_utf8(h.value.to_vec()).expect("an ASCII value");
                (h.name.to_string(), value)
            })
            .collect(),
        body: bytes[end..].to_vec(),
    }
}

#[test]
fn a_get_for_the_root_returns_the_page() {
    let service = Fixture::start();

    let response = http(&service, "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");

    let page = integral_web_assets::serve("/");
    assert_eq!(response.status, 200);
    assert_eq!(
        response.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    let length = page.body.len().to_string();
    assert_eq!(response.header("Content-Length"), Some(length.as_str()));
    assert_eq!(response.body, page.body);
}

#[test]
fn a_get_ignores_the_query_and_a_missing_file_is_not_found() {
    let service = Fixture::start();

    let response = http(&service, "GET /?from=log HTTP/1.1\r\n\r\n");
    assert_eq!(response.status, 200);
    assert_eq!(response.body, integral_web_assets::serve("/").body);

    let response = http(&service, "GET /no-such-file.js HTTP/1.1\r\n\r\n");
    assert_eq!(response.status, 404);
    assert!(response.body.is_empty());
}

#[test]
fn a_head_request_has_no_body_and_other_methods_are_not_allowed() {
    let service = Fixture::start();

    let response = http(&service, "HEAD / HTTP/1.1\r\n\r\n");
    assert_eq!(response.status, 200);
    assert!(response.body.is_empty());

    // A body the server never reads must not cost the client its answer.
    let response = http(
        &service,
        "POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello",
    );
    assert_eq!(response.status, 405);
    assert_eq!(response.header("Allow"), Some("GET, HEAD"));
}

#[test]
fn the_servers_own_origin_is_admitted() {
    let service = Fixture::start();
    let server = service.server.as_ref().expect("server");
    let own = format!("http://{}", server.local_addr());

    assert_eq!(server.page_url(), format!("{own}/"));
    assert!(service.connect(Some(&own)).is_ok());
    // Not another port, nor `localhost`, which may be another listener's.
    let other_port = format!("http://127.0.0.1:{}", server.local_addr().port() ^ 1);
    assert_eq!(service.refused_with(Some(&other_port)), 403);
    let localhost = format!("http://localhost:{}", server.local_addr().port());
    assert_eq!(service.refused_with(Some(&localhost)), 403);
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

    // A system message is neither SysEx nor a note.
    page.send(Message::binary(vec![0xF2, 0x00, 0x00]))
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

const NOTE_ON: [u8; 3] = [0x90, 0x3C, 0x7F];
const NOTE_OFF: [u8; 3] = [0x80, 0x3C, 0x00];

#[test]
fn a_note_frame_reaches_the_link_ahead_of_queued_sysex() {
    let service = Fixture::start();
    let mut page = service.connect_soon(ALLOWED);
    // SysEx waits as it would behind the throttle; notes do not.
    service.link.hold_sysex(Duration::from_secs(60));

    let write = dt1(21);
    page.send(Message::binary(write.clone())).expect("send");
    page.send(Message::binary(NOTE_ON.to_vec())).expect("send");

    wait_for("the note on the link", || {
        service.link.sent().contains(&NOTE_ON.to_vec())
    });
    assert!(!service.link.sent().contains(&write), "the DT1 was held");

    service.link.hold_sysex(Duration::ZERO);
    // Another note wakes the service if it sleeps, and it sends what it held.
    page.send(Message::binary(NOTE_OFF.to_vec())).expect("send");
    wait_for("the DT1 on the link", || {
        service.link.sent().contains(&write)
    });
    let sent = service.link.sent();
    let at = |m: &[u8]| sent.iter().position(|s| s == m).expect("sent");
    assert!(at(&NOTE_ON) < at(&write));
}

#[test]
fn a_frame_of_several_channel_voice_messages_reaches_the_link_as_each() {
    let service = Fixture::start();
    let mut page = service.connect_soon(ALLOWED);
    let program = [0xC1, 0x05];
    let bend = [0xE2, 0x00, 0x40];

    page.send(Message::binary(
        [NOTE_ON.as_slice(), &program, &bend].concat(),
    ))
    .expect("send");

    wait_for("the bend on the link", || {
        service.link.sent().contains(&bend.to_vec())
    });
    let sent = service.link.sent();
    let at = |m: &[u8]| sent.iter().position(|s| s == m).expect("sent");
    assert!(at(&NOTE_ON) < at(&program) && at(&program) < at(&bend));
}

#[test]
fn a_note_frame_that_is_not_whole_channel_voice_messages_is_refused() {
    let service = Fixture::start();
    let mut page = service.connect_soon(ALLOWED);
    let bad: [&[u8]; 6] = [
        &[0x90, 0x3C],                   // cut short
        &[0x90, 0x3C, 0x7F, 0x40],       // a stray data byte (running status)
        &[0x90, 0x3C, 0x90],             // a status byte where data goes
        &[0x90, 0x3C, 0x7F, 0xF8],       // a real-time message after it
        &[0x90, 0x3C, 0x7F, 0xF0, 0xF7], // SysEx after it
        &[0xF8],                         // a system message alone
    ];
    for frame in bad {
        page.send(Message::binary(frame.to_vec())).expect("send");
    }
    // Frames are handled in order, so this one on the link means the others
    // were seen.
    page.send(Message::binary(NOTE_OFF.to_vec())).expect("send");

    wait_for("the good note on the link", || {
        service.link.sent().contains(&NOTE_OFF.to_vec())
    });
    assert_eq!(service.link.sent(), vec![NOTE_OFF.to_vec()]);
}

#[test]
fn a_page_from_an_origin_off_the_list_cannot_play_notes() {
    let service = Fixture::start();
    let addr = service.url.trim_start_matches("ws://");
    let mut stranger = TcpStream::connect(addr).expect("connect");
    stranger.set_read_timeout(Some(WAIT)).expect("timeout");
    // The handshake and a note frame (masked with a zero key, so the payload
    // reads as is) in one write, as a client that does not wait would send.
    let handshake = "GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nUpgrade: websocket\r\n\
        Connection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\
        Sec-WebSocket-Version: 13\r\nOrigin: https://evil.example\r\n\r\n";
    let frame = [&[0x82, 0x83, 0, 0, 0, 0][..], &NOTE_ON].concat();
    stranger
        .write_all(&[handshake.as_bytes(), &frame].concat())
        .expect("write");
    // Refused (403), or dropped unanswered for bytes after its request: not
    // upgraded either way.
    let mut response = Vec::new();
    let _ = stranger.read_to_end(&mut response);
    let response = String::from_utf8_lossy(&response);
    assert!(!response.starts_with("HTTP/1.1 101"), "{response}");
    // A browser waits for the upgrade, and is refused before it can send.
    assert_eq!(service.refused_with(Some("https://evil.example")), 403);

    // The service has had time to act on the stranger's frame, had it
    // taken it: an allowed page's note gets there.
    let mut page = service.connect_soon(ALLOWED);
    page.send(Message::binary(NOTE_OFF.to_vec())).expect("send");
    wait_for("the allowed note on the link", || {
        service.link.sent().contains(&NOTE_OFF.to_vec())
    });
    assert!(!service.link.sent().contains(&NOTE_ON.to_vec()));
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
    let mut page = service.connect_soon_small_window(ALLOWED);
    // The page never reads, so its queue and the socket's buffers fill, and
    // the service drops it as too far behind to serve: no close frame says so.
    // The page's receive buffer is capped: Linux autotunes it up to about
    // 6 MiB, which with the service's send buffer could absorb the whole
    // flood, and the page would never be dropped.
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
