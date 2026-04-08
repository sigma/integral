//! Undocumented catalog queries for reading preset/user names.
//!
//! The INTEGRA-7 supports a short-form RQ1 to address `0F 00 03 02` that
//! returns names for all entries in a bank (e.g., all 64 Studio Set names).
//!
//! Reference: `docs/midi/99-undocumented.md`

use crate::sysex;

/// Catalog query address (undocumented).
const CATALOG_ADDR: [u8; 4] = [0x0F, 0x00, 0x03, 0x02];

/// Studio Set bank: MSB = 85 (0x55), LSB = 0.
pub const STUDIO_SET_MSB: u8 = 0x55;
pub const STUDIO_SET_LSB: u8 = 0x00;

/// Build a catalog query for Studio Set names.
///
/// This undocumented command uses a non-standard format with NO checksum.
/// The last byte before F7 is the starting program number (0-indexed).
///
/// ```text
/// F0 41 <dev> 00 00 64 11 0F 00 03 02 55 00 <start> F7
/// ```
pub fn build_studio_set_catalog_request(device_id: u8) -> Vec<u8> {
    vec![
        0xF0,
        sysex::ROLAND_ID,
        device_id,
        sysex::MODEL_ID[0],
        sysex::MODEL_ID[1],
        sysex::MODEL_ID[2],
        sysex::CMD_RQ1,
        CATALOG_ADDR[0],
        CATALOG_ADDR[1],
        CATALOG_ADDR[2],
        CATALOG_ADDR[3],
        STUDIO_SET_MSB,
        STUDIO_SET_LSB,
        0x00, // start from index 0
        0xF7,
    ]
}

/// A parsed catalog entry from a DT1 response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// Bank Select MSB.
    pub bank_msb: u8,
    /// Bank Select LSB.
    pub bank_lsb: u8,
    /// Program number (0-indexed).
    pub pc: u8,
    /// Entry name (up to 16 ASCII characters, trimmed).
    pub name: String,
}

/// Parse a DT1 response as a catalog entry.
///
/// Expected data layout (21 bytes):
/// `[MSB, LSB, PC, 00, 00, name[16]]`
///
/// Returns `None` for delimiter messages (all-zero data) or data that's too short.
pub fn parse_catalog_entry(data: &[u8]) -> Option<CatalogEntry> {
    if data.len() < 21 {
        return None;
    }
    // Skip delimiter messages (all zeros)
    if data.iter().all(|&b| b == 0) {
        return None;
    }

    let name_bytes = &data[5..21];
    let name = String::from_utf8_lossy(name_bytes).trim_end().to_string();

    Some(CatalogEntry {
        bank_msb: data[0],
        bank_lsb: data[1],
        pc: data[2],
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_catalog_request() {
        let msg = build_studio_set_catalog_request(0x10);
        // No checksum — last byte before F7 is start index (0x00)
        assert_eq!(
            msg,
            vec![
                0xF0, 0x41, 0x10, 0x00, 0x00, 0x64, 0x11, 0x0F, 0x00, 0x03, 0x02, 0x55, 0x00,
                0x00, 0xF7
            ]
        );
    }

    #[test]
    fn parse_integra_preview() {
        let data = [
            0x55, 0x00, 0x00, 0x00, 0x00, // MSB, LSB, PC, pad, pad
            0x49, 0x6E, 0x74, 0x65, 0x67, 0x72, 0x61, 0x20, // "Integra "
            0x50, 0x72, 0x65, 0x76, 0x69, 0x65, 0x77, 0x20, // "Preview "
        ];
        let entry = parse_catalog_entry(&data).unwrap();
        assert_eq!(entry.bank_msb, 0x55);
        assert_eq!(entry.bank_lsb, 0x00);
        assert_eq!(entry.pc, 0x00);
        assert_eq!(entry.name, "Integra Preview");
    }

    #[test]
    fn parse_delimiter_returns_none() {
        let data = [0u8; 21];
        assert_eq!(parse_catalog_entry(&data), None);
    }

    #[test]
    fn parse_too_short_returns_none() {
        let data = [0x55, 0x00, 0x00];
        assert_eq!(parse_catalog_entry(&data), None);
    }
}
