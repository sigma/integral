//! Reads sent to the device and not yet answered, and the bytes a write
//! made stale in them.
//!
//! The device answers in order. A read sent before a write that overlaps it
//! is therefore answered with the value from before the write: taken as the
//! device's current value it would undo the write. [`SentReads`] follows
//! the SysEx the state sends, so it can tell the bytes of a reply that are
//! older than a write from the ones that are not.

use std::ops::Range;

use crate::sysex;

/// One RQ1 on its way.
struct SentRead {
    /// The addresses asked for.
    span: Range<u32>,
    /// Where the next reply to it starts: the device answers a long read in
    /// DT1s of ascending addresses.
    cursor: u32,
    /// The addresses written since the request left.
    stale: Vec<Range<u32>>,
    /// When the request left (ms).
    sent_at: f64,
}

/// The RQ1s sent and not yet answered, with the bytes of each that a later
/// write made stale.
#[derive(Default)]
pub struct SentReads {
    reads: Vec<SentRead>,
}

impl SentReads {
    /// Note a message the state sends at `now_ms`: an RQ1 becomes an
    /// outstanding read, a DT1 makes the overlapping bytes of the reads
    /// before it stale. Other messages are ignored.
    ///
    /// A read is forgotten `timeout_ms` after it left, answered or not, so a
    /// reply that never comes cannot hide a later change.
    pub fn sent(&mut self, message: &[u8], now_ms: f64, timeout_ms: f64) {
        self.expire(now_ms, timeout_ms);
        if let Some(rq1) = sysex::parse_rq1(message) {
            let start = rq1.address.value();
            self.reads.push(SentRead {
                span: start..start + rq1.size.value(),
                cursor: start,
                stale: Vec::new(),
                sent_at: now_ms,
            });
        } else if let Ok(dt1) = sysex::parse_dt1(message) {
            let start = dt1.address.value();
            let written = start..start + dt1.data.len() as u32;
            for read in &mut self.reads {
                if let Some(hit) = overlap(&read.span, &written) {
                    read.stale.push(hit);
                }
            }
        }
    }

    /// The parts of a reply covering `reply` that are not stale: the ranges
    /// to take, in ascending order. A reply no outstanding read accounts for
    /// (a front-panel edit, a reply that came late) is taken whole.
    ///
    /// The reply is matched to the oldest read it falls in, which it then
    /// advances, and which is done once its last byte has come.
    pub fn reply(&mut self, reply: &Range<u32>, now_ms: f64, timeout_ms: f64) -> Vec<Range<u32>> {
        self.expire(now_ms, timeout_ms);
        let Some(i) =
            (self.reads.iter()).position(|r| reply.start >= r.cursor && reply.end <= r.span.end)
        else {
            return vec![reply.clone()];
        };
        let read = &mut self.reads[i];
        read.cursor = reply.end;
        let mut fresh = vec![reply.clone()];
        for stale in &read.stale {
            fresh = fresh.into_iter().flat_map(|r| minus(r, stale)).collect();
        }
        if read.cursor >= read.span.end {
            self.reads.remove(i);
        }
        fresh
    }

    /// Forget the reads sent `timeout_ms` or longer before `now_ms`.
    fn expire(&mut self, now_ms: f64, timeout_ms: f64) {
        self.reads.retain(|r| now_ms - r.sent_at < timeout_ms);
    }
}

/// The addresses both ranges hold, if any.
fn overlap(a: &Range<u32>, b: &Range<u32>) -> Option<Range<u32>> {
    let (start, end) = (a.start.max(b.start), a.end.min(b.end));
    (start < end).then_some(start..end)
}

/// What is left of `range` once `cut` is taken out: up to two ranges.
pub(crate) fn minus(range: Range<u32>, cut: &Range<u32>) -> Vec<Range<u32>> {
    let Some(hit) = overlap(&range, cut) else {
        return vec![range];
    };
    [range.start..hit.start, hit.end..range.end]
        .into_iter()
        .filter(|r| !r.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::{Address, DataSize};

    const TIMEOUT: f64 = 2000.0;

    fn at(n: u32) -> Address {
        Address::from_value(n).unwrap()
    }

    fn pairs(ranges: Vec<Range<u32>>) -> Vec<(u32, u32)> {
        ranges.into_iter().map(|r| (r.start, r.end)).collect()
    }

    fn rq1(start: u32, len: u32) -> Vec<u8> {
        sysex::build_rq1(0x10, &at(start), &DataSize::from_value(len).unwrap())
    }

    fn dt1(start: u32, len: usize) -> Vec<u8> {
        sysex::build_dt1(0x10, &at(start), &vec![1; len])
    }

    #[test]
    fn an_unasked_reply_is_taken_whole() {
        let mut t = SentReads::default();
        assert_eq!(pairs(t.reply(&(10..14), 0.0, TIMEOUT)), [(10, 14)]);
    }

    #[test]
    fn a_write_after_the_request_makes_its_bytes_stale_once() {
        let mut t = SentReads::default();
        t.sent(&rq1(10, 4), 0.0, TIMEOUT);
        t.sent(&dt1(11, 2), 1.0, TIMEOUT);
        assert_eq!(
            pairs(t.reply(&(10..14), 2.0, TIMEOUT)),
            [(10, 11), (13, 14)]
        );
        assert_eq!(
            pairs(t.reply(&(10..14), 3.0, TIMEOUT)),
            [(10, 14)],
            "answered"
        );
    }

    #[test]
    fn a_write_before_the_request_is_not_stale() {
        let mut t = SentReads::default();
        t.sent(&dt1(10, 1), 0.0, TIMEOUT);
        t.sent(&rq1(10, 1), 1.0, TIMEOUT);
        assert_eq!(pairs(t.reply(&(10..11), 2.0, TIMEOUT)), [(10, 11)]);
    }

    #[test]
    fn a_later_read_is_not_made_stale_by_an_earlier_one() {
        let mut t = SentReads::default();
        t.sent(&rq1(10, 1), 0.0, TIMEOUT);
        t.sent(&dt1(10, 1), 1.0, TIMEOUT);
        t.sent(&rq1(10, 1), 2.0, TIMEOUT);
        assert!(t.reply(&(10..11), 3.0, TIMEOUT).is_empty());
        assert_eq!(pairs(t.reply(&(10..11), 4.0, TIMEOUT)), [(10, 11)]);
    }

    #[test]
    fn a_long_read_answered_in_several_dt1s() {
        let mut t = SentReads::default();
        t.sent(&rq1(10, 6), 0.0, TIMEOUT);
        t.sent(&dt1(14, 1), 1.0, TIMEOUT);
        assert_eq!(pairs(t.reply(&(10..13), 2.0, TIMEOUT)), [(10, 13)]);
        assert_eq!(
            pairs(t.reply(&(13..16), 3.0, TIMEOUT)),
            [(13, 14), (15, 16)]
        );
    }

    #[test]
    fn a_read_expires_with_its_stale_bytes() {
        let mut t = SentReads::default();
        t.sent(&rq1(10, 1), 0.0, TIMEOUT);
        t.sent(&dt1(10, 1), 1.0, TIMEOUT);
        assert_eq!(pairs(t.reply(&(10..11), TIMEOUT, TIMEOUT)), [(10, 11)]);
    }
}
