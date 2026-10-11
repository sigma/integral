//! Hex parsing and formatting utilities for CLI arguments and output.

use anyhow::{Context, Result};
use integral_core::address::{Address, DataSize};

/// Parse a hex string into a 4-byte SysEx address.
///
/// # Examples
/// ```ignore
/// let addr = parse_hex_addr("10000000")?;
/// ```
pub fn parse_hex_addr(s: &str) -> Result<Address> {
    let bytes = parse_hex_u32(s).context("invalid hex address")?;
    Ok(Address::new(
        ((bytes >> 24) & 0xFF) as u8,
        ((bytes >> 16) & 0xFF) as u8,
        ((bytes >> 8) & 0xFF) as u8,
        (bytes & 0xFF) as u8,
    ))
}

/// Parse a hex string into a 4-byte SysEx data size.
///
/// # Examples
/// ```ignore
/// let size = parse_hex_size("00000010")?;
/// ```
pub fn parse_hex_size(s: &str) -> Result<DataSize> {
    let bytes = parse_hex_u32(s).context("invalid hex size")?;
    Ok(DataSize::new(
        ((bytes >> 24) & 0xFF) as u8,
        ((bytes >> 16) & 0xFF) as u8,
        ((bytes >> 8) & 0xFF) as u8,
        (bytes & 0xFF) as u8,
    ))
}

/// Parse a hex string into a vector of bytes.
///
/// The string must have an even number of hex characters.
pub fn parse_hex_bytes(s: &str) -> Result<Vec<u8>> {
    anyhow::ensure!(
        s.is_ascii() && s.len().is_multiple_of(2),
        "expected an even number of hex digits"
    );
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .with_context(|| format!("invalid hex at position {i}"))
        })
        .collect()
}

/// Parse a hex string into a u32 value.
fn parse_hex_u32(s: &str) -> Result<u32> {
    Ok(u32::from_str_radix(s, 16)?)
}

/// Format bytes as uppercase two-digit hex, joined by `sep`.
pub fn format_hex(bytes: &[u8], sep: &str) -> String {
    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
    hex.join(sep)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_hex_with_separator() {
        assert_eq!(format_hex(&[0xF0, 0x0A, 0x7F], " "), "F0 0A 7F");
        assert_eq!(format_hex(&[0x5F, 0x01], ", "), "5F, 01");
        assert_eq!(format_hex(&[], " "), "");
    }

    #[test]
    fn rejects_odd_or_non_ascii_hex() {
        assert_eq!(parse_hex_bytes("F07E").unwrap(), [0xF0, 0x7E]);
        assert!(parse_hex_bytes("F07").is_err());
        assert!(parse_hex_bytes("aéb").is_err());
    }
}
