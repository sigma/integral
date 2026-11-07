//! An instance against a real Device service on a fake link: the status it
//! reports, and the target it adopts.

#![cfg(unix)]

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use integral_core::host_params::{Binding, PluginKind};
use integral_device_service::ensure_private_dir;
use integral_vst::family::link::Connection;
use integral_vst::family::{Instance, Status};

mod common;

use common::{TempDir, TestService, WAIT, wait_for};

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
