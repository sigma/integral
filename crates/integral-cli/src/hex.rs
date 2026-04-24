//! Hex parsing utilities for CLI address/size/data arguments.

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
