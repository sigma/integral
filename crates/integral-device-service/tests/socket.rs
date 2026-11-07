//! The client protocol over a real Unix socket, against a fake link.

#![cfg(unix)]

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use integral_core::address::{Address, DataSize};
use integral_core::host_params::{Binding, ParamIndex, PluginKind};
use integral_core::params::{self, part};
use integral_core::sysex;
use integral_device_service::protocol::{
    Bound, BulkKind, ClientId, Message, PROTOCOL_VERSION, ParamId, Reject, Request, read_frame,
    write_frame,
};
use integral_device_service::{
    Client, ClientEvent, Config, Events, FakeLink, Running, SequentialSessionIds, Service,
    SessionId, SocketServer, check_default_path_length, ensure_private_dir, socket_dir_in,
    socket_path_in, spawn,
};

const DEVICE_ID: u8 = 0x10;
const WAIT: Duration = Duration::from_secs(5);

/// A private directory of its own under the target directory, removed on
/// drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "s{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        ensure_private_dir(&dir).expect("private temp dir");
        Self(dir)
    }

    fn socket(&self) -> PathBuf {
        socket_path_in(&self.0)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A service on a fake link, serving a socket.
struct Fixture {
    link: FakeLink,
    running: Option<Running>,
    server: Option<SocketServer>,
}

impl Fixture {
    /// Start at `path`, with session IDs counting up from `first_session`,
    /// and the link up.
    fn start(path: &Path, first_session: u64) -> Self {
        let link = FakeLink::new();
        let config = Config {
            device_id: Some(DEVICE_ID),
            session_ids: Box::new(SequentialSessionIds(first_session)),
        };
        let service = Service::new(Box::new(link.clone()), config);
        let running = spawn(service, None).expect("start service");
        link.bring_up(None);
        let server = SocketServer::bind(path, running.handle()).expect("bind");
        Self {
            link,
            running: Some(running),
            server: Some(server),
        }
    }

    /// Stop it as a clean exit would: the socket goes, and so do the
    /// connections.
    fn kill(mut self) {
        self.server.take();
        self.running.take().expect("running").stop();
    }

    /// The service's current `parts[part].level`.
    fn level(&self, part: u8) -> u8 {
        let (tx, rx) = mpsc::channel();
        let running = self.running.as_ref().expect("running");
        running.handle().run(move |service| {
            let _ = tx.send(service.device().state().parts[usize::from(part)].level);
        });
        rx.recv_timeout(WAIT).expect("service answers")
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

/// The next event satisfying `pick`, skipping the others.
fn next<T>(events: &Events, what: &str, mut pick: impl FnMut(ClientEvent) -> Option<T>) -> T {
    let deadline = Instant::now() + WAIT;
    loop {
        let left = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_else(|| panic!("timed out waiting for {what}"));
        let event = events
            .wait_event(left)
            .unwrap_or_else(|| panic!("timed out waiting for {what}"));
        if let Some(found) = pick(event) {
            return found;
        }
    }
}

/// Wait for the client to be connected with the link up; returns its ID and
/// the session.
fn connected(events: &Events) -> (ClientId, SessionId) {
    let mut id = None;
    let mut session = None;
    next(events, "connect", |event| {
        match event {
            ClientEvent::Connected { client, session: s } => {
                id = Some(client);
                session = session.or(s);
            }
            ClientEvent::Message(Message::LinkUp { session: s }) => session = Some(s),
            _ => {}
        }
        (id.is_some() && session.is_some()).then_some(())
    });
    (id.expect("id"), session.expect("session"))
}

/// A client already connected.
fn join(path: &Path) -> (Client, Events, ClientId) {
    let (client, events) = Client::connect(path);
    let (id, _) = connected(&events);
    (client, events, id)
}

fn param_at(address: &Address) -> ParamId {
    let index = ParamIndex::new();
    let hit = index.locate(address)[0];
    ParamId::of(&index, hit.handle)
}

fn level_address(part_index: u8) -> Address {
    params::part_address(part_index, part::LEVEL)
}

fn level(part_index: u8) -> ParamId {
    param_at(&level_address(part_index))
}

fn dt1(part_index: u8, value: u8) -> Vec<u8> {
    sysex::build_dt1(DEVICE_ID, &level_address(part_index), &[value])
}

/// Make sure the service has handled everything `client` sent so far, by
/// round-tripping a change through its own subscription: it pushes Part 16's
/// mute, which the tests leave alone. A push, because the service drops a
/// write of a value it knows the device has, and `settle` may be called twice.
fn settle(client: &Client, events: &Events) {
    let mute = param_at(&params::part_address(15, part::MUTE));
    client.subscribe(Binding::ANY);
    client.send(Request::Push {
        requests: vec![Request::Write {
            param: mute.clone(),
            binding: Binding::ANY,
            value: 0,
        }],
    });
    next(events, "its own change back", |event| match event {
        ClientEvent::Message(Message::Changed { param, .. }) if param == mute => Some(()),
        _ => None,
    });
}

#[test]
fn a_typed_write_reaches_the_link_as_the_right_dt1() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (client, _events, _) = join(&dir.socket());

    assert!(client.write(level(2), Binding::ANY, 100));

    wait_for("the DT1", || !service.link.sent().is_empty());
    assert_eq!(service.link.sent(), vec![dt1(2, 100)]);
    assert_eq!(service.level(2), 100);
}

#[test]
fn a_write_names_its_writer_and_the_writer_hears_it_too() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (client, events, me) = join(&dir.socket());
    client.subscribe(Binding::ANY);
    client.write(level(2), Binding::ANY, 77);

    let (writer, value) = next(&events, "its change", |event| match event {
        ClientEvent::Message(Message::Changed {
            writer,
            param,
            value,
            ..
        }) if param == level(2) => Some((writer, value)),
        _ => None,
    });
    assert_eq!((writer, value), (me, 77));
}

#[test]
fn a_raw_clients_dt1_updates_the_cache_and_notifies_a_typed_subscriber() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (typed, typed_events, _) = join(&dir.socket());
    settle(&typed, &typed_events);
    let (raw, _raw_events, raw_id) = join(&dir.socket());

    assert!(raw.raw(dt1(2, 33)));

    let (writer, param, value) = next(&typed_events, "the change", |event| match event {
        ClientEvent::Message(Message::Changed {
            writer,
            param,
            value,
            ..
        }) if param == level(2) => Some((writer, param, value)),
        _ => None,
    });
    assert_eq!((writer, value), (raw_id, 33));
    assert_eq!(param, level(2));
    assert_eq!(service.level(2), 33);
    wait_for("the DT1 on the link", || {
        service.link.sent().contains(&dt1(2, 33))
    });
}

#[test]
fn a_reply_to_a_raw_clients_rq1_reaches_every_raw_client() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (first, first_events, _) = join(&dir.socket());
    let (second, second_events, _) = join(&dir.socket());
    let (bystander, bystander_events, _) = join(&dir.socket());
    for (client, events) in [(&first, &first_events), (&second, &second_events)] {
        client.receive_raw(true);
        settle(client, events);
    }
    settle(&bystander, &bystander_events);

    let size = DataSize::from_value(1).expect("one byte");
    let rq1 = sysex::build_rq1(DEVICE_ID, &level_address(4), &size);
    assert!(first.raw(rq1.clone()));
    wait_for("the RQ1", || service.link.sent().contains(&rq1));
    let reply = dt1(4, 91);
    service.link.device_says(&reply);

    for events in [&first_events, &second_events] {
        let bytes = next(events, "the reply", |event| match event {
            ClientEvent::Message(Message::Sysex { bytes }) => Some(bytes),
            _ => None,
        });
        assert_eq!(bytes, reply);
    }
    // A client that did not ask never sees SysEx.
    while let Some(event) = bystander_events.wait_event(Duration::from_millis(100)) {
        assert!(!matches!(
            event,
            ClientEvent::Message(Message::Sysex { .. })
        ));
    }
}

#[test]
fn a_typed_read_is_answered_to_the_reader_only() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (reader, reader_events, _) = join(&dir.socket());
    let (watcher, watcher_events, _) = join(&dir.socket());
    settle(&watcher, &watcher_events);
    reader.read(level(5), Binding::ANY);
    let size = DataSize::from_value(1).expect("one byte");
    let rq1 = sysex::build_rq1(DEVICE_ID, &level_address(5), &size);
    wait_for("the RQ1", || service.link.sent().contains(&rq1));
    service.link.device_says(&dt1(5, 64));

    let value = next(&reader_events, "the value", |event| match event {
        ClientEvent::Message(Message::Value { param, value, .. }) if param == level(5) => {
            Some(value)
        }
        _ => None,
    });
    assert_eq!(value, 64);
    // The watcher hears nothing of it.
    while let Some(event) = watcher_events.wait_event(Duration::from_millis(100)) {
        assert!(
            !matches!(
                event,
                ClientEvent::Message(Message::Changed { .. } | Message::Value { .. })
            ),
            "{event:?}"
        );
    }
}

#[test]
fn a_read_the_device_never_answers_is_refused_after_a_while() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (client, events, _) = join(&dir.socket());
    client.read(level(5), Binding::ANY);
    let reason = next(&events, "the timeout", |event| match event {
        ClientEvent::Message(Message::Rejected { reason, .. }) => Some(reason),
        _ => None,
    });
    assert_eq!(reason, Reject::Timeout);
}

#[test]
fn a_tone_change_is_one_bulk_event() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (client, events, me) = join(&dir.socket());
    client.subscribe(Binding::of_part(3));
    client.send(Request::ChangeTone {
        part: 3,
        msb: 89,
        lsb: 64,
        pc: 1,
    });

    let bulk = next(&events, "the bulk event", |event| match event {
        ClientEvent::Message(Message::Bulk {
            writer,
            kind,
            binding,
            ..
        }) => Some((writer, kind, binding)),
        _ => None,
    });
    assert_eq!(bulk, (me, BulkKind::Tone, Binding::of_part(3)));
}

#[test]
fn a_bad_request_is_rejected_and_nothing_reaches_the_link() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (client, events, _) = join(&dir.socket());

    client.write(level(2), Binding::ANY, 200); // above 127
    client.write(
        ParamId {
            plugin: level(2).plugin,
            id: "no/such/0x00".to_string(),
        },
        Binding::ANY,
        1,
    );
    let tone = {
        let index = ParamIndex::new();
        let handle = index
            .find(
                integral_core::host_params::PluginKind::PcmSynth,
                "pcms/common/0x0E",
            )
            .expect("a tone parameter");
        ParamId::of(&index, handle)
    };
    client.write(tone.clone(), Binding::ANY, 1); // no Part given
    client.write(tone, Binding::of_part(40), 1); // no such Part
    client.raw(vec![0xF0, 0x90, 0xF7]); // a byte over 127 inside
    let mut bad_dt1 = dt1(1, 5);
    let checksum = bad_dt1.len() - 2;
    bad_dt1[checksum] ^= 1;
    client.raw(bad_dt1);
    for _ in 0..6 {
        let reason = next(&events, "a rejection", |event| match event {
            ClientEvent::Message(Message::Rejected { reason, .. }) => Some(reason),
            _ => None,
        });
        assert!(matches!(reason, Reject::Invalid { .. }), "{reason:?}");
    }
    assert!(service.link.sent().is_empty());
}

#[test]
fn writes_are_refused_while_the_link_is_down() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (client, events, _) = join(&dir.socket());
    service.link.bring_down();
    next(&events, "link down", |event| {
        matches!(event, ClientEvent::Message(Message::LinkDown)).then_some(())
    });

    client.write(level(1), Binding::ANY, 5);
    let reason = next(&events, "a rejection", |event| match event {
        ClientEvent::Message(Message::Rejected { reason, .. }) => Some(reason),
        _ => None,
    });
    assert_eq!(reason, Reject::LinkDown);

    // A new session begins when it comes back, and writes are taken again.
    service.link.bring_up(None);
    next(&events, "link up", |event| {
        matches!(event, ClientEvent::Message(Message::LinkUp { .. })).then_some(())
    });
    client.write(level(1), Binding::ANY, 5);
    wait_for("the DT1", || service.link.sent().contains(&dt1(1, 5)));
}

fn hello() -> Request {
    Request::Hello {
        version: PROTOCOL_VERSION,
        subscriptions: Vec::new(),
        raw: false,
        bound: None,
    }
}

#[test]
fn a_client_with_another_version_is_turned_away() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let mut stream = UnixStream::connect(dir.socket()).expect("connect");
    write_frame(
        &mut stream,
        &Request::Hello {
            version: PROTOCOL_VERSION + 1,
            subscriptions: Vec::new(),
            raw: false,
            bound: None,
        },
    )
    .expect("hello");
    let reply: Message = read_frame(&mut stream).expect("reply");
    assert_eq!(
        reply,
        Message::Rejected {
            reason: Reject::Version {
                server: PROTOCOL_VERSION
            },
            param: None
        }
    );
    assert!(
        read_frame::<Message>(&mut stream).is_err(),
        "then it closes"
    );
}

#[test]
fn a_refusal_of_the_version_is_final_for_the_client() {
    let dir = TempDir::new();
    let listener = UnixListener::bind(dir.socket()).expect("bind");
    let accepts = std::sync::Arc::new(AtomicUsize::new(0));
    let counter = std::sync::Arc::clone(&accepts);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            counter.fetch_add(1, Ordering::SeqCst);
            let _ = read_frame::<Request>(&mut stream);
            let reason = Reject::Version { server: 99 };
            let _ = write_frame(
                &mut stream,
                &Message::Rejected {
                    reason,
                    param: None,
                },
            );
        }
    });
    let (client, events) = Client::connect(dir.socket());
    let server = next(&events, "the refusal", |event| match event {
        ClientEvent::Incompatible { server } => Some(server),
        _ => None,
    });
    assert_eq!(server, 99);
    // It does not come back, and it says so: nothing more is accepted.
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(accepts.load(Ordering::SeqCst), 1);
    assert!(events.try_event().is_none());
    assert!(!client.subscribe(Binding::ANY), "the client has stopped");
}

#[test]
fn a_request_before_hello_is_refused() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let mut stream = UnixStream::connect(dir.socket()).expect("connect");
    write_frame(&mut stream, &Request::LoadMixer).expect("request");
    let reply: Message = read_frame(&mut stream).expect("reply");
    assert_eq!(
        reply,
        Message::Rejected {
            reason: Reject::NotGreeted,
            param: None
        }
    );
}

#[test]
fn a_frame_that_does_not_decode_costs_only_that_frame() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let mut stream = UnixStream::connect(dir.socket()).expect("connect");
    write_frame(&mut stream, &hello()).expect("hello");
    let _welcome: Message = read_frame(&mut stream).expect("welcome");
    for body in [&b"{x}"[..], br#"{"type":"from_the_future"}"#] {
        stream
            .write_all(&(body.len() as u32).to_be_bytes())
            .expect("length");
        stream.write_all(body).expect("body");
        let reply: Message = read_frame(&mut stream).expect("a refusal, not a close");
        assert!(matches!(
            reply,
            Message::Rejected {
                reason: Reject::Malformed { .. },
                ..
            }
        ));
    }
    // The connection still works.
    write_frame(
        &mut stream,
        &Request::Subscribe {
            binding: Binding::ANY,
        },
    )
    .expect("subscribe");
    write_frame(
        &mut stream,
        &Request::Read {
            param: level(0),
            binding: Binding::ANY,
        },
    )
    .expect("read");
    let mut reply = BufReader::new(&stream);
    let _ = reply.fill_buf().expect("still open");
}

#[test]
fn an_oversized_frame_ends_the_connection() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let mut stream = UnixStream::connect(dir.socket()).expect("connect");
    stream.write_all(&u32::MAX.to_be_bytes()).expect("length");
    let mut byte = [0u8; 1];
    assert_eq!(stream.read(&mut byte).unwrap_or(0), 0, "closed");
}

#[test]
fn a_second_service_cannot_take_a_held_path_but_can_take_a_stale_socket() {
    let dir = TempDir::new();
    let first = Fixture::start(&dir.socket(), 1);
    let link = FakeLink::new();
    let config = Config {
        device_id: Some(DEVICE_ID),
        session_ids: Box::new(SequentialSessionIds(1)),
    };
    let running = spawn(Service::new(Box::new(link), config), None).expect("start");
    let refused = SocketServer::bind(dir.socket(), running.handle()).err();
    assert_eq!(
        refused.map(|e| e.kind()),
        Some(std::io::ErrorKind::AddrInUse)
    );

    // A service that died without cleaning up leaves a file nobody holds.
    let stale = dir.0.join("stale.sock");
    drop(UnixListener::bind(&stale).expect("bind"));
    let server = SocketServer::bind(&stale, running.handle()).expect("rebind");
    drop(server);
    running.stop();
    drop(first);
}

#[test]
fn a_socket_in_a_directory_others_can_enter_is_refused() {
    let dir = TempDir::new();
    fs::set_permissions(&dir.0, fs::Permissions::from_mode(0o755)).expect("loosen");
    let link = FakeLink::new();
    let config = Config {
        device_id: Some(DEVICE_ID),
        session_ids: Box::new(SequentialSessionIds(1)),
    };
    let running = spawn(Service::new(Box::new(link), config), None).expect("start");
    let refused = SocketServer::bind(dir.socket(), running.handle()).err();
    assert_eq!(
        refused.map(|e| e.kind()),
        Some(std::io::ErrorKind::PermissionDenied)
    );
    assert!(ensure_private_dir(&dir.0).is_err());
    running.stop();
}

#[test]
fn the_socket_directory_is_made_private_under_a_base() {
    let base = TempDir::new();
    let dir = socket_dir_in(&base.0).expect("dir");
    let mode = fs::metadata(&dir).expect("exists").permissions().mode();
    assert_eq!(mode & 0o077, 0, "{mode:o}");
    assert_eq!(socket_dir_in(&base.0).expect("again"), dir);
}

#[test]
fn the_default_socket_path_fits_a_socket() {
    check_default_path_length().expect("short enough");
}

#[test]
fn killing_the_service_and_restarting_it_reconnects_the_client() {
    let dir = TempDir::new();
    let first = Fixture::start(&dir.socket(), 10);
    let (client, events) = Client::connect(dir.socket());
    let (_, first_session) = connected(&events);
    client.subscribe(Binding::of_part(2));
    settle(&client, &events);

    first.kill();
    next(&events, "the disconnect", |event| {
        matches!(event, ClientEvent::Disconnected).then_some(())
    });

    let _second = Fixture::start(&dir.socket(), 20);
    let (_, second_session) = connected(&events);
    assert_ne!(first_session, second_session, "a restart is a new session");
    assert_subscription_restored(&dir, &events);
}

/// Another client's change reaches the client at once after it connected:
/// its subscription was in force before the connection was reported.
fn assert_subscription_restored(dir: &TempDir, events: &Events) {
    let (other, _other_events, other_id) = join(&dir.socket());
    other.write(level(2), Binding::ANY, 12);
    let writer = next(events, "the change", |event| match event {
        ClientEvent::Message(Message::Changed {
            writer, value: 12, ..
        }) => Some(writer),
        _ => None,
    });
    assert_eq!(writer, other_id);
}

/// Not a test: the body of the service process that
/// `a_service_that_is_killed_leaves_its_socket_and_the_client_reconnects`
/// starts and kills. Reads a socket path from stdin, serves it, and runs
/// until stdin closes.
#[test]
#[ignore = "run by the test that starts it"]
fn service_process() {
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).expect("a path");
    let service = Fixture::start(Path::new(line.trim()), 100);
    println!("ready");
    let _ = std::io::stdin().read_to_end(&mut Vec::new());
    drop(service);
}

#[test]
fn a_service_that_is_killed_leaves_its_socket_and_the_client_reconnects() {
    let dir = TempDir::new();
    let mut child = Command::new(std::env::current_exe().expect("test binary"))
        .args(["--exact", "service_process", "--ignored", "--nocapture"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start the service process");
    writeln!(
        child.stdin.as_mut().expect("stdin"),
        "{}",
        dir.socket().display()
    )
    .expect("send the path");
    let mut out = BufReader::new(child.stdout.take().expect("stdout"));
    let mut line = String::new();
    while line.trim() != "ready" {
        line.clear();
        assert_ne!(out.read_line(&mut line).expect("read"), 0, "child exited");
    }

    let (client, events) = Client::connect(dir.socket());
    let (_, first_session) = connected(&events);
    client.subscribe(Binding::ANY);
    settle(&client, &events);

    child.kill().expect("SIGKILL"); // no cleanup runs
    child.wait().expect("reaped");
    assert!(dir.socket().exists(), "the dead service left its socket");
    next(&events, "the disconnect", |event| {
        matches!(event, ClientEvent::Disconnected).then_some(())
    });

    let _second = Fixture::start(&dir.socket(), 200);
    let (_, second_session) = connected(&events);
    assert_ne!(first_session, second_session);
    assert_subscription_restored(&dir, &events);
}

#[test]
fn a_client_that_falls_behind_is_told_to_re_read() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (client, events, _) = join(&dir.socket());
    settle(&client, &events);
    // A flood of device changes the caller does not read.
    for i in 0..3000u32 {
        let value = u8::try_from(1 + i % 100).expect("small");
        service.link.device_says(&dt1(7, value));
    }
    std::thread::sleep(Duration::from_millis(500));
    let mut overflowed = false;
    while let Some(event) = events.wait_event(Duration::from_millis(200)) {
        overflowed |= event == ClientEvent::Overflowed;
    }
    assert!(overflowed);
}

#[test]
fn a_client_that_never_reads_is_dropped_and_the_others_carry_on() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let mut stuck = UnixStream::connect(dir.socket()).expect("connect");
    write_frame(
        &mut stuck,
        &Request::Hello {
            version: PROTOCOL_VERSION,
            subscriptions: Vec::new(),
            raw: true,
            bound: None,
        },
    )
    .expect("hello");
    let (other, other_events, _) = join(&dir.socket());
    settle(&other, &other_events);

    // Far more SysEx than a socket buffer and the queue behind it hold.
    let big = sysex::build_dt1(DEVICE_ID, &level_address(0), &[1; 200]);
    for _ in 0..12_000 {
        service.link.device_says(&big);
    }
    // The service is not stuck behind it: a write still goes through.
    other.write(level(3), Binding::ANY, 9);
    wait_for("the DT1", || service.link.sent().contains(&dt1(3, 9)));
    // And the stuck client was cut off, which it finds when it finally reads.
    stuck.set_read_timeout(Some(WAIT)).expect("timeout");
    let mut reader = BufReader::new(&stuck);
    let mut frames = 0;
    while read_frame::<Message>(&mut reader).is_ok() {
        frames += 1;
    }
    assert!(
        frames < 12_000,
        "it was cut off, not served to the end ({frames})"
    );
}

#[test]
fn a_full_queue_says_so_and_a_hello_is_not_sent_by_hand() {
    let dir = TempDir::new();
    // No service: the client keeps trying, and its queue fills.
    let (client, _events) = Client::connect(dir.socket());
    assert!(!client.send(hello()));
    let index = ParamIndex::new();
    let param = level(0);
    let full = (0..1_000_000).any(|_| !client.write(param.clone(), Binding::ANY, 1));
    assert!(full, "a bounded queue refuses at some point");

    // Room again, once the client thread has taken what it can.
    let mut sender = loop {
        if let Some(sender) = client.sender() {
            break sender;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let handle = param.handle(&index).expect("handle");
    let full = (0..1_000_000).any(|_| !sender.write(handle, Binding::ANY, 1));
    assert!(full, "so does a ring");
}

#[test]
fn a_real_time_sender_writes_through_its_ring() {
    let dir = TempDir::new();
    let service = Fixture::start(&dir.socket(), 1);
    let (client, _events, _) = join(&dir.socket());
    let mut sender = client.sender().expect("a ring");
    let index = ParamIndex::new();
    let handle = level(6).handle(&index).expect("handle");

    assert!(sender.write(handle, Binding::ANY, 42));

    wait_for("the DT1", || service.link.sent().contains(&dt1(6, 42)));
}

#[test]
fn dropping_the_client_stops_it_and_closes_its_connection() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (client, events, _) = join(&dir.socket());
    let clone = client.clone();
    drop(client);
    assert!(clone.subscribe(Binding::ANY), "a clone keeps it alive");
    drop(clone); // joins the threads
    // The threads are gone, so the event queue is closed: a wait ends at
    // once instead of timing out.
    let started = Instant::now();
    while events.wait_event(WAIT).is_some() {}
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// What `client` is given when it claims a target of `plugin`.
fn claim(client: &Client, events: &Events, plugin: PluginKind) -> Option<Binding> {
    client.claim(plugin);
    next(events, "the claim", |event| match event {
        ClientEvent::Message(Message::Claimed {
            plugin: p, binding, ..
        }) if p == plugin => Some(binding),
        _ => None,
    })
}

fn bound(plugin: PluginKind, binding: Binding) -> Option<Bound> {
    Some(Bound { plugin, binding })
}

/// Bind `client` and wait until the service has taken it: it refuses a write
/// to a parameter that does not exist right behind, on the same connection.
fn bind(client: &Client, events: &Events, bound: Option<Bound>) {
    client.bind(bound);
    let nothing = ParamId {
        plugin: PluginKind::StudioSet,
        id: "no/such/parameter".to_string(),
    };
    client.write(nothing.clone(), Binding::ANY, 0);
    next(events, "the end of the bind", |event| match event {
        ClientEvent::Message(Message::Rejected { param, .. })
            if param.as_ref() == Some(&nothing) =>
        {
            Some(())
        }
        _ => None,
    });
}

fn part(n: u8) -> Option<Binding> {
    Some(Binding::of_part(n))
}

#[test]
fn claims_take_the_next_free_part_and_a_leaver_frees_its_own() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (first, first_events, _) = join(&dir.socket());
    let (second, second_events, _) = join(&dir.socket());
    let (third, third_events, _) = join(&dir.socket());

    assert_eq!(claim(&first, &first_events, PluginKind::PcmSynth), part(0));
    assert_eq!(
        claim(&second, &second_events, PluginKind::PcmSynth),
        part(1)
    );
    // Claiming again keeps the claimer's own Part.
    assert_eq!(claim(&first, &first_events, PluginKind::PcmSynth), part(0));

    drop((first, first_events));
    wait_for("the first to leave", || {
        claim(&third, &third_events, PluginKind::SnSynth) == part(0)
    });
}

#[test]
fn a_part_is_taken_by_any_tone_plugin_or_a_kit_wide_kit_plugin_but_not_by_a_key() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (asker, asker_events, _) = join(&dir.socket());
    let holders: Vec<_> = [
        (PluginKind::SnSynth, Binding::of_part(0)),
        (PluginKind::PcmDrum, Binding::of_part(1)),
        (PluginKind::SnDrum, Binding::of_part(2).with_key(40)),
        (PluginKind::StudioSet, Binding::ANY),
    ]
    .into_iter()
    .map(|(plugin, binding)| {
        let (client, events, _) = join(&dir.socket());
        bind(&client, &events, bound(plugin, binding));
        (client, events)
    })
    .collect();

    // Part 0 (a Tone plugin of another type) and Part 1 (a whole kit) are
    // taken; Part 2 has only a key bound, and the Studio Set takes no Part.
    assert_eq!(claim(&asker, &asker_events, PluginKind::PcmSynth), part(2));
    assert_eq!(claim(&asker, &asker_events, PluginKind::SnDrum), part(2));
    drop(holders);
}

#[test]
fn the_studio_set_plugin_has_one_target_and_a_full_kind_has_none() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (first, first_events, _) = join(&dir.socket());
    let (second, second_events, _) = join(&dir.socket());
    let studio = PluginKind::StudioSet;
    assert_eq!(claim(&first, &first_events, studio), Some(Binding::ANY));
    assert_eq!(claim(&second, &second_events, studio), None);

    let kind = PluginKind::SnDrum;
    let fillers: Vec<_> = kind
        .targets()
        .into_iter()
        .map(|target| {
            let (client, events, _) = join(&dir.socket());
            assert_eq!(claim(&client, &events, kind), Some(target));
            (client, events)
        })
        .collect();
    assert_eq!(claim(&second, &second_events, kind), None);
    drop(fillers);
}

#[test]
fn concurrent_claims_get_distinct_parts() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let path = dir.socket();
    let clients: Vec<_> = (0..12).map(|_| join(&path)).collect();
    // All twelve ask at once, each from a thread of its own.
    let start = std::sync::Arc::new(std::sync::Barrier::new(clients.len()));
    let handles: Vec<_> = clients
        .into_iter()
        .map(|(client, events, _)| {
            let start = std::sync::Arc::clone(&start);
            std::thread::spawn(move || {
                start.wait();
                let got = claim(&client, &events, PluginKind::PcmSynth);
                (got, client, events)
            })
        })
        .collect();
    let got: Vec<_> = handles
        .into_iter()
        .map(|h| h.join().expect("claimer"))
        .collect();
    let got: Vec<Option<Binding>> = got.iter().map(|(b, ..)| *b).collect();
    let mut parts: Vec<u8> = got
        .iter()
        .map(|b| b.expect("a free part").part.unwrap())
        .collect();
    parts.sort_unstable();
    assert_eq!(parts, (0..12).collect::<Vec<u8>>());
}

#[test]
fn a_free_part_holding_the_plugins_tone_type_is_preferred() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (client, events, _) = join(&dir.socket());
    client.subscribe(Binding::ANY);
    // Part 6 (index 5) plays an SN Acoustic tone (Bank Select MSB 89).
    client.send(Request::ChangeTone {
        part: 5,
        msb: 89,
        lsb: 64,
        pc: 0,
    });
    next(&events, "the tone change", |event| match event {
        ClientEvent::Message(Message::Bulk {
            kind: BulkKind::Tone,
            ..
        }) => Some(()),
        _ => None,
    });
    let (other, other_events, _) = join(&dir.socket());
    assert_eq!(
        claim(&other, &other_events, PluginKind::SnAcoustic),
        part(5)
    );
    // Taken now: the next one falls back to the first free Part.
    let (third, third_events, _) = join(&dir.socket());
    assert_eq!(
        claim(&third, &third_events, PluginKind::SnAcoustic),
        part(0)
    );
}

#[test]
fn a_binding_the_plugin_cannot_have_is_refused_and_binds_nothing() {
    let dir = TempDir::new();
    let _service = Fixture::start(&dir.socket(), 1);
    let (client, events, _) = join(&dir.socket());
    let (other, other_events, _) = join(&dir.socket());
    for (plugin, binding) in [
        (PluginKind::PcmSynth, Binding::of_part(16)),
        (PluginKind::PcmSynth, Binding::of_part(0).with_key(40)),
        (PluginKind::SnDrum, Binding::of_part(0).with_key(26)),
        (PluginKind::PcmDrum, Binding::of_part(0).with_key(109)),
        (PluginKind::StudioSet, Binding::of_part(0)),
    ] {
        client.bind(bound(plugin, binding));
        next(&events, "a refusal", |event| match event {
            ClientEvent::Message(Message::Rejected {
                reason: Reject::Invalid { .. },
                ..
            }) => Some(()),
            _ => None,
        });
    }
    assert_eq!(claim(&other, &other_events, PluginKind::PcmSynth), part(0));
}

#[test]
fn a_reconnecting_client_is_bound_again_before_it_is_welcomed() {
    let dir = TempDir::new();
    let first = Fixture::start(&dir.socket(), 10);
    let (client, events) = Client::connect(dir.socket());
    connected(&events);
    let kind = PluginKind::SnAcoustic;
    bind(&client, &events, bound(kind, Binding::of_part(3)));

    first.kill();
    next(&events, "the disconnect", |event| {
        matches!(event, ClientEvent::Disconnected).then_some(())
    });
    let _second = Fixture::start(&dir.socket(), 20);
    connected(&events);

    // Parts 0 to 2 are claimed by others and Part 3 by the reconnected
    // client, which the new service knew from the Hello: the next free is 4.
    let fillers: Vec<_> = (0..3)
        .map(|n| {
            let (filler, filler_events, _) = join(&dir.socket());
            assert_eq!(claim(&filler, &filler_events, kind), part(n));
            (filler, filler_events)
        })
        .collect();
    let (asker, asker_events, _) = join(&dir.socket());
    assert_eq!(claim(&asker, &asker_events, kind), part(4));
    drop(fillers);
}

#[test]
fn after_a_restart_a_claim_waits_for_the_restored_clients_to_say_what_they_hold() {
    let dir = TempDir::new();
    let first = Fixture::start(&dir.socket(), 10);
    let (restored, restored_events, _) = join(&dir.socket());
    bind(
        &restored,
        &restored_events,
        bound(PluginKind::PcmSynth, Binding::of_part(0)),
    );
    first.kill();
    next(&restored_events, "the disconnect", |event| {
        matches!(event, ClientEvent::Disconnected).then_some(())
    });

    // A fresh instance reaches the new service and claims before the
    // restored one is back (it retries with a delay); it must not be given
    // Part 0, which the restored one will say it holds.
    let _second = Fixture::start(&dir.socket(), 20);
    let (fresh, fresh_events, _) = join(&dir.socket());
    assert_eq!(claim(&fresh, &fresh_events, PluginKind::PcmSynth), part(1));
    connected(&restored_events);
}

#[test]
fn a_server_for_one_process_refuses_every_other() {
    let dir = TempDir::new();
    let link = FakeLink::new();
    let running = spawn(
        Service::new(
            Box::new(link.clone()),
            Config {
                device_id: Some(DEVICE_ID),
                session_ids: Box::new(SequentialSessionIds(1)),
            },
        ),
        None,
    )
    .expect("spawn");
    link.bring_up(Some(DEVICE_ID));
    let ours = std::process::id();

    // Not this process: the connection is closed before any hello.
    let other = dir.0.join("o.sock");
    let server = SocketServer::bind_only(&other, running.handle(), Some(ours + 1)).expect("bind");
    let mut stream = UnixStream::connect(&other).expect("connect");
    stream.set_read_timeout(Some(WAIT)).expect("a read timeout");
    let mut byte = [0u8; 1];
    assert_eq!(stream.read(&mut byte).expect("closed, not timed out"), 0);
    drop(server);

    // This process: served.
    let own = dir.0.join("p.sock");
    let _server = SocketServer::bind_only(&own, running.handle(), Some(ours)).expect("bind");
    let _joined = join(&own);
    running.stop();
}
