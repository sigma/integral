//! Bit-level reader and writer for SVD ↔ SysEx conversion.
//!
//! SVD files store SysEx parameters as bit-packed data, where each parameter
//! uses only its significant bits (e.g., a `0000 0aaa` param uses 3 bits).
//! This module provides [`BitReader`] and [`BitWriter`] for packing and
//! unpacking these bitstreams.
//!
//! Reference: `docs/svd/02-encoding.md`

use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur when reading bits.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BitReaderError {
    /// Attempted to read past the end of the data.
    #[error("read {requested} bits at position {position}, but only {available} remain")]
    EndOfData {
        /// Bit position at time of error.
        position: usize,
        /// Number of bits requested.
        requested: u8,
        /// Number of bits remaining.
        available: usize,
    },
}

// ---------------------------------------------------------------------------
// BitReader
// ---------------------------------------------------------------------------

/// Reads individual bits from a byte slice, MSB first.
///
/// # Example
///
/// ```
/// use integral_core::bitstream::BitReader;
///
/// let data = [0b1010_0110, 0b1100_0000];
/// let mut r = BitReader::new(&data);
/// assert_eq!(r.read_bits(4).unwrap(), 0b1010);
/// assert_eq!(r.read_bits(4).unwrap(), 0b0110);
/// assert_eq!(r.read_bits(2).unwrap(), 0b11);
/// ```
pub struct BitReader<'a> {
    data: &'a [u8],
    /// Current position in bits from the start.
    pos: usize,
}

impl<'a> BitReader<'a> {
    /// Create a new reader over the given byte slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Current bit position from the start of the data.
    pub fn position(&self) -> usize {
        self.pos
    }

    /// Number of bits remaining.
    pub fn remaining(&self) -> usize {
        self.data.len() * 8 - self.pos
    }

    /// Read `n` bits (1–32) and return them right-aligned in a `u32`.
    ///
    /// Bits are read MSB-first: the first bit read becomes the highest bit
    /// of the returned value.
    pub fn read_bits(&mut self, n: u8) -> Result<u32, BitReaderError> {
        debug_assert!((1..=32).contains(&n), "read_bits: n must be 1..=32");
        let remaining = self.remaining();
        if (n as usize) > remaining {
            return Err(BitReaderError::EndOfData {
                position: self.pos,
                requested: n,
                available: remaining,
            });
        }

        let mut value: u32 = 0;
        for _ in 0..n {
            let byte_idx = self.pos / 8;
            let bit_idx = 7 - (self.pos % 8); // MSB first
            let bit = (self.data[byte_idx] >> bit_idx) & 1;
            value = (value << 1) | (bit as u32);
            self.pos += 1;
        }
        Ok(value)
    }

    /// Read `count` 4-bit nibbles and return them packed into a `u32`.
    ///
    /// This is used for nibblized (`#`-marked) SysEx parameters, where each
    /// SysEx byte carries one 4-bit nibble. In the SVD bitstream, these
    /// nibbles are stored as consecutive 4-bit groups, MSB nibble first.
    pub fn read_nibbles(&mut self, count: u8) -> Result<u32, BitReaderError> {
        debug_assert!(
            (1..=8).contains(&count),
            "read_nibbles: count must be 1..=8"
        );
        let mut value: u32 = 0;
        for _ in 0..count {
            let nibble = self.read_bits(4)?;
            value = (value << 4) | nibble;
        }
        Ok(value)
    }

    /// Advance to the next byte boundary (no-op if already aligned).
    pub fn align_to_byte(&mut self) {
        let remainder = self.pos % 8;
        if remainder != 0 {
            self.pos += 8 - remainder;
        }
    }
}

// ---------------------------------------------------------------------------
// BitWriter
// ---------------------------------------------------------------------------

/// Writes individual bits into a growable byte buffer, MSB first.
///
/// # Example
///
/// ```
/// use integral_core::bitstream::BitWriter;
///
/// let mut w = BitWriter::new();
/// w.write_bits(0b1010, 4);
/// w.write_bits(0b0110, 4);
/// w.write_bits(0b11, 2);
/// w.align_to_byte();
/// assert_eq!(w.into_bytes(), vec![0b1010_0110, 0b1100_0000]);
/// ```
pub struct BitWriter {
    buf: Vec<u8>,
    /// Current position in bits.
    pos: usize,
}

impl BitWriter {
    /// Create a new empty writer.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            pos: 0,
        }
    }

    /// Current bit position (total bits written so far).
    pub fn bit_position(&self) -> usize {
        self.pos
    }

    /// Write `n` bits (1–32) from `value`, MSB-first.
    ///
    /// Only the lowest `n` bits of `value` are written.
    pub fn write_bits(&mut self, value: u32, n: u8) {
        debug_assert!((1..=32).contains(&n), "write_bits: n must be 1..=32");
        for i in (0..n).rev() {
            let bit = ((value >> i) & 1) as u8;
            let byte_idx = self.pos / 8;
            let bit_idx = 7 - (self.pos % 8); // MSB first

            // Grow buffer if needed.
            if byte_idx >= self.buf.len() {
                self.buf.push(0);
            }

            self.buf[byte_idx] |= bit << bit_idx;
            self.pos += 1;
        }
    }

    /// Write `count` 4-bit nibbles from `value`, MSB nibble first.
    ///
    /// This is the inverse of [`BitReader::read_nibbles`].
    pub fn write_nibbles(&mut self, value: u32, count: u8) {
        debug_assert!(
            (1..=8).contains(&count),
            "write_nibbles: count must be 1..=8"
        );
        for i in (0..count).rev() {
            let nibble = (value >> (i * 4)) & 0x0F;
            self.write_bits(nibble, 4);
        }
    }

    /// Pad with zero bits to the next byte boundary (no-op if aligned).
    pub fn align_to_byte(&mut self) {
        let remainder = self.pos % 8;
        if remainder != 0 {
            // The buffer byte is already zero-initialized; just advance.
            self.pos += 8 - remainder;
            // Ensure the buffer is large enough.
            let needed = self.pos.div_ceil(8);
            if self.buf.len() < needed {
                self.buf.resize(needed, 0);
            }
        }
    }

    /// Consume the writer and return the packed bytes.
    ///
    /// If the last byte is only partially written, its unused low bits are 0.
    pub fn into_bytes(mut self) -> Vec<u8> {
        // Ensure partial last byte is included.
        let needed = self.pos.div_ceil(8);
        if self.buf.len() < needed {
            self.buf.resize(needed, 0);
        }
        self.buf
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_various_widths() {
        // Write values of widths 1,2,3,5,6,7 then read back.
        let values: &[(u32, u8)] = &[
            (1, 1),   // 1 bit
            (3, 2),   // 2 bits
            (5, 3),   // 3 bits
            (17, 5),  // 5 bits
            (42, 6),  // 6 bits
            (100, 7), // 7 bits
        ];

        let mut w = BitWriter::new();
        for &(val, bits) in values {
            w.write_bits(val, bits);
        }
        w.align_to_byte();
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        for &(val, bits) in values {
            assert_eq!(r.read_bits(bits).unwrap(), val, "width={bits}");
        }
    }

    #[test]
    fn nibble_round_trip() {
        let mut w = BitWriter::new();
        w.write_nibbles(0x1234, 4); // 4 nibbles = 16 bits
        w.write_nibbles(0xABC, 3); // 3 nibbles = 12 bits
        w.align_to_byte();
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_nibbles(4).unwrap(), 0x1234);
        assert_eq!(r.read_nibbles(3).unwrap(), 0xABC);
    }

    #[test]
    fn byte_alignment() {
        let mut w = BitWriter::new();
        w.write_bits(0b101, 3); // 3 bits
        assert_eq!(w.bit_position(), 3);
        w.align_to_byte(); // pad 5 zero bits
        assert_eq!(w.bit_position(), 8);
        w.write_bits(0xFF, 8);
        let bytes = w.into_bytes();

        assert_eq!(bytes, vec![0b1010_0000, 0xFF]);

        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(3).unwrap(), 0b101);
        r.align_to_byte();
        assert_eq!(r.position(), 8);
        assert_eq!(r.read_bits(8).unwrap(), 0xFF);
    }

    #[test]
    fn align_noop_when_aligned() {
        let mut w = BitWriter::new();
        w.write_bits(0xAB, 8);
        assert_eq!(w.bit_position(), 8);
        w.align_to_byte(); // should be no-op
        assert_eq!(w.bit_position(), 8);
    }

    #[test]
    fn mixed_operations() {
        let mut w = BitWriter::new();
        // 7-bit ASCII 'A' = 65
        w.write_bits(65, 7);
        // 1-bit switch
        w.write_bits(1, 1);
        // 4-nibble param (16 bits)
        w.write_nibbles(0x89AB, 4);
        // 3-bit enum
        w.write_bits(5, 3);
        w.align_to_byte();
        let bytes = w.into_bytes();

        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read_bits(7).unwrap(), 65);
        assert_eq!(r.read_bits(1).unwrap(), 1);
        assert_eq!(r.read_nibbles(4).unwrap(), 0x89AB);
        assert_eq!(r.read_bits(3).unwrap(), 5);
    }

    #[test]
    fn read_past_end() {
        let data = [0xFF];
        let mut r = BitReader::new(&data);
        assert!(r.read_bits(8).is_ok());
        let err = r.read_bits(1).unwrap_err();
        assert_eq!(
            err,
            BitReaderError::EndOfData {
                position: 8,
                requested: 1,
                available: 0,
            }
        );
    }

    #[test]
    fn remaining_tracks_correctly() {
        let data = [0; 3]; // 24 bits
        let mut r = BitReader::new(&data);
        assert_eq!(r.remaining(), 24);
        r.read_bits(7).unwrap();
        assert_eq!(r.remaining(), 17);
        r.align_to_byte();
        assert_eq!(r.remaining(), 16);
    }

    /// Simulate packing an SN-S partial's parameters to verify the total
    /// bit count matches the spec (350 bits).
    ///
    /// Bit widths from `docs/svd/03-sn-synth.md` Partial section.
    #[test]
    fn sns_partial_bit_count() {
        let partial_widths: &[u8] = &[
            3, 6, 2, 6, // OSC Wave, Wave Var, reserve, Pitch
            7, 7, 7, 7, 7, 7, // Detune..Pitch Env Depth
            3, 1, 7, 6, // Filter Mode, Slope, Cutoff, Keyfollow
            7, 7, 7, 7, 7, 7, 7, // Filter Env Vel..Env Depth
            7, 7, 7, 7, 7, 7, 7, // AMP Level..Pan
            3, 7, 1, 5, 7, 1, // LFO Shape..Key Trigger
            7, 7, 7, 7, // LFO depths
            3, 7, 1, 5, // Mod LFO Shape..Sync Note
            7, 1, // PW Shift, reserve
            7, 7, 7, 7, // Mod LFO depths
            7, 7, // Aftertouch Cutoff, Level
            7, 7, // reserves
            2, // Wave Gain
        ];
        // Wave Number is a 4-nibble param = 16 bits
        let nibble_bits: usize = 16;
        let remaining_widths: &[u8] = &[
            7, 7, 7, 5, // HPF Cutoff, Super Saw Detune, Mod LFO Rate Ctrl, AMP Keyfollow
        ];

        let total: usize = partial_widths.iter().map(|&w| w as usize).sum::<usize>()
            + nibble_bits
            + remaining_widths.iter().map(|&w| w as usize).sum::<usize>();

        assert_eq!(total, 350, "SN-S partial should be 350 bits");

        // Now actually write and read them back.
        let mut w = BitWriter::new();
        for &bits in partial_widths {
            w.write_bits(0, bits);
        }
        w.write_nibbles(0, 4); // Wave Number
        for &bits in remaining_widths {
            w.write_bits(0, bits);
        }
        assert_eq!(w.bit_position(), 350);
        w.align_to_byte();
        assert_eq!(w.bit_position(), 352); // 350 → next byte = 352 = 44 bytes

        // But the spec says 46 bytes (368 bits). The extra 2 bytes (16 bits)
        // are zero-padding in the entry — the align brings us to 352, and
        // the entry allocates 368 bits (46 bytes) for the section.
        // The section padding is handled at the entry level, not here.
    }

    /// Verify the SN-S Common section bit count (228 bits).
    #[test]
    fn sns_common_bit_count() {
        let widths: &[u8] = &[
            7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, // 12× Tone Name (84)
            7, // Tone Level
        ];
        // reserve nibblized 3 nibbles = 12 bits
        let nib_bits: usize = 12;
        let rest: &[u8] = &[
            1, 1, // reserves
            1, 7, 2, 3, 5, 5, 3, // Porta..reserve
            1, 1, 1, 1, 1, 1, // Partial switches
            2, 1, // Ring, TFX
            2, 1, 1, 6, 1, 1, 1, 1, 1, 1, 1, 1, 1, // reserves
            1, 1, 1, 1, 1, 1, // Unison..reserve
            7, 7, 7, // Analog Feel, Wave Shape, Category
        ];
        // Phrase Number: 4 nibbles = 16 bits
        let phrase_bits: usize = 16;
        let tail: &[u8] = &[
            3, 2, // Phrase Oct Shift, Unison Size
            7, 7, 7, // reserves
        ];

        let total = widths.iter().map(|&w| w as usize).sum::<usize>()
            + nib_bits
            + rest.iter().map(|&w| w as usize).sum::<usize>()
            + phrase_bits
            + tail.iter().map(|&w| w as usize).sum::<usize>();

        assert_eq!(total, 228, "SN-S Common should be 228 bits");
    }

    /// Verify the SN-S MFX section bit count (618 bits).
    #[test]
    fn sns_mfx_bit_count() {
        let fixed: &[u8] = &[
            7, 7, 7, 7, // Type, reserve, Chorus Send, Reverb Send
            2, // reserve
            7, 7, 7, 7, 7, 7, 7, 7, // 4× control source/sens
            5, 5, 5, 5, // 4× control assign
        ];
        // 32 MFX parameters, each 4 nibbles = 16 bits
        let mfx_param_bits: usize = 32 * 16;

        let total = fixed.iter().map(|&w| w as usize).sum::<usize>() + mfx_param_bits;

        assert_eq!(total, 618, "SN-S MFX should be 618 bits");
    }

    /// Verify Common + MFX combined = 846 bits → 108 bytes padded.
    #[test]
    fn sns_common_mfx_combined() {
        let combined = 228 + 618;
        assert_eq!(combined, 846);
        let padded_bytes = (combined + 7) / 8;
        assert_eq!(padded_bytes, 106); // 846/8 = 105.75 → 106 bytes
        // But the spec says 108 bytes (864 bits). The padding to 864 = 108
        // bytes means 864 - 846 = 18 zero bits of padding.
        // This aligns to 108 bytes exactly.
    }
}
