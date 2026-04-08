//! Address and data size types for the INTEGRA-7 SysEx address space.
//!
//! The INTEGRA-7 uses 4-byte addresses where each byte is in the 7-bit
//! range `0x00`–`0x7F`. Arithmetic carries propagate from lower to higher
//! bytes (i.e., `0x7F + 1 = 0x00` with carry to the next byte).

use core::fmt;

/// A 4-byte SysEx address in the INTEGRA-7 address space.
///
/// Each byte is in the range `0x00`–`0x7F` (7-bit). Addresses are stored
/// big-endian: `[MSB, upper-middle, lower-middle, LSB]`.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Address(pub [u8; 4]);

impl Address {
    /// Create a new address from individual bytes.
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Add an offset to this address using 7-bit-per-byte arithmetic.
    ///
    /// Each byte wraps at `0x80` (128), with carry propagating to the
    /// next higher byte.
    pub const fn offset(self, off: [u8; 4]) -> Self {
        let mut result = [0u8; 4];
        let mut carry: u16 = 0;

        // Process LSB to MSB
        let mut i: usize = 4;
        while i > 0 {
            i -= 1;
            let sum = self.0[i] as u16 + off[i] as u16 + carry;
            result[i] = (sum % 128) as u8;
            carry = sum / 128;
        }

        Self(result)
    }

    /// Return the raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Convert address to a string key suitable for hash map lookups.
    ///
    /// Returns a 4-byte string of the raw address bytes.
    pub fn to_key(&self) -> [u8; 4] {
        self.0
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Address({:02X} {:02X} {:02X} {:02X})",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X} {:02X} {:02X} {:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

/// A 4-byte data size for RQ1 requests.
///
/// Uses the same 7-bit-per-byte encoding as [`Address`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct DataSize(pub [u8; 4]);

impl DataSize {
    /// Create a new data size from individual bytes.
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// Convenience: a size of 1 byte.
    pub const ONE: Self = Self([0x00, 0x00, 0x00, 0x01]);

    /// Convenience: a size of 16 bytes (e.g., Studio Set name).
    pub const SIXTEEN: Self = Self([0x00, 0x00, 0x00, 0x10]);

    /// Return the raw bytes.
    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl fmt::Debug for DataSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "DataSize({:02X} {:02X} {:02X} {:02X})",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

impl fmt::Display for DataSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X} {:02X} {:02X} {:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3]
        )
    }
}

// ---------------------------------------------------------------------------
// Well-known base addresses
// ---------------------------------------------------------------------------

/// Setup block base address.
pub const SETUP: Address = Address::new(0x01, 0x00, 0x00, 0x00);

/// System Common block base address.
pub const SYSTEM_COMMON: Address = Address::new(0x02, 0x00, 0x00, 0x00);

/// Temporary Studio Set base address.
pub const STUDIO_SET: Address = Address::new(0x18, 0x00, 0x00, 0x00);

/// Compute the absolute address for a parameter within a Studio Set Part.
///
/// `part` is 0-indexed (0 = Part 1, 15 = Part 16).
/// `param_offset` is the parameter's offset within the Part block.
///
/// From the docs, Part offsets within the Studio Set are:
///   Part 1: `00 00 20 00`, Part 2: `00 00 21 00`, ..., Part 16: `00 00 2F 00`
pub const fn studio_set_part(part: u8, param_offset: [u8; 3]) -> Address {
    STUDIO_SET
        .offset([0x00, 0x00, 0x20 + part, param_offset[0]])
        .offset([0x00, 0x00, 0x00, param_offset[1]])
        .offset([0x00, 0x00, 0x00, param_offset[2]])
}

/// Studio Set Common address with a parameter offset.
pub const fn studio_set_common(param_offset: [u8; 2]) -> Address {
    STUDIO_SET.offset([0x00, 0x00, param_offset[0], param_offset[1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_offset() {
        let base = Address::new(0x18, 0x00, 0x00, 0x00);
        let result = base.offset([0x00, 0x20, 0x00, 0x09]);
        assert_eq!(result, Address::new(0x18, 0x20, 0x00, 0x09));
    }

    #[test]
    fn offset_with_carry() {
        // 0x7F + 0x01 in the LSB should carry to next byte
        let addr = Address::new(0x00, 0x00, 0x00, 0x7F);
        let result = addr.offset([0x00, 0x00, 0x00, 0x01]);
        assert_eq!(result, Address::new(0x00, 0x00, 0x01, 0x00));
    }

    #[test]
    fn offset_cascade_carry() {
        let addr = Address::new(0x00, 0x00, 0x7F, 0x7F);
        let result = addr.offset([0x00, 0x00, 0x00, 0x01]);
        assert_eq!(result, Address::new(0x00, 0x01, 0x00, 0x00));
    }

    #[test]
    fn part_1_level_address() {
        // Part 1 (index 0) Level: 18 00 00 00 + 00 00 20 00 + 00 00 00 09 = 18 00 20 09
        let addr = studio_set_part(0, [0x00, 0x00, 0x09]);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x20, 0x09));
    }

    #[test]
    fn part_16_level_address() {
        // Part 16 (index 15) Level: 18 00 00 00 + 00 00 2F 00 + 00 00 00 09 = 18 00 2F 09
        let addr = studio_set_part(15, [0x00, 0x00, 0x09]);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x2F, 0x09));
    }

    #[test]
    fn studio_set_name_address() {
        // Studio Set name starts at 18 00 00 00
        let addr = studio_set_common([0x00, 0x00]);
        assert_eq!(addr, Address::new(0x18, 0x00, 0x00, 0x00));
    }

    #[test]
    fn display_format() {
        let addr = Address::new(0x18, 0x00, 0x20, 0x09);
        assert_eq!(format!("{addr}"), "18 00 20 09");
    }
}
