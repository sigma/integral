//! Shared helpers for tone data parsing.
//!
//! These functions consolidate patterns repeated across the tone parsing
//! modules (pcm_synth, pcm_drum, sn_synth, sn_acoustic, sn_drum).

/// Parse an ASCII name from a 12-byte region, trimming trailing spaces.
///
/// Non-printable bytes (outside 32–127) are replaced with spaces.
pub fn parse_ascii_name(data: &[u8]) -> String {
    data[..12]
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
