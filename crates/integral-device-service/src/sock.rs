//! Unix stream sockets that cannot kill the process with `SIGPIPE`.
//!
//! A plugin lives in a host that did not ask for `SIGPIPE`: writing to a
//! socket whose service has died must be an error, not a signal. Where the
//! platform has `SO_NOSIGPIPE` (macOS) it is set on the socket; elsewhere
//! every write passes `MSG_NOSIGNAL`. Both ends use this, since a service
//! must survive a client that vanished too.

use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

/// `send(2)` flags that keep a write from raising `SIGPIPE`, on platforms
/// without the socket option.
#[cfg(target_vendor = "apple")]
const SEND_FLAGS: libc::c_int = 0;
#[cfg(not(target_vendor = "apple"))]
const SEND_FLAGS: libc::c_int = libc::MSG_NOSIGNAL;

/// Make writes to `stream` fail with `EPIPE` instead of raising `SIGPIPE`.
///
/// # Errors
///
/// The OS error if the socket option cannot be set.
pub(crate) fn no_sigpipe(stream: &UnixStream) -> io::Result<()> {
    #[cfg(target_vendor = "apple")]
    {
        let on: libc::c_int = 1;
        // SAFETY: the fd is open for the life of `stream`, and `on` is a
        // valid `c_int` of the size passed.
        let status = unsafe {
            libc::setsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_NOSIGPIPE,
                (&raw const on).cast(),
                size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if status != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    #[cfg(not(target_vendor = "apple"))]
    let _ = stream;
    Ok(())
}

/// A writer over a socket whose writes never raise `SIGPIPE`.
pub(crate) struct Quiet<'a>(pub &'a UnixStream);

impl Write for Quiet<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            // SAFETY: the fd is open for the life of the borrow, and `buf`
            // is a valid slice of the length passed.
            let sent = unsafe {
                libc::send(
                    self.0.as_raw_fd(),
                    buf.as_ptr().cast(),
                    buf.len(),
                    SEND_FLAGS,
                )
            };
            if let Ok(sent) = usize::try_from(sent) {
                return Ok(sent);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The user ID this process runs as.
pub(crate) fn my_uid() -> u32 {
    // SAFETY: `geteuid` has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

/// The user ID of the process at the other end of `stream`.
///
/// # Errors
///
/// The OS error, or `Unsupported` on a platform with neither `getpeereid`
/// nor `SO_PEERCRED`.
pub(crate) fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    #[cfg(target_vendor = "apple")]
    {
        let (mut uid, mut gid) = (0, 0);
        // SAFETY: the fd is open, and `uid` and `gid` are valid for writes.
        let status = unsafe { libc::getpeereid(stream.as_raw_fd(), &raw mut uid, &raw mut gid) };
        if status != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(uid)
    }
    #[cfg(target_os = "linux")]
    {
        // SAFETY: an all-zero `ucred` is a valid value.
        let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
        let mut size = size_of::<libc::ucred>() as libc::socklen_t;
        // SAFETY: the fd is open, and `cred` and `size` are valid for the call.
        let status = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&raw mut cred).cast(),
                &raw mut size,
            )
        };
        if status != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(cred.uid)
    }
    #[cfg(not(any(target_vendor = "apple", target_os = "linux")))]
    {
        let _ = stream;
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

/// Whether the process at the other end of `stream` is this user's. Anything
/// else, including a failure to tell, is no.
pub(crate) fn peer_is_me(stream: &UnixStream) -> bool {
    peer_uid(stream).is_ok_and(|uid| uid == my_uid())
}

/// Take an exclusive, non-blocking `flock` on `file`. Returns `false` if
/// another holder has it. The lock lasts as long as the open file.
///
/// # Errors
///
/// The OS error if `flock` fails for another reason.
pub(crate) fn try_lock(file: &std::fs::File) -> io::Result<bool> {
    // SAFETY: the fd is open for the life of `file`.
    let status = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if status == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(false)
    } else {
        Err(error)
    }
}

/// Bytes of a socket path the OS accepts (`sun_path`, less its NUL).
pub(crate) const PATH_LIMIT: usize =
    size_of::<libc::sockaddr_un>() - std::mem::offset_of!(libc::sockaddr_un, sun_path) - 1;

/// The directory macOS keeps per user (`confstr(_CS_DARWIN_USER_TEMP_DIR)`),
/// without the environment.
#[cfg(target_vendor = "apple")]
pub(crate) fn user_temp_dir() -> io::Result<std::path::PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let mut buffer = vec![0u8; libc::PATH_MAX as usize];
    // SAFETY: `buffer` is valid for writes of its length.
    let length = unsafe {
        libc::confstr(
            libc::_CS_DARWIN_USER_TEMP_DIR,
            buffer.as_mut_ptr().cast(),
            buffer.len(),
        )
    };
    if length == 0 || length > buffer.len() {
        return Err(io::Error::other("no per-user temporary directory"));
    }
    buffer.truncate(length - 1); // the NUL
    Ok(std::ffi::OsString::from_vec(buffer).into())
}

#[cfg(all(test, target_vendor = "apple"))]
mod tests {
    use super::*;

    #[test]
    fn the_option_is_set() {
        let (stream, _other) = UnixStream::pair().expect("pair");
        no_sigpipe(&stream).expect("set");
        let mut value: libc::c_int = 0;
        let mut size = size_of::<libc::c_int>() as libc::socklen_t;
        // SAFETY: the fd is open, and `value` and `size` are valid for the call.
        let status = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_NOSIGPIPE,
                (&raw mut value).cast(),
                &raw mut size,
            )
        };
        assert_eq!((status, value), (0, 1));
    }
}

#[cfg(all(test, unix))]
mod write_tests {
    use super::*;

    #[test]
    fn the_peer_of_a_pair_is_this_user() {
        let (stream, _other) = UnixStream::pair().expect("pair");
        assert!(peer_is_me(&stream));
    }

    #[test]
    fn a_dead_peer_is_an_error_not_a_signal() {
        let (stream, other) = UnixStream::pair().expect("pair");
        no_sigpipe(&stream).expect("set");
        drop(other);
        let error = Quiet(&stream)
            .write_all(b"hello")
            .expect_err("peer is gone");
        assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    }
}
