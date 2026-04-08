# Undocumented INTEGRA-7 SysEx Behavior

This document records SysEx behaviors discovered through reverse-engineering
the Roland iPad Editor app's communication with the INTEGRA-7. These are NOT
part of the official MIDI Implementation (v1.00) and may change with firmware
updates.

**Discovery method:** MIDI traffic capture using `integral monitor` while
operating the iPad app, followed by systematic probing of address and payload
variations via the CLI.

---

## 1. Preset/User Name Catalog Query

### Purpose

Read the names of all 64 Studio Sets (or potentially other preset categories)
without loading them into the Temporary area. This is how the iPad app
populates the Studio Set selector dropdown.

### Request Format

A short-form RQ1 to address `0F 00 03 02` with a 3-byte payload:

```
F0 41 <dev> 00 00 64 11 0F 00 03 02 <MSB> <LSB> <start> <checksum> F7
```

| Byte(s)       | Value          | Description                                |
|---------------|----------------|--------------------------------------------|
| `F0`          | `F0`           | SysEx start                                |
| `41`          | `41`           | Roland manufacturer ID                     |
| `dev`         | `10`–`1F`     | Device ID                                  |
| `00 00 64`    | `00 00 64`     | INTEGRA-7 model ID                         |
| `11`          | `11`           | Command ID (RQ1)                           |
| `0F 00 03 02` | `0F 00 03 02`  | Catalog query address (undocumented)       |
| `MSB`         | e.g. `55`      | Bank Select MSB of the category to query   |
| `LSB`         | e.g. `00`      | Bank Select LSB                            |
| `start`       | `00`–`3F`     | Starting program number (0-indexed)        |
| `checksum`    |                | Roland checksum over addr + MSB + LSB + start |
| `F7`          | `F7`           | SysEx end                                  |

**Note:** This is a non-standard RQ1 — the "size" field is only 3 bytes
(`MSB LSB start`) instead of the standard 4 bytes. The device accepts this
shortened format and responds normally.

### Known Bank Values

| MSB  | LSB  | Category        | Entries | Notes                          |
|------|------|-----------------|---------|--------------------------------|
| `55` | `00` | Studio Sets     | 64      | 1–16 factory, 17–64 user      |

Other MSB/LSB combinations likely work for tone categories (PCM Synth, SN
Acoustic, etc.) but have not been tested yet. The Bank Select values match
the standard bank select table from the MIDI Implementation.

### Response Format

The device responds with multiple DT1 messages at the same address
(`0F 00 03 02`), one per entry from `start` through the last available
entry. Each response has 21 data bytes:

```
F0 41 <dev> 00 00 64 12 0F 00 03 02 <data[21]> <checksum> F7
```

Data layout (21 bytes):

| Offset | Size | Description                              |
|--------|------|------------------------------------------|
| 0      | 1    | Bank Select MSB (echo of request MSB)    |
| 1      | 1    | Bank Select LSB (echo of request LSB)    |
| 2      | 1    | Program number (0-indexed)               |
| 3      | 1    | `00` (padding/reserved)                  |
| 4      | 1    | `00` (padding/reserved)                  |
| 5–20   | 16   | Name (ASCII, space-padded)               |

### Delimiter Messages

Between batches of responses (approximately every 8–16 entries), the device
inserts a zero-data delimiter message:

```
F0 41 10 00 00 64 12 0F 00 03 02 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 6C F7
```

All 21 data bytes are `00`. These should be ignored when parsing the
response stream.

### Batching / Lazy Loading

The iPad app requests names in batches as the user scrolls:

1. Initial load: `start=00` → receives entries 0–15 (factory presets)
2. Scroll to 17–32: `start=10` → receives entries 16 onward
3. Continue scrolling: further requests with higher start indices

Each request returns all entries from `start` through the end (entry 63),
so overlapping responses are expected. Clients should deduplicate by program
number.

### Example

**Request all Studio Set names starting from index 0:**

```
F0 41 10 00 00 64 11 0F 00 03 02 55 00 00 17 F7
```

Checksum: `0F + 00 + 03 + 02 + 55 + 00 + 00 = 69H → (128 - 105) % 128 = 23 = 17H`

**Response (first entry):**

```
F0 41 10 00 00 64 12 0F 00 03 02 55 00 00 00 00
49 6E 74 65 67 72 61 20 50 72 65 76 69 65 77 20 2B F7
```

Data bytes: `55 00 00 00 00` + "Integra Preview " (16 ASCII chars)

**Request names starting from index 4 (Techno Set):**

```
F0 41 10 00 00 64 11 0F 00 03 02 55 00 04 13 F7
```

First response: `55 00 04 00 00` + "Techno Set      "

### Verified Factory Studio Set Names (Indices 0–15)

| Index | PC | Name              |
|-------|----|-------------------|
| 0     | 1  | Integra Preview   |
| 1     | 2  | Full Orch Set     |
| 2     | 3  | Chamber Orch Set  |
| 3     | 4  | Electro Set       |
| 4     | 5  | Techno Set        |
| 5     | 6  | Rock Band Set     |
| 6     | 7  | Jazz Band Set     |
| 7     | 8  | Big Band Set      |
| 8     | 9  | Ac Pop Set        |
| 9     | 10 | R&B Set           |
| 10    | 11 | Country Set       |
| 11    | 12 | World Pop Set     |
| 12    | 13 | Keyboard Set      |
| 13    | 14 | Guitar Set        |
| 14    | 15 | House Set         |
| 15    | 16 | Game Set          |

User Studio Sets (indices 16–63) default to "INIT STUDIO".

---

## 2. User Storage Address Space

### Purpose

Direct read/write access to user-stored Studio Set data, independent of the
Temporary area.

### Address Map

| Base Address   | Description                       |
|----------------|-----------------------------------|
| `10 00 00 00`  | User Studio Set 1 (stored data)   |
| `20 NN 00 00`  | User Studio Set NN (name only?)   |

The address `20 NN 00 00` (byte 1 = set index 0–63) returns the user-stored
Studio Set name for that slot. On a factory-default device, all 64 entries
return "INIT LIVE" (not "INIT STUDIO" — the distinction between the catalog
query result and direct read is noted but not fully understood).

The address `10 00 00 00` contains the full Studio Set structure for user
slot 0, mirroring the Temporary Studio Set layout (Common at `00 00`,
Chorus at `00 04 00`, Parts at `00 20 00`–`00 2F 00`, etc.).

**Note:** Factory preset names ("Integra Preview", etc.) are NOT available
at these addresses — they are only accessible via the catalog query at
`0F 00 03 02` (Section 1 above) or by reading the Temporary Studio Set
after loading the preset.

---

## Future Investigation

- **Tone catalog queries**: The `0F 00 03 02` address with different MSB/LSB
  values (e.g., MSB=87 for PCM Synth, MSB=89 for SN Acoustic) likely returns
  tone name catalogs. This would enable a tone browser without loading each
  tone individually.

- **Expansion board catalog**: Expansion sounds (SRX, ExSN, ExPCM) may also
  be queryable via the same mechanism with their respective bank select values.

- **Other catalog addresses**: The `0F 00 03 XX` range may have additional
  catalog endpoints for different data types (e.g., `0F 00 03 00` for a
  top-level index, `0F 00 03 01` for something else).
