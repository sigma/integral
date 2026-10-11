//! Shared helpers for tone data parsing.
//!
//! These functions consolidate patterns repeated across the tone parsing
//! modules (pcm_synth, pcm_drum, sn_synth, sn_acoustic, sn_drum).

/// Parse an ASCII name from a 12-byte region, trimming trailing spaces.
///
/// Non-printable bytes (outside 32–127) are replaced with spaces.
pub fn parse_ascii_name(data: &[u8]) -> String {
    parse_ascii_name_len(data, 12)
}

/// Parse an ASCII name from the first `len` bytes of `data`, as
/// [`parse_ascii_name`] does for 12.
pub fn parse_ascii_name_len(data: &[u8], len: usize) -> String {
    data[..len]
        .iter()
        .map(|&b| {
            if (32..=127).contains(&b) {
                b as char
            } else {
                ' '
            }
        })
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Encode `name` as a fixed-length ASCII name: truncated to `len` bytes and
/// space-padded. Characters outside 32–127 become `?`.
pub fn encode_ascii_name(name: &str, len: usize) -> Vec<u8> {
    name.chars()
        .map(|c| match u8::try_from(c) {
            Ok(b) if (32..=127).contains(&b) => b,
            _ => b'?',
        })
        .chain(std::iter::repeat(b' '))
        .take(len)
        .collect()
}

/// Decode a nibblized 2-byte value (each byte carries 4 bits).
pub fn nibble2(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16 & 0x0F) << 4) | (data[offset + 1] as u16 & 0x0F)
}

/// Decode a nibblized 4-byte value (each byte carries 4 bits).
pub fn nibble4(data: &[u8], offset: usize) -> u16 {
    ((data[offset] as u16 & 0x0F) << 12)
        | ((data[offset + 1] as u16 & 0x0F) << 8)
        | ((data[offset + 2] as u16 & 0x0F) << 4)
        | (data[offset + 3] as u16 & 0x0F)
}

/// Encode a value as 4 nibblized bytes, the inverse of [`nibble4`].
pub fn encode_nibble4(value: u16) -> [u8; 4] {
    [
        ((value >> 12) & 0x0F) as u8,
        ((value >> 8) & 0x0F) as u8,
        ((value >> 4) & 0x0F) as u8,
        (value & 0x0F) as u8,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_ascii_name_pads_truncates_and_replaces() {
        assert_eq!(encode_ascii_name("Pad", 5), b"Pad  ");
        assert_eq!(encode_ascii_name("Vintage 9 Long", 12), b"Vintage 9 Lo");
        assert_eq!(encode_ascii_name("Café", 6), b"Caf?  ");
        assert_eq!(encode_ascii_name("", 3), b"   ");
    }

    #[test]
    fn encode_then_parse_round_trips() {
        assert_eq!(
            parse_ascii_name(&encode_ascii_name("Vintage 9", 12)),
            "Vintage 9"
        );
    }
}
