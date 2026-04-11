# SVD Container Format

## File Header (16 bytes)

| Offset | Size | Content          | Notes                        |
|--------|------|------------------|------------------------------|
| 0x00   | 6    | `\x00nSVD1`     | Magic signature              |
| 0x06   | 10   | `\x00` × 10     | Padding (always zero)        |

## Chunk Directory (offset 0x10)

Immediately after the header, a sequence of 16-byte directory entries
describes the chunks in the file. The directory is terminated by an entry
whose first byte is `0x00`.

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

Each entry is a fixed-size byte array containing:

1. **Bit-packed parameter data** — one or more sections, each independently
   byte-aligned (see [Bit-Packing Encoding](02-encoding.md))
2. **End marker** — a single byte `0x0E`
3. **Zero padding** — fills the remainder of the entry to reach the fixed size

```
[section 0 bits | align] [section 1 bits | align] ... [0x0E] [0x00 ...]
```

## Example: Synth Legends SVD (70,224 bytes)

```
Header:     00 6E 53 56 44 31 00 00 00 00 00 00 00 00 00 00

Directory:
  PRFb MI69  offset=0x00070  size=0x08590   (32 studio sets)
  RFPa MI69  offset=0x08600  size=0x00010   (0 PCM synth tones)
  RFRa MI69  offset=0x08610  size=0x00010   (0 PCM drum kits)
  SHPa MI69  offset=0x08620  size=0x08C10   (128 SN synth tones)
  SNTa MI69  offset=0x11230  size=0x00010   (0 SN acoustic tones)
  SDKa MI69  offset=0x11240  size=0x00010   (0 SN drum kits)
```
