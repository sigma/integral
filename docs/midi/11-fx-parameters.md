# FX Parameters — Chorus and Reverb

Source: Roland INTEGRA-7 Parameter Guide (p. 98-99).

The Studio Set has one Chorus (FX1) and one Reverb (FX2) effect. Each has
a Type selector and type-dependent parameters stored as nibblized 4-byte
values at fixed offsets. The parameter names change based on the selected
type.

---

## Chorus (FX1)

Base address: `18 00 04 00`. Switch at `18 00 00 41`.

### Core Parameters

| Offset | Parameter | Range | Display |
|--------|-----------|-------|---------|
| `00` | Chorus Type | 0-3 | OFF, Chorus, Delay, GM2 Chorus |
| `01` | Chorus Level | 0-127 | 0-127 |
| `02` | (reserved) | — | — |
| `03` | Chorus Output Select | 0-2 | MAIN, REV, MAIN+REV |

### Type 00: OFF

No parameters.

### Type 01: Chorus

| Param # | Offset | Name | Range | Unit |
|---------|--------|------|-------|------|
| 1 | `04` | Filter Type | 0-2 | OFF, LPF, HPF |
| 2 | `08` | Cutoff Freq | 200-8000 | Hz |
| 3 | `0C` | Pre Delay | 0.0-100.0 | msec |
| 4 | `10` | Rate | 0.05-10.00 | Hz (or note) |
| 5 | `14` | Depth | 0-127 | — |
| 6 | `18` | Phase | 0-180 | deg |
| 7 | `1C` | Feedback | 0-127 | — |
| 8 | `20` | Send Level to Reverb | 0-127 | — |

### Type 02: Delay

| Param # | Offset | Name | Range | Unit |
|---------|--------|------|-------|------|
| 1 | `04` | Delay Left | 0-1000 | msec (or note) |
| 2 | `08` | Delay Right | 0-1000 | msec (or note) |
| 3 | `0C` | Delay Center | 0-1000 | msec (or note) |
| 4 | `10` | Center Feedback | -98 to +98 | % |
| 5 | `14` | HF Damp | 200-8000, BYPASS | Hz |
| 6 | `18` | Left Level | 0-127 | — |
| 7 | `1C` | Right Level | 0-127 | — |
| 8 | `20` | Center Level | 0-127 | — |
| 9 | `24` | Send Level to Reverb | 0-127 | — |

### Type 03: GM2 Chorus

| Param # | Offset | Name | Range | Unit |
|---------|--------|------|-------|------|
| 1 | `04` | Pre-LPF | 0-7 | — |
| 2 | `08` | Level | 0-127 | — |
| 3 | `0C` | Feedback | 0-127 | — |
| 4 | `10` | Delay | 0-127 | — |
| 5 | `14` | Rate | 0-127 | — |
| 6 | `18` | Depth | 0-127 | — |
| 7 | `1C` | Send Level to Reverb | 0-127 | — |

---

## Reverb (FX2)

Base address: `18 00 06 00`. Switch at `18 00 00 40`.

### Core Parameters

| Offset | Parameter | Range | Display |
|--------|-----------|-------|---------|
| `00` | Reverb Type | 0-6 | OFF, Room 1, Room 2, Hall 1, Hall 2, Plate, GM2 Reverb |
| `01` | Reverb Level | 0-127 | 0-127 |
| `02` | Reverb Output Assign | 0-3 | A, B, C, D |

### Type 00: OFF

No parameters.

### Types 01-05: Room 1, Room 2, Hall 1, Hall 2, Plate

| Param # | Offset | Name | Range | Unit |
|---------|--------|------|-------|------|
| 1 | `03` | Pre Delay | 0-100 | msec |
| 2 | `07` | Time | 0.1-10.0 | sec |
| 3 | `0B` | Density | 0-127 | — |
| 4 | `0F` | Diffusion | 0-127 | — |
| 5 | `13` | LF Damp | 0-100 | — |
| 6 | `17` | HF Damp | 0-100 | — |
| 7 | `1B` | Spread | 0-127 | — |
| 8 | `1F` | Tone | 0-127 | — |

### Type 06: GM2 Reverb

| Param # | Offset | Name | Range | Unit |
|---------|--------|------|-------|------|
| 1 | `03` | Character | 0-5 | — |
| 2 | `07` | Time | 0-127 | — |

---

## Nibblized Parameter Encoding

All type-dependent parameters are stored as 4-byte nibblized values.

**Encoding:** A 16-bit value with offset 32768 (center = 0) is split into
4 nibbles transmitted as individual bytes:

```
Value = raw_16bit - 32768  (display range: -20000 to +20000)
Byte 0 = (raw_16bit >> 12) & 0x0F
Byte 1 = (raw_16bit >> 8)  & 0x0F
Byte 2 = (raw_16bit >> 4)  & 0x0F
Byte 3 = raw_16bit & 0x0F
```

**Decoding:**
```
raw_16bit = (byte0 << 12) | (byte1 << 8) | (byte2 << 4) | byte3
display_value = raw_16bit - 32768
```

**Example:** A value of `08 00 00 02` decodes as:
`(8 << 12) | (0 << 8) | (0 << 4) | 2 = 32770`, display = 32770 - 32768 = **+2**.

Note: For most parameters, the display value maps directly to the parameter's
native range (e.g., Level 0-127 stores as 32768-32895). The offset ensures
all stored values are positive.
