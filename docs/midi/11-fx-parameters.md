# FX Parameters — Chorus and Reverb

Sources: Roland INTEGRA-7 Parameter Guide (p. 98-99) for names and ranges;
Roland's INTEGRA-7 Editor for the slot layout and encodings, checked against
device dumps. See [fx-param-layout.md](../research/fx-param-layout.md).

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
| `02` | Chorus Output Assign | 0-3 | A, B, C, D |
| `03` | Chorus Output Select | 0-2 | MAIN, REV, MAIN+REV |

The MIDI Implementation lists `02` as reserved. The Parameter Guide and
Roland's editor treat it as Output Assign. The device agrees: it stores 0-3
and reads back 0 for anything higher.

### Type 00: OFF

No parameters.

### Value tables

Stored values are indices into these tables (Roland INTEGRA-7 Editor; see
[the layout research](../research/fx-param-layout.md#value-tables-r3)).

| Name | Index | Display |
|------|-------|---------|
| `NOTE` | 0-21 | 1/64T, 1/64, 1/32T, 1/32, 1/16T, 1/32., 1/16, 1/8T, 1/16., 1/8, 1/4T, 1/8., 1/4, 1/2T, 1/4., 1/2, 1/1T, 1/2., 1/1, 2/1T, 1/1., 2/1 |
| `FREQ` | 0-16 | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz |
| `FREQ_BYPASS` | 0-17 | `FREQ`, then 17 = BYPASS |
| `PREDELAY` | 0-125 | 0-49: 0.0-4.9 ms (step 0.1); 50-59: 5.0-9.5 (0.5); 60-99: 10-49 (1); 100-125: 50-100 (2) |

### Type 01: Chorus

| Param # | Offset | Name | Range | Display |
|---------|--------|------|-------|---------|
| 1 | `04` | Filter Type | 0-2 | OFF, LPF, HPF |
| 2 | `08` | Cutoff Freq | 0-16 | `FREQ` |
| 3 | `0C` | Pre Delay | 0-125 | `PREDELAY` |
| 4 | `10` | Rate Hz/Note switch | 0-1 | Hz, NOTE |
| 5 | `14` | Rate (Hz) | 1-200 | × 0.05 Hz (0.05-10.00) |
| 6 | `18` | Rate (note) | 0-21 | `NOTE` |
| 7 | `1C` | Depth | 0-127 | — |
| 8 | `20` | Phase | 0-90 | × 2 deg (0-180) |
| 9 | `24` | Feedback | 0-127 | — |

### Type 02: Delay

| Param # | Offset | Name | Range | Display |
|---------|--------|------|-------|---------|
| 1 | `04` | Delay Left msec/Note switch | 0-1 | msec, NOTE |
| 2 | `08` | Delay Left (msec) | 0-1000 | msec |
| 3 | `0C` | Delay Left (note) | 0-21 | `NOTE` |
| 4 | `10` | Delay Right msec/Note switch | 0-1 | msec, NOTE |
| 5 | `14` | Delay Right (msec) | 0-1000 | msec |
| 6 | `18` | Delay Right (note) | 0-21 | `NOTE` |
| 7 | `1C` | Delay Center msec/Note switch | 0-1 | msec, NOTE |
| 8 | `20` | Delay Center (msec) | 0-1000 | msec |
| 9 | `24` | Delay Center (note) | 0-21 | `NOTE` |
| 10 | `28` | Center Feedback | 0-98 | % = 2v − 98 (−98 to +98) |
| 11 | `2C` | HF Damp | 0-17 | `FREQ_BYPASS` |
| 12 | `30` | Left Level | 0-127 | — |
| 13 | `34` | Right Level | 0-127 | — |
| 14 | `38` | Center Level | 0-127 | — |

Slots 15-20 are unused. There is no Send Level to Reverb.

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

| Param # | Offset | Name | Range | Display |
|---------|--------|------|-------|---------|
| 1 | `03` | (hidden level) | 0-127 | not shown; don't write |
| 2 | `07` | Pre Delay | 0-100 | msec |
| 3 | `0B` | Time | 1-100 | ÷ 10 sec (0.1-10.0) |
| 4 | `0F` | Density | 0-127 | — |
| 5 | `13` | Diffusion | 0-127 | — |
| 6 | `17` | LF Damp | 0-100 | — |
| 7 | `1B` | HF Damp | 0-100 | — |
| 8 | `1F` | Spread | 0-127 | — |
| 9 | `23` | Tone | 0-127 | — |

### Type 06: GM2 Reverb

| Param # | Offset | Name | Range | Display |
|---------|--------|------|-------|---------|
| 1 | `03` | Character | 0-5 | — |
| 2 | `07` | (hidden level) | 0-127 | not shown |
| 3 | `0B` | (hidden pre delay) | 0-100 | not shown |
| 4 | `0F` | Time | 0-127 | — |

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

Note: the decoded value is the parameter's stored index (e.g., Level 0-127
stores as 32768-32895). Many parameters map that index through a table or
scale (see the Display column above), and a time can take three slots: a
msec/note (or Hz/note) switch, the msec/Hz value and the note value.
