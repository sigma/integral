//! The four Expansion slots and the undocumented SysEx that reads them.
//!
//! An RQ1 at `0F 00 00 10` with a zero size field asks for the slots; the
//! device answers with a DT1 at the same address carrying one slot value per
//! slot A–D.
//!
//! Reference: `docs/midi/99-undocumented.md`, section 5.

use thiserror::Error;

use crate::address::{Address, DataSize};
use crate::expansions::{self, Expansion};
use crate::sysex;

/// Address of the slot read request and its reply.
pub const SLOT_READ_ADDR: Address = Address::new(0x0F, 0x00, 0x00, 0x10);

/// Number of Expansion slots.
pub const SLOT_COUNT: usize = 4;

/// Slot names, in slot order.
pub const SLOT_NAMES: [char; SLOT_COUNT] = ['A', 'B', 'C', 'D'];

/// Slot value for an empty slot.
const OFF: u8 = 0x00;

/// What sits in each Expansion slot A–D (`None` = OFF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExpansionSlots(pub [Option<&'static Expansion>; SLOT_COUNT]);

/// A slot reply that doesn't decode to Expansion slots.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SlotsError {
    /// The reply doesn't carry exactly four slot values.
    #[error("expected {SLOT_COUNT} slot values, got {0}")]
    Length(usize),
    /// A slot value names no Expansion.
    #[error("unknown Expansion slot value {0:#04x}")]
    UnknownValue(u8),
}

impl ExpansionSlots {
    /// Decode four slot values as sent by the device.
    ///
    /// ExPCM fills all four slots: it loads as `13 13 13 13` and reads back
    /// as `13 14 15 16`; both decode to ExPCM in every slot.
    pub fn from_values(values: &[u8]) -> Result<Self, SlotsError> {
        let values: [u8; SLOT_COUNT] = values
            .try_into()
            .map_err(|_| SlotsError::Length(values.len()))?;
        let expcm = expansions::by_name("ExPCM").expect("ExPCM in catalog");
        let v = expcm.load_value;
        if values == [v; SLOT_COUNT] || values == [v, v + 1, v + 2, v + 3] {
            return Ok(Self([Some(expcm); SLOT_COUNT]));
        }
        let mut slots = [None; SLOT_COUNT];
        for (slot, value) in slots.iter_mut().zip(values) {
            if value == OFF {
                continue;
            }
            let expansion = expansions::by_load_value(value)
                .filter(|e| e.slot_cost == 1)
                .ok_or(SlotsError::UnknownValue(value))?;
            *slot = Some(expansion);
        }
        Ok(Self(slots))
    }

    /// The slot values a load request sends for these slots.
    pub fn values(&self) -> [u8; SLOT_COUNT] {
        self.0.map(|slot| slot.map_or(OFF, |e| e.load_value))
    }

    /// The slot (0 = A) holding `expansion`, if it is loaded.
    pub fn slot_of(&self, expansion: &Expansion) -> Option<usize> {
        self.0.iter().position(|slot| *slot == Some(expansion))
    }

    /// Whether `expansion` sits in a slot.
    pub fn contains(&self, expansion: &Expansion) -> bool {
        self.slot_of(expansion).is_some()
    }
}

/// Build the RQ1 that reads the Expansion slots.
pub fn build_read_request(device_id: u8) -> Vec<u8> {
    sysex::build_rq1(device_id, &SLOT_READ_ADDR, &DataSize::new(0, 0, 0, 0))
}

/// Parse a DT1 as a slot read reply, or `None` if it is something else.
pub fn parse_read_reply(
    address: &Address,
    data: &[u8],
) -> Option<Result<ExpansionSlots, SlotsError>> {
    (*address == SLOT_READ_ADDR).then(|| ExpansionSlots::from_values(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(slots: ExpansionSlots) -> [&'static str; SLOT_COUNT] {
        slots.0.map(|s| s.map_or("OFF", |e| e.name))
    }

    #[test]
    fn builds_documented_read_request() {
        assert_eq!(
            build_read_request(0x10),
            [
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11, 0x0F, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00,
                0x00, 0x61, 0xF7
            ]
        );
        assert_eq!(build_read_request(0x11)[2], 0x11);
    }

    #[test]
    fn parses_captured_reply() {
        let slots = parse_read_reply(&SLOT_READ_ADDR, &[0x03, 0x05, 0x07, 0x08])
            .unwrap()
            .unwrap();
        assert_eq!(names(slots), ["SRX-03", "SRX-05", "SRX-07", "SRX-08"]);
        assert_eq!(slots.values(), [0x03, 0x05, 0x07, 0x08]);
    }

    #[test]
    fn parses_expcm_read_back() {
        for values in [[0x13, 0x14, 0x15, 0x16], [0x13; 4]] {
            let slots = ExpansionSlots::from_values(&values).unwrap();
            assert_eq!(names(slots), ["ExPCM"; 4]);
            assert_eq!(slots.values(), [0x13; 4]);
        }
    }

    #[test]
    fn parses_off_slots() {
        let slots = ExpansionSlots::from_values(&[0x00, 0x0D, 0x00, 0x12]).unwrap();
        assert_eq!(names(slots), ["OFF", "ExSN1", "OFF", "ExSN6"]);
        assert_eq!(slots, ExpansionSlots::from_values(&slots.values()).unwrap());
    }

    #[test]
    fn rejects_invalid_values() {
        for (values, err) in [
            (
                &[0x03, 0x17, 0x00, 0x00][..],
                SlotsError::UnknownValue(0x17),
            ),
            (&[0x03, 0x14, 0x00, 0x00], SlotsError::UnknownValue(0x14)),
            (&[0x13, 0x00, 0x00, 0x00], SlotsError::UnknownValue(0x13)),
            (&[0x03, 0x05, 0x07], SlotsError::Length(3)),
        ] {
            assert_eq!(ExpansionSlots::from_values(values), Err(err));
        }
    }

    #[test]
    fn ignores_other_addresses() {
        let addr = Address::new(0x0F, 0x00, 0x30, 0x02);
        assert_eq!(parse_read_reply(&addr, &[0; 4]), None);
    }
}
