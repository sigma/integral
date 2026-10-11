# SVD Container Format

## File Header (16 bytes)

| Offset | Size | Content          | Notes                                         |
|--------|------|------------------|-----------------------------------------------|
| 0x00   | 2    | u16 BE           | Header length = directory end − 2             |
| 0x02   | 4    | `SVD1`           | Fixed signature                               |
| 0x06   | 10   | `\x00` × 10      | Always zero                                   |

Bytes 0–1 are **not** part of a fixed magic. They hold the length of
everything after them up to the end of the chunk directory:
`header_length = 0x10 + 16 × chunk_count − 2`. The INTEGRA-7 writes two values:

| File kind | Chunks | Header length | Directory end |
|-----------|--------|---------------|---------------|
| SOUND     | 6      | `00 6E` (110) | `0x70`        |
| BACKUP    | 7      | `00 7E` (126) | `0x80`        |

See [SOUND vs BACKUP](11-sound-vs-backup.md) for what each kind holds.

## Chunk Directory (offset 0x10)

Immediately after the header, a sequence of 16-byte directory entries
describes the chunks in the file. There is **no terminator entry**. The header
length bounds the directory, so
`chunk_count = (header_length + 2 − 0x10) / 16`. Chunk data starts right after
the last directory entry.

Each directory entry:

| Offset | Size | Type    | Content                              |
|--------|------|---------|--------------------------------------|
| 0      | 4    | ASCII   | Chunk type code (e.g. `SHPa`)       |
| 4      | 4    | ASCII   | Model identifier (`MI69` = INTEGRA-7)|
| 8      | 4    | u32 BE  | Absolute file offset to chunk data   |
| 12     | 4    | u32 BE  | Total chunk data size in bytes       |

The model identifier `MI69` is constant for all INTEGRA-7 SVD files. Other
Roland models use different identifiers (e.g. `XP50`, `ZCOR`).

## Zone Header (16 bytes at chunk offset)

Each chunk's data begins with a 16-byte zone header:

| Offset | Size | Type    | Content                              |
|--------|------|---------|--------------------------------------|
| 0      | 4    | u32 BE  | Number of entries in this chunk      |
| 4      | 4    | u32 BE  | Size of each entry in bytes          |
| 8      | 4    | u32 BE  | Unknown (always `0x00000010`)        |
| 12     | 4    | u32 BE  | Unknown (always `0x00000000`)        |

The entry size is constant per chunk type (see [Chunk Type Reference](04-chunk-types.md)).

Entries follow immediately after the zone header. Total chunk data size =
16 (zone header) + count × entry_size.

Chunks with zero entries still have a zone header (count = 0), and the
entry_size field still carries the correct size for that chunk type. This is
useful for determining entry sizes from files that don't contain patches of
a particular type.

## Entry Structure

Each entry is a fixed-size byte array made of **bit-packed sections**. Each
section is byte-aligned and has a fixed byte size per chunk type (see
[Bit-Packing Encoding](02-encoding.md)). What follows the last section depends
on the type:

| Chunk type                    | After the last section                           |
|-------------------------------|--------------------------------------------------|
| SHPa                          | `0x0E` end marker, then 33 bytes (mostly zero; see [SN Synth](03-sn-synth.md#bytes-after-the-end-marker)) |
| RFPa, RFRa                    | Zero padding (2 and 1 bytes), no end marker      |
| SNTa, SDKa, PRFb              | Nothing: the sections fill the entry exactly     |
| SYSa                          | Nothing: the entry ends in undocumented system data (see [System](10-system.md)) |

**Only SHPa has an end marker.** A reader must not look for `0x0E` in any
other type.

## Example: Synth Legends SOUND File (70,224 bytes)

```
Header:     00 6E 53 56 44 31 00 00 00 00 00 00 00 00 00 00
            ^^^^^ header length 0x6E: directory ends at 0x70
                  ^^^^^^^^^^^ "SVD1"

Directory:
  PRFb MI69  offset=0x00070  size=0x08590   (32 studio sets)
  RFPa MI69  offset=0x08600  size=0x00010   (0 PCM synth tones)
  RFRa MI69  offset=0x08610  size=0x00010   (0 PCM drum kits)
  SHPa MI69  offset=0x08620  size=0x08C10   (128 SN synth tones)
  SNTa MI69  offset=0x11230  size=0x00010   (0 SN acoustic tones)
  SDKa MI69  offset=0x11240  size=0x00010   (0 SN drum kits)
```

The first chunk starts at `0x70`, directly after the sixth directory entry.
A BACKUP file has a seventh entry (`SYSa`), header length `00 7E`, and its
first chunk at `0x80`.
