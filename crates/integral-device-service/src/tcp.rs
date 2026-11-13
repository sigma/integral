//! TCP socket helpers shared by the service's sockets: the WebSocket
//! endpoint and the Wireless link.
//!
//! macOS answers `setsockopt(SO_RCVTIMEO)` and `setsockopt(SO_SNDTIMEO)`
//! with `EINVAL` once a TCP socket is no longer connected: its peer sent a
//! reset, or it is shut down (issue #516). Such a socket cannot block, so the
//! timeout has nothing to bound, and its next read or write reports the
//! peer. A call that sets a timeout must therefore take that refusal as
//! success, or the refused setting, not the dead peer, becomes the failure
//! the caller sees.

use std::io;
use std::net::TcpStream;
use std::time::Duration;

/// [`TcpStream::set_read_timeout`] as this module sets it: a refusal on a
/// socket that can no longer communicate is success, and any other error is
/// returned.
pub(crate) fn set_read_timeout(stream: &TcpStream, timeout: Option<Duration>) -> io::Result<()> {
    tolerate_dead_socket(stream.set_read_timeout(timeout))
}

/// [`TcpStream::set_write_timeout`] as this module sets it: a refusal on a
/// socket that can no longer communicate is success, and any other error is
/// returned.
pub(crate) fn set_write_timeout(stream: &TcpStream, timeout: Option<Duration>) -> io::Result<()> {
    tolerate_dead_socket(stream.set_write_timeout(timeout))
}

/// Take the `EINVAL` of a dead socket as success. Every other refusal
/// stands: in particular std's own rejection of a zero duration carries no
/// OS error, so it still fails.
fn tolerate_dead_socket(result: io::Result<()>) -> io::Result<()> {
    match result {
        #[cfg(unix)]
        Err(e) if e.raw_os_error() == Some(libc::EINVAL) => Ok(()),
        result => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::{Shutdown, TcpListener};

    const WAIT: Duration = Duration::from_secs(1);

    /// A connected pair of streams over loopback.
    fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let peer = TcpStream::connect(listener.local_addr().expect("addr")).expect("connect");
        let (ours, _) = listener.accept().expect("accept");
        (ours, peer)
    }

    #[test]
    fn a_timeout_is_set_on_a_live_socket() {
        let (stream, _) = pair();
        set_read_timeout(&stream, Some(WAIT)).expect("read timeout");
        set_write_timeout(&stream, Some(WAIT)).expect("write timeout");
        set_read_timeout(&stream, None).expect("no read timeout");
    }

    #[test]
    fn a_bad_timeout_is_still_refused() {
        let (stream, _) = pair();
        // std rejects a zero duration itself, with no OS error to tolerate.
        assert!(set_read_timeout(&stream, Some(Duration::ZERO)).is_err());
        assert!(set_write_timeout(&stream, Some(Duration::ZERO)).is_err());
    }

    #[test]
    fn a_timeout_on_a_socket_that_cannot_block_is_tolerated() {
        let (mut stream, peer) = pair();
        // A both-ways shutdown is the same state a peer's reset leaves the
        // socket in, and needs no race to produce: macOS refuses the timeout
        // from there. Shut down before the peer closes: once its close lands,
        // macOS refuses the shutdown itself (`ENOTCONN`).
        stream.shutdown(Shutdown::Both).expect("shutdown");
        drop(peer);
        #[cfg(target_os = "macos")]
        {
            let raw = stream.set_read_timeout(Some(WAIT)).expect_err("EINVAL");
            assert_eq!(raw.raw_os_error(), Some(libc::EINVAL));
        }
        set_read_timeout(&stream, Some(WAIT)).expect("read timeout");
        set_write_timeout(&stream, Some(WAIT)).expect("write timeout");
        // The stream reads as closed: nothing blocks, so nothing was lost.
        let mut byte = [0u8; 1];
        assert_eq!(stream.read(&mut byte).expect("closed"), 0);
    }
}
