//! Roland SysEx frames of any model: splitting a byte stream into
//! messages, and parsing a DT1 whatever the length of its Model ID.
//!
//! [`sysex::parse_dt1`] accepts only the INTEGRA-7 and stays the parser for
//! device traffic.
//!
//! Reference: `docs/midi/01-protocol.md`, `docs/research/xv-patch-format.md`
//! and `docs/research/jv-patch-format.md` (framing).

use thiserror::Error;

use crate::address::Address;
use crate::sysex::{self, CMD_DT1, ROLAND_ID};

/// SysEx start byte.
pub(super) const SYSEX_START: u8 = 0xF0;

/// SysEx end byte.
const SYSEX_END: u8 = 0xF7;

/// Bytes of a DT1 address, on every Roland model in scope.
const ADDRESS_LEN: usize = 4;

/// A Roland DT1 message of any model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame<'a> {
    /// Model ID: leading `00` bytes, then one non-zero byte (JV `6A`, XV
    /// `00 10`, INTEGRA-7 `00 00 64`).
    pub model: &'a [u8],
    /// Start address.
    pub address: [u8; ADDRESS_LEN],
    /// Data bytes.
    pub data: &'a [u8],
}

/// Why a message is not a usable [`Frame`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum FrameError {
    /// Not a Roland DT1 (another manufacturer, a Universal message, an RQ1),
    /// or too short to be one.
    #[error("not a Roland DT1 message")]
    NotDt1,
    /// The checksum doesn't match the address and data.
    #[error("checksum mismatch")]
    BadChecksum,
}

/// Parse one `F0 … F7` message as a Roland DT1:
/// `F0 41 dev <model> 12 <address> <data> <sum> F7`.
pub fn parse(raw: &[u8]) -> Result<Frame<'_>, FrameError> {
    let body = raw
        .strip_prefix(&[SYSEX_START, ROLAND_ID])
        .and_then(|b| b.strip_suffix(&[SYSEX_END]))
        .and_then(|b| b.get(1..)) // device ID, ignored
        .ok_or(FrameError::NotDt1)?;
    let model_len = body
        .iter()
        .position(|&b| b != 0)
        .ok_or(FrameError::NotDt1)?
        + 1;
    let (model, rest) = body.split_at(model_len);
    let Some((&CMD_DT1, rest)) = rest.split_first() else {
        return Err(FrameError::NotDt1);
    };
    let Some((&sum, payload)) = rest.split_last() else {
        return Err(FrameError::NotDt1);
    };
    if payload.len() <= ADDRESS_LEN {
        return Err(FrameError::NotDt1);
    }
    if sysex::checksum(payload) != sum {
        return Err(FrameError::BadChecksum);
    }
    let (address, data) = payload.split_at(ADDRESS_LEN);
    Ok(Frame {
        model,
        address: address.try_into().expect("split at ADDRESS_LEN"),
        data,
    })
}

/// The `F0 … F7` messages of a byte stream. `incomplete` is set when bytes
/// lie outside any message, or a message is cut short by another status
/// byte or the end of the stream.
#[derive(Debug, Default)]
pub struct Messages<'a> {
    /// The complete messages, `F0` and `F7` included.
    pub messages: Vec<&'a [u8]>,
    /// Whether any bytes were left over.
    pub incomplete: bool,
}

/// Split `bytes` into its SysEx messages.
pub fn split(bytes: &[u8]) -> Messages<'_> {
    let mut out = Messages::default();
    let mut start = None;
    for (i, &b) in bytes.iter().enumerate() {
        match (start, b) {
            (Some(s), SYSEX_END) => {
                out.messages.push(&bytes[s..=i]);
                start = None;
            }
            (_, SYSEX_START) => {
                out.incomplete |= start.is_some();
                start = Some(i);
            }
            (Some(_), 0x80..) => {
                out.incomplete = true;
                start = None;
            }
            (Some(_), _) => {}
            (None, _) => out.incomplete = true,
        }
    }
    out.incomplete |= start.is_some();
    out
}

/// Rebuild the blocks of `layout`, each `(start address value, size)`, from
/// DT1s `(address, data)`, or `None` if a block is not fully covered.
///
/// Blocks are rebuilt from addresses, not message boundaries: one DT1 may
/// span several blocks, and one block may be split across several DT1s.
/// Bytes between blocks are ignored; later bytes overwrite earlier ones.
pub(super) fn assemble<'a>(
    layout: &[(u32, usize)],
    frames: impl IntoIterator<Item = (Address, &'a [u8])>,
) -> Option<Vec<Vec<u8>>> {
    let mut blocks: Vec<Vec<u8>> = layout.iter().map(|&(_, n)| vec![0; n]).collect();
    let mut filled: Vec<Vec<bool>> = layout.iter().map(|&(_, n)| vec![false; n]).collect();
    for (address, data) in frames {
        let start = address.value() as usize;
        let end = start + data.len();
        for (i, &(block, size)) in layout.iter().enumerate() {
            let block = block as usize;
            let (lo, hi) = (start.max(block), end.min(block + size));
            if lo < hi {
                blocks[i][lo - block..hi - block].copy_from_slice(&data[lo - start..hi - start]);
                filled[i][lo - block..hi - block].fill(true);
            }
        }
    }
    filled
        .iter()
        .all(|f| f.iter().all(|&b| b))
        .then_some(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A DT1 for `model` with a valid checksum.
    fn dt1(model: &[u8], address: [u8; 4], data: &[u8]) -> Vec<u8> {
        let payload: Vec<u8> = address.iter().chain(data).copied().collect();
        [&[SYSEX_START, ROLAND_ID, 0x10], model, &[CMD_DT1]]
            .concat()
            .into_iter()
            .chain(payload.iter().copied())
            .chain([sysex::checksum(&payload), SYSEX_END])
            .collect()
    }

    #[test]
    fn parses_any_model_length() {
        for model in [&[0x6A][..], &[0x00, 0x10], &sysex::MODEL_ID] {
            let raw = dt1(model, [0x11, 0x02, 0x03, 0x04], &[0x05, 0x06]);
            let frame = parse(&raw).unwrap();
            assert_eq!(frame.model, model);
            assert_eq!(frame.address, [0x11, 0x02, 0x03, 0x04]);
            assert_eq!(frame.data, [0x05, 0x06]);
        }
    }

    #[test]
    fn integra_frames_match_build_dt1() {
        let addr = crate::address::Address::new(0x19, 0x00, 0x00, 0x00);
        let raw = sysex::build_dt1(0x17, &addr, &[1, 2, 3]);
        let frame = parse(&raw).unwrap();
        assert_eq!(
            (frame.model, frame.data),
            (&sysex::MODEL_ID[..], &[1, 2, 3][..])
        );
    }

    #[test]
    fn rejects_bad_checksums_and_other_messages() {
        let mut raw = dt1(&[0x6A], [0; 4], &[1]);
        let at = raw.len() - 2;
        raw[at] ^= 1;
        assert_eq!(parse(&raw), Err(FrameError::BadChecksum));
        let rq1 = [
            0xF0, 0x41, 0x10, 0x6A, 0x11, 0, 0, 0, 0, 0, 0, 0, 1, 0x7F, 0xF7,
        ];
        let gm_off = [0xF0, 0x7E, 0x7F, 0x09, 0x02, 0xF7];
        for raw in [&rq1[..], &gm_off, &[0xF0, 0x41, 0x10, 0x00, 0x00, 0xF7]] {
            assert_eq!(parse(raw), Err(FrameError::NotDt1), "{raw:02X?}");
        }
    }

    #[test]
    fn splits_messages_and_flags_leftovers() {
        let split_of = |b: &[u8]| {
            let m = split(b);
            (m.messages.len(), m.incomplete)
        };
        assert_eq!(split_of(&[0xF0, 1, 0xF7, 0xF0, 2, 0xF7]), (2, false));
        assert_eq!(split_of(&[0xF0, 1, 0xF0, 2, 0xF7]), (1, true));
        assert_eq!(split_of(&[0x0D, 0xF0, 2, 0xF7]), (1, true));
        assert_eq!(split_of(&[0xF0, 1, 0x90, 2, 0xF7]), (0, true));
        assert_eq!(split_of(&[0xF0, 2, 0xF7, 0xF0, 1]), (1, true));
    }
}
