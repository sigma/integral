//! `Client::flush` against a plain listener: what it promises about frames
//! queued before a drop.
#![cfg(unix)]

use std::fs;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use integral_device_service::protocol::{Message, Reject, Request, read_frame, write_frame};
use integral_device_service::{Client, ClientEvent, Events, ensure_private_dir, socket_path_in};

const WAIT: Duration = Duration::from_secs(5);

/// A private directory of its own under the target directory, removed on
/// drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("flush{}-{name}", std::process::id()));
        ensure_private_dir(&dir).expect("private temp dir");
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A listener, a client of it, and the server end of the client's
/// connection once the client has greeted.
fn connected(dir: &TempDir) -> (Client, Events, UnixStream) {
    let listener = UnixListener::bind(socket_path_in(&dir.0)).expect("bind");
    let (client, events) = Client::connect(socket_path_in(&dir.0));
    let (mut server, _) = listener.accept().expect("accept");
    server.set_read_timeout(Some(WAIT)).expect("timeout");
    let hello: Request = read_frame(&mut server).expect("hello");
    assert!(matches!(hello, Request::Hello { .. }));
    (client, events, server)
}

#[test]
fn a_flushed_frame_survives_dropping_the_client() {
    let dir = TempDir::new("survives");
    let (client, _events, mut server) = connected(&dir);
    let bytes = vec![0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7];

    assert!(client.raw(bytes.clone()));
    assert!(client.flush(WAIT));
    drop(client);

    let frame: Request = read_frame(&mut server).expect("the frame");
    assert_eq!(frame, Request::Raw { bytes });
}

#[test]
fn flush_is_false_without_a_connection() {
    let dir = TempDir::new("unconnected");
    let (client, _events) = Client::connect(socket_path_in(&dir.0));
    assert!(!client.flush(Duration::from_millis(500)));
}

#[test]
fn flush_is_false_once_the_client_has_stopped() {
    let dir = TempDir::new("stopped");
    let (client, events, mut server) = connected(&dir);
    write_frame(
        &mut server,
        &Message::Rejected {
            reason: Reject::Version { server: u32::MAX },
            param: None,
        },
    )
    .expect("refuse");
    loop {
        match events.wait_event(WAIT) {
            Some(ClientEvent::Incompatible { .. }) => break,
            Some(_) => {}
            None => panic!("the client never gave up"),
        }
    }
    assert!(!client.flush(Duration::from_millis(500)));
}
