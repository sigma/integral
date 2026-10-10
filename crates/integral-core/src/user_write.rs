//! Undocumented write (save) of Temporary data into user memory.
//!
//! The INTEGRA-7 stores a part's Temporary tone, or the Temporary Studio
//! Set, into a user slot when it receives an RQ1 at `0F 00 10 00` whose
//! 4-byte size field carries the destination bank select and the source.
//! It answers with a DT1 at `0F 00 10 01` when the write starts and one at
//! `0F 00 10 02` when it finishes (2–3 s later).
//!
//! Reference: `docs/midi/99-undocumented.md`, section 4.

use crate::address::{Address, DataSize};
use crate::sysex;

/// Address of the write request.
const WRITE_REQUEST_ADDR: Address = Address::new(0x0F, 0x00, 0x10, 0x00);
/// Address of the "write started" reply.
const WRITE_STARTED_ADDR: Address = Address::new(0x0F, 0x00, 0x10, 0x01);
/// Address of the "write finished" reply.
const WRITE_FINISHED_ADDR: Address = Address::new(0x0F, 0x00, 0x10, 0x02);
/// Source byte selecting the Studio Set instead of a part.
const STUDIO_SET_SOURCE: u8 = 0x7F;

/// Bank Select triple identifying a user slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankSelect {
    /// Bank Select MSB.
    pub msb: u8,
    /// Bank Select LSB.
    pub lsb: u8,
    /// Program number, 0-indexed.
    pub pc: u8,
}

/// What to store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteSource {
    /// The Temporary tone of a part (0-indexed).
    Part(u8),
    /// The Temporary Studio Set.
    StudioSet,
}

impl WriteSource {
    fn byte(self) -> u8 {
        match self {
            Self::Part(part) => part,
            Self::StudioSet => STUDIO_SET_SOURCE,
        }
    }

    fn from_byte(b: u8) -> Self {
        match b {
            STUDIO_SET_SOURCE => Self::StudioSet,
            part => Self::Part(part),
        }
    }
}

/// Progress reported by the device for a write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WritePhase {
    /// The device accepted the request and started writing.
    Started,
    /// The slot is written; the device is ready for further requests.
    Finished,
}

/// A parsed write reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteReply {
    /// Started or finished.
    pub phase: WritePhase,
    /// Destination slot, echoed from the request.
    pub target: BankSelect,
    /// Source, echoed from the request.
    pub source: WriteSource,
}

/// Build the request that stores `source` into the user slot `target`.
pub fn build_write_request(device_id: u8, target: BankSelect, source: WriteSource) -> Vec<u8> {
    let payload = DataSize::new(target.msb, target.lsb, target.pc, source.byte());
    sysex::build_rq1(device_id, &WRITE_REQUEST_ADDR, &payload)
}

/// Parse a DT1 as a write reply, or `None` if it is something else.
pub fn parse_write_reply(address: &Address, data: &[u8]) -> Option<WriteReply> {
    let phase = match *address {
        WRITE_STARTED_ADDR => WritePhase::Started,
        WRITE_FINISHED_ADDR => WritePhase::Finished,
        _ => return None,
    };
    let [_, _, _, msb, lsb, pc, source] = *data else {
        return None;
    };
    Some(WriteReply {
        phase,
        target: BankSelect { msb, lsb, pc },
        source: WriteSource::from_byte(source),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bs(msb: u8, lsb: u8, pc: u8) -> BankSelect {
        BankSelect { msb, lsb, pc }
    }

    /// Requests captured from Roland's iPad editor (docs/midi/99-undocumented.md §4).
    #[test]
    fn builds_captured_requests() {
        let cases = [
            (bs(0x5F, 0x01, 0x00), WriteSource::Part(0), 0x00, 0x01),
            (bs(0x59, 0x01, 0x00), WriteSource::Part(1), 0x01, 0x06),
            (bs(0x57, 0x00, 0x00), WriteSource::Part(4), 0x04, 0x06),
            (bs(0x56, 0x00, 0x00), WriteSource::Part(4), 0x04, 0x07),
            (bs(0x58, 0x00, 0x00), WriteSource::Part(4), 0x04, 0x05),
            (bs(0x55, 0x00, 0x3F), WriteSource::StudioSet, 0x7F, 0x4E),
        ];
        for (target, source, source_byte, checksum) in cases {
            assert_eq!(
                build_write_request(0x10, target, source),
                [
                    0xF0,
                    0x41,
                    0x10,
                    0x00,
                    0x00,
                    0x64,
                    0x11,
                    0x0F,
                    0x00,
                    0x10,
                    0x00,
                    target.msb,
                    target.lsb,
                    target.pc,
                    source_byte,
                    checksum,
                    0xF7,
                ],
                "{target:?} {source:?}"
            );
        }
    }

    #[test]
    fn uses_given_device_id() {
        let msg = build_write_request(0x11, bs(0x5F, 0x01, 0x00), WriteSource::Part(0));
        assert_eq!(msg[2], 0x11);
    }

    #[test]
    fn parses_started_and_finished() {
        let data = [0x00, 0x00, 0x00, 0x5F, 0x01, 0x00, 0x00];
        let started = parse_write_reply(&WRITE_STARTED_ADDR, &data).unwrap();
        assert_eq!(
            started,
            WriteReply {
                phase: WritePhase::Started,
                target: bs(0x5F, 0x01, 0x00),
                source: WriteSource::Part(0),
            }
        );
        let finished = parse_write_reply(&WRITE_FINISHED_ADDR, &data).unwrap();
        assert_eq!(finished.phase, WritePhase::Finished);
    }

    #[test]
    fn parses_studio_set_source() {
        let data = [0x00, 0x00, 0x00, 0x55, 0x00, 0x3F, 0x7F];
        let reply = parse_write_reply(&WRITE_FINISHED_ADDR, &data).unwrap();
        assert_eq!(reply.source, WriteSource::StudioSet);
        assert_eq!(reply.target, bs(0x55, 0x00, 0x3F));
    }

    #[test]
    fn ignores_other_messages() {
        let data = [0x00, 0x00, 0x00, 0x5F, 0x01, 0x00, 0x00];
        assert_eq!(parse_write_reply(&WRITE_REQUEST_ADDR, &data), None);
        assert_eq!(
            parse_write_reply(&Address::new(0x0F, 0x00, 0x04, 0x02), &data),
            None
        );
        assert_eq!(parse_write_reply(&WRITE_FINISHED_ADDR, &data[..6]), None);
    }
}
