//! Typed parameter fields of a tone parameter block.
//!
//! Each tone block (e.g. PCM Synth Tone Partial) has a field enum whose
//! discriminant is the field's byte position within the block, so the
//! position is defined once and shared by the parser, the setters and the
//! UI. [`BlockField`] adds what the discriminant alone can't say: how many
//! values a field holds and how wide each one is.

use core::fmt::Debug;
use thiserror::Error;

/// A parameter of a tone block, at a byte position within the block.
///
/// A field holds one value, or [`count`](Self::count) values
/// [`stride`](Self::stride) bytes apart (an envelope's times, one entry
/// per Partial). Each value takes [`width`](Self::width) bytes: one, or one
/// per nibble when it is nibblized.
pub trait BlockField: Copy + Debug {
    /// Byte position of the field's first value within its block.
    fn position(self) -> usize;

    /// Number of values the field holds.
    fn count(self) -> usize {
        1
    }

    /// Bytes each value takes: 1, or its nibble count when nibblized.
    fn width(self) -> usize {
        1
    }

    /// Byte distance from one value of the field to the next.
    fn stride(self) -> usize {
        self.width()
    }

    /// Whether each value is nibblized across [`width`](Self::width) bytes.
    fn is_nibblized(self) -> bool {
        self.width() > 1
    }

    /// Byte position of value `index` (0-based) within the block.
    ///
    /// # Errors
    /// `index` is not below [`count`](Self::count).
    fn element_position(self, index: u8) -> Result<u8, FieldIndexError<Self>> {
        let index_usize = usize::from(index);
        if index_usize >= self.count() {
            return Err(FieldIndexError { field: self, index });
        }
        let position = self.position() + index_usize * self.stride();
        u8::try_from(position).map_err(|_| FieldIndexError { field: self, index })
    }

    /// The single-byte value of the field in a block dump.
    fn byte(self, data: &[u8]) -> u8 {
        data[self.position()]
    }

    /// Value `index` (0-based) of a single-byte field in a block dump.
    fn element(self, data: &[u8], index: usize) -> u8 {
        data[self.position() + index * self.stride()]
    }

    /// All [`count`](Self::count) values of a single-byte field in a block
    /// dump.
    fn elements<const N: usize>(self, data: &[u8]) -> [u8; N] {
        debug_assert_eq!(N, self.count(), "{self:?} holds {} values", self.count());
        core::array::from_fn(|i| self.element(data, i))
    }

    /// The nibblized value of the field in a block dump.
    fn nibbles(self, data: &[u8]) -> u16 {
        data[self.position()..self.position() + self.width()]
            .iter()
            .fold(0, |acc, &b| (acc << 4) | u16::from(b & 0x0F))
    }

    /// Write the single-byte value of the field into a block dump: the
    /// inverse of [`byte`](Self::byte).
    fn set_byte(self, data: &mut [u8], value: u8) {
        data[self.position()] = value;
    }

    /// Write value `index` (0-based) of a single-byte field into a block
    /// dump: the inverse of [`element`](Self::element).
    fn set_element(self, data: &mut [u8], index: usize, value: u8) {
        data[self.position() + index * self.stride()] = value;
    }

    /// Write all [`count`](Self::count) values of a single-byte field into
    /// a block dump: the inverse of [`elements`](Self::elements).
    fn set_elements(self, data: &mut [u8], values: &[u8]) {
        debug_assert_eq!(
            values.len(),
            self.count(),
            "{self:?} holds {} values",
            self.count()
        );
        for (i, &value) in values.iter().enumerate() {
            self.set_element(data, i, value);
        }
    }

    /// Write the nibblized value of the field into a block dump: the
    /// inverse of [`nibbles`](Self::nibbles).
    fn set_nibbles(self, data: &mut [u8], value: u16) {
        let start = self.position();
        data[start..start + self.width()].copy_from_slice(&self.encode(value));
    }

    /// The bytes that set one value of the field to `value`: the value
    /// itself, or its nibbles (most significant first) when nibblized.
    fn encode(self, value: u16) -> Vec<u8> {
        if !self.is_nibblized() {
            return vec![value as u8];
        }
        (0..self.width())
            .rev()
            .map(|i| ((value >> (4 * i)) & 0x0F) as u8)
            .collect()
    }
}

/// Error for a value index beyond what a field holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{field:?} has no element {index}")]
pub struct FieldIndexError<F: Debug> {
    /// The field addressed.
    pub field: F,
    /// The out-of-range value index.
    pub index: u8,
}

/// Helpers for checking that a parser reads each field at its position.
#[cfg(test)]
pub(crate) mod test_support {
    use super::BlockField;

    /// A block dump whose byte at each position is the position itself, so
    /// that every field reads a value of its own.
    pub(crate) fn positional_block(len: usize) -> Vec<u8> {
        (0..len).map(|i| i as u8).collect()
    }

    /// The values `field` holds in a [`positional_block`], worked out from
    /// its position, count, stride and width alone.
    pub(crate) fn positional_values<F: BlockField>(field: F) -> Vec<u16> {
        (0..field.count())
            .map(|i| {
                let start = field.position() + i * field.stride();
                if field.width() == 1 {
                    start as u16
                } else {
                    (start..start + field.width()).fold(0, |acc, b| (acc << 4) | (b as u16 & 0x0F))
                }
            })
            .collect()
    }

    /// A per-entry field and the getter reading its value from one entry
    /// (e.g. a PMT entry, one per Partial).
    pub(crate) type EntryField<F, E> = (F, fn(&E) -> u8);

    /// Assert that each of `fields`, read from every entry by its getter,
    /// holds the field's [`positional_values`] (one per entry).
    pub(crate) fn assert_entries_read_at_positions<E, F: BlockField>(
        entries: &[E],
        fields: &[EntryField<F, E>],
    ) {
        for &(field, get) in fields {
            let read: Vec<u16> = entries.iter().map(|e| u16::from(get(e))).collect();
            assert_eq!(read, positional_values(field), "{field:?}");
        }
    }

    /// A parsed field's value(s), for comparison with [`positional_values`].
    pub(crate) trait Values {
        /// The value(s), widened.
        fn values(&self) -> Vec<u16>;
    }

    impl Values for u8 {
        fn values(&self) -> Vec<u16> {
            vec![u16::from(*self)]
        }
    }

    impl Values for u16 {
        fn values(&self) -> Vec<u16> {
            vec![*self]
        }
    }

    impl Values for bool {
        fn values(&self) -> Vec<u16> {
            vec![u16::from(*self)]
        }
    }

    impl<const N: usize> Values for [u8; N] {
        fn values(&self) -> Vec<u16> {
            self.iter().map(|&v| u16::from(v)).collect()
        }
    }

    /// Assert that each parsed struct field holds the [`positional_values`]
    /// of its field enum variant.
    macro_rules! assert_reads_at_positions {
        ($parsed:expr, { $($field:ident: $variant:expr),+ $(,)? }) => {
            $(
                assert_eq!(
                    $crate::block_field::test_support::Values::values(&$parsed.$field),
                    $crate::block_field::test_support::positional_values($variant),
                    "{} vs {:?}",
                    stringify!($field),
                    $variant,
                );
            )+
        };
    }
    pub(crate) use assert_reads_at_positions;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum F {
        Byte,
        Steps,
        Entry,
        Nib2,
        Nib4,
    }

    impl BlockField for F {
        fn position(self) -> usize {
            match self {
                Self::Byte => 0,
                Self::Steps => 1,
                Self::Entry => 4,
                Self::Nib2 => 10,
                Self::Nib4 => 12,
            }
        }
        fn count(self) -> usize {
            match self {
                Self::Steps => 3,
                Self::Entry => 2,
                _ => 1,
            }
        }
        fn width(self) -> usize {
            match self {
                Self::Nib2 => 2,
                Self::Nib4 => 4,
                _ => 1,
            }
        }
        fn stride(self) -> usize {
            match self {
                Self::Entry => 3,
                _ => self.width(),
            }
        }
    }

    #[test]
    fn element_positions_follow_count_and_stride() {
        assert_eq!(F::Byte.element_position(0), Ok(0));
        assert_eq!(F::Steps.element_position(2), Ok(3));
        assert_eq!(F::Entry.element_position(1), Ok(7));
        assert_eq!(
            F::Steps.element_position(3),
            Err(FieldIndexError {
                field: F::Steps,
                index: 3
            })
        );
    }

    #[test]
    fn reads_bytes_elements_and_nibbles() {
        let data = [
            9, 1, 2, 3, 4, 0, 0, 5, 0, 0, 0x0A, 0x0B, 0x01, 0x02, 0x03, 0x04,
        ];
        assert_eq!(F::Byte.byte(&data), 9);
        assert_eq!(F::Steps.elements::<3>(&data), [1, 2, 3]);
        assert_eq!(F::Entry.elements::<2>(&data), [4, 5]);
        assert_eq!(F::Nib2.nibbles(&data), 0xAB);
        assert_eq!(F::Nib4.nibbles(&data), 0x1234);
    }

    #[test]
    fn encodes_bytes_and_nibbles() {
        assert_eq!(F::Byte.encode(100), [100]);
        assert_eq!(F::Nib2.encode(0x95), [0x09, 0x05]);
        assert_eq!(F::Nib4.encode(0x1234), [0x01, 0x02, 0x03, 0x04]);
    }
}
