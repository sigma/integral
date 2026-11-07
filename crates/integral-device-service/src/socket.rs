//! The service's end of the client socket: a Unix domain socket server.
//!
//! One accept thread, and per client a reader thread (requests in, as
//! [`Input`]s to the service) and a writer thread (messages out of a bounded
//! queue). The service thread only ever puts a frame on that queue, so a slow
//! or stuck client cannot stall pacing; a client whose queue fills is dropped
//! and reconnects. See [`protocol`](crate::protocol) for what is on the wire.

use std::fs;
use std::io::{self, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::clients::Outbox;
use crate::protocol::{ClientId, FrameError, Message, Request, encode_frame, read_frame};
use crate::runtime::Handle;
use crate::service::Input;
use crate::sock::{PATH_LIMIT, Quiet, my_uid, no_sigpipe, peer_is_me, try_lock};

/// File name of the socket within its directory.
pub const SOCKET_FILE_NAME: &str = "integral-device-service.sock";

/// Name of the directory under the per-user one that holds the socket.
const DIR_NAME: &str = "Integral";

/// Frames a client's outgoing queue holds before the service gives up on the
/// client.
const OUTBOX_FRAMES: usize = 4096;

/// Longest wait after a failed `accept`.
const LONGEST_ACCEPT_RETRY: Duration = Duration::from_secs(1);

/// Make `dir` (whose parent must exist) a directory only this user can
/// enter, or check that it already is.
///
/// A socket is only as private as the directory it is in: the file's own
/// mode is set after it exists, and a directory that others can search lets
/// them find it first.
///
/// # Errors
///
/// `PermissionDenied` if `dir` exists but is another user's, is not a plain
/// directory, or is open to group or others; else the OS error.
pub fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    match fs::DirBuilder::new().mode(0o700).create(dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => check_private(dir),
        Err(e) => Err(e),
    }
}

fn check_private(dir: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(dir)?;
    if meta.is_dir() && meta.uid() == my_uid() && meta.mode() & 0o077 == 0 {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} is not a directory only this user can enter",
                dir.display()
            ),
        ))
    }
}

/// The per-user temporary directory, without the environment: on macOS the
/// one `confstr` names (not `$TMPDIR`); elsewhere `/run/user/<uid>` if there
/// is one, else `/tmp`.
///
/// # Errors
///
/// The OS error, on macOS.
pub fn user_temp_dir() -> io::Result<PathBuf> {
    #[cfg(target_vendor = "apple")]
    {
        crate::sock::user_temp_dir()
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        let run = PathBuf::from(format!("/run/user/{}", my_uid()));
        Ok(if run.is_dir() {
            run
        } else {
            PathBuf::from("/tmp")
        })
    }
}

/// The directory for the socket under `base`: `Integral`, created private
/// (see [`ensure_private_dir`]).
///
/// # Errors
///
/// As [`ensure_private_dir`].
pub fn socket_dir_in(base: &Path) -> io::Result<PathBuf> {
    let dir = base.join(DIR_NAME);
    ensure_private_dir(&dir)?;
    Ok(dir)
}

/// The directory the service socket goes in by default: [`socket_dir_in`]
/// the [`user_temp_dir`]. It reads no environment variable and no working
/// directory, so the service and every plugin agree on it.
///
/// # Errors
///
/// The OS error, or `PermissionDenied` if the directory exists and is not
/// private.
pub fn default_socket_dir() -> io::Result<PathBuf> {
    socket_dir_in(&user_temp_dir()?)
}

/// The socket path in `dir`: [`SOCKET_FILE_NAME`] beneath it.
pub fn socket_path_in(dir: &Path) -> PathBuf {
    dir.join(SOCKET_FILE_NAME)
}

/// Whether the default socket path, under [`user_temp_dir`], fits a Unix
/// socket. Creates nothing.
///
/// # Errors
///
/// `InvalidInput` if it is too long, else as [`user_temp_dir`].
pub fn check_default_path_length() -> io::Result<()> {
    check_length(&socket_path_in(&user_temp_dir()?.join(DIR_NAME)))
}

/// The default socket path: [`socket_path_in`] [`default_socket_dir`].
///
/// # Errors
///
/// As [`default_socket_dir`], or `InvalidInput` if the path is too long for
/// a Unix socket (it is under the limit on a stock macOS or Linux).
pub fn default_socket_path() -> io::Result<PathBuf> {
    let path = socket_path_in(&default_socket_dir()?);
    check_length(&path)?;
    Ok(path)
}

fn check_length(path: &Path) -> io::Result<()> {
    if path.as_os_str().len() > PATH_LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} is longer than a socket path can be ({PATH_LIMIT})",
                path.display()
            ),
        ));
    }
    Ok(())
}

/// A server accepting clients of a running service on a Unix socket.
///
/// Dropping it stops accepting and removes the socket file. Clients already
/// connected stay until the service stops.
pub struct SocketServer {
    path: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Held for as long as the server lives: whoever holds it owns the path.
    _lock: fs::File,
}

impl SocketServer {
    /// Listen on `path` and serve the service behind `handle`.
    ///
    /// The directory holding `path` must be private (see
    /// [`ensure_private_dir`]), so the socket is never visible to others.
    /// Two servers cannot take one path: the second finds the first's lock
    /// (`<path>.lock`, released when its process dies) and fails. Holding
    /// the lock, the server replaces a socket file left by a dead service.
    /// Only connections from this user are served.
    ///
    /// # Errors
    ///
    /// `AddrInUse` if a service already holds the path, `AlreadyExists` if
    /// something that is not a socket is there, `PermissionDenied` if the
    /// directory is not private, `InvalidInput` if the path is too long,
    /// else the OS error.
    pub fn bind(path: impl Into<PathBuf>, handle: Handle) -> io::Result<Self> {
        let path = path.into();
        check_length(&path)?;
        let dir = path
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no directory"))?;
        check_private(dir)?;
        let mut lock_path = path.clone().into_os_string();
        lock_path.push(".lock");
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .open(lock_path)?;
        if !try_lock(&lock)? {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("a Device service already serves {}", path.display()),
            ));
        }
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_socket() => fs::remove_file(&path)?,
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!("{} exists and is not a socket", path.display()),
                ));
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let listener = UnixListener::bind(&path)?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            thread::Builder::new()
                .name("device-service-accept".into())
                .spawn(move || accept_loop(&listener, &handle, &stop))?
        };
        Ok(Self {
            path,
            stop,
            thread: Some(thread),
            _lock: lock,
        })
    }

    /// The path the server listens on.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SocketServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocked accept.
        let _ = UnixStream::connect(&self.path);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = fs::remove_file(&self.path);
    }
}

fn accept_loop(listener: &UnixListener, handle: &Handle, stop: &AtomicBool) {
    let mut next = 1_u64; // 0 is the device
    let mut retry = Duration::from_millis(10);
    loop {
        let accepted = listener.accept();
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let stream = match accepted {
            Ok((stream, _)) => stream,
            Err(_) => {
                // Out of descriptors, say: wait rather than spin on it.
                thread::sleep(retry);
                retry = (retry * 2).min(LONGEST_ACCEPT_RETRY);
                continue;
            }
        };
        retry = Duration::from_millis(10);
        if !peer_is_me(&stream) {
            continue;
        }
        let client = ClientId(next);
        next += 1;
        let handle = handle.clone();
        let _ = thread::Builder::new()
            .name(format!("device-service-client-{client}"))
            .spawn(move || {
                let _ = serve(&stream, client, &handle);
            });
    }
}

/// Serve one client until it or the service goes away.
fn serve(stream: &UnixStream, client: ClientId, handle: &Handle) -> io::Result<()> {
    no_sigpipe(stream)?;
    let (frames, queue) = mpsc::sync_channel(OUTBOX_FRAMES);
    let out = stream.try_clone()?;
    thread::Builder::new()
        .name(format!("device-service-client-{client}-out"))
        .spawn(move || write_loop(&out, &queue))?;
    let connect = Input::Connect {
        client,
        outbox: Box::new(SocketOutbox(frames)),
    };
    if handle.client_input(connect) {
        let mut reader = BufReader::new(stream);
        loop {
            match read_frame::<Request>(&mut reader) {
                Ok(request) => {
                    if !handle.client_input(Input::Client { client, request }) {
                        break;
                    }
                }
                // One bad frame is one lost request: the length prefix kept
                // the stream aligned.
                Err(FrameError::Malformed(e)) => {
                    let message = e.to_string();
                    if !handle.client_input(Input::Malformed { client, message }) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        handle.client_input(Input::Disconnect { client });
    }
    stream.shutdown(Shutdown::Both)
}

/// Write queued frames to the client until the queue closes (the service
/// dropped the client) or the socket fails, then close the socket.
fn write_loop(stream: &UnixStream, queue: &Receiver<Vec<u8>>) {
    for frame in queue {
        if Quiet(stream).write_all(&frame).is_err() {
            break;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
}

/// The service's [`Outbox`] for a socket client: encodes the message and
/// queues the frame without blocking.
struct SocketOutbox(SyncSender<Vec<u8>>);

impl Outbox for SocketOutbox {
    fn deliver(&mut self, message: &Message) -> bool {
        match encode_frame(message) {
            Ok(frame) => self.0.try_send(frame).is_ok(),
            // A message too long to frame is lost; the client is fine.
            Err(_) => true,
        }
    }
}
