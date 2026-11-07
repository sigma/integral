//! An instance against a real Device service on a fake link: the status it
//! reports, and the target it adopts.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use integral_core::host_params::{Binding, PluginKind};
use integral_device_service::{
    Config, FakeLink, Running, SequentialSessionIds, Service, SocketServer, ensure_private_dir,
    socket_path_in, spawn,
};
use integral_vst::family::link::Connection;
use integral_vst::family::{Instance, Status};

const WAIT: Duration = Duration::from_secs(5);

/// A private directory under the target directory, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "v{}-{}",
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
struct TestService {
    link: FakeLink,
    running: Option<Running>,
    server: Option<SocketServer>,
}

impl TestService {
    fn start(path: &Path) -> Self {
        let link = FakeLink::new();
        let config = Config {
            device_id: Some(0x10),
            session_ids: Box::new(SequentialSessionIds(1)),
        };
        let running = spawn(Service::new(Box::new(link.clone()), config), None).expect("spawn");
        let server = SocketServer::bind(path, running.handle()).expect("bind");
        Self {
            link,
            running: Some(running),
            server: Some(server),
        }
    }
}

impl Drop for TestService {
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

/// A settled instance of `kind` connected to the service at `path`.
fn settled(kind: PluginKind, path: &Path) -> (Arc<Instance>, Connection) {
    let instance = Arc::new(Instance::new(kind));
    instance.settle();
    let connection = Connection::start(path, Arc::clone(&instance));
    (instance, connection)
}

#[test]
fn the_status_goes_from_no_service_to_device_offline_to_connected() {
    let dir = TempDir::new();
    let (instance, _connection) = settled(PluginKind::StudioSet, &dir.socket());
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(instance.status.get(), Status::ServiceNotRunning);

    // The service starts later: the instance finds it, and the link is down.
    let service = TestService::start(&dir.socket());
    wait_for("the service", || {
        instance.status.get() == Status::DeviceOffline
    });
    service.link.bring_up(None);
    wait_for("the link", || instance.status.get() == Status::Connected);
    service.link.bring_down();
    wait_for("the link to drop", || {
        instance.status.get() == Status::DeviceOffline
    });

    drop(service);
    wait_for("the service to go", || {
        instance.status.get() == Status::ServiceNotRunning
    });
}

#[test]
fn new_instances_claim_the_next_free_part_and_a_leaver_frees_its_own() {
    let dir = TempDir::new();
    let _service = TestService::start(&dir.socket());
    let kind = PluginKind::PcmSynth;

    // A claim binds the instance at the service as it is answered, so the
    // next one never sees a Part as free when it is not.
    let take = || {
        let (instance, connection) = settled(kind, &dir.socket());
        wait_for("a target", || instance.binding.get().is_some());
        (instance, connection)
    };
    let (first, first_connection) = take();
    let (second, _second_connection) = take();
    assert_eq!(first.binding.get(), Some(Binding::of_part(0)));
    assert_eq!(second.binding.get(), Some(Binding::of_part(1)));

    // The service frees the Part when it notices the connection is gone:
    // try until a new instance is given it.
    drop(first_connection);
    let deadline = Instant::now() + WAIT;
    loop {
        let (third, _third_connection) = take();
        if third.binding.get() == Some(Binding::of_part(0)) {
            break;
        }
        assert!(Instant::now() < deadline, "Part 0 was never freed");
    }
}

#[test]
fn instances_claiming_at_once_get_distinct_parts() {
    let dir = TempDir::new();
    let _service = TestService::start(&dir.socket());
    let path = dir.socket();
    let all: Vec<_> = std::thread::scope(|scope| {
        let starts: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| settled(PluginKind::SnSynth, &path)))
            .collect();
        starts.into_iter().map(|h| h.join().unwrap()).collect()
    });
    wait_for("every target", || {
        all.iter()
            .all(|(instance, _)| instance.binding.get().is_some())
    });
    let mut parts: Vec<u8> = all
        .iter()
        .map(|(instance, _)| instance.binding.get().unwrap().part.unwrap())
        .collect();
    parts.sort_unstable();
    assert_eq!(parts, (0..8).collect::<Vec<u8>>());
}

#[test]
fn a_restored_instance_keeps_its_part_and_others_skip_it() {
    let dir = TempDir::new();
    let _service = TestService::start(&dir.socket());
    let kind = PluginKind::SnSynth;

    let restored = Arc::new(Instance::new(kind));
    restored.binding.set(Some(Binding::of_part(0)));
    restored.settle();
    let _connection = Connection::start(dir.socket(), Arc::clone(&restored));
    // Welcomed means its Hello, which carries the Binding, was handled.
    wait_for("the restored instance", || {
        restored.status.get() != Status::ServiceNotRunning
    });

    let (fresh, _fresh_connection) = settled(kind, &dir.socket());
    wait_for("a target", || fresh.binding.get().is_some());
    assert_eq!(fresh.binding.get(), Some(Binding::of_part(1)));
    assert_eq!(restored.binding.get(), Some(Binding::of_part(0)));
}

#[test]
fn an_instance_that_has_not_settled_takes_no_part() {
    let dir = TempDir::new();
    let _service = TestService::start(&dir.socket());
    let instance = Arc::new(Instance::new(PluginKind::PcmSynth));
    let _connection = Connection::start(dir.socket(), Arc::clone(&instance));
    wait_for("the service", || {
        instance.status.get() != Status::ServiceNotRunning
    });
    // Another instance's claim shows what is free; this one holds nothing.
    let (other, _other_connection) = settled(PluginKind::PcmSynth, &dir.socket());
    wait_for("a target", || other.binding.get().is_some());
    assert_eq!(other.binding.get(), Some(Binding::of_part(0)));
    assert_eq!(instance.binding.get(), None);
    instance.settle();
    wait_for("its target", || instance.binding.get().is_some());
    assert_eq!(instance.binding.get(), Some(Binding::of_part(1)));
}

#[test]
fn a_service_without_claim_makes_the_instance_report_it_incompatible() {
    use integral_device_service::SessionId;
    use integral_device_service::protocol::{
        ClientId, Message, PROTOCOL_VERSION, Reject, read_frame, write_frame,
    };
    use std::os::unix::net::UnixListener;

    let dir = TempDir::new();
    ensure_private_dir(&dir.0).expect("dir");
    let listener = UnixListener::bind(dir.socket()).expect("bind");
    // A service from before Bind and Claim: it welcomes, and answers any
    // request it does not know as malformed.
    let old = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let _hello: serde_json::Value = read_frame(&mut stream).expect("hello");
        let welcome = Message::Welcome {
            version: PROTOCOL_VERSION,
            client: ClientId(1),
            session: Some(SessionId(1)),
        };
        write_frame(&mut stream, &welcome).expect("welcome");
        while let Ok(request) = read_frame::<serde_json::Value>(&mut stream) {
            if matches!(request["type"].as_str(), Some("claim" | "bind")) {
                let refusal = Message::Rejected {
                    reason: Reject::Malformed {
                        message: "unknown variant".into(),
                    },
                    param: None,
                };
                write_frame(&mut stream, &refusal).expect("refusal");
            }
        }
    });

    let (instance, connection) = settled(PluginKind::PcmSynth, &dir.socket());
    wait_for("the incompatibility", || {
        instance.status.get() == Status::ServiceIncompatible
    });
    assert_eq!(instance.binding.get(), None);
    drop(connection);
    old.join().expect("old service");
}
