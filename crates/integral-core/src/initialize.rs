//! Undocumented Init of a Temporary area.
//!
//! The INTEGRA-7 resets part of its Temporary area to INIT when it receives
//! an RQ1 at `0F 00 18 TT` whose 4-byte size field carries `7F 7F`, the
//! Part and the Partial. It answers with a DT1 at `0F 00 18 01` when the
//! Init starts and one at `0F 00 18 02` when it finishes. The result equals
//! an unwritten User slot.
//!
//! Reference: `docs/midi/99-undocumented.md`, section 7.

use crate::address::{Address, DataSize};
use crate::sysex;

/// Address of the "Init finished" reply.
const INIT_FINISHED_ADDR: Address = Address::new(0x0F, 0x00, 0x18, 0x02);

/// What to reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitTarget {
    /// The Temporary Studio Set, every Part's tone included.
    StudioSet,
    /// The Temporary tone of a Part (0-indexed). It keeps its type.
    Tone(u8),
}

/// Build the request that resets `target` to INIT.
pub fn build_init_request(device_id: u8, target: InitTarget) -> Vec<u8> {
    let (tt, part) = match target {
        InitTarget::StudioSet => (0x00, 0x00),
        InitTarget::Tone(part) => (0x20, part),
    };
    let address = Address::new(0x0F, 0x00, 0x18, tt);
    sysex::build_rq1(device_id, &address, &DataSize::new(0x7F, 0x7F, part, 0x00))
}

/// Whether a DT1 is the "Init finished" reply.
pub fn is_init_finished(address: &Address, data: &[u8]) -> bool {
    *address == INIT_FINISHED_ADDR && data == [0, 0, 0, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Requests sent on our device (docs/midi/99-undocumented.md §7).
    #[test]
    fn builds_device_checked_requests() {
        assert_eq!(
            build_init_request(0x10, InitTarget::StudioSet),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11, 0x0F, 0x00, 0x18, 0x00, 0x7F, 0x7F, 0x00,
                0x00, 0x5B, 0xF7
            ]
        );
        assert_eq!(
            build_init_request(0x10, InitTarget::Tone(1)),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11, 0x0F, 0x00, 0x18, 0x20, 0x7F, 0x7F, 0x01,
                0x00, 0x3A, 0xF7
            ]
        );
    }

    #[test]
    fn recognises_only_finished() {
        let started = Address::new(0x0F, 0x00, 0x18, 0x01);
        assert!(is_init_finished(&INIT_FINISHED_ADDR, &[0; 4]));
        assert!(!is_init_finished(&started, &[0; 4]));
        assert!(!is_init_finished(&INIT_FINISHED_ADDR, &[0, 0, 0, 1]));
    }
}
