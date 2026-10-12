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

/// Signature of the JV/XP-era SVD files, whose payload isn't decoded.
///
/// Reference: `docs/research/xv-svd-format.md`.
const SIGNATURE_SVD0: [u8; 4] = *b"SVD0";

/// Model identifiers of XV-series SVD directory entries, which share the
/// `SVD1` signature.
const XV_MODELS: [[u8; 4]; 2] = [*b"XV88", *b"NEXY"];

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

    /// A directory entry has a model identifier other than its family's
    /// (`MI69` for the INTEGRA-7).
    #[error("invalid model identifier")]
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

/// Device-level description of one [`ChunkType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkTypeSpec {
    /// The type this descriptor describes.
    pub kind: ChunkType,
    /// 4-byte ASCII chunk type code in the SVD directory.
    pub code: [u8; 4],
    /// Kebab-case identifier for CLI filtering.
    pub cli_name: &'static str,
    /// Human-readable label.
    pub label: &'static str,
    /// Entry size in bytes.
    pub entry_size: usize,
    /// Patch name length in characters, `None` for unnamed entries.
    pub name_length: Option<usize>,
}

/// Every chunk type, in SVD directory order and in [`ChunkType`]
/// declaration order (so a type's discriminant indexes its descriptor).
const CHUNK_TYPES: [ChunkTypeSpec; 7] = [
    ChunkTypeSpec {
        kind: ChunkType::StudioSet,
        code: *b"PRFb",
        cli_name: "studio-set",
        label: "Studio Set",
        entry_size: 1068,
        name_length: Some(16),
    },
    ChunkTypeSpec {
        kind: ChunkType::PcmSynthTone,
        code: *b"RFPa",
        cli_name: "pcm-synth",
        label: "PCM Synth Tone",
        entry_size: 590,
        name_length: Some(12),
    },
    ChunkTypeSpec {
        kind: ChunkType::PcmDrumKit,
        code: *b"RFRa",
        cli_name: "pcm-drum",
        label: "PCM Drum Kit",
        entry_size: 10890,
        name_length: Some(12),
    },
    ChunkTypeSpec {
        kind: ChunkType::SnSynthTone,
        code: *b"SHPa",
        cli_name: "sn-synth",
        label: "SN Synth Tone",
        entry_size: 280,
        name_length: Some(12),
    },
    ChunkTypeSpec {
        kind: ChunkType::SnAcousticTone,
        code: *b"SNTa",
        cli_name: "sn-acoustic",
        label: "SN Acoustic Tone",
        entry_size: 138,
        name_length: Some(12),
    },
    ChunkTypeSpec {
        kind: ChunkType::SnDrumKit,
        code: *b"SDKa",
        cli_name: "sn-drum",
        label: "SN Drum Kit",
        entry_size: 1006,
        name_length: Some(12),
    },
    ChunkTypeSpec {
        kind: ChunkType::SystemSettings,
        code: *b"SYSa",
        cli_name: "system",
        label: "System Settings",
        entry_size: 108,
        name_length: None,
    },
];

/// Chunks of a SOUND export file, in directory order. A BACKUP adds
/// [`ChunkType::SystemSettings`] after them.
pub const SOUND_CHUNKS: [ChunkType; 6] = [
    ChunkType::StudioSet,
    ChunkType::PcmSynthTone,
    ChunkType::PcmDrumKit,
    ChunkType::SnSynthTone,
    ChunkType::SnAcousticTone,
    ChunkType::SnDrumKit,
];

// `ChunkType::spec` indexes the table by discriminant.
const _: () = {
    let mut i = 0;
    while i < CHUNK_TYPES.len() {
        assert!(CHUNK_TYPES[i].kind as usize == i);
        i += 1;
    }
};

impl ChunkType {
    /// The descriptor of this type.
    pub fn spec(self) -> &'static ChunkTypeSpec {
        &CHUNK_TYPES[self as usize]
    }

    /// Parse a 4-byte ASCII chunk type code into a [`ChunkType`].
    ///
    /// Returns `None` if the code is not recognised.
    pub fn from_code(code: &[u8; 4]) -> Option<Self> {
        CHUNK_TYPES.iter().find(|s| s.code == *code).map(|s| s.kind)
    }

    /// Return the 4-byte ASCII chunk type code for this variant.
    pub fn to_code(&self) -> [u8; 4] {
        self.spec().code
    }

    /// Kebab-case identifier for CLI filtering.
    pub fn cli_name(&self) -> &'static str {
        self.spec().cli_name
    }

    /// The type whose [`ChunkType::cli_name`] is `name`.
    pub fn from_cli_name(name: &str) -> Option<Self> {
        CHUNK_TYPES
            .iter()
            .find(|s| s.cli_name == name)
            .map(|s| s.kind)
    }

    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        self.spec().label
    }

    /// Return the default entry size in bytes for this chunk type.
    pub fn default_entry_size(&self) -> usize {
        self.spec().entry_size
    }

    /// Return the length (in characters) of patch names for this chunk type.
    ///
    /// Studio Sets use 16-character names; all tone/drum types use 12.
    /// System settings have no name (`None`).
    pub fn name_length(&self) -> Option<usize> {
        self.spec().name_length
    }
}

// ---------------------------------------------------------------------------
// Tone Category
// ---------------------------------------------------------------------------

include!(concat!(env!("OUT_DIR"), "/tone_category_data.rs"));

/// Category of every drum kit: drum kits carry no Tone Category byte.
pub const DRUMS_CATEGORY: u8 = 46;

impl fmt::Display for ChunkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// Serialized as its [`ChunkType::cli_name`].
#[cfg(feature = "serde")]
impl serde::Serialize for ChunkType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.cli_name())
    }
}

/// Deserialized from its [`ChunkType::cli_name`].
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ChunkType {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let name = String::deserialize(d)?;
        Self::from_cli_name(&name)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown type: {name}")))
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
    let spec = match chunk_type {
        ChunkType::SnSynthTone => &SNS_TONE_SPEC,
        ChunkType::SnAcousticTone => &SNA_TONE_SPEC,
        ChunkType::PcmSynthTone => &PCMS_TONE_SPEC,
        _ => return None,
    };
    blocks_category(&svd_to_sysex(entry, spec).ok()?, chunk_type)
}

/// The tone category byte of an entry decoded into SysEx blocks (in its
/// Temporary layout), if the type has one.
pub fn blocks_category(blocks: &[Vec<u8>], chunk_type: ChunkType) -> Option<u8> {
    let (block, offset) = match chunk_type {
        ChunkType::SnSynthTone => (0, 0x36),
        ChunkType::SnAcousticTone => (0, 0x1B),
        // Common 2 `00 10`.
        ChunkType::PcmSynthTone => (7, 0x10),
        _ => return None,
    };
    blocks.get(block)?.get(offset).copied()
}

// ---------------------------------------------------------------------------
// SvdChunk / SvdFile
// ---------------------------------------------------------------------------

/// A single chunk within an SVD file, containing zero or more fixed-size
/// entries. `K` is the type of its data: [`ChunkType`] in an INTEGRA-7 file,
/// [`XvChunkType`] in an XV one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvdChunk<K = ChunkType> {
    /// The type of patch data in this chunk.
    pub chunk_type: K,
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

/// The synth family that wrote an SVD file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvdFamily {
    /// INTEGRA-7 (`SVD1`, model `MI69`), or any other `SVD1` file.
    Integra,
    /// XV series (`SVD1`, a directory model `XV88` or `NEXY`).
    Xv,
    /// JV/XP series (`SVD0`).
    JvXp,
}

impl SvdFamily {
    /// The family of `data` from its signature and directory model IDs, or
    /// `None` if it is not an SVD file.
    pub fn of(data: &[u8]) -> Option<Self> {
        let signature = data.get(LENGTH_FIELD_SIZE..LENGTH_FIELD_SIZE + SIGNATURE.len())?;
        if signature == SIGNATURE_SVD0 {
            return Some(Self::JvXp);
        }
        if signature != SIGNATURE {
            return None;
        }
        let header_length = usize::from(u16::from_be_bytes([data[0], data[1]]));
        let dir_end = (header_length + LENGTH_FIELD_SIZE).min(data.len());
        let xv = data
            .get(HEADER_SIZE..dir_end)
            .unwrap_or_default()
            .chunks_exact(DIR_ENTRY_SIZE)
            .any(|entry| XV_MODELS.iter().any(|m| entry[4..8] == *m));
        Some(if xv { Self::Xv } else { Self::Integra })
    }
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

/// Read the chunks of an `SVD1` container, whatever the family. `kind_of`
/// turns each directory entry's chunk code and model ID into the chunk's
/// type, or rejects the file.
fn parse_chunks<K>(
    data: &[u8],
    kind_of: impl Fn(&[u8; 4], &[u8; 4]) -> Result<K, SvdError>,
) -> Result<Vec<SvdChunk<K>>, SvdError> {
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
    for entry in data[HEADER_SIZE..dir_end].chunks_exact(DIR_ENTRY_SIZE) {
        let code: [u8; 4] = entry[0..4].try_into().expect("4-byte slice");
        let model: [u8; 4] = entry[4..8].try_into().expect("4-byte slice");
        let chunk_type = kind_of(&code, &model)?;

        let chunk_offset = read_u32_be(&entry[8..12]) as usize;
        let chunk_size = read_u32_be(&entry[12..16]) as usize;

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
    Ok(chunks)
}

// ---------------------------------------------------------------------------
// XV series
// ---------------------------------------------------------------------------

/// What an XV-series SVD chunk holds.
///
/// Reference: `docs/research/xv-svd-format.md`, section 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XvChunkType {
    /// Patches (`PATa`).
    Patch,
    /// Performances (`PRFa`, and `MLTa` on the XV-2020).
    Performance,
    /// Rhythm Sets (`RHYa`).
    Rhythm,
    /// System settings (`SYSa`, `SYSb`).
    System,
    /// Data about the other chunks, or for a host sequencer: Patch
    /// categories (`CTGa`), the Favorites list (`FVRa`), and `SEQa`, `VIPa`
    /// and `VIFa`, empty in every file seen.
    Aux,
}

/// Every XV-series chunk code and what its chunk holds.
const XV_CHUNK_TYPES: [([u8; 4], XvChunkType); 11] = [
    (*b"PATa", XvChunkType::Patch),
    (*b"PRFa", XvChunkType::Performance),
    (*b"MLTa", XvChunkType::Performance),
    (*b"RHYa", XvChunkType::Rhythm),
    (*b"SYSa", XvChunkType::System),
    (*b"SYSb", XvChunkType::System),
    (*b"CTGa", XvChunkType::Aux),
    (*b"FVRa", XvChunkType::Aux),
    (*b"SEQa", XvChunkType::Aux),
    (*b"VIPa", XvChunkType::Aux),
    (*b"VIFa", XvChunkType::Aux),
];

impl XvChunkType {
    /// Parse a 4-byte ASCII XV chunk code, or `None` if it is not one.
    pub fn from_code(code: &[u8; 4]) -> Option<Self> {
        XV_CHUNK_TYPES
            .iter()
            .find(|(c, _)| c == code)
            .map(|&(_, kind)| kind)
    }
}

/// Parse an XV-series SVD file (see [`SvdFamily::Xv`]): the INTEGRA-7
/// container with XV chunk codes and models `XV88` and `NEXY`. Empty chunks
/// record entry size 0.
pub fn parse_xv(data: &[u8]) -> Result<Vec<SvdChunk<XvChunkType>>, SvdError> {
    parse_chunks(data, |code, model| {
        let kind = XvChunkType::from_code(code).ok_or(SvdError::InvalidChunkType(*code))?;
        if !XV_MODELS.contains(model) {
            return Err(SvdError::InvalidModel);
        }
        Ok(kind)
    })
}

// ---------------------------------------------------------------------------
// SvdFile implementation
// ---------------------------------------------------------------------------

impl SvdFile {
    /// Parse an INTEGRA-7 SVD file from raw bytes.
    ///
    /// Validates the header magic, reads the chunk directory, and extracts
    /// each chunk's zone header and entries.
    pub fn parse(data: &[u8]) -> Result<Self, SvdError> {
        let chunks = parse_chunks(data, |code, model| {
            let chunk_type = ChunkType::from_code(code).ok_or(SvdError::InvalidChunkType(*code))?;
            if *model != MODEL_MI69 {
                return Err(SvdError::InvalidModel);
            }
            Ok(chunk_type)
        })?;
        Ok(SvdFile { chunks })
    }

    /// A SOUND export file: all of [`SOUND_CHUNKS`] in order, each with its
    /// type's entry size, holding the `entries` of its type in input order.
    /// Types with no entries get an empty chunk; entries of types outside
    /// the SOUND shape are ignored.
    pub fn sound(entries: impl IntoIterator<Item = (ChunkType, Vec<u8>)>) -> Self {
        let mut chunks = SOUND_CHUNKS.map(|kind| SvdChunk {
            chunk_type: kind,
            entry_size: kind.default_entry_size(),
            entries: Vec::new(),
        });
        for (kind, entry) in entries {
            if let Some(chunk) = chunks.iter_mut().find(|c| c.chunk_type == kind) {
                chunk.entries.push(entry);
            }
        }
        Self {
            chunks: chunks.into(),
        }
    }

    /// Serialize this SVD file to raw bytes.
    ///
    /// Writes the header, chunk directory, and all zone headers with their
    /// entries, in `chunks` order. Empty chunks still get a zone header with
    /// count=0 and the correct entry size. Build a SOUND file with
    /// [`SvdFile::sound`]; a BACKUP adds `SYSa` as the seventh chunk.
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
    fn drums_category_is_named_drums() {
        assert_eq!(tone_category_name(DRUMS_CATEGORY), Some("Drums"));
    }

    #[test]
    fn chunk_type_roundtrip() {
        for spec in &CHUNK_TYPES {
            let ct = spec.kind;
            assert_eq!(ChunkType::from_code(&ct.to_code()), Some(ct));
            assert_eq!(ChunkType::from_cli_name(ct.cli_name()), Some(ct));
        }
    }

    #[test]
    fn name_lengths() {
        assert_eq!(ChunkType::StudioSet.name_length(), Some(16));
        assert_eq!(ChunkType::SnSynthTone.name_length(), Some(12));
        assert_eq!(ChunkType::SystemSettings.name_length(), None);
    }

    #[test]
    fn family_routes_on_signature_and_model() {
        let integra = SvdFile::sound(Vec::new()).write();
        assert_eq!(SvdFamily::of(&integra), Some(SvdFamily::Integra));
        let mut xv = integra.clone();
        xv[0x14..0x18].copy_from_slice(b"XV88");
        assert_eq!(SvdFamily::of(&xv), Some(SvdFamily::Xv));
        let mut jv = integra.clone();
        jv[2..6].copy_from_slice(b"SVD0");
        assert_eq!(SvdFamily::of(&jv), Some(SvdFamily::JvXp));
        assert_eq!(SvdFamily::of(b"\xF0\x41"), None);
        assert_eq!(SvdFamily::of(b"MThd\0\0\0\x06"), None);
    }

    /// An XV file: a `PATa` chunk of one 4-byte entry and an empty `MLTa`.
    fn xv_file(patch_model: &[u8; 4]) -> Vec<u8> {
        let mut data = b"\x00\x2ESVD1\0\0\0\0\0\0\0\0\0\0".to_vec();
        data.extend_from_slice(b"PATa");
        data.extend_from_slice(patch_model);
        data.extend_from_slice(&[0, 0, 0, 0x30, 0, 0, 0, 0x14]);
        data.extend_from_slice(b"MLTaNEXY");
        data.extend_from_slice(&[0, 0, 0, 0x44, 0, 0, 0, 0x10]);
        data.extend_from_slice(&[0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 0, 0x10, 0, 0, 0, 0]);
        data.extend_from_slice(b"\x01\x02\x03\x04");
        data.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x10, 0, 0, 0, 0]);
        data
    }

    #[test]
    fn parses_xv_chunks() {
        let data = xv_file(b"XV88");
        assert_eq!(SvdFamily::of(&data), Some(SvdFamily::Xv));
        assert_eq!(
            parse_xv(&data),
            Ok(vec![
                SvdChunk {
                    chunk_type: XvChunkType::Patch,
                    entry_size: 4,
                    entries: vec![vec![1, 2, 3, 4]],
                },
                SvdChunk {
                    chunk_type: XvChunkType::Performance,
                    entry_size: 0,
                    entries: vec![],
                },
            ])
        );
        assert_eq!(
            SvdFile::parse(&data),
            Err(SvdError::InvalidChunkType(*b"PATa"))
        );
        assert_eq!(parse_xv(&xv_file(b"MI69")), Err(SvdError::InvalidModel));
    }

    #[test]
    fn chunk_type_unknown_code() {
        assert_eq!(ChunkType::from_code(b"XXXX"), None);
    }

    #[test]
    fn default_entry_sizes() {
        let sizes = CHUNK_TYPES.map(|s| (s.code, s.kind.default_entry_size()));
        assert_eq!(
            sizes,
            [
                (*b"PRFb", 1068),
                (*b"RFPa", 590),
                (*b"RFRa", 10890),
                (*b"SHPa", 280),
                (*b"SNTa", 138),
                (*b"SDKa", 1006),
                (*b"SYSa", 108),
            ]
        );
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
    fn sound_fills_in_the_six_chunks() {
        let entry = vec![0x2A; ChunkType::SnSynthTone.default_entry_size()];
        let data = SvdFile::sound([(ChunkType::SnSynthTone, entry.clone())]).write();
        assert_eq!(&data[..6], b"\x00\x6ESVD1");
        // First chunk starts right after the sixth directory entry.
        assert_eq!(read_u32_be(&data[0x18..0x1C]), 0x70);
        let chunks: Vec<_> = SvdFile::parse(&data)
            .unwrap()
            .chunks
            .into_iter()
            .map(|c| (c.chunk_type, c.entry_size, c.entries))
            .collect();
        let expected: Vec<_> = SOUND_CHUNKS
            .into_iter()
            .map(|k| {
                let entries = match k {
                    ChunkType::SnSynthTone => vec![entry.clone()],
                    _ => vec![],
                };
                (k, k.default_entry_size(), entries)
            })
            .collect();
        assert_eq!(chunks, expected);
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
