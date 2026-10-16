//! Sans-IO protocol for the INTEGRA-7 Wireless link (WNA1100-RL adapter).
//!
//! Spec: `docs/specs/wireless-transport.md`. Wire facts:
//! `docs/research/integra-wireless-capture.md`.
//!
//! This module owns no sockets, threads or clocks: callers feed it bytes and
//! a monotonic time in milliseconds, and send the bytes it returns. The
//! Bridge (`integral-cli bridge`) does the I/O.

use thiserror::Error;

use crate::device_spec::INTEGRA7;

/// UDP port the device listens on for RDDP discovery probes.
pub const DISCOVERY_PORT: u16 = 9314;

/// TCP port of the editor session.
pub const SESSION_PORT: u16 = 23847;

/// Interval between our heartbeats, in ms.
pub const HEARTBEAT_INTERVAL_MS: u64 = 1000;

/// Silence after which an up session is dead, in ms (the device times out
/// at about 3 s too).
pub const LIVENESS_TIMEOUT_MS: u64 = 3000;

/// Wait for the greeting after which the device is reported in use by
/// another editor, in ms.
pub const IN_USE_AFTER_MS: u64 = 5000;

const RDDP_MAGIC: &[u8; 6] = b"RDDPv1";

const FRAME_HEARTBEAT: u8 = 0x00;
const FRAME_GREETING: u8 = 0x01;
const FRAME_MIDI: u8 = 0x02;
const FRAME_HEADER_LEN: usize = 4;

/// Name of the virtual MIDI port the Bridge exposes.
pub fn port_name() -> String {
    format!("{} (Wireless)", INTEGRA7.name)
}

// ---------------------------------------------------------------------------
// RDDP discovery
// ---------------------------------------------------------------------------

/// Build the 8-byte RDDP probe asking devices to call back on TCP `listen_port`.
pub fn rddp_probe(listen_port: u16) -> [u8; 8] {
    let [hi, lo] = listen_port.to_be_bytes();
    let mut probe = [0; 8];
    probe[..6].copy_from_slice(RDDP_MAGIC);
    probe[6] = hi;
    probe[7] = lo;
    probe
}

/// Identity strings from an RDDP reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    /// Manufacturer, e.g. `Roland`.
    pub manufacturer: String,
    /// WirelessID shown on the device, e.g. `41288`.
    pub wireless_id: String,
    /// System version, e.g. `1.21`.
    pub version: String,
    /// Model name, e.g. `INTEGRA-7`.
    pub model: String,
}

/// Why an RDDP reply was rejected.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RddpError {
    /// Too short or missing the `RDDPv1` magic.
    #[error("not an RDDP reply")]
    NotRddp,
    /// A string field is not NUL-terminated printable ASCII.
    #[error("malformed RDDP {0} field")]
    BadField(&'static str),
    /// The replying device is not an INTEGRA-7.
    #[error("wrong model: {0}")]
    WrongModel(String),
}

/// String fields of the 84-byte reply: (name, offset, width). Each holds a
/// NUL-terminated string followed by bytes we leave opaque (in the capture
/// the byte after "41288\0" is `q`, so scanning for strings would misread).
const RDDP_FIELDS: [(&str, usize, usize); 4] = [
    ("manufacturer", 20, 16),
    ("wireless_id", 36, 8),
    ("version", 44, 8),
    ("model", 52, 32),
];

/// Parse an RDDP reply, accepting only an INTEGRA-7.
pub fn parse_rddp_reply(reply: &[u8]) -> Result<DeviceInfo, RddpError> {
    let end = RDDP_FIELDS
        .iter()
        .map(|&(_, at, w)| at + w)
        .max()
        .unwrap_or(0);
    if reply.len() < end || !reply.starts_with(RDDP_MAGIC) {
        return Err(RddpError::NotRddp);
    }
    let field = |i: usize| {
        let (name, at, width) = RDDP_FIELDS[i];
        let raw = &reply[at..at + width];
        let len = raw
            .iter()
            .position(|&b| b == 0)
            .ok_or(RddpError::BadField(name))?;
        let s = &raw[..len];
        if s.is_empty() || !s.iter().all(|b| b.is_ascii_graphic() || *b == b' ') {
            return Err(RddpError::BadField(name));
        }
        Ok(String::from_utf8_lossy(s).into_owned())
    };
    let info = DeviceInfo {
        manufacturer: field(0)?,
        wireless_id: field(1)?,
        version: field(2)?,
        model: field(3)?,
    };
    if info.model != INTEGRA7.name {
        return Err(RddpError::WrongModel(info.model));
    }
    Ok(info)
}

// ---------------------------------------------------------------------------
// Frames
// ---------------------------------------------------------------------------

/// A session frame: `[type u8][length u24 BE][body]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// Type `00`, no body.
    Heartbeat,
    /// Type `01`, the device's first frame: its uptime in ms.
    Greeting {
        /// Device uptime in ms.
        uptime_ms: u32,
    },
    /// Type `02`: a timestamp and USB-MIDI 1.0 event packets.
    Midi {
        /// Sender timestamp (ms uptime from the device, `0` from us).
        timestamp: u32,
        /// 4-byte USB-MIDI event packets.
        packets: Vec<[u8; 4]>,
    },
    /// An unknown type, or a known type with a body of the wrong length.
    Other {
        /// Frame type byte.
        kind: u8,
        /// Raw body.
        body: Vec<u8>,
    },
}

impl Frame {
    /// Encode the frame for the wire.
    pub fn encode(&self) -> Vec<u8> {
        let (kind, body) = match self {
            Frame::Heartbeat => (FRAME_HEARTBEAT, Vec::new()),
            Frame::Greeting { uptime_ms } => (FRAME_GREETING, uptime_ms.to_be_bytes().to_vec()),
            Frame::Midi { timestamp, packets } => {
                let mut body = timestamp.to_be_bytes().to_vec();
                body.extend(packets.iter().flatten());
                (FRAME_MIDI, body)
            }
            Frame::Other { kind, body } => (*kind, body.clone()),
        };
        assert!(body.len() < 1 << 24, "frame body fits u24");
        let mut out = vec![kind];
        out.extend_from_slice(&(body.len() as u32).to_be_bytes()[1..]);
        out.extend(body);
        out
    }

    fn decode(kind: u8, body: &[u8]) -> Frame {
        match (kind, body.len()) {
            (FRAME_HEARTBEAT, 0) => Frame::Heartbeat,
            (FRAME_GREETING, 4) => Frame::Greeting {
                uptime_ms: u32::from_be_bytes(body.try_into().expect("4 bytes")),
            },
            (FRAME_MIDI, n) if n >= 4 && n % 4 == 0 => Frame::Midi {
                timestamp: u32::from_be_bytes(body[..4].try_into().expect("4 bytes")),
                packets: body[4..]
                    .chunks_exact(4)
                    .map(|p| p.try_into().expect("4 bytes"))
                    .collect(),
            },
            _ => Frame::Other {
                kind,
                body: body.to_vec(),
            },
        }
    }
}

/// Splits a TCP byte stream into frames. Every frame, known or not, is
/// consumed by its length, so the stream never desyncs.
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buf: Vec<u8>,
}

impl FrameDecoder {
    /// Append received bytes.
    pub fn push(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Take the next complete frame, if any.
    pub fn next_frame(&mut self) -> Option<Frame> {
        let header = self.buf.get(..FRAME_HEADER_LEN)?;
        let len = u32::from_be_bytes([0, header[1], header[2], header[3]]) as usize;
        let end = FRAME_HEADER_LEN + len;
        if self.buf.len() < end {
            return None;
        }
        let frame = Frame::decode(header[0], &self.buf[FRAME_HEADER_LEN..end]);
        self.buf.drain(..end);
        Some(frame)
    }
}

// ---------------------------------------------------------------------------
// USB-MIDI packets
// ---------------------------------------------------------------------------

/// Expected length of a non-SysEx message from its status byte, or `None`
/// if the byte cannot start one.
fn message_len(status: u8) -> Option<usize> {
    match status {
        0x80..=0xBF | 0xE0..=0xEF | 0xF2 => Some(3),
        0xC0..=0xDF | 0xF1 | 0xF3 => Some(2),
        0xF6 | 0xF8..=0xFF => Some(1),
        _ => None,
    }
}

/// Whether `msg` is one complete, well-formed MIDI message.
fn well_formed(msg: &[u8]) -> bool {
    match msg {
        [0xF0, data @ .., 0xF7] => data.iter().all(|b| *b < 0x80),
        [status, data @ ..] => {
            message_len(*status) == Some(msg.len()) && data.iter().all(|b| *b < 0x80)
        }
        [] => false,
    }
}

/// Pack one MIDI message into USB-MIDI 1.0 event packets on cable 0.
/// Returns `None` for a message that isn't well formed.
pub fn pack_usb_midi(msg: &[u8]) -> Option<Vec<[u8; 4]>> {
    if !well_formed(msg) {
        return None;
    }
    let packet = |cin: u8, bytes: &[u8]| {
        let mut p = [cin, 0, 0, 0];
        p[1..=bytes.len()].copy_from_slice(bytes);
        p
    };
    if msg[0] != 0xF0 {
        let cin = match msg[0] {
            0x80..=0xEF => msg[0] >> 4,
            0xF8..=0xFF => 0x0F,
            _ => [0, 0x05, 0x02, 0x03][msg.len()],
        };
        return Some(vec![packet(cin, msg)]);
    }
    let chunks: Vec<&[u8]> = msg.chunks(3).collect();
    let last = chunks.len() - 1;
    Some(
        chunks
            .iter()
            .enumerate()
            .map(|(i, c)| packet(if i < last { 0x04 } else { 0x04 + c.len() as u8 }, c))
            .collect(),
    )
}

/// Rebuilds whole MIDI messages from USB-MIDI packets, keeping SysEx state
/// across frames (the device sends one packet per frame).
#[derive(Debug, Default)]
pub struct Reassembler {
    sysex: Option<Vec<u8>>,
}

impl Reassembler {
    /// Feed one packet; returns a message once one is complete and well
    /// formed. CIN 0/1 packets and malformed messages are dropped.
    pub fn push(&mut self, packet: [u8; 4]) -> Option<Vec<u8>> {
        let data = &packet[1..];
        let bytes = match packet[0] & 0x0F {
            0x00 | 0x01 => return None,
            0x04 => {
                if data[0] == 0xF0 {
                    self.sysex = Some(Vec::new());
                }
                self.sysex.as_mut()?.extend_from_slice(data);
                return None;
            }
            cin @ 0x05..=0x07 => {
                let tail = &data[..usize::from(cin - 0x04)];
                if tail[0] == 0xF0 {
                    self.sysex = Some(Vec::new());
                }
                match self.sysex.take() {
                    Some(mut buf) => {
                        buf.extend_from_slice(tail);
                        buf
                    }
                    None => tail.to_vec(),
                }
            }
            0x02 | 0x0C | 0x0D => data[..2].to_vec(),
            0x0F => data[..1].to_vec(),
            _ => data.to_vec(),
        };
        well_formed(&bytes).then_some(bytes)
    }
}

/// Encode one MIDI message as an outbound `02` frame (timestamp 0, cable 0).
/// Returns `None` for a message that isn't well formed.
pub fn encode_midi(msg: &[u8]) -> Option<Vec<u8>> {
    let packets = pack_usb_midi(msg)?;
    Some(
        Frame::Midi {
            timestamp: 0,
            packets,
        }
        .encode(),
    )
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// What the session asks of its caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Write these bytes to the TCP stream.
    Transmit(Vec<u8>),
    /// The greeting arrived: the session is up. Create the port.
    Up {
        /// Device uptime in ms.
        uptime_ms: u32,
    },
    /// No greeting after [`IN_USE_AFTER_MS`]: another editor holds the device.
    /// Reported once; keep waiting.
    InUse,
    /// A MIDI message from the device, for the port.
    Midi(Vec<u8>),
    /// A frame of unknown type or bad length was skipped.
    Skipped {
        /// Frame type byte.
        kind: u8,
        /// Body length.
        len: usize,
    },
    /// No inbound frame for [`LIVENESS_TIMEOUT_MS`]: close and reconnect.
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    AwaitGreeting,
    Up,
    Dead,
}

/// One TCP connection to the device, from connect to death.
#[derive(Debug)]
pub struct Session {
    state: State,
    connected_at: u64,
    last_rx: u64,
    next_heartbeat: u64,
    in_use_reported: bool,
    frames: FrameDecoder,
    reassembler: Reassembler,
}

impl Session {
    /// Start a session on a freshly connected stream at time `now` (ms).
    /// The first heartbeat is due at once.
    pub fn new(now: u64) -> Self {
        Self {
            state: State::AwaitGreeting,
            connected_at: now,
            last_rx: now,
            next_heartbeat: now,
            in_use_reported: false,
            frames: FrameDecoder::default(),
            reassembler: Reassembler::default(),
        }
    }

    /// Whether the greeting has arrived and the session is not dead.
    pub fn is_up(&self) -> bool {
        self.state == State::Up
    }

    /// Feed bytes read from the stream.
    pub fn receive(&mut self, bytes: &[u8], now: u64) -> Vec<Event> {
        let mut events = Vec::new();
        if self.state == State::Dead {
            return events;
        }
        self.frames.push(bytes);
        while let Some(frame) = self.frames.next_frame() {
            self.last_rx = now;
            match frame {
                Frame::Heartbeat => {}
                Frame::Greeting { uptime_ms } if self.state == State::AwaitGreeting => {
                    self.state = State::Up;
                    events.push(Event::Up { uptime_ms });
                }
                Frame::Greeting { .. } => {}
                Frame::Midi { packets, .. } => events.extend(
                    packets
                        .into_iter()
                        .filter_map(|p| self.reassembler.push(p))
                        .map(Event::Midi),
                ),
                Frame::Other { kind, body } => events.push(Event::Skipped {
                    kind,
                    len: body.len(),
                }),
            }
        }
        events
    }

    /// Run timers: heartbeat, in-use report, liveness.
    pub fn tick(&mut self, now: u64) -> Vec<Event> {
        let mut events = Vec::new();
        match self.state {
            State::Dead => return events,
            State::Up if now >= self.last_rx + LIVENESS_TIMEOUT_MS => {
                self.state = State::Dead;
                events.push(Event::Dead);
                return events;
            }
            State::AwaitGreeting
                if !self.in_use_reported && now >= self.connected_at + IN_USE_AFTER_MS =>
            {
                self.in_use_reported = true;
                events.push(Event::InUse);
            }
            _ => {}
        }
        if now >= self.next_heartbeat {
            events.push(Event::Transmit(Frame::Heartbeat.encode()));
            self.next_heartbeat = now + HEARTBEAT_INTERVAL_MS;
        }
        events
    }

    /// Time (ms) of the next timer, so the caller knows how long to block.
    pub fn next_deadline(&self) -> u64 {
        let timer = match self.state {
            State::Up => self.last_rx + LIVENESS_TIMEOUT_MS,
            State::AwaitGreeting if !self.in_use_reported => self.connected_at + IN_USE_AFTER_MS,
            _ => u64::MAX,
        };
        timer.min(self.next_heartbeat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 84-byte RDDP reply from the capture.
    const RDDP_REPLY: [u8; 84] = [
        0x52, 0x44, 0x44, 0x50, 0x76, 0x31, 0x5d, 0x27, 0x6f, 0x8c, 0xc3, 0x13, 0x83, 0x04, 0x00,
        0x00, 0x0f, 0xd8, 0x05, 0xfc, 0x52, 0x6f, 0x6c, 0x61, 0x6e, 0x64, 0x00, 0x08, 0x0f, 0xd8,
        0x06, 0x04, 0x0f, 0xa0, 0x82, 0x84, 0x34, 0x31, 0x32, 0x38, 0x38, 0x00, 0x71, 0xd8, 0x31,
        0x2e, 0x32, 0x31, 0x00, 0xd8, 0x05, 0xc8, 0x49, 0x4e, 0x54, 0x45, 0x47, 0x52, 0x41, 0x2d,
        0x37, 0x00, 0x01, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0f, 0xa0, 0x82,
        0x84, 0x0f, 0xd8, 0x06, 0x0a, 0x0f, 0xd8, 0x06, 0x04,
    ];

    const GREETING: [u8; 8] = [0x01, 0x00, 0x00, 0x04, 0x00, 0x13, 0xc8, 0x9e];

    /// RQ1 for Part 1 tone, as the iPad editor framed it.
    const RQ1_FRAME: [u8; 32] = [
        0x02, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x00, 0x04, 0xf0, 0x41, 0x10, 0x04, 0x00, 0x00,
        0x64, 0x04, 0x11, 0x18, 0x00, 0x04, 0x20, 0x06, 0x00, 0x04, 0x00, 0x00, 0x03, 0x06, 0x3f,
        0xf7, 0x00,
    ];
    const RQ1: [u8; 17] = [
        0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11, 0x18, 0x00, 0x20, 0x06, 0x00, 0x00, 0x00, 0x03,
        0x3F, 0xF7,
    ];

    /// The device's DT1 reply packets, each sent in its own frame.
    const DT1_PACKETS: [[u8; 4]; 7] = [
        [0x00, 0x00, 0x00, 0x07],
        [0x04, 0xf0, 0x41, 0x10],
        [0x04, 0x00, 0x00, 0x64],
        [0x04, 0x12, 0x18, 0x00],
        [0x04, 0x20, 0x06, 0x5f],
        [0x04, 0x40, 0x26, 0x7d],
        [0x05, 0xf7, 0x00, 0x00],
    ];
    const DT1: [u8; 16] = [
        0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x12, 0x18, 0x00, 0x20, 0x06, 0x5F, 0x40, 0x26, 0x7D,
        0xF7,
    ];

    fn device_frame(uptime: u32, packet: [u8; 4]) -> Vec<u8> {
        Frame::Midi {
            timestamp: uptime,
            packets: vec![packet],
        }
        .encode()
    }

    #[test]
    fn probe_carries_listen_port() {
        assert_eq!(
            rddp_probe(0xd0e7),
            [0x52, 0x44, 0x44, 0x50, 0x76, 0x31, 0xd0, 0xe7]
        );
    }

    #[test]
    fn parses_captured_rddp_reply() {
        assert_eq!(
            parse_rddp_reply(&RDDP_REPLY),
            Ok(DeviceInfo {
                manufacturer: "Roland".into(),
                wireless_id: "41288".into(),
                version: "1.21".into(),
                model: "INTEGRA-7".into(),
            })
        );
    }

    #[test]
    fn rejects_bad_rddp_replies() {
        assert_eq!(parse_rddp_reply(&RDDP_REPLY[..60]), Err(RddpError::NotRddp));
        let mut bad = RDDP_REPLY;
        bad[0] = b'X';
        assert_eq!(parse_rddp_reply(&bad), Err(RddpError::NotRddp));
        let mut other = RDDP_REPLY;
        other[52..61].copy_from_slice(b"JUNO-DS\0\0");
        assert_eq!(
            parse_rddp_reply(&other),
            Err(RddpError::WrongModel("JUNO-DS".into()))
        );
        let mut unterminated = RDDP_REPLY;
        unterminated[36..44].copy_from_slice(b"41288888");
        assert_eq!(
            parse_rddp_reply(&unterminated),
            Err(RddpError::BadField("wireless_id"))
        );
    }

    #[test]
    fn frames_round_trip() {
        assert_eq!(Frame::Heartbeat.encode(), [0, 0, 0, 0]);
        let mut d = FrameDecoder::default();
        d.push(&GREETING);
        assert_eq!(
            d.next_frame(),
            Some(Frame::Greeting {
                uptime_ms: 1_296_542
            })
        );
        assert_eq!(d.next_frame(), None);
    }

    #[test]
    fn decoder_handles_split_and_unknown_frames() {
        let mut stream = vec![0x07, 0x00, 0x00, 0x02, 0xAA, 0xBB];
        stream.extend(GREETING);
        let mut d = FrameDecoder::default();
        d.push(&stream[..3]);
        assert_eq!(d.next_frame(), None);
        d.push(&stream[3..9]);
        assert_eq!(
            d.next_frame(),
            Some(Frame::Other {
                kind: 7,
                body: vec![0xAA, 0xBB]
            })
        );
        assert_eq!(d.next_frame(), None);
        d.push(&stream[9..]);
        assert!(matches!(d.next_frame(), Some(Frame::Greeting { .. })));
    }

    #[test]
    fn encodes_rq1_like_the_editor() {
        assert_eq!(encode_midi(&RQ1).unwrap(), RQ1_FRAME);
    }

    #[test]
    fn packs_sysex_ends_and_short_messages() {
        let three = [0xF0, 0x7E, 0xF7];
        assert_eq!(pack_usb_midi(&three).unwrap(), [[0x07, 0xF0, 0x7E, 0xF7]]);
        let six = [0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7];
        assert_eq!(
            pack_usb_midi(&six).unwrap(),
            [[0x04, 0xF0, 0x7E, 0x7F], [0x07, 0x06, 0x01, 0xF7]]
        );
        assert_eq!(
            pack_usb_midi(&[0x90, 0x3C, 0x64]).unwrap(),
            [[0x09, 0x90, 0x3C, 0x64]]
        );
        assert_eq!(
            pack_usb_midi(&[0xC1, 0x05]).unwrap(),
            [[0x0C, 0xC1, 0x05, 0]]
        );
        assert_eq!(pack_usb_midi(&[0xF8]).unwrap(), [[0x0F, 0xF8, 0, 0]]);
        assert_eq!(pack_usb_midi(&[0xF6]).unwrap(), [[0x05, 0xF6, 0, 0]]);
        assert_eq!(
            pack_usb_midi(&[0xF2, 0x01, 0x02]).unwrap(),
            [[0x03, 0xF2, 0x01, 0x02]]
        );
        assert_eq!(pack_usb_midi(&[0x90, 0x3C]), None);
        assert_eq!(pack_usb_midi(&[0xF0, 0x80, 0xF7]), None);
    }

    #[test]
    fn reassembles_and_repacks_every_message_kind() {
        let msgs: [&[u8]; 5] = [
            &DT1,
            &[0xF0, 0xF7],
            &[0x90, 0x3C, 0x64],
            &[0xD0, 0x10],
            &[0xFE],
        ];
        let mut r = Reassembler::default();
        for msg in msgs {
            let out: Vec<_> = pack_usb_midi(msg)
                .unwrap()
                .into_iter()
                .filter_map(|p| r.push(p))
                .collect();
            assert_eq!(out, [msg.to_vec()]);
        }
    }

    #[test]
    fn reassembler_drops_strays() {
        let mut r = Reassembler::default();
        assert_eq!(r.push([0x08, 0x08, 0x08, 0x08]), None);
        assert_eq!(r.push([0x0F, 0xD6, 0x9C, 0x04]), None);
        assert_eq!(r.push([0x05, 0xF7, 0, 0]), None);
        assert_eq!(r.push([0x04, 0x41, 0x10, 0x00]), None);
    }

    fn connect_and_greet() -> Session {
        let mut s = Session::new(0);
        assert_eq!(s.tick(0), [Event::Transmit(vec![0, 0, 0, 0])]);
        assert_eq!(
            s.receive(&GREETING, 10),
            [Event::Up {
                uptime_ms: 1_296_542
            }]
        );
        s
    }

    #[test]
    fn session_forwards_dt1_reply_split_across_frames() {
        let mut s = connect_and_greet();
        let mut events = Vec::new();
        for (i, p) in DT1_PACKETS.into_iter().enumerate() {
            events.extend(s.receive(&device_frame(1_300_000 + i as u32, p), 50));
        }
        assert_eq!(events, [Event::Midi(DT1.to_vec())]);
    }

    #[test]
    fn session_heartbeats_every_second() {
        let mut s = connect_and_greet();
        assert_eq!(s.next_deadline(), 1000);
        assert_eq!(s.tick(999), []);
        assert_eq!(s.tick(1000), [Event::Transmit(vec![0, 0, 0, 0])]);
        assert_eq!(s.tick(1500), []);
    }

    #[test]
    fn session_dies_after_three_silent_seconds() {
        let mut s = connect_and_greet();
        s.receive(&[0, 0, 0, 0], 1000);
        s.tick(2000);
        s.tick(3000);
        assert_eq!(s.next_deadline(), 4000);
        assert!(!s.tick(3999).contains(&Event::Dead));
        assert_eq!(s.tick(4000), [Event::Dead]);
        assert!(!s.is_up());
        assert_eq!(s.tick(9000), []);
    }

    #[test]
    fn session_reports_in_use_once_and_keeps_waiting() {
        let mut s = Session::new(0);
        for t in (0..5000).step_by(1000) {
            assert_eq!(s.tick(t), [Event::Transmit(vec![0, 0, 0, 0])]);
        }
        assert_eq!(s.next_deadline(), 5000);
        assert_eq!(
            s.tick(5000),
            [Event::InUse, Event::Transmit(vec![0, 0, 0, 0])]
        );
        assert_eq!(s.tick(60_000), [Event::Transmit(vec![0, 0, 0, 0])]);
        // The other editor leaves: we get greeted and served.
        assert!(matches!(
            s.receive(&GREETING, 61_000)[..],
            [Event::Up { .. }]
        ));
        assert!(s.is_up());
    }

    #[test]
    fn session_skips_unknown_frames() {
        let mut s = connect_and_greet();
        assert_eq!(
            s.receive(&[0x09, 0x00, 0x00, 0x01, 0xFF], 20),
            [Event::Skipped { kind: 9, len: 1 }]
        );
    }
}
