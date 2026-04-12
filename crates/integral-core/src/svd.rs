//! SVD container format parser and writer for Roland INTEGRA-7 backup files.
//!
//! The SVD (Sound Visual Data) format is used by Roland to store instrument
//! patches in backup files. Each file contains a header, a chunk directory,
//! and one or more chunks of typed patch entries.
//!
//! Reference: `docs/svd/01-container.md`, `docs/svd/04-chunk-types.md`

use std::fmt;
use thiserror::Error;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// SVD file magic signature: `\x00nSVD1`.
const MAGIC: [u8; 6] = [0x00, 0x6E, 0x53, 0x56, 0x44, 0x31];

/// INTEGRA-7 model identifier in SVD directory entries.
const MODEL_MI69: [u8; 4] = [b'M', b'I', b'6', b'9'];

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
    /// The file does not start with the expected `\x00nSVD1` magic signature.
    #[error("invalid SVD magic signature")]
    InvalidMagic,

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
        }
    }
}

// ---------------------------------------------------------------------------
// Tone Category
// ---------------------------------------------------------------------------

/// Human-readable names for the Integra-7 tone category values (0–35).
///
/// Source: INTEGRA-7 Owner's Manual — Tone Category parameter.
/// Values 36–127 are undocumented and display as the raw number.
const TONE_CATEGORY_NAMES: [&str; 36] = [
    "No assign",           // 0
    "Ac.Piano",            // 1
    "E.Piano",             // 2
    "Organ",               // 3
    "Other Keyboards",     // 4
    "Accordion/Harmonica", // 5
    "Bell/Mallet",         // 6
    "Ac.Guitar",           // 7
    "E.Guitar",            // 8
    "Dist.Guitar",         // 9
    "Ac.Bass",             // 10
    "E.Bass",              // 11
    "Synth Bass",          // 12
    "Plucked/Stroke",      // 13
    "Strings",             // 14
    "Brass",               // 15
    "Wind",                // 16
    "Flute",               // 17
    "Sax",                 // 18
    "Recorder",            // 19
    "Vox/Choir",           // 20
    "Synth Lead",          // 21
    "Synth Brass",         // 22
    "Synth Pad/Strings",   // 23
    "Synth Bellpad",       // 24
    "Synth PolyKey",       // 25
    "FX",                  // 26
    "Synth Seq/Pop",       // 27
    "Phrase",              // 28
    "Pulsating",           // 29
    "Beat&Groove",         // 30
    "Hit",                 // 31
    "Sound FX",            // 32
    "Drums",               // 33
    "Percussion",          // 34
    "Combination",         // 35
];

/// Return the human-readable name for a tone category value.
///
/// Returns the name for values 0–35, or `None` for undocumented values.
pub fn tone_category_name(value: u8) -> Option<&'static str> {
    TONE_CATEGORY_NAMES.get(value as usize).copied()
}

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
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
        if data[..6] != MAGIC {
            return Err(SvdError::InvalidMagic);
        }

        // Parse chunk directory starting at offset 0x10
        let mut chunks = Vec::new();
        let mut dir_offset = HEADER_SIZE;

        loop {
            if dir_offset + DIR_ENTRY_SIZE > data.len() {
                return Err(SvdError::UnexpectedEof);
            }

            // Null terminator — first byte 0x00 ends directory
            if data[dir_offset] == 0x00 {
                break;
            }

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
            if chunk_offset + chunk_size > data.len() {
                return Err(SvdError::ChunkOverflow);
            }

            // Read zone header
            if chunk_size < ZONE_HEADER_SIZE {
                return Err(SvdError::UnexpectedEof);
            }
            let zone = &data[chunk_offset..chunk_offset + ZONE_HEADER_SIZE];
            let count = read_u32_be(&zone[0..4]) as usize;
            let entry_size = read_u32_be(&zone[4..8]) as usize;

            // Extract entries
            let entries_start = chunk_offset + ZONE_HEADER_SIZE;
            let mut entries = Vec::with_capacity(count);
            for i in 0..count {
                let start = entries_start + i * entry_size;
                let end = start + entry_size;
                if end > data.len() {
                    return Err(SvdError::UnexpectedEof);
                }
                entries.push(data[start..end].to_vec());
            }

            chunks.push(SvdChunk {
                chunk_type,
                entry_size,
                entries,
            });

            dir_offset += DIR_ENTRY_SIZE;
        }

        Ok(SvdFile { chunks })
    }

    /// Serialize this SVD file to raw bytes.
    ///
    /// Writes the header, chunk directory, null terminator, and all zone
    /// headers with their entries. Empty chunks still get a zone header
    /// with count=0 and the correct entry size.
    pub fn write(&self) -> Vec<u8> {
        let num_chunks = self.chunks.len();
        // Directory size: entries + null terminator (16 bytes)
        let dir_size = num_chunks * DIR_ENTRY_SIZE + DIR_ENTRY_SIZE;
        let data_start = HEADER_SIZE + dir_size;

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
        buf.extend_from_slice(&MAGIC);
        buf.extend_from_slice(&[0u8; 10]); // padding

        // Write directory entries
        for (i, chunk) in self.chunks.iter().enumerate() {
            buf.extend_from_slice(&chunk.chunk_type.to_code());
            buf.extend_from_slice(&MODEL_MI69);
            let chunk_data_size = ZONE_HEADER_SIZE + chunk.entries.len() * chunk.entry_size;
            write_u32_be(&mut buf, chunk_offsets[i] as u32);
            write_u32_be(&mut buf, chunk_data_size as u32);
        }

        // Write null terminator entry
        buf.extend_from_slice(&[0u8; DIR_ENTRY_SIZE]);

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
        ];
        for ct in &types {
            let code = ct.to_code();
            assert_eq!(ChunkType::from_code(&code), Some(*ct));
        }
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
        let written = svd.write();
        let reparsed = SvdFile::parse(&written).expect("failed to re-parse SVD");
        assert_eq!(svd, reparsed);
    }

    #[test]
    fn error_bad_magic() {
        let mut data = vec![0u8; 32];
        // Write wrong magic
        data[0] = 0xFF;
        assert_eq!(SvdFile::parse(&data), Err(SvdError::InvalidMagic));
    }

    #[test]
    fn error_truncated_file() {
        // Too short to contain even a header
        let data = vec![0u8; 4];
        assert_eq!(SvdFile::parse(&data), Err(SvdError::UnexpectedEof));
    }

    #[test]
    fn error_truncated_header_only() {
        // Valid magic but no directory space — the null-terminator check
        // should trigger UnexpectedEof.
        let mut data = vec![0u8; HEADER_SIZE];
        data[..6].copy_from_slice(&MAGIC);
        assert_eq!(SvdFile::parse(&data), Err(SvdError::UnexpectedEof));
    }

    #[test]
    fn parse_empty_file_with_terminator() {
        // Header + immediate null terminator (no chunks)
        let mut data = vec![0u8; HEADER_SIZE + DIR_ENTRY_SIZE];
        data[..6].copy_from_slice(&MAGIC);
        let svd = SvdFile::parse(&data).expect("should parse empty SVD");
        assert!(svd.chunks.is_empty());
    }

    #[test]
    fn write_empty_file() {
        let svd = SvdFile { chunks: Vec::new() };
        let data = svd.write();
        // Header (16) + null terminator (16) = 32
        assert_eq!(data.len(), 32);
        assert_eq!(&data[..6], &MAGIC);
        // Null terminator at offset 16
        assert_eq!(data[16], 0x00);
    }
}
