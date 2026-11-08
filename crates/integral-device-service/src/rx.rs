//! Which Parts a Program Change on the note port reaches.
//!
//! The device applies a Program Change to every Part whose Receive Channel
//! is the message's and whose Receive Program Change is on, and to the
//! Studio Set if it is on the System Studio Set Control Channel. The service
//! reads those settings (they are not what a state's defaults say) and keeps
//! what it has read or seen written.

use integral_core::address::{Address, DataSize};
use integral_core::device::DeviceState;
use integral_core::params::{self, part};
use integral_core::sysex;

/// Bytes one RQ1 reads from a Part's block: from Receive Channel through
/// Receive Program Change, both included.
const PART_READ_LEN: u32 = part::RECEIVE_PROGRAM_CHANGE[2] as u32 + 1;

/// What a Program Change reaches.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reached {
    /// The Studio Set is selected.
    pub studio_set: bool,
    /// These Parts' tones are selected.
    pub parts: Vec<u8>,
}

/// The receive settings read so far.
#[derive(Debug, Default)]
pub(crate) struct RxMap {
    channel: [Option<u8>; DeviceState::NUM_PARTS as usize],
    program_change: [Option<bool>; DeviceState::NUM_PARTS as usize],
    studio_set: Option<u8>,
}

impl RxMap {
    /// Forget everything read.
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    /// Take the settings in `data` written or reported at `address`.
    pub(crate) fn observe(&mut self, address: &Address, data: &[u8]) {
        let start = address.value();
        let at = |value: u32| -> Option<u8> {
            let i = usize::try_from(value.checked_sub(start)?).ok()?;
            data.get(i).copied()
        };
        for p in 0..DeviceState::NUM_PARTS {
            let block = params::part_address(p, part::RECEIVE_CHANNEL).value();
            let slot = usize::from(p);
            if let Some(c) = at(block) {
                self.channel[slot] = Some(c);
            }
            if let Some(on) = at(params::part_address(p, part::RECEIVE_PROGRAM_CHANGE).value()) {
                self.program_change[slot] = Some(on == 1);
            }
        }
        if let Some(c) = at(params::SYSTEM_STUDIO_SET_CONTROL_CHANNEL.value()) {
            self.studio_set = Some(c);
        }
    }

    /// Whether the Studio Set Control Channel has been read.
    pub(crate) fn studio_set_known(&self) -> bool {
        self.studio_set.is_some()
    }

    /// Whether everything is known.
    pub(crate) fn is_complete(&self) -> bool {
        self.studio_set.is_some()
            && self.channel.iter().all(Option::is_some)
            && self.program_change.iter().all(Option::is_some)
    }

    /// The Parts with a setting not known.
    pub(crate) fn unknown_parts(&self) -> Vec<u8> {
        (0..DeviceState::NUM_PARTS)
            .filter(|p| {
                let slot = usize::from(*p);
                self.channel[slot].is_none() || self.program_change[slot].is_none()
            })
            .collect()
    }

    /// The RQ1s that read what is not known.
    pub(crate) fn requests(&self, device_id: u8) -> Vec<Vec<u8>> {
        let size = DataSize::from_value(PART_READ_LEN).expect("a few bytes");
        let mut requests: Vec<_> = self
            .unknown_parts()
            .into_iter()
            .map(|p| {
                let at = params::part_address(p, part::RECEIVE_CHANNEL);
                sysex::build_rq1(device_id, &at, &size)
            })
            .collect();
        if self.studio_set.is_none() {
            let at = params::SYSTEM_STUDIO_SET_CONTROL_CHANNEL;
            requests.push(sysex::build_rq1(device_id, &at, &DataSize::ONE));
        }
        requests
    }

    /// What a Program Change on `channel` reaches. A setting not known
    /// counts as reaching it if `unknown_reaches`.
    pub(crate) fn reached(&self, channel: u8, unknown_reaches: bool) -> Reached {
        let is = |known: Option<u8>| known.map_or(unknown_reaches, |c| c == channel);
        Reached {
            studio_set: is(self.studio_set),
            parts: (0..DeviceState::NUM_PARTS)
                .filter(|p| {
                    let slot = usize::from(*p);
                    is(self.channel[slot]) && self.program_change[slot].unwrap_or(unknown_reaches)
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(rx: &mut RxMap, channel_of: impl Fn(u8) -> u8, studio: u8) {
        for p in 0..DeviceState::NUM_PARTS {
            let mut data = vec![0u8; PART_READ_LEN as usize];
            data[0] = channel_of(p);
            data[usize::from(part::RECEIVE_PROGRAM_CHANGE[2])] = 1;
            rx.observe(&params::part_address(p, part::RECEIVE_CHANNEL), &data);
        }
        rx.observe(&params::SYSTEM_STUDIO_SET_CONTROL_CHANNEL, &[studio]);
    }

    #[test]
    fn nothing_is_known_until_read_and_each_unknown_setting_has_a_request() {
        let rx = RxMap::default();
        assert!(!rx.is_complete());
        assert_eq!(rx.requests(0x10).len(), 17, "16 Parts and the Studio Set");
        assert_eq!(rx.reached(3, false), Reached::default());
        assert_eq!(rx.reached(3, true).parts.len(), 16);
        assert!(rx.reached(3, true).studio_set);
    }

    #[test]
    fn a_program_change_reaches_the_parts_on_its_channel_with_it_switched_on() {
        let mut rx = RxMap::default();
        read_all(&mut rx, |p| p % 4, 9);
        assert!(rx.is_complete());
        assert!(rx.requests(0x10).is_empty());
        let reached = rx.reached(1, false);
        assert_eq!(reached.parts, [1, 5, 9, 13]);
        assert!(!reached.studio_set);
        assert!(rx.reached(9, false).studio_set);

        // Part 5 stops listening to Program Change.
        let off = params::part_address(5, part::RECEIVE_PROGRAM_CHANGE);
        rx.observe(&off, &[0]);
        assert_eq!(rx.reached(1, false).parts, [1, 9, 13]);
        // Part 1 moves to channel 4.
        rx.observe(&params::part_address(1, part::RECEIVE_CHANNEL), &[4]);
        assert_eq!(rx.reached(1, false).parts, [9, 13]);
        assert_eq!(rx.reached(4, false).parts, [1]);
    }
}
