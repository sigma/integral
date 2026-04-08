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

Read the names of all 64 Studio Sets (both factory presets and user sets)
without loading them into the Temporary area. This is how the iPad app
populates the Studio Set selector dropdown.

### Request Format

A short-form RQ1 to address `0F 00 03 02` with **no checksum**:

```
F0 41 <dev> 00 00 64 11 0F 00 03 02 <MSB> <LSB> <start> F7
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
| `F7`          | `F7`           | SysEx end                                  |

**Important:** This command does NOT use a Roland checksum. The byte
immediately before `F7` is the starting program number, not a checksum.
If an extra byte is appended (e.g. a computed checksum), the device
interprets it as part of the payload, changing the effective start index.

### Known Addresses and Bank Values

**Address `0F 00 03 02` — Studio Set catalog:**

| MSB  | LSB  | Category        | Entries | Notes                          |
|------|------|-----------------|---------|--------------------------------|
| `55` | `00` | Studio Sets     | 64      | 0–15 factory, 16–63 user      |

**Address `0F 00 04 02` — Tone catalog:**

| MSB  | LSB  | Category        | Entries tested | Notes                 |
|------|------|-----------------|----------------|-----------------------|
| `59` | `40` | SN Acoustic Preset | 247         | Full Grand 1, etc.    |
| `57` | `40` | PCM Synth Preset   | 247         | 128voicePno, etc.     |
| `5F` | `40` | SN Synth Preset    | 247         | JP8 Strings1, etc.    |

Other MSB/LSB combinations (User banks, SRX expansions, GM2, Drum kits)
likely work at the same `0F 00 04 02` address but have not been tested.
The Bank Select MSB/LSB values match the standard bank select table from
the MIDI Implementation.

**Note:** The padding bytes (offsets 3-4 in each response) are always `00`
for tones — no category information is included. The iPad app's "By Category"
grouping must use an internal lookup table.

### Response Format

The device responds with multiple standard DT1 messages at address
`0F 00 03 02`, one per entry. The responses use normal DT1 framing
including a valid Roland checksum:

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

The device inserts zero-data delimiter messages at irregular intervals
within the response stream:

```
F0 41 10 00 00 64 12 0F 00 03 02 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 6C F7
```

All 21 data bytes are `00`. These should be filtered out when parsing.
The delimiters do NOT indicate the end of the stream — more entries may
follow after a delimiter.

### Response Behavior

With `start=00`, the device returns all 64 entries (plus delimiters) in a
single stream. The entries include both factory preset names ("Integra
Preview", "Techno Set", etc. for indices 0–15) and user set names ("INIT
STUDIO" for unmodified user slots 16–63).

The total response is approximately 65 messages (64 entries + delimiters)
and takes several seconds to arrive. Clients should collect responses until
either all 64 unique program numbers have been seen, or a sufficient silence
timeout (3+ seconds) has elapsed.

### Example

**Request all Studio Set names starting from index 0:**

```
F0 41 10 00 00 64 11 0F 00 03 02 55 00 00 F7
```

No checksum. The `00` before `F7` is the start index.

**Response (first entry):**

```
F0 41 10 00 00 64 12 0F 00 03 02 55 00 00 00 00
49 6E 74 65 67 72 61 20 50 72 65 76 69 65 77 20 2B F7
```

Data bytes: `55 00 00 00 00` + "Integra Preview " (16 ASCII chars)

**Request names starting from index 4 (Techno Set):**

```
F0 41 10 00 00 64 11 0F 00 03 02 55 00 04 F7
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

| Base Address   | Description                            |
|----------------|----------------------------------------|
| `10 00 00 00`  | User Studio Set 1 (full structure)     |
| `20 NN 00 00`  | User Studio Set by index (NN = 0–63)  |

The address `20 NN 00 00` (byte 1 = set index 0–63) provides access to
user-stored Studio Set data. On a factory-default device, all 64 slots
return "INIT LIVE" as the name — this is the user storage default, distinct
from the factory preset names returned by the catalog query.

The address `10 00 00 00` contains the full Studio Set structure for user
slot 0, mirroring the Temporary Studio Set layout (Common at `00 00`,
Chorus at `00 04 00`, Parts at `00 20 00`–`00 2F 00`, etc.).

**Note:** Factory preset names ("Integra Preview", etc.) are NOT accessible
via user storage addresses. They are only available via:
1. The catalog query at `0F 00 03 02` (Section 1 above)
2. Reading the Temporary Studio Set at `18 00 00 00` after loading the preset

---

## Future Investigation

- **Expansion board tone catalogs**: SRX, ExSN, ExPCM tones at `0F 00 04 02`
  with their respective MSB/LSB values (MSB 93, 92, 97, 96, etc.) — untested
  but expected to work.

- **User tone catalogs**: User tone banks (LSB 0-3 instead of 64+) likely
  return user-defined tone names via the same mechanism.

- **Drum kit catalogs**: MSB 88 (SN Drum) and MSB 86 (PCM Drum) catalogs —
  untested.

- **Other catalog addresses**: The `0F 00 XX 02` range may have additional
  catalog endpoints (we know `03 02` = Studio Sets, `04 02` = Tones).
