//! Host-driven reader that fetches whole blocks with one RQ1 each.
//!
//! The device can answer an RQ1 with several DT1 packets, and only covers
//! whole parameters at addresses that exist (e.g. a 384-byte read at
//! `2F 00 20 00` comes back as three 77-byte Studio Set Part blocks, with
//! gaps between them). The reader places each packet by its address and
//! reports each block as complete, with a gap, or timed out.
//!
//! The reader does no I/O. The host sends [`BlockReader::next_request`],
//! feeds every DT1 it receives to [`BlockReader::on_dt1`] while
//! [`BlockReader::is_waiting`], and calls [`BlockReader::on_timeout`] when
//! its timeout elapses first. Reading a block whole never splits a
//! nibblized parameter, which the device would not answer.
//!
//! Reference: `docs/midi/99-undocumented.md` §2, "Read Notes".

use core::ops::Range;

use crate::address::{Address, DataSize};
use crate::sysex;

/// What a block read produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockData {
    /// Every byte arrived.
    Complete(Vec<u8>),
    /// Some bytes arrived; `missing` lists the byte ranges that did not,
    /// which read as zero in `data`.
    Gap {
        /// The block, zero where bytes are missing.
        data: Vec<u8>,
        /// Byte ranges, relative to the block start, that never arrived.
        missing: Vec<Range<usize>>,
    },
    /// No byte arrived.
    Timeout,
}

/// The outcome of reading one block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockRead {
    /// Start address of the block.
    pub address: Address,
    /// What arrived.
    pub data: BlockData,
}

struct Block {
    address: Address,
    size: DataSize,
    data: Vec<u8>,
    filled: Vec<bool>,
}

impl Block {
    fn is_complete(&self) -> bool {
        self.filled.iter().all(|&f| f)
    }

    fn outcome(self) -> BlockRead {
        let mut missing: Vec<Range<usize>> = Vec::new();
        for (i, _) in self.filled.iter().enumerate().filter(|(_, f)| !**f) {
            match missing.last_mut() {
                Some(r) if r.end == i => r.end += 1,
                _ => missing.push(i..i + 1),
            }
        }
        let data = match missing.as_slice() {
            [] => BlockData::Complete(self.data),
            [r] if *r == (0..self.data.len()) => BlockData::Timeout,
            _ => BlockData::Gap {
                data: self.data,
                missing,
            },
        };
        BlockRead {
            address: self.address,
            data,
        }
    }
}

/// Reads a list of blocks, one RQ1 at a time.
pub struct BlockReader {
    device_id: u8,
    blocks: Vec<Block>,
    /// Index of the next block to request.
    next: usize,
    /// Whether the last requested block still awaits packets.
    in_flight: bool,
}

impl BlockReader {
    /// A reader for `blocks`, as `(address, size)` pairs (e.g. from
    /// [`crate::device_spec::UserMemorySpec::blocks`]).
    pub fn new(device_id: u8, blocks: impl IntoIterator<Item = (Address, DataSize)>) -> Self {
        let blocks = blocks
            .into_iter()
            .map(|(address, size)| {
                let len = size.value() as usize;
                Block {
                    address,
                    size,
                    data: vec![0; len],
                    filled: vec![false; len],
                }
            })
            .collect();
        Self {
            device_id,
            blocks,
            next: 0,
            in_flight: false,
        }
    }

    /// The RQ1 for the next block, or `None` while a block is awaiting
    /// packets or once every block has been requested.
    pub fn next_request(&mut self) -> Option<Vec<u8>> {
        if self.is_waiting() {
            return None;
        }
        let block = self.blocks.get(self.next)?;
        self.next += 1;
        self.in_flight = true;
        Some(sysex::build_rq1(
            self.device_id,
            &block.address,
            &block.size,
        ))
    }

    /// Whether the last requested block still awaits packets.
    pub fn is_waiting(&self) -> bool {
        self.in_flight && !self.blocks[self.next - 1].is_complete()
    }

    /// Whether every block has been requested and none awaits packets.
    pub fn is_done(&self) -> bool {
        self.next == self.blocks.len() && !self.is_waiting()
    }

    /// Place a DT1 packet in the block it falls in. Bytes past the block's
    /// end are dropped. Returns `false` if the packet starts in no block.
    pub fn on_dt1(&mut self, address: &Address, data: &[u8]) -> bool {
        let at = address.value();
        let Some(block) = self.blocks.iter_mut().find(|b| {
            let start = b.address.value();
            (start..start + b.size.value()).contains(&at)
        }) else {
            return false;
        };
        let start = (at - block.address.value()) as usize;
        let end = (start + data.len()).min(block.data.len());
        block.data[start..end].copy_from_slice(&data[..end - start]);
        block.filled[start..end].fill(true);
        true
    }

    /// The host's timeout elapsed: stop waiting for the current block.
    pub fn on_timeout(&mut self) {
        self.in_flight = false;
    }

    /// The outcome of every block, in request order. Blocks not requested
    /// yet read as [`BlockData::Timeout`].
    pub fn result(self) -> Vec<BlockRead> {
        self.blocks.into_iter().map(Block::outcome).collect()
    }
}

#[cfg(test)]
#[allow(clippy::single_range_in_vec_init)] // `missing` is a list of ranges
mod tests {
    use super::*;

    const MFX: Address = Address::new(0x50, 0x00, 0x02, 0x00);
    const MFX_SIZE: DataSize = DataSize::new(0x00, 0x00, 0x01, 0x11);

    fn bytes(range: Range<u8>) -> Vec<u8> {
        range.collect()
    }

    #[test]
    fn single_packet() {
        let addr = Address::new(0x2F, 0x00, 0x00, 0x00);
        let mut reader = BlockReader::new(0x10, [(addr, DataSize::SIXTEEN)]);
        let rq1 = reader.next_request().unwrap();
        assert_eq!(rq1, sysex::build_rq1(0x10, &addr, &DataSize::SIXTEEN));
        assert!(reader.is_waiting());
        assert!(reader.next_request().is_none());
        assert!(reader.on_dt1(&addr, &bytes(0..16)));
        assert!(reader.is_done());
        assert_eq!(
            reader.result(),
            [BlockRead {
                address: addr,
                data: BlockData::Complete(bytes(0..16)),
            }]
        );
    }

    /// A block split across two packets is whole once both arrive.
    #[test]
    fn two_packet_reply() {
        let mut reader = BlockReader::new(0x10, [(MFX, MFX_SIZE)]);
        reader.next_request().unwrap();
        let data: Vec<u8> = (0..145).map(|i| (i % 128) as u8).collect();
        reader.on_dt1(&MFX, &data[..61]);
        assert!(reader.is_waiting());
        reader.on_dt1(&Address::new(0x50, 0x00, 0x02, 0x3D), &data[61..]);
        assert!(reader.is_done());
        assert_eq!(reader.result()[0].data, BlockData::Complete(data));
    }

    #[test]
    fn out_of_order_packets_across_blocks() {
        let common = Address::new(0x50, 0x00, 0x00, 0x00);
        let mut reader = BlockReader::new(0x10, [(common, DataSize::SIXTEEN), (MFX, MFX_SIZE)]);
        reader.next_request().unwrap();
        reader.on_dt1(&Address::new(0x50, 0x00, 0x00, 0x08), &bytes(8..16));
        reader.on_dt1(&common, &bytes(0..8));
        assert!(!reader.is_waiting());
        let rq1 = reader.next_request().unwrap();
        assert_eq!(rq1, sysex::build_rq1(0x10, &MFX, &MFX_SIZE));
        let data = vec![0x7F; 145];
        reader.on_dt1(&Address::new(0x50, 0x00, 0x02, 0x3D), &data[61..]);
        reader.on_dt1(&MFX, &data[..61]);
        assert!(reader.is_done());
        let result = reader.result();
        assert_eq!(result[0].data, BlockData::Complete(bytes(0..16)));
        assert_eq!(result[1].data, BlockData::Complete(data));
    }

    #[test]
    fn missing_packet_is_a_gap() {
        let mut reader = BlockReader::new(0x10, [(MFX, MFX_SIZE)]);
        reader.next_request().unwrap();
        reader.on_dt1(&Address::new(0x50, 0x00, 0x02, 0x3D), &[1; 84]);
        assert!(reader.is_waiting());
        reader.on_timeout();
        assert!(reader.is_done());
        let mut expected = vec![0; 61];
        expected.extend([1; 84]);
        assert_eq!(
            reader.result()[0].data,
            BlockData::Gap {
                data: expected,
                missing: vec![0..61],
            }
        );
    }

    #[test]
    fn no_reply_is_a_timeout() {
        let mut reader = BlockReader::new(0x10, [(MFX, MFX_SIZE), (MFX, MFX_SIZE)]);
        reader.next_request().unwrap();
        reader.on_timeout();
        assert!(!reader.is_done());
        assert!(reader.next_request().is_some());
        reader.on_timeout();
        assert!(reader.is_done());
        assert!(reader.next_request().is_none());
        assert!(reader.result().iter().all(|r| r.data == BlockData::Timeout));
    }

    #[test]
    fn ignores_foreign_packets_and_clips_overruns() {
        let mut reader = BlockReader::new(0x10, [(MFX, DataSize::new(0, 0, 0, 4))]);
        reader.next_request().unwrap();
        assert!(!reader.on_dt1(&Address::new(0x0F, 0x00, 0x00, 0x10), &[1, 2, 3, 4]));
        assert!(!reader.on_dt1(&Address::new(0x50, 0x00, 0x02, 0x04), &[9]));
        assert!(reader.on_dt1(&Address::new(0x50, 0x00, 0x02, 0x02), &[3, 4, 5]));
        reader.on_timeout();
        assert_eq!(
            reader.result()[0].data,
            BlockData::Gap {
                data: vec![0, 0, 3, 4],
                missing: vec![0..2],
            }
        );
    }
}
