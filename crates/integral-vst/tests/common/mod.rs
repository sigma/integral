//! A Device service on a fake link, serving a socket: what the instance and
//! editor-link tests share. Included by `tests/connection.rs` and, with
//! `#[path]`, by the link's unit tests, so it reads no build-time
//! environment.

#![allow(dead_code)]

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

/// Whether `message` is a request to read (RQ1).
pub fn is_rq1(message: &[u8]) -> bool {
    message.first() == Some(&integral_core::sysex::SYSEX_START)
        && message.get(6) == Some(&integral_core::sysex::CMD_RQ1)
}

/// A device that answers reads: its memory by address, and a thread that
/// replies to every RQ1 the service sends on the fake link with the bytes it
/// holds (0 where nothing was put), and keeps what every DT1 writes.
pub struct Memory {
    bytes: std::sync::Arc<std::sync::Mutex<std::collections::BTreeMap<u32, u8>>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Memory {
    /// Start answering reads on `link`.
    pub fn answer(link: &integral_device_service::FakeLink) -> Self {
        use integral_core::address::{Address, DataSize};
        use integral_core::sysex;
        use std::sync::atomic::Ordering;

        let bytes = std::sync::Arc::new(std::sync::Mutex::new(std::collections::BTreeMap::new()));
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let thread = {
            let (link, bytes, stop) = (link.clone(), bytes.clone(), stop.clone());
            std::thread::spawn(move || {
                let mut done = 0;
                while !stop.load(Ordering::Acquire) {
                    let sent = link.sent();
                    // A test clears the recording now and then.
                    done = done.min(sent.len());
                    for message in &sent[done..] {
                        // A write is kept, as a device would keep it.
                        if message.get(6) == Some(&integral_core::sysex::CMD_DT1)
                            && let Ok(dt1) = integral_core::sysex::parse_dt1(message)
                        {
                            let mut memory = bytes.lock().expect("memory");
                            for (k, byte) in dt1.data.iter().enumerate() {
                                memory.insert(dt1.address.value() + k as u32, *byte);
                            }
                        }
                        // F0 41 id 00 00 64 11 a0..a3 s0..s3 cs F7
                        if is_rq1(message) {
                            let address =
                                Address([message[7], message[8], message[9], message[10]]);
                            let size =
                                DataSize([message[11], message[12], message[13], message[14]])
                                    .value();
                            let memory = bytes.lock().expect("memory");
                            let start = address.value();
                            let data: Vec<u8> = (0..size)
                                .map(|k| memory.get(&(start + k)).copied().unwrap_or(0))
                                .collect();
                            drop(memory);
                            for (n, chunk) in data.chunks(64).enumerate() {
                                let at = Address::from_value(start + (n * 64) as u32)
                                    .expect("an address");
                                link.device_says(&sysex::build_dt1(message[2], &at, chunk));
                            }
                        }
                    }
                    done = sent.len();
                    std::thread::sleep(Duration::from_millis(5));
                }
            })
        };
        Self {
            bytes,
            stop,
            thread: Some(thread),
        }
    }

    /// The byte at `address` (a linear address value); 0 where nothing was put.
    pub fn get(&self, address: u32) -> u8 {
        let memory = self.bytes.lock().expect("memory");
        memory.get(&address).copied().unwrap_or(0)
    }

    /// Put `data` at `address` (a linear address value).
    pub fn put(&self, address: u32, data: &[u8]) {
        let mut memory = self.bytes.lock().expect("memory");
        for (k, byte) in data.iter().enumerate() {
            memory.insert(address + k as u32, *byte);
        }
    }
}

impl Drop for Memory {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
