//! Plain HTTP on the WebSocket port: the web app the service serves
//! ([#572](https://github.com/sigma/integral/issues/572)).
//!
//! A connection's request head is read and parsed with `httparse`, the
//! parser `tungstenite` itself uses. A request to upgrade to a WebSocket is
//! [`Replay`]ed into the WebSocket handshake; a `GET` or `HEAD` is answered
//! with a file of the embedded web build (`integral-web-assets`), and other
//! methods with `405`. One response per connection, then it is closed.

use std::io::{self, Cursor, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::time::Duration;

use tungstenite::http::header::{
    ALLOW, CACHE_CONTROL, CONNECTION, CONTENT_LENGTH, CONTENT_TYPE, HeaderName, UPGRADE,
    X_CONTENT_TYPE_OPTIONS,
};
use tungstenite::http::{Method, StatusCode, Uri};

/// Longest request head (request line and headers) a connection may send.
const MAX_HEAD: usize = 16 * 1024;

/// Most headers a request may have.
const MAX_HEADERS: usize = 64;

/// The `Upgrade` protocol of a WebSocket.
const WEBSOCKET: &str = "websocket";

/// Longest the server waits for a client to finish sending once answered.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(1);

/// What a connection's request asks for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Asked {
    /// A WebSocket: the endpoint.
    Upgrade,
    /// A file of the web build.
    File {
        /// The URI's path, no query.
        path: String,
        /// Whether to send the file's bytes (`GET`), or only its headers
        /// (`HEAD`).
        body: bool,
    },
    /// Another method.
    Other,
}

/// A stream that reads `head` (bytes already read from `inner`) before
/// reading `inner` itself, so a handshake sees the request whole.
pub(crate) struct Replay<S> {
    head: Cursor<Vec<u8>>,
    inner: S,
}

impl<S> Replay<S> {
    /// `head` then `inner`.
    pub(crate) fn new(head: Vec<u8>, inner: S) -> Self {
        Self {
            head: Cursor::new(head),
            inner,
        }
    }

    /// The stream underneath.
    pub(crate) fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Read> Read for Replay<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.head.read(buf)? {
            0 => self.inner.read(buf),
            n => Ok(n),
        }
    }
}

impl<S: Write> Write for Replay<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Read a request head from `stream` and say what it asks for, with the
/// bytes read (the whole head, and whatever followed it).
///
/// # Errors
///
/// `InvalidData` if the head is not HTTP or is longer than [`MAX_HEAD`],
/// `UnexpectedEof` if the stream ends first, or the read's error.
pub(crate) fn read_request(stream: &mut impl Read) -> io::Result<(Asked, Vec<u8>)> {
    let mut head = Vec::new();
    let mut chunk = [0; 2048];
    loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        head.extend_from_slice(&chunk[..n]);
        let mut headers = [httparse::EMPTY_HEADER; MAX_HEADERS];
        let mut request = httparse::Request::new(&mut headers);
        if request.parse(&head).map_err(invalid)?.is_complete() {
            let asked = asked(&request)?;
            return Ok((asked, head));
        }
        if head.len() > MAX_HEAD {
            return Err(invalid("request head too long"));
        }
    }
}

fn invalid(error: impl ToString) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

/// What a parsed request asks for.
fn asked(request: &httparse::Request<'_, '_>) -> io::Result<Asked> {
    let upgrade = request.headers.iter().any(|header| {
        HeaderName::from_bytes(header.name.as_bytes()).is_ok_and(|name| name == UPGRADE)
            && header.value.split(|&b| b == b',').any(|protocol| {
                protocol
                    .trim_ascii()
                    .eq_ignore_ascii_case(WEBSOCKET.as_bytes())
            })
    });
    if upgrade {
        return Ok(Asked::Upgrade);
    }
    let method = request
        .method
        .map(|m| Method::from_bytes(m.as_bytes()))
        .transpose();
    let body = match method.map_err(invalid)? {
        Some(Method::GET) => true,
        Some(Method::HEAD) => false,
        _ => return Ok(Asked::Other),
    };
    let uri: Uri = request.path.unwrap_or_default().parse().map_err(invalid)?;
    Ok(Asked::File {
        path: uri.path().to_string(),
        body,
    })
}

/// Answer one plain HTTP request on `stream`, then close it: the write side
/// first, and the rest of the request is read and dropped, so the client
/// gets the response rather than a reset.
pub(crate) fn answer(stream: TcpStream, asked: &Asked) -> io::Result<()> {
    let written = respond(&stream, asked);
    let _ = stream.shutdown(Shutdown::Write);
    let _ = crate::tcp::set_read_timeout(&stream, Some(DRAIN_TIMEOUT));
    let _ = io::copy(&mut (&stream), &mut io::sink());
    written
}

/// Write the response to `asked`.
fn respond(mut stream: impl Write, asked: &Asked) -> io::Result<()> {
    let (status, mime, file, send) = match asked {
        Asked::File { path, body } => {
            let served = integral_web_assets::serve(path);
            let status = StatusCode::from_u16(served.status).map_err(invalid)?;
            (status, served.mime, served.body, *body)
        }
        Asked::Upgrade | Asked::Other => (
            StatusCode::METHOD_NOT_ALLOWED,
            "text/plain",
            &b""[..],
            false,
        ),
    };
    let mut head = format!(
        "HTTP/1.1 {status}\r\n\
         {CONTENT_TYPE}: {mime}\r\n\
         {CONTENT_LENGTH}: {}\r\n\
         {X_CONTENT_TYPE_OPTIONS}: nosniff\r\n\
         {CACHE_CONTROL}: no-cache\r\n\
         {CONNECTION}: close\r\n",
        file.len()
    );
    if status == StatusCode::METHOD_NOT_ALLOWED {
        head.push_str(&format!("{ALLOW}: {}, {}\r\n", Method::GET, Method::HEAD));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if send {
        stream.write_all(file)?;
    }
    stream.flush()
}
