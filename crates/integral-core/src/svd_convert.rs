//! SVD ↔ SysEx conversion functions.
//!
//! Converts between SVD bit-packed entries and SysEx parameter byte arrays.
//! Each direction uses the [`SvdToneSpec`] tables to know the bit widths.
//!
//! Reference: `docs/svd/02-encoding.md`

use crate::bitstream::{BitReader, BitReaderError, BitWriter};
use crate::svd::ChunkType;
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

    /// The spec's sections take more bytes than its entry holds.
    #[error("{kind} spec sections take {data} bytes, more than its {entry}-byte entry")]
    SpecOverflow {
        /// The chunk type of the spec.
        kind: ChunkType,
        /// Sum of the spec's section sizes.
        data: usize,
        /// Entry size of the spec.
        entry: usize,
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
        if param.sysex_bytes == 0 {
            // Padding: no SysEx byte.
            reader.read_bits(param.bits)?;
        } else if param.sysex_bytes == 1 {
            // Normal parameter: read `bits` significant bits (0 = not stored).
            let svd_value = match param.bits {
                0 => 0,
                bits => reader.read_bits(bits)?,
            };
            // Apply signed bias correction if needed.
            let sysex_value = if param.signed {
                // SVD centers at 2^(bits-1), SysEx centers at 64.
                svd_value + 64 - (1 << (param.bits - 1))
            } else {
                svd_value
            };
            sysex.push(sysex_value as u8);
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
    if spec.data_bytes() > spec.entry_size {
        return Err(ConvertError::SpecOverflow {
            kind: spec.kind,
            data: spec.data_bytes(),
            entry: spec.entry_size,
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
        if param.sysex_bytes == 0 {
            writer.write_bits(0, param.bits);
        } else if param.bits == 0 {
            // Not stored in the SVD.
            idx += 1;
        } else if param.sysex_bytes == 1 {
            // Apply inverse signed bias correction if needed.
            let svd_value = if param.signed {
                // SysEx centers at 64, SVD centers at 2^(bits-1).
                (sysex[idx] as i32 - 64 + (1i32 << (param.bits - 1))) as u32
            } else {
                sysex[idx] as u32
            };
            writer.write_bits(svd_value, param.bits);
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
/// for the corresponding spec section. The bytes after the data sections are
/// copied from `source`, the entry the sections were decoded from, so that
/// undocumented trailing bytes survive a round trip. Without a source they
/// are `spec.init_tail` followed by zero fill up to `spec.entry_size`.
pub fn sysex_to_svd(sections: &[Vec<u8>], spec: &SvdToneSpec, source: Option<&[u8]>) -> Vec<u8> {
    let mut result = Vec::with_capacity(spec.entry_size);

    for (section_data, section_spec) in sections.iter().zip(spec.sections.iter()) {
        let mut writer = BitWriter::new();
        pack_section(&mut writer, section_data, section_spec);
        let mut section_bytes = writer.into_bytes();
        // Pad section to its known SVD size.
        section_bytes.resize(section_spec.svd_bytes, 0x00);
        result.extend_from_slice(&section_bytes);
    }

    let tail_start = result.len();
    match source {
        Some(entry) => result.extend_from_slice(&entry[tail_start..spec.entry_size]),
        None => result.extend_from_slice(spec.init_tail),
    }
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
    use crate::svd::{ChunkType, SvdFile};
    use crate::svd_specs::{
        MFX_PARAMS, PCMD_TONE_SPEC, PCMS_TONE_SPEC, SNA_TONE_SPEC, SND_TONE_SPEC, SNS_TONE_SPEC,
        SS_TONE_SPEC,
    };
    use std::ops::RangeInclusive;
    use std::path::Path;

    /// Write `value` as `bits` bits, MSB first, at entry bit `bit`.
    ///
    /// Builds synthetic entries from the documented bit positions, independently
    /// of the spec under test.
    fn put(entry: &mut [u8], bit: usize, bits: usize, value: u32) {
        for i in 0..bits {
            let b = (value >> (bits - 1 - i)) & 1;
            let pos = bit + i;
            entry[pos / 8] |= (b as u8) << (7 - pos % 8);
        }
    }

    /// `n` copies of `range`.
    fn rep(n: usize, range: RangeInclusive<u32>) -> Vec<RangeInclusive<u32>> {
        vec![range; n]
    }

    /// Decode `entry` with `spec` and check every SysEx parameter against its
    /// documented range. `ranges` lists one range per parameter in spec order;
    /// nibblized parameters are checked as their combined value.
    fn assert_in_range(entry: &[u8], spec: &SvdToneSpec, ranges: &[RangeInclusive<u32>]) {
        let sections = svd_to_sysex(entry, spec).unwrap();
        let mut values = Vec::new();
        for (sysex, section) in sections.iter().zip(spec.sections) {
            let mut bytes = sysex.iter();
            for param in section.params.iter().filter(|p| p.sysex_bytes > 0) {
                // A single byte folds to itself; nibbles combine MSB first.
                let value = bytes
                    .by_ref()
                    .take(param.sysex_bytes as usize)
                    .fold(0, |acc, &b| (acc << 4) | b as u32);
                values.push(value);
            }
        }
        assert_eq!(values.len(), ranges.len(), "parameter count");
        for (i, (value, range)) in values.iter().zip(ranges).enumerate() {
            assert!(range.contains(value), "param {i}: {value} not in {range:?}");
        }
    }

    #[test]
    fn spec_larger_than_entry_is_an_error() {
        // SN-S sections (246 bytes) declared for a 138-byte SN-A entry.
        let spec = SvdToneSpec {
            kind: ChunkType::SnAcousticTone,
            sections: SNS_TONE_SPEC.sections,
            entry_size: 138,
            init_tail: &[],
        };
        let err = svd_to_sysex(&[0; 138], &spec).unwrap_err();
        assert_eq!(
            err.to_string(),
            "SN Acoustic Tone spec sections take 246 bytes, more than its 138-byte entry"
        );
    }

    /// Load the test SVD fixture (3 SN-S patches).
    fn load_test_fixture() -> SvdFile {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_sns.svd");
        let data = std::fs::read(path).expect("failed to read test fixture");
        SvdFile::parse(&data).expect("failed to parse test fixture")
    }

    #[test]
    fn round_trip_structure() {
        // Verify round-trip produces correct entry size and end marker.
        // Note: all-zero SVD data is not fully round-trip-able because
        // signed params get bias-corrected (0 → 60 → back to different bits).
        // Use real entries for full round-trip testing.
        let entry = vec![0u8; 280];
        let sections = svd_to_sysex(&entry, &SNS_TONE_SPEC).unwrap();
        let repacked = sysex_to_svd(&sections, &SNS_TONE_SPEC, None);
        assert_eq!(repacked.len(), 280);
        assert_eq!(&repacked[246..252], [0x0E, 0x00, 0x00, 0x00, 0x02, 0xC0]);
        assert!(repacked[252..].iter().all(|&b| b == 0));
    }

    #[test]
    fn section_sysex_sizes() {
        let sections = svd_to_sysex(&[0u8; 280], &SNS_TONE_SPEC).unwrap();
        assert_eq!(sections.len(), 5);
        // Section 0: Common (64 SysEx bytes)
        assert_eq!(sections[0].len(), 64);
        // Section 1: MFX (145 SysEx bytes)
        assert_eq!(sections[1].len(), 145);
        // Sections 2-4: Partial (61 each)
        assert_eq!(sections[2].len(), 61);
        assert_eq!(sections[3].len(), 61);
        assert_eq!(sections[4].len(), 61);
    }

    #[test]
    fn decode_tone_name_from_fixture() {
        let svd = load_test_fixture();

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

        assert_eq!(name, "SL-JP8 1");
    }

    #[test]
    fn round_trip_real_entries() {
        let svd = load_test_fixture();

        let sns_chunk = svd
            .chunks
            .iter()
            .find(|c| c.chunk_type == crate::svd::ChunkType::SnSynthTone)
            .unwrap();

        // Round-trip every entry, with a non-INIT tail after the end marker.
        for (i, entry) in sns_chunk.entries.iter().enumerate() {
            let mut entry = entry.clone();
            entry[250..252].copy_from_slice(&[0x44, 0xC0]);
            let sections = svd_to_sysex(&entry, &SNS_TONE_SPEC).unwrap();
            let repacked = sysex_to_svd(&sections, &SNS_TONE_SPEC, Some(&entry));
            assert_eq!(repacked, entry, "Entry {i} mismatch");
        }
    }

    #[test]
    fn sysex_to_svd_to_sysex_round_trip() {
        let svd = load_test_fixture();
        let sns_chunk = svd
            .chunks
            .iter()
            .find(|c| c.chunk_type == crate::svd::ChunkType::SnSynthTone)
            .unwrap();

        for (i, entry) in sns_chunk.entries.iter().enumerate() {
            // SVD → SysEx
            let sysex_sections = svd_to_sysex(entry, &SNS_TONE_SPEC).unwrap();
            // SysEx → SVD
            let svd_entry = sysex_to_svd(&sysex_sections, &SNS_TONE_SPEC, None);
            // SVD → SysEx again
            let sysex_sections2 = svd_to_sysex(&svd_entry, &SNS_TONE_SPEC).unwrap();

            // SysEx bytes must match exactly.
            for (s, (a, b)) in sysex_sections
                .iter()
                .zip(sysex_sections2.iter())
                .enumerate()
            {
                assert_eq!(a, b, "Entry {i} section {s} SysEx mismatch");
            }
        }
    }

    /// Documented ranges of the MFX block shared by every tone type
    /// (`docs/svd/06-pcm-synth.md`, Common MFX).
    fn mfx_ranges() -> Vec<RangeInclusive<u32>> {
        let mut r = vec![0..=67, 0..=127, 0..=127, 0..=127, 0..=3];
        for _ in 0..4 {
            r.extend([0..=101, 1..=127]);
        }
        r.extend(rep(4, 0..=16));
        r.extend(rep(32, 12768..=52768));
        r
    }

    /// Fill an MFX section starting at entry bit `base` with in-range values
    /// at the documented section bits.
    fn put_mfx(entry: &mut [u8], base: usize) {
        put(entry, base + 7, 7, 127); // reserve, always 127
        for n in 0..4 {
            put(entry, base + 37 + 14 * n, 7, 64); // Control n Sens
        }
        for n in 0..32 {
            put(entry, base + 106 + 16 * n, 16, 32768); // Parameter n
        }
    }

    #[test]
    fn sna_entry_decodes_in_range_and_round_trips() {
        // Synthetic SNTa entry laid out per docs/svd/08-sn-acoustic.md:
        // Common at bit 0 (60 bytes), MFX at byte 60, no end marker.
        let mut entry = vec![0u8; 138];
        for (i, c) in "SYNTH ACOUST    ".bytes().enumerate() {
            put(&mut entry, 7 * i, 7, c.into());
        }
        put(&mut entry, 112, 7, 100); // Tone Level
        put(&mut entry, 176, 3, 4); // Octave Shift (bias: SysEx 64)
        put(&mut entry, 179, 7, 9); // Category
        put(&mut entry, 186, 8, 200); // Phrase Number
        put(&mut entry, 194, 3, 4); // Phrase Octave Shift
        put(&mut entry, 197, 1, 1); // TFX Switch
        put(&mut entry, 429, 7, 127); // Modify Parameter 32
        put_mfx(&mut entry, 60 * 8);

        let mut ranges = rep(16, 32..=127);
        ranges.extend([0..=127, 0..=1]);
        ranges.extend(rep(8, 0..=127));
        ranges.extend([61..=67, 0..=127, 0..=255, 61..=67, 0..=1]);
        ranges.extend(rep(34, 0..=127));
        ranges.extend(rep(4, 0..=127)); // reserves
        ranges.extend(mfx_ranges());
        assert_in_range(&entry, &SNA_TONE_SPEC, &ranges);

        let sections = svd_to_sysex(&entry, &SNA_TONE_SPEC).unwrap();
        assert_eq!(sections[0][0x1B], 9, "Category");
        assert_eq!(sections[0][0x41], 127, "Modify Parameter 32");
        assert_eq!(sysex_to_svd(&sections, &SNA_TONE_SPEC, Some(&entry)), entry);
    }

    /// Documented ranges of every PRFb parameter in spec order
    /// (`docs/svd/05-studio-set.md`). Reserves get the full 7-bit range.
    fn ss_ranges() -> Vec<RangeInclusive<u32>> {
        let any = 0..=127;
        let fx = 12768..=52768;
        // Common
        let mut r = rep(16, 32..=127);
        r.extend(rep(8, any.clone()));
        r.extend(rep(16, 0..=64)); // Voice Reserve 1-16
        r.extend(rep(17, any.clone()));
        r.extend(rep(4, 0..=97));
        r.extend([20..=250, 0..=16]);
        r.extend(rep(4, 0..=1));
        r.push(0..=15);
        r.extend(rep(6, 0..=12));
        r.extend(rep(4, any.clone()));
        r.push(0..=1);
        r.extend(rep(4, any.clone()));
        // Chorus
        r.extend([0..=3, 0..=127, any.clone(), 0..=2]);
        r.extend(rep(20, fx.clone()));
        // Reverb
        r.extend([0..=6, 0..=127, 0..=3]);
        r.extend(rep(24, fx));
        // Motional Surround
        r.extend([0..=1, 0..=3, 0..=127, 0..=2, 0..=100, 0..=100, 0..=100]);
        r.extend([0..=127, 0..=127, 0..=32, 0..=127, 0..=16, 0..=100]);
        r.extend(rep(3, any.clone()));
        // Master EQ
        r.extend([0..=1, 0..=30, 0..=16, 0..=30, 0..=4, 0..=2, 0..=30]);
        // MIDI
        r.extend(rep(16, 0..=1));
        // Parts
        for _ in 0..16 {
            r.extend([0..=15, 0..=1]);
            r.extend(rep(4, any.clone()));
            r.extend(rep(5, 0..=127)); // MSB, LSB, PC, Level, Pan
            r.extend([16..=112, 14..=114, 0..=2, 0..=2, 0..=25, 0..=2, 0..=128]);
            r.extend(rep(8, 0..=127));
            r.extend([61..=67, 1..=127]);
            r.extend(rep(4, 0..=127)); // Key Range, Key Fade
            r.extend([1..=127, 0..=127, 0..=127, 0..=127, 0..=1, any.clone()]);
            r.extend([0..=127, 0..=127, 0..=11, any.clone(), 0..=8, 0..=11]);
            r.extend(rep(12, 0..=127));
            r.extend(rep(10, 0..=1));
            r.extend([
                0..=4,
                0..=127,
                any.clone(),
                0..=127,
                any.clone(),
                0..=32,
                0..=127,
            ]);
            r.extend(rep(3, any.clone()));
        }
        // Part EQs
        for _ in 0..16 {
            r.extend([0..=1, 0..=1, 0..=30, 0..=16, 0..=30, 0..=4, 0..=2, 0..=30]);
        }
        r
    }

    #[test]
    fn ss_entry_decodes_in_range_and_round_trips() {
        // Synthetic PRFb entry laid out per docs/svd/05-studio-set.md, using
        // the documented entry bit of each field.
        let mut entry = vec![0u8; 1068];
        for (i, c) in "Integral Test 01".bytes().enumerate() {
            put(&mut entry, 7 * i, 7, c.into());
        }
        put(&mut entry, 112, 32, 0xFE82_0878); // reserves 0x10-0x17, always set
        for (bit, value) in [(369, 1), (376, 97), (383, 2), (390, 4)] {
            put(&mut entry, bit, 7, value); // Tone Control 1-4 Source
        }
        put(&mut entry, 397, 8, 120); // Tempo
        put(&mut entry, 410, 1, 1); // Reverb Switch
        put(&mut entry, 443, 7, 100); // Ext Part Level
        put(&mut entry, 495, 4, 3); // Chorus Type
        put(&mut entry, 506, 2, 2); // Chorus Output Select
        for n in 0..20 {
            put(&mut entry, 511 + 16 * n, 16, 32768); // Chorus Parameter
        }
        put(&mut entry, 832, 4, 6); // Reverb Type
        for n in 0..24 {
            put(&mut entry, 845 + 16 * n, 16, 32768); // Reverb Parameter
        }
        put(&mut entry, 1302, 7, 100); // Motional Surround Depth
        put(&mut entry, 1365, 5, 30); // Master EQ High Gain
        put(&mut entry, 1376 + 8 * 15, 1, 1); // MIDI 16 Phase Lock
        for n in 0..16 {
            let part = 1504 + 408 * n;
            put(&mut entry, part, 4, n as u32); // Receive Channel
            put(&mut entry, part + 5, 4, 0b1111); // reserves, always 1
            put(&mut entry, part + 23, 7, 127 - n as u32); // PC
            put(&mut entry, part + 44, 7, 64); // Coarse Tune
            put(&mut entry, part + 51, 7, 64); // Fine Tune
            put(&mut entry, part + 133, 3, 4); // Octave Shift (bias: 64)
            put(&mut entry, part + 136, 7, 64); // Velocity Sens Offset
            put(&mut entry, part + 171, 7, 1); // Velocity Range Lower
            put(&mut entry, part + 366, 6, 32); // Motional Surround Width
            put(&mut entry, 8032 + 32 * n + 22, 5, 30); // Part EQ High Gain
        }

        assert_in_range(&entry, &SS_TONE_SPEC, &ss_ranges());

        let sections = svd_to_sysex(&entry, &SS_TONE_SPEC).unwrap();
        assert_eq!(&sections[0][..16], b"Integral Test 01");
        assert_eq!(&sections[0][0x10..0x18], [127, 16, 16, 16, 1, 1, 1, 1]);
        assert_eq!(sections[0][0x3F], 0, "Solo Part");
        assert_eq!(sections[1][0x00], 3, "Chorus Type");
        assert_eq!(sections[1][0x03], 2, "Chorus Output Select");
        assert_eq!(sections[20][0], 1, "MIDI 16 Phase Lock");
        let part16 = &sections[21 + 15];
        assert_eq!(part16[0x00], 15, "Part 16 Receive Channel");
        assert_eq!(part16[0x08], 112, "Part 16 PC");
        assert_eq!(part16[0x1B], 64, "Part 16 Octave Shift");
        assert_eq!(part16[0x48], 32, "Part 16 Motional Surround Width");
        assert_eq!(sections[37 + 15][0x07], 30, "Part EQ 16 High Gain");
        assert_eq!(sysex_to_svd(&sections, &SS_TONE_SPEC, Some(&entry)), entry);
    }

    const PCM_SYNTH_DOC: &str = include_str!("../../../docs/svd/06-pcm-synth.md");
    const PCM_DRUM_DOC: &str = include_str!("../../../docs/svd/07-pcm-drum.md");
    const SN_DRUM_DOC: &str = include_str!("../../../docs/svd/09-sn-drum.md");

    /// A field documented in a `docs/svd` table.
    #[derive(Clone)]
    struct DocField {
        /// Section bit.
        bit: usize,
        /// Stored width.
        bits: usize,
        /// Documented SysEx range; the full width for reserves.
        range: RangeInclusive<u32>,
    }

    /// Read the field table that follows `heading` in a `docs/svd` page.
    ///
    /// Cells are split on `|`. A row covering several offsets
    /// (`(reserve) × 14`, `7 each`) expands to one field per offset.
    fn doc_fields(doc: &str, heading: &str) -> Vec<DocField> {
        let cells = |line: &str| -> Vec<String> {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect()
        };
        let mut lines = doc
            .lines()
            .skip_while(|l| !l.starts_with(heading))
            .skip_while(|l| !l.starts_with('|'));
        let header = cells(lines.next().expect("table header"));
        let col = |name: &str| header.iter().position(|h| h == name).unwrap();
        let (param, bits, bit, range) = (
            col("Parameter"),
            col("Bits"),
            col("Section bit"),
            col("Range"),
        );
        let mut fields = Vec::new();
        for line in lines.skip(1).take_while(|l| l.starts_with('|')) {
            let row = cells(line);
            let width: usize = row[bits]
                .trim_end_matches(" each")
                .split('+')
                .map(|b| b.parse::<usize>().unwrap())
                .sum();
            let count = row[param]
                .split_once('×')
                .map_or(1, |(_, n)| n.trim().parse().unwrap());
            let start: usize = row[bit].split('–').next().unwrap().parse().unwrap();
            // Reserves leave the range blank or write `–`: take the full width.
            let range = match row[range].split_once('–') {
                Some((lo, hi)) if !lo.is_empty() => lo.parse().unwrap()..=hi.parse().unwrap(),
                _ => 0..=(1u32 << width) - 1,
            };
            for i in 0..count {
                fields.push(DocField {
                    bit: start + i * width,
                    bits: width,
                    range: range.clone(),
                });
            }
        }
        fields
    }

    /// Build a synthetic entry from documented sections, given as (entry byte
    /// offset, fields), then decode it with `spec`.
    ///
    /// Fields alternate between the low and high end of their range, stored
    /// with the bias rule of `docs/svd/02-encoding.md`. Every decoded value
    /// must equal the one written and lie in its range, and re-encoding must
    /// reproduce the entry byte for byte.
    fn assert_doc_layout(spec: &SvdToneSpec, layout: &[(usize, Vec<DocField>)]) -> Vec<Vec<u8>> {
        let mut entry = vec![0u8; spec.entry_size];
        let mut expected = Vec::new();
        for (offset, fields) in layout {
            let mut prev_end = 0;
            for (i, f) in fields.iter().enumerate() {
                assert_eq!(f.bit, prev_end, "documented fields must be contiguous");
                prev_end = f.bit + f.bits;
                let value = if i % 2 == 0 {
                    *f.range.start()
                } else {
                    *f.range.end()
                };
                let biased = f.bits < 7 && *f.range.end() >= 1 << f.bits;
                let stored = if biased {
                    value + (1 << (f.bits - 1)) - 64
                } else {
                    value
                };
                assert!(stored < 1 << f.bits);
                put(&mut entry, offset * 8 + f.bit, f.bits, stored);
                expected.push((value, f.range.clone()));
            }
        }

        let sections = svd_to_sysex(&entry, spec).unwrap();
        let mut values = Vec::new();
        for (sysex, section) in sections.iter().zip(spec.sections) {
            let mut bytes = sysex.iter();
            for param in section.params.iter().filter(|p| p.sysex_bytes > 0) {
                let value = bytes
                    .by_ref()
                    .take(param.sysex_bytes as usize)
                    .fold(0, |acc, &b| (acc << 4) | b as u32);
                values.push(value);
            }
        }
        assert_eq!(values.len(), expected.len(), "parameter count");
        for (i, (value, (want, range))) in values.iter().zip(&expected).enumerate() {
            assert!(range.contains(value), "param {i}: {value} not in {range:?}");
            assert_eq!(value, want, "param {i}");
        }
        assert_eq!(sysex_to_svd(&sections, spec, Some(&entry)), entry);
        sections
    }

    #[test]
    fn snd_entry_matches_documented_layout() {
        let mfx = doc_fields(PCM_SYNTH_DOC, "## Common MFX");
        let mut layout = vec![
            (0, doc_fields(SN_DRUM_DOC, "## Common (")),
            (18, mfx),
            (96, doc_fields(PCM_DRUM_DOC, "## Common Comp/EQ")),
        ];
        let note = doc_fields(SN_DRUM_DOC, "## Note (");
        layout.extend((0..62).map(|k| (138 + 14 * k, note.clone())));
        assert_doc_layout(&SND_TONE_SPEC, &layout);
    }

    #[test]
    fn pcms_entry_matches_documented_layout() {
        let partial = doc_fields(PCM_SYNTH_DOC, "## Partial (");
        let mut layout = vec![
            (0, doc_fields(PCM_SYNTH_DOC, "## Common (")),
            (56, doc_fields(PCM_SYNTH_DOC, "## Common MFX")),
            (134, doc_fields(PCM_SYNTH_DOC, "## PMT")),
        ];
        layout.extend((0..4).map(|n| (166 + 99 * n, partial.clone())));
        layout.push((562, doc_fields(PCM_SYNTH_DOC, "## Common 2 (")));
        assert_doc_layout(&PCMS_TONE_SPEC, &layout);
    }

    #[test]
    fn pcmd_entry_matches_documented_layout() {
        let partial = doc_fields(PCM_DRUM_DOC, "## Partial (");
        let mut layout = vec![
            (0, doc_fields(PCM_DRUM_DOC, "## Common (")),
            (14, doc_fields(PCM_SYNTH_DOC, "## Common MFX")),
            (92, doc_fields(PCM_DRUM_DOC, "## Common Comp/EQ")),
        ];
        layout.extend((0..88).map(|n| (134 + 122 * n, partial.clone())));
        layout.push((10870, doc_fields(PCM_DRUM_DOC, "## Common 2 (")));
        assert_doc_layout(&PCMD_TONE_SPEC, &layout);
    }

    #[test]
    fn pcms_long_name_and_category() {
        // Common 2 starts at byte 562: long name at bit 0, Category at 112.
        let mut entry = vec![0u8; 590];
        for (i, c) in "Synthetic Tone 1".bytes().enumerate() {
            put(&mut entry, 562 * 8 + 7 * i, 7, c.into());
        }
        put(&mut entry, 562 * 8 + 112, 7, 34);
        let sections = svd_to_sysex(&entry, &PCMS_TONE_SPEC).unwrap();
        assert_eq!(&sections[7][..16], b"Synthetic Tone 1");
        let category =
            crate::svd::extract_entry_category(&entry, crate::svd::ChunkType::PcmSynthTone);
        assert_eq!(category, Some(34));
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
