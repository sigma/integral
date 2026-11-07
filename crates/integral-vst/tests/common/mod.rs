//! A Device service on a fake link, serving a socket: what the instance and
//! editor-link tests share. Included by `tests/connection.rs` and, with
//! `#[path]`, by the link's unit tests, so it reads no build-time
//! environment.

#![allow(dead_code, unused_imports)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use integral_device_service::{
    Config, FakeLink, Running, SequentialSessionIds, Service, SocketServer, ensure_private_dir,
    socket_path_in, spawn, user_temp_dir,
};

/// How long a test waits for something to happen.
pub const WAIT: Duration = Duration::from_secs(5);

/// A private directory in the user's temp directory (a short path: sockets
/// are limited to about 100 bytes), removed on drop.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = user_temp_dir().expect("temp dir").join(format!(
            "iv{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        ensure_private_dir(&dir).expect("private temp dir");
        Self(dir)
    }

    pub fn socket(&self) -> PathBuf {
        socket_path_in(&self.0)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A service on a fake link, serving a socket.
pub struct TestService {
    pub link: FakeLink,
    running: Option<Running>,
    server: Option<SocketServer>,
}

impl TestService {
    pub fn start(path: &Path) -> Self {
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

/// Wait until `condition` holds, or fail naming `what`.
pub fn wait_for(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The address whose linear value is `n`.
pub fn addr(n: u32) -> integral_core::address::Address {
    integral_core::address::Address::from_value(n).expect("an address")
}

/// Whether `message` is a request to read (RQ1).
pub use integral_device_service::is_rq1;

/// A device that answers reads: see [`integral_device_service::FakeMemory`].
pub use integral_device_service::FakeMemory as Memory;
