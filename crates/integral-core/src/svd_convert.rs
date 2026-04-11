//! SVD ↔ SysEx conversion functions.
//!
//! Converts between SVD bit-packed entries and SysEx parameter byte arrays.
//! Each direction uses the [`SvdToneSpec`] tables to know the bit widths.
//!
//! Reference: `docs/svd/02-encoding.md`

use crate::bitstream::{BitReader, BitReaderError, BitWriter};
use crate::svd_specs::{ParamBits, SvdSection, SvdToneSpec};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors during SVD ↔ SysEx conversion.
#[derive(Debug, Error)]
pub enum ConvertError {
    /// The SVD entry is too short for the spec.
    #[error("SVD entry too short: expected {expected} bytes, got {actual}")]
    EntryTooShort {
        /// Expected entry size from the spec.
        expected: usize,
        /// Actual entry size.
        actual: usize,
    },

    /// Bit reading failed.
    #[error("bitstream read error: {0}")]
    BitRead(#[from] BitReaderError),
}

// ---------------------------------------------------------------------------
// SVD → SysEx
// ---------------------------------------------------------------------------

/// Unpack a single section from the bitstream into SysEx parameter bytes.
///
/// Returns one byte per SysEx parameter address. Nibblized parameters expand
/// to multiple bytes (one per nibble).
fn unpack_section(
    reader: &mut BitReader<'_>,
    section: &SvdSection,
) -> Result<Vec<u8>, ConvertError> {
    let mut sysex = Vec::with_capacity(section.sysex_size());

    for param in section.params {
        if param.sysex_bytes == 1 {
            // Normal parameter: read `bits` significant bits.
            let value = reader.read_bits(param.bits)?;
            sysex.push(value as u8);
        } else {
            // Nibblized parameter: each SysEx byte carries one 4-bit nibble.
            let nibble_count = param.sysex_bytes;
            let combined = reader.read_nibbles(nibble_count)?;
            // Unpack into individual SysEx bytes, MSB nibble first.
            for i in (0..nibble_count).rev() {
                sysex.push(((combined >> (i * 4)) & 0x0F) as u8);
            }
        }
    }

    reader.align_to_byte();
    Ok(sysex)
}

/// Unpack an SVD entry into per-section SysEx byte vectors.
///
/// Returns one `Vec<u8>` per section in the spec. Each vector contains one
/// byte per SysEx parameter address (nibblized params expanded to multiple
/// bytes).
///
/// # Example
///
/// ```no_run
/// use integral_core::svd_convert::svd_to_sysex;
/// use integral_core::svd_specs::SNS_TONE_SPEC;
///
/// let entry: Vec<u8> = vec![0; 280]; // placeholder
/// let sections = svd_to_sysex(&entry, &SNS_TONE_SPEC).unwrap();
/// // sections[0] = Common + MFX SysEx bytes
/// // sections[1] = Partial 1
/// // sections[2] = Partial 2
/// // sections[3] = Partial 3
/// ```
pub fn svd_to_sysex(entry: &[u8], spec: &SvdToneSpec) -> Result<Vec<Vec<u8>>, ConvertError> {
    if entry.len() < spec.entry_size {
        return Err(ConvertError::EntryTooShort {
            expected: spec.entry_size,
            actual: entry.len(),
        });
    }

    let mut sections = Vec::with_capacity(spec.sections.len());
    let mut offset = 0;

    for section in spec.sections {
        let section_slice = &entry[offset..offset + section.svd_bytes];
        let mut reader = BitReader::new(section_slice);
        sections.push(unpack_section(&mut reader, section)?);
        offset += section.svd_bytes;
    }

    Ok(sections)
}

// ---------------------------------------------------------------------------
// SysEx → SVD
// ---------------------------------------------------------------------------

/// Pack a single section's SysEx bytes into the bitstream.
fn pack_section(writer: &mut BitWriter, sysex: &[u8], section: &SvdSection) {
    let mut idx = 0;
    for param in section.params {
        if param.sysex_bytes == 1 {
            writer.write_bits(sysex[idx] as u32, param.bits);
            idx += 1;
        } else {
            // Nibblized: recombine individual SysEx nibble bytes.
            let mut combined: u32 = 0;
            for _ in 0..param.sysex_bytes {
                combined = (combined << 4) | (sysex[idx] as u32 & 0x0F);
                idx += 1;
            }
            writer.write_nibbles(combined, param.sysex_bytes);
        }
    }
    writer.align_to_byte();
}

/// Pack SysEx byte sections into an SVD entry.
///
/// Each element of `sections` must contain the correct number of SysEx bytes
/// for the corresponding spec section. The returned `Vec<u8>` is padded to
/// `spec.entry_size` with a `0x0E` end marker and zero fill.
pub fn sysex_to_svd(sections: &[Vec<u8>], spec: &SvdToneSpec) -> Vec<u8> {
    let mut result = Vec::with_capacity(spec.entry_size);

    for (section_data, section_spec) in sections.iter().zip(spec.sections.iter()) {
        let mut writer = BitWriter::new();
        pack_section(&mut writer, section_data, section_spec);
        let mut section_bytes = writer.into_bytes();
        // Pad section to its known SVD size.
        section_bytes.resize(section_spec.svd_bytes, 0x00);
        result.extend_from_slice(&section_bytes);
    }

    // Append end marker.
    result.push(0x0E);

    // Pad to entry size.
    result.resize(spec.entry_size, 0x00);

    result
}

// ---------------------------------------------------------------------------
// Helpers for splitting combined sections
// ---------------------------------------------------------------------------

/// Split a combined Common + MFX section into separate SysEx byte vectors.
///
/// The SN-S SVD format packs Common and MFX into a single section, but
/// SysEx addresses them separately. This helper splits the combined bytes
/// using the provided param counts.
pub fn split_common_mfx(
    combined: &[u8],
    common_params: &[ParamBits],
    mfx_params: &[ParamBits],
) -> (Vec<u8>, Vec<u8>) {
    let common_sysex_len: usize = common_params.iter().map(|p| p.sysex_bytes as usize).sum();
    let mfx_sysex_len: usize = mfx_params.iter().map(|p| p.sysex_bytes as usize).sum();

    let common = combined[..common_sysex_len].to_vec();
    let mfx = combined[common_sysex_len..common_sysex_len + mfx_sysex_len].to_vec();
    (common, mfx)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svd::SvdFile;
    use crate::svd_specs::{MFX_PARAMS, SNS_TONE_SPEC};

    /// Helper: load Synth Legends SVD if available.
    fn load_synth_legends() -> Option<SvdFile> {
        let path = "/Users/yann/Downloads/INTEGRA-7_Synth_Legends/ROLAND/SOUND/I7SL.SVD";
        let data = std::fs::read(path).ok()?;
        SvdFile::parse(&data).ok()
    }

    #[test]
    fn round_trip_zeros() {
        // An entry of all zeros should round-trip cleanly.
        let entry = vec![0u8; 280];
        let sections = svd_to_sysex(&entry, &SNS_TONE_SPEC).unwrap();
        let repacked = sysex_to_svd(&sections, &SNS_TONE_SPEC);
        // The repacked entry should match the original data portion,
        // end marker at the correct position, and zero padding.
        assert_eq!(repacked.len(), 280);
        assert_eq!(repacked[246], 0x0E);
        // Data bytes should match.
        assert_eq!(&repacked[..246], &entry[..246]);
    }

    #[test]
    fn section_sysex_sizes() {
        let sections = svd_to_sysex(&[0u8; 280], &SNS_TONE_SPEC).unwrap();
        // Section 0: Common (64) + MFX (145) = 209
        assert_eq!(sections[0].len(), 64 + 145);
        // Sections 1-3: Partial (61 each)
        assert_eq!(sections[1].len(), 61);
        assert_eq!(sections[2].len(), 61);
        assert_eq!(sections[3].len(), 61);
    }

    #[test]
    fn decode_tone_name_from_synth_legends() {
        let svd = match load_synth_legends() {
            Some(s) => s,
            None => {
                eprintln!("Synth Legends SVD not found, skipping");
                return;
            }
        };

        let sns_chunk = svd
            .chunks
            .iter()
            .find(|c| c.chunk_type == crate::svd::ChunkType::SnSynthTone)
            .unwrap();

        // Decode entry 0.
        let sections = svd_to_sysex(&sns_chunk.entries[0], &SNS_TONE_SPEC).unwrap();

        // Extract tone name from first 12 bytes of the Common+MFX section.
        let name: String = sections[0][..12]
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
            .to_string();

        // First patch in Synth Legends should start with "SL-".
        assert!(
            name.starts_with("SL-"),
            "Expected name starting with 'SL-', got '{name}'"
        );
    }

    #[test]
    fn round_trip_real_entry() {
        let svd = match load_synth_legends() {
            Some(s) => s,
            None => {
                eprintln!("Synth Legends SVD not found, skipping");
                return;
            }
        };

        let sns_chunk = svd
            .chunks
            .iter()
            .find(|c| c.chunk_type == crate::svd::ChunkType::SnSynthTone)
            .unwrap();

        // Round-trip every entry.
        for (i, entry) in sns_chunk.entries.iter().enumerate() {
            let sections = svd_to_sysex(entry, &SNS_TONE_SPEC).unwrap();
            let repacked = sysex_to_svd(&sections, &SNS_TONE_SPEC);

            // Compare data portion (before end marker).
            let data_end = SNS_TONE_SPEC.data_bytes();
            assert_eq!(
                &repacked[..data_end],
                &entry[..data_end],
                "Entry {i} data mismatch"
            );
        }
    }

    #[test]
    fn split_common_mfx_sizes() {
        let combined = vec![0u8; 209]; // 64 + 145
        let (common, mfx) = split_common_mfx(
            &combined,
            crate::svd_specs::SNS_COMMON_SECTION.params,
            MFX_PARAMS,
        );
        assert_eq!(common.len(), 64);
        assert_eq!(mfx.len(), 145);
    }

    #[test]
    fn entry_too_short_error() {
        let short = vec![0u8; 100];
        let result = svd_to_sysex(&short, &SNS_TONE_SPEC);
        assert!(result.is_err());
    }
}
