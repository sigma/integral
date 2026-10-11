//! SVD container format parser and writer for Roland INTEGRA-7 backup files.
//!
//! The SVD (Sound Visual Data) format is used by Roland to store instrument
//! patches in backup files. Each file contains a header, a chunk directory,
//! and one or more chunks of typed patch entries.
//!
//! Reference: `docs/svd/01-container.md`, `docs/svd/04-chunk-types.md`

use std::fmt;
use thiserror::Error;

use crate::bitstream::BitReader;
use crate::svd_convert::svd_to_sysex;
use crate::svd_specs::{PCMS_TONE_SPEC, SNA_TONE_SPEC, SNS_TONE_SPEC};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// SVD file signature at bytes 2–5, after the big-endian header length.
const SIGNATURE: [u8; 4] = *b"SVD1";

/// INTEGRA-7 model identifier in SVD directory entries.
const MODEL_MI69: [u8; 4] = [b'M', b'I', b'6', b'9'];

/// Size of the big-endian header length at bytes 0–1, which excludes itself.
const LENGTH_FIELD_SIZE: usize = 2;

/// Size of the file header in bytes.
const HEADER_SIZE: usize = 16;

/// Size of each directory entry in bytes.
const DIR_ENTRY_SIZE: usize = 16;

/// Size of a zone header in bytes.
const ZONE_HEADER_SIZE: usize = 16;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur when parsing an SVD file.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SvdError {
    /// Bytes 2–5 are not the `SVD1` signature.
    #[error("invalid SVD magic signature")]
    InvalidMagic,

    /// The header length does not span a whole number of directory entries.
    #[error("invalid SVD header length: {0:#06x}")]
    InvalidHeaderLength(u16),

    /// A directory entry has a model identifier other than `MI69`.
    #[error("invalid model identifier (expected MI69)")]
    InvalidModel,

    /// A directory entry has an unrecognised chunk type code.
    #[error("invalid chunk type code: {0:?}")]
    InvalidChunkType([u8; 4]),

    /// The file is truncated; a read would exceed the available data.
    #[error("unexpected end of file")]
    UnexpectedEof,

    /// A chunk's declared size exceeds the remaining file data.
    #[error("chunk data overflows file boundary")]
    ChunkOverflow,
}

// ---------------------------------------------------------------------------
// ChunkType
// ---------------------------------------------------------------------------

/// Identifies the type of patch data stored in an SVD chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkType {
    /// Studio Set (`PRFb`).
    StudioSet,
    /// PCM Synth Tone (`RFPa`).
    PcmSynthTone,
    /// PCM Drum Kit (`RFRa`).
    PcmDrumKit,
    /// SuperNATURAL Synth Tone (`SHPa`).
    SnSynthTone,
    /// SuperNATURAL Acoustic Tone (`SNTa`).
    SnAcousticTone,
    /// SuperNATURAL Drum Kit (`SDKa`).
    SnDrumKit,
    /// System settings (`SYSa`), present only in BACKUP files.
    SystemSettings,
}

impl ChunkType {
    /// Parse a 4-byte ASCII chunk type code into a [`ChunkType`].
    ///
    /// Returns `None` if the code is not recognised.
    pub fn from_code(code: &[u8; 4]) -> Option<Self> {
        match code {
            b"PRFb" => Some(Self::StudioSet),
            b"RFPa" => Some(Self::PcmSynthTone),
            b"RFRa" => Some(Self::PcmDrumKit),
            b"SHPa" => Some(Self::SnSynthTone),
            b"SNTa" => Some(Self::SnAcousticTone),
            b"SDKa" => Some(Self::SnDrumKit),
            b"SYSa" => Some(Self::SystemSettings),
            _ => None,
        }
    }

    /// Return the 4-byte ASCII chunk type code for this variant.
    pub fn to_code(&self) -> [u8; 4] {
        match self {
            Self::StudioSet => *b"PRFb",
            Self::PcmSynthTone => *b"RFPa",
            Self::PcmDrumKit => *b"RFRa",
            Self::SnSynthTone => *b"SHPa",
            Self::SnAcousticTone => *b"SNTa",
            Self::SnDrumKit => *b"SDKa",
            Self::SystemSettings => *b"SYSa",
        }
    }

    /// Kebab-case identifier for CLI filtering.
    pub fn cli_name(&self) -> &'static str {
        match self {
            Self::StudioSet => "studio-set",
            Self::PcmSynthTone => "pcm-synth",
            Self::PcmDrumKit => "pcm-drum",
            Self::SnSynthTone => "sn-synth",
            Self::SnAcousticTone => "sn-acoustic",
            Self::SnDrumKit => "sn-drum",
            Self::SystemSettings => "system",
        }
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Self::StudioSet => "Studio Set",
            Self::PcmSynthTone => "PCM Synth Tone",
            Self::PcmDrumKit => "PCM Drum Kit",
            Self::SnSynthTone => "SN Synth Tone",
            Self::SnAcousticTone => "SN Acoustic Tone",
            Self::SnDrumKit => "SN Drum Kit",
            Self::SystemSettings => "System Settings",
        }
    }

    /// Return the default entry size in bytes for this chunk type.
    pub fn default_entry_size(&self) -> usize {
        match self {
            Self::StudioSet => 1068,
            Self::PcmSynthTone => 590,
            Self::PcmDrumKit => 10890,
            Self::SnSynthTone => 280,
            Self::SnAcousticTone => 138,
            Self::SnDrumKit => 1006,
            Self::SystemSettings => 108,
        }
    }
}

// ---------------------------------------------------------------------------
// Tone Category
// ---------------------------------------------------------------------------

include!(concat!(env!("OUT_DIR"), "/tone_category_data.rs"));

impl ChunkType {
    /// Return the length (in characters) of patch names for this chunk type.
    ///
    /// Studio Sets use 16-character names; all tone/drum types use 12.
    /// System settings have no name (`None`).
    pub fn name_length(&self) -> Option<usize> {
        match self {
            Self::StudioSet => Some(16),
            Self::SystemSettings => None,
            _ => Some(12),
        }
    }
}

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

// ---------------------------------------------------------------------------
// Entry extraction helpers
// ---------------------------------------------------------------------------

/// Extract the patch name from a raw SVD entry's bitstream.
///
/// The name is stored as 7-bit ASCII characters at the start of the entry.
/// `name_len` specifies how many characters to read (use
/// [`ChunkType::name_length`] for the correct value).
pub fn extract_entry_name(entry: &[u8], name_len: usize) -> String {
    let mut reader = BitReader::new(entry);
    let mut name = String::with_capacity(name_len);
    for _ in 0..name_len {
        if let Ok(ch) = reader.read_bits(7) {
            let ch = ch as u8;
            name.push(if (32..=127).contains(&ch) {
                ch as char
            } else {
                ' '
            });
        }
    }
    name.trim_end().to_string()
}

/// Extract the tone category byte from a raw SVD entry, if the chunk type
/// has a known spec table.
///
/// Returns `None` for chunk types without category information (e.g.
/// `StudioSet`, `PcmDrumKit`).
pub fn extract_entry_category(entry: &[u8], chunk_type: ChunkType) -> Option<u8> {
    match chunk_type {
        ChunkType::SnSynthTone => {
            let sections = svd_to_sysex(entry, &SNS_TONE_SPEC).ok()?;
            Some(sections[0][0x36])
        }
        ChunkType::SnAcousticTone => {
            let sections = svd_to_sysex(entry, &SNA_TONE_SPEC).ok()?;
            Some(sections[0][0x1B])
        }
        ChunkType::PcmSynthTone => {
            // Common 2 `00 10`.
            let sections = svd_to_sysex(entry, &PCMS_TONE_SPEC).ok()?;
            Some(sections[7][0x10])
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// SvdChunk / SvdFile
// ---------------------------------------------------------------------------

/// A single chunk within an SVD file, containing zero or more fixed-size entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvdChunk {
    /// The type of patch data in this chunk.
    pub chunk_type: ChunkType,
    /// Size of each entry in bytes.
    pub entry_size: usize,
    /// The raw entry data. Each entry is exactly `entry_size` bytes.
    pub entries: Vec<Vec<u8>>,
}

/// A parsed SVD backup file containing one or more chunks of patch data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvdFile {
    /// The chunks in this file, in directory order.
    pub chunks: Vec<SvdChunk>,
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Read a big-endian `u32` from a 4-byte slice.
fn read_u32_be(data: &[u8]) -> u32 {
    u32::from_be_bytes([data[0], data[1], data[2], data[3]])
}

/// Write a big-endian `u32` into a `Vec<u8>`.
fn write_u32_be(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_be_bytes());
}

// ---------------------------------------------------------------------------
// SvdFile implementation
// ---------------------------------------------------------------------------

impl SvdFile {
    /// Parse an SVD file from raw bytes.
    ///
    /// Validates the header magic, reads the chunk directory, and extracts
    /// each chunk's zone header and entries.
    pub fn parse(data: &[u8]) -> Result<Self, SvdError> {
        // Validate header
        if data.len() < HEADER_SIZE {
            return Err(SvdError::UnexpectedEof);
        }
        if data[2..6] != SIGNATURE {
            return Err(SvdError::InvalidMagic);
        }

        // The header length bounds the directory, which starts at 0x10.
        let header_length = u16::from_be_bytes([data[0], data[1]]);
        let dir_end = header_length as usize + LENGTH_FIELD_SIZE;
        if dir_end < HEADER_SIZE || !(dir_end - HEADER_SIZE).is_multiple_of(DIR_ENTRY_SIZE) {
            return Err(SvdError::InvalidHeaderLength(header_length));
        }
        if dir_end > data.len() {
            return Err(SvdError::UnexpectedEof);
        }

        let mut chunks = Vec::new();
        for dir_offset in (HEADER_SIZE..dir_end).step_by(DIR_ENTRY_SIZE) {
            // Read directory entry fields
            let code: [u8; 4] = [
                data[dir_offset],
                data[dir_offset + 1],
                data[dir_offset + 2],
                data[dir_offset + 3],
            ];
            let chunk_type = ChunkType::from_code(&code).ok_or(SvdError::InvalidChunkType(code))?;

            let model: [u8; 4] = [
                data[dir_offset + 4],
                data[dir_offset + 5],
                data[dir_offset + 6],
                data[dir_offset + 7],
            ];
            if model != MODEL_MI69 {
                return Err(SvdError::InvalidModel);
            }

            let chunk_offset = read_u32_be(&data[dir_offset + 8..dir_offset + 12]) as usize;
            let chunk_size = read_u32_be(&data[dir_offset + 12..dir_offset + 16]) as usize;

            // Validate chunk bounds
            let chunk_end = chunk_offset
                .checked_add(chunk_size)
                .ok_or(SvdError::ChunkOverflow)?;
            if chunk_end > data.len() {
                return Err(SvdError::ChunkOverflow);
            }

            // Read zone header
            if chunk_size < ZONE_HEADER_SIZE {
                return Err(SvdError::UnexpectedEof);
            }
            let zone = &data[chunk_offset..chunk_offset + ZONE_HEADER_SIZE];
            let count = read_u32_be(&zone[0..4]) as usize;
            let entry_size = read_u32_be(&zone[4..8]) as usize;

            // Entries must fit in their chunk; zero-size entries carry no data.
            let entries_start = chunk_offset + ZONE_HEADER_SIZE;
            let entries_len = count.checked_mul(entry_size);
            if (count > 0 && entry_size == 0)
                || entries_len.is_none_or(|len| len > chunk_end - entries_start)
            {
                return Err(SvdError::ChunkOverflow);
            }
            let entries = data[entries_start..chunk_end]
                .chunks_exact(entry_size.max(1))
                .take(count)
                .map(<[u8]>::to_vec)
                .collect();

            chunks.push(SvdChunk {
                chunk_type,
                entry_size,
                entries,
            });
        }

        Ok(SvdFile { chunks })
    }

    /// Serialize this SVD file to raw bytes.
    ///
    /// Writes the header, chunk directory, and all zone headers with their
    /// entries, in `chunks` order. Empty chunks still get a zone header with
    /// count=0 and the correct entry size. A SOUND file needs all 6 tone and
    /// studio set chunks; a BACKUP adds `SYSa` as the seventh.
    pub fn write(&self) -> Vec<u8> {
        let num_chunks = self.chunks.len();
        let data_start = HEADER_SIZE + num_chunks * DIR_ENTRY_SIZE;

        // Calculate chunk sizes and offsets
        let mut chunk_offsets = Vec::with_capacity(num_chunks);
        let mut offset = data_start;
        for chunk in &self.chunks {
            chunk_offsets.push(offset);
            let chunk_data_size = ZONE_HEADER_SIZE + chunk.entries.len() * chunk.entry_size;
            offset += chunk_data_size;
        }
        let total_size = offset;

        let mut buf = Vec::with_capacity(total_size);

        // Write header (16 bytes)
        buf.extend_from_slice(&((data_start - LENGTH_FIELD_SIZE) as u16).to_be_bytes());
        buf.extend_from_slice(&SIGNATURE);
        buf.extend_from_slice(&[0u8; 10]); // padding

        // Write directory entries
        for (i, chunk) in self.chunks.iter().enumerate() {
            buf.extend_from_slice(&chunk.chunk_type.to_code());
            buf.extend_from_slice(&MODEL_MI69);
            let chunk_data_size = ZONE_HEADER_SIZE + chunk.entries.len() * chunk.entry_size;
            write_u32_be(&mut buf, chunk_offsets[i] as u32);
            write_u32_be(&mut buf, chunk_data_size as u32);
        }

        // Write chunk data
        for chunk in &self.chunks {
            // Zone header
            write_u32_be(&mut buf, chunk.entries.len() as u32);
            write_u32_be(&mut buf, chunk.entry_size as u32);
            write_u32_be(&mut buf, 0x0000_0010); // unknown field, always 0x10
            write_u32_be(&mut buf, 0x0000_0000); // unknown field, always 0

            // Entries
            for entry in &chunk.entries {
                buf.extend_from_slice(entry);
            }
        }

        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn chunk_type_roundtrip() {
        let types = [
            ChunkType::StudioSet,
            ChunkType::PcmSynthTone,
            ChunkType::PcmDrumKit,
            ChunkType::SnSynthTone,
            ChunkType::SnAcousticTone,
            ChunkType::SnDrumKit,
            ChunkType::SystemSettings,
        ];
        for ct in &types {
            let code = ct.to_code();
            assert_eq!(ChunkType::from_code(&code), Some(*ct));
        }
    }

    #[test]
    fn name_lengths() {
        assert_eq!(ChunkType::StudioSet.name_length(), Some(16));
        assert_eq!(ChunkType::SnSynthTone.name_length(), Some(12));
        assert_eq!(ChunkType::SystemSettings.name_length(), None);
    }

    #[test]
    fn chunk_type_unknown_code() {
        assert_eq!(ChunkType::from_code(b"XXXX"), None);
    }

    #[test]
    fn default_entry_sizes() {
        assert_eq!(ChunkType::StudioSet.default_entry_size(), 1068);
        assert_eq!(ChunkType::PcmSynthTone.default_entry_size(), 590);
        assert_eq!(ChunkType::PcmDrumKit.default_entry_size(), 10890);
        assert_eq!(ChunkType::SnSynthTone.default_entry_size(), 280);
        assert_eq!(ChunkType::SnAcousticTone.default_entry_size(), 138);
        assert_eq!(ChunkType::SnDrumKit.default_entry_size(), 1006);
        assert_eq!(ChunkType::SystemSettings.default_entry_size(), 108);
    }

    /// Path to the test SVD fixture (3 SN-S patches from Synth Legends).
    fn fixture_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test_sns.svd")
    }

    #[test]
    fn parse_test_fixture() {
        let data = std::fs::read(fixture_path()).expect("failed to read fixture");
        let svd = SvdFile::parse(&data).expect("failed to parse SVD");

        assert_eq!(svd.chunks.len(), 6);

        // PRFb: 0 entries
        assert_eq!(svd.chunks[0].chunk_type, ChunkType::StudioSet);
        assert_eq!(svd.chunks[0].entries.len(), 0);
        assert_eq!(svd.chunks[0].entry_size, 1068);

        // SHPa: 3 entries, 280 B each
        assert_eq!(svd.chunks[3].chunk_type, ChunkType::SnSynthTone);
        assert_eq!(svd.chunks[3].entries.len(), 3);
        assert_eq!(svd.chunks[3].entry_size, 280);

        // All chunk types present with correct entry sizes
        assert_eq!(svd.chunks[1].entry_size, 590); // RFPa
        assert_eq!(svd.chunks[2].entry_size, 10890); // RFRa
        assert_eq!(svd.chunks[4].entry_size, 138); // SNTa
        assert_eq!(svd.chunks[5].entry_size, 1006); // SDKa
    }

    #[test]
    fn roundtrip_test_fixture() {
        let original = std::fs::read(fixture_path()).expect("failed to read fixture");
        let svd = SvdFile::parse(&original).expect("failed to parse SVD");
        assert_eq!(svd.write(), original);
    }

    /// A BACKUP-shaped file: the 6 SOUND chunks plus `SYSa`.
    fn backup_file() -> SvdFile {
        let original = std::fs::read(fixture_path()).expect("failed to read fixture");
        let mut svd = SvdFile::parse(&original).expect("failed to parse SVD");
        let entry_size = ChunkType::SystemSettings.default_entry_size();
        svd.chunks.push(SvdChunk {
            chunk_type: ChunkType::SystemSettings,
            entry_size,
            entries: vec![vec![0x5A; entry_size]],
        });
        svd
    }

    #[test]
    fn write_backup_layout() {
        let data = backup_file().write();
        assert_eq!(&data[..6], b"\x00\x7ESVD1");
        assert_eq!(&data[0x70..0x78], b"SYSaMI69");
        // First chunk starts right after the seventh directory entry.
        assert_eq!(read_u32_be(&data[0x18..0x1C]), 0x80);
    }

    #[test]
    fn parse_backup_file() {
        let svd = backup_file();
        assert_eq!(SvdFile::parse(&svd.write()), Ok(svd));
    }

    #[test]
    fn error_bad_signature() {
        let mut data = std::fs::read(fixture_path()).expect("failed to read fixture");
        data[2] = b'X';
        assert_eq!(SvdFile::parse(&data), Err(SvdError::InvalidMagic));
    }

    #[test]
    fn error_bad_header_length() {
        let mut data = std::fs::read(fixture_path()).expect("failed to read fixture");
        data[1] = 0x6F;
        assert_eq!(
            SvdFile::parse(&data),
            Err(SvdError::InvalidHeaderLength(0x6F))
        );
    }

    #[test]
    fn error_entries_overflow_chunk() {
        let mut data = std::fs::read(fixture_path()).expect("failed to read fixture");
        // SHPa zone header at 0xA0 claims a 4th entry its chunk can't hold.
        data[0xA3] = 4;
        assert_eq!(SvdFile::parse(&data), Err(SvdError::ChunkOverflow));
    }

    #[test]
    fn error_huge_entry_count() {
        let mut data = std::fs::read(fixture_path()).expect("failed to read fixture");
        // SHPa: count 0xFFFFFFFF, entry size 0.
        data[0xA0..0xA8].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0]);
        assert_eq!(SvdFile::parse(&data), Err(SvdError::ChunkOverflow));
    }

    #[test]
    fn error_truncated_file() {
        // Too short to contain even a header
        let data = vec![0u8; 4];
        assert_eq!(SvdFile::parse(&data), Err(SvdError::UnexpectedEof));
    }

    #[test]
    fn error_truncated_directory() {
        let data = std::fs::read(fixture_path()).expect("failed to read fixture");
        assert_eq!(SvdFile::parse(&data[..0x40]), Err(SvdError::UnexpectedEof));
    }

    #[test]
    fn empty_file_roundtrip() {
        let svd = SvdFile { chunks: Vec::new() };
        let data = svd.write();
        assert_eq!(data, b"\x00\x0ESVD1\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(SvdFile::parse(&data), Ok(svd));
    }
}
