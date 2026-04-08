# PCM Synth Tone Parameters

The PCM Synth Tone starts at offset `00 00 00` within the Temporary Tone block. It is divided into the following sub-sections:

| Sub-section | Offset | Size |
|---|---|---|
| [PCM Synth Tone Common](#pcm-synth-tone-common) | `00 00 00` | `00 00 00 50` |
| [PCM Synth Tone Common MFX](#pcm-synth-tone-common-mfx) | `00 02 00` | `00 00 01 11` |
| [PCM Synth Tone PMT](#pcm-synth-tone-pmt-partial-mix-table) | `00 10 00` | `00 00 00 29` |
| [PCM Synth Tone Partial 1](#pcm-synth-tone-partial) | `00 20 00` | `00 00 01 1A` |
| [PCM Synth Tone Partial 2](#pcm-synth-tone-partial) | `00 22 00` | `00 00 01 1A` |
| [PCM Synth Tone Partial 3](#pcm-synth-tone-partial) | `00 24 00` | `00 00 01 1A` |
| [PCM Synth Tone Partial 4](#pcm-synth-tone-partial) | `00 26 00` | `00 00 01 1A` |
| [PCM Synth Tone Common 2](#pcm-synth-tone-common-2) | `00 30 00` | `00 00 00 3C` |

---

## PCM Synth Tone Common

Offset: `00 00 00` | Total size: `00 00 00 50`

### Tone Name

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 00` | `0aaa aaaa` | PCM Synth Tone Name 1 | 32 - 127 | ASCII |
| `00 01` | `0aaa aaaa` | PCM Synth Tone Name 2 | 32 - 127 | ASCII |
| `00 02` | `0aaa aaaa` | PCM Synth Tone Name 3 | 32 - 127 | ASCII |
| `00 03` | `0aaa aaaa` | PCM Synth Tone Name 4 | 32 - 127 | ASCII |
| `00 04` | `0aaa aaaa` | PCM Synth Tone Name 5 | 32 - 127 | ASCII |
| `00 05` | `0aaa aaaa` | PCM Synth Tone Name 6 | 32 - 127 | ASCII |
| `00 06` | `0aaa aaaa` | PCM Synth Tone Name 7 | 32 - 127 | ASCII |
| `00 07` | `0aaa aaaa` | PCM Synth Tone Name 8 | 32 - 127 | ASCII |
| `00 08` | `0aaa aaaa` | PCM Synth Tone Name 9 | 32 - 127 | ASCII |
| `00 09` | `0aaa aaaa` | PCM Synth Tone Name 10 | 32 - 127 | ASCII |
| `00 0A` | `0aaa aaaa` | PCM Synth Tone Name 11 | 32 - 127 | ASCII |
| `00 0B` | `0aaa aaaa` | PCM Synth Tone Name 12 | 32 - 127 | ASCII |

### General Parameters

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 0E` | `0aaa aaaa` | PCM Synth Tone Level | 0 - 127 | 0 - 127 |
| `00 0F` | `0aaa aaaa` | PCM Synth Tone Pan | 0 - 127 | L64 - 63R |
| `00 10` | `0000 000a` | PCM Synth Tone Priority | 0 - 1 | LAST, LOUDEST |
| `00 11` | `0aaa aaaa` | PCM Synth Tone Coarse Tune | 16 - 112 | -48 - +48 |
| `00 12` | `0aaa aaaa` | PCM Synth Tone Fine Tune | 14 - 114 | -50 - +50 |
| `00 13` | `0000 0aaa` | Octave Shift | 61 - 67 | -3 - +3 |
| `00 14` | `0000 00aa` | Stretch Tune Depth | 0 - 3 | OFF, 1 - 3 |
| `00 15` | `0aaa aaaa` | Analog Feel | 0 - 127 | 0 - 127 |
| `00 16` | `0000 000a` | Mono/Poly | 0 - 1 | MONO, POLY |
| `00 17` | `0000 000a` | Legato Switch | 0 - 1 | OFF, ON |
| `00 18` | `0000 000a` | Legato Retrigger | 0 - 1 | OFF, ON |

### Portamento

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 19` | `0000 000a` | Portamento Switch | 0 - 1 | OFF, ON |
| `00 1A` | `0000 000a` | Portamento Mode | 0 - 1 | NORMAL, LEGATO |
| `00 1B` | `0000 000a` | Portamento Type | 0 - 1 | RATE, TIME |
| `00 1C` | `0000 000a` | Portamento Start | 0 - 1 | PITCH, NOTE |
| `00 1D` | `0aaa aaaa` | Portamento Time | 0 - 127 | 0 - 127 |

### Offsets

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 22` | `0aaa aaaa` | Cutoff Offset | 1 - 127 | -63 - +63 |
| `00 23` | `0aaa aaaa` | Resonance Offset | 1 - 127 | -63 - +63 |
| `00 24` | `0aaa aaaa` | Attack Time Offset | 1 - 127 | -63 - +63 |
| `00 25` | `0aaa aaaa` | Release Time Offset | 1 - 127 | -63 - +63 |
| `00 26` | `0aaa aaaa` | Velocity Sens Offset | 1 - 127 | -63 - +63 |

### PMT and Pitch Bend

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 28` | `0000 000a` | PMT Control Switch | 0 - 1 | OFF, ON |
| `00 29` | `00aa aaaa` | Pitch Bend Range Up | 0 - 48 | 0 - 48 |
| `00 2A` | `00aa aaaa` | Pitch Bend Range Down | 0 - 48 | 0 - 48 |

### Matrix Control 1

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 2B` | `0aaa aaaa` | Matrix Control 1 Source | 0 - 109 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT, CTRL1 - CTRL4, VELOCITY, KEYFOLLOW, TEMPO, LFO1, LFO2, PIT-ENV, TVF-ENV, TVA-ENV |
| `00 2C` | `00aa aaaa` | Matrix Control 1 Destination 1 | 0 - 33 | OFF, PCH, CUT, RES, LEV, PAN, DRY, CHO, REV, PIT-LFO1, PIT-LFO2, TVF-LFO1, TVF-LFO2, TVA-LFO1, TVA-LFO2, PAN-LFO1, PAN-LFO2, LFO1-RATE, LFO2-RATE, PIT-ATK, PIT-DCY, PIT-REL, TVF-ATK, TVF-DCY, TVF-REL, TVA-ATK, TVA-DCY, TVA-REL, PMT, FXM |
| `00 2D` | `0aaa aaaa` | Matrix Control 1 Sens 1 | 1 - 127 | -63 - +63 |
| `00 2E` | `00aa aaaa` | Matrix Control 1 Destination 2 | 0 - 33 | (same as Destination 1) |
| `00 2F` | `0aaa aaaa` | Matrix Control 1 Sens 2 | 1 - 127 | -63 - +63 |
| `00 30` | `00aa aaaa` | Matrix Control 1 Destination 3 | 0 - 33 | (same as Destination 1) |
| `00 31` | `0aaa aaaa` | Matrix Control 1 Sens 3 | 1 - 127 | -63 - +63 |
| `00 32` | `00aa aaaa` | Matrix Control 1 Destination 4 | 0 - 33 | (same as Destination 1) |
| `00 33` | `0aaa aaaa` | Matrix Control 1 Sens 4 | 1 - 127 | -63 - +63 |

### Matrix Control 2

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 34` | `0aaa aaaa` | Matrix Control 2 Source | 0 - 109 | (same as Matrix Control 1 Source) |
| `00 35` | `00aa aaaa` | Matrix Control 2 Destination 1 | 0 - 33 | (same as Destination 1) |
| `00 36` | `0aaa aaaa` | Matrix Control 2 Sens 1 | 1 - 127 | -63 - +63 |
| `00 37` | `00aa aaaa` | Matrix Control 2 Destination 2 | 0 - 33 | (same as Destination 1) |
| `00 38` | `0aaa aaaa` | Matrix Control 2 Sens 2 | 1 - 127 | -63 - +63 |
| `00 39` | `00aa aaaa` | Matrix Control 2 Destination 3 | 0 - 33 | (same as Destination 1) |
| `00 3A` | `0aaa aaaa` | Matrix Control 2 Sens 3 | 1 - 127 | -63 - +63 |
| `00 3B` | `00aa aaaa` | Matrix Control 2 Destination 4 | 0 - 33 | (same as Destination 1) |
| `00 3C` | `0aaa aaaa` | Matrix Control 2 Sens 4 | 1 - 127 | -63 - +63 |

### Matrix Control 3

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 3D` | `0aaa aaaa` | Matrix Control 3 Source | 0 - 109 | (same as Matrix Control 1 Source) |
| `00 3E` | `00aa aaaa` | Matrix Control 3 Destination 1 | 0 - 33 | (same as Destination 1) |
| `00 3F` | `0aaa aaaa` | Matrix Control 3 Sens 1 | 1 - 127 | -63 - +63 |
| `00 40` | `00aa aaaa` | Matrix Control 3 Destination 2 | 0 - 33 | (same as Destination 1) |
| `00 41` | `0aaa aaaa` | Matrix Control 3 Sens 2 | 1 - 127 | -63 - +63 |
| `00 42` | `00aa aaaa` | Matrix Control 3 Destination 3 | 0 - 33 | (same as Destination 1) |
| `00 43` | `0aaa aaaa` | Matrix Control 3 Sens 3 | 1 - 127 | -63 - +63 |
| `00 44` | `00aa aaaa` | Matrix Control 3 Destination 4 | 0 - 33 | (same as Destination 1) |
| `00 45` | `0aaa aaaa` | Matrix Control 3 Sens 4 | 1 - 127 | -63 - +63 |

### Matrix Control 4

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 46` | `0aaa aaaa` | Matrix Control 4 Source | 0 - 109 | (same as Matrix Control 1 Source) |
| `00 47` | `00aa aaaa` | Matrix Control 4 Destination 1 | 0 - 33 | (same as Destination 1) |
| `00 48` | `0aaa aaaa` | Matrix Control 4 Sens 1 | 1 - 127 | -63 - +63 |
| `00 49` | `00aa aaaa` | Matrix Control 4 Destination 2 | 0 - 33 | (same as Destination 1) |
| `00 4A` | `0aaa aaaa` | Matrix Control 4 Sens 2 | 1 - 127 | -63 - +63 |
| `00 4B` | `00aa aaaa` | Matrix Control 4 Destination 3 | 0 - 33 | (same as Destination 1) |
| `00 4C` | `0aaa aaaa` | Matrix Control 4 Sens 3 | 1 - 127 | -63 - +63 |
| `00 4D` | `00aa aaaa` | Matrix Control 4 Destination 4 | 0 - 33 | (same as Destination 1) |
| `00 4E` | `0aaa aaaa` | Matrix Control 4 Sens 4 | 1 - 127 | -63 - +63 |

---

## PCM Synth Tone Common MFX

Offset: `00 02 00` | Total size: `00 00 01 11`

### MFX General

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 00` | `0aaa aaaa` | MFX Type | 0 - 67 | 0 - 67 |
| `00 02` | `0aaa aaaa` | MFX Chorus Send Level | 0 - 127 | 0 - 127 |
| `00 03` | `0aaa aaaa` | MFX Reverb Send Level | 0 - 127 | 0 - 127 |

### MFX Controls

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 05` | `0aaa aaaa` | MFX Control 1 Source | 0 - 101 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT, SYS1 - SYS4 |
| `00 06` | `0aaa aaaa` | MFX Control 1 Sens | 1 - 127 | -63 - +63 |
| `00 07` | `0aaa aaaa` | MFX Control 2 Source | 0 - 101 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT, SYS1 - SYS4 |
| `00 08` | `0aaa aaaa` | MFX Control 2 Sens | 1 - 127 | -63 - +63 |
| `00 09` | `0aaa aaaa` | MFX Control 3 Source | 0 - 101 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT, SYS1 - SYS4 |
| `00 0A` | `0aaa aaaa` | MFX Control 3 Sens | 1 - 127 | -63 - +63 |
| `00 0B` | `0aaa aaaa` | MFX Control 4 Source | 0 - 101 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT, SYS1 - SYS4 |
| `00 0C` | `0aaa aaaa` | MFX Control 4 Sens | 1 - 127 | -63 - +63 |

### MFX Control Assigns

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 0D` | `000a aaaa` | MFX Control Assign 1 | 0 - 16 | OFF, 1 - 16 |
| `00 0E` | `000a aaaa` | MFX Control Assign 2 | 0 - 16 | OFF, 1 - 16 |
| `00 0F` | `000a aaaa` | MFX Control Assign 3 | 0 - 16 | OFF, 1 - 16 |
| `00 10` | `000a aaaa` | MFX Control Assign 4 | 0 - 16 | OFF, 1 - 16 |

### MFX Parameters 1-32

All MFX Parameters are nibblized (#) 4-byte values. Each parameter occupies 4 consecutive bytes using nibble encoding (`0000 aaaa`, `0000 bbbb`, `0000 cccc`, `0000 dddd`).

| Offset | Parameter | Range | Display |
|---|---|---|---|
| `00 11` # | MFX Parameter 1 | 12768 - 52768 | -20000 - +20000 |
| `00 15` # | MFX Parameter 2 | 12768 - 52768 | -20000 - +20000 |
| `00 19` # | MFX Parameter 3 | 12768 - 52768 | -20000 - +20000 |
| `00 1D` # | MFX Parameter 4 | 12768 - 52768 | -20000 - +20000 |
| `00 21` # | MFX Parameter 5 | 12768 - 52768 | -20000 - +20000 |
| `00 25` # | MFX Parameter 6 | 12768 - 52768 | -20000 - +20000 |
| `00 29` # | MFX Parameter 7 | 12768 - 52768 | -20000 - +20000 |
| `00 2D` # | MFX Parameter 8 | 12768 - 52768 | -20000 - +20000 |
| `00 31` # | MFX Parameter 9 | 12768 - 52768 | -20000 - +20000 |
| `00 35` # | MFX Parameter 10 | 12768 - 52768 | -20000 - +20000 |
| `00 39` # | MFX Parameter 11 | 12768 - 52768 | -20000 - +20000 |
| `00 3D` # | MFX Parameter 12 | 12768 - 52768 | -20000 - +20000 |
| `00 41` # | MFX Parameter 13 | 12768 - 52768 | -20000 - +20000 |
| `00 45` # | MFX Parameter 14 | 12768 - 52768 | -20000 - +20000 |
| `00 49` # | MFX Parameter 15 | 12768 - 52768 | -20000 - +20000 |
| `00 4D` # | MFX Parameter 16 | 12768 - 52768 | -20000 - +20000 |
| `00 51` # | MFX Parameter 17 | 12768 - 52768 | -20000 - +20000 |
| `00 55` # | MFX Parameter 18 | 12768 - 52768 | -20000 - +20000 |
| `00 59` # | MFX Parameter 19 | 12768 - 52768 | -20000 - +20000 |
| `00 5D` # | MFX Parameter 20 | 12768 - 52768 | -20000 - +20000 |
| `00 61` # | MFX Parameter 21 | 12768 - 52768 | -20000 - +20000 |
| `00 65` # | MFX Parameter 22 | 12768 - 52768 | -20000 - +20000 |
| `00 69` # | MFX Parameter 23 | 12768 - 52768 | -20000 - +20000 |
| `00 6D` # | MFX Parameter 24 | 12768 - 52768 | -20000 - +20000 |
| `00 71` # | MFX Parameter 25 | 12768 - 52768 | -20000 - +20000 |
| `00 75` # | MFX Parameter 26 | 12768 - 52768 | -20000 - +20000 |
| `00 79` # | MFX Parameter 27 | 12768 - 52768 | -20000 - +20000 |
| `00 7D` # | MFX Parameter 28 | 12768 - 52768 | -20000 - +20000 |
| `01 01` # | MFX Parameter 29 | 12768 - 52768 | -20000 - +20000 |
| `01 05` # | MFX Parameter 30 | 12768 - 52768 | -20000 - +20000 |
| `01 09` # | MFX Parameter 31 | 12768 - 52768 | -20000 - +20000 |
| `01 0D` # | MFX Parameter 32 | 12768 - 52768 | -20000 - +20000 |

---

## PCM Synth Tone PMT (Partial Mix Table)

Offset: `00 10 00` | Total size: `00 00 00 29`

### Structure

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 00` | `0000 aaaa` | Structure Type 1 & 2 | 0 - 9 | 1 - 10 |
| `00 01` | `0000 00aa` | Booster 1 & 2 | 0 - 3 | 0, +6, +12, +18 [dB] |
| `00 02` | `0000 aaaa` | Structure Type 3 & 4 | 0 - 9 | 1 - 10 |
| `00 03` | `0000 00aa` | Booster 3 & 4 | 0 - 3 | 0, +6, +12, +18 [dB] |
| `00 04` | `0000 00aa` | PMT Velocity Control | 0 - 3 | OFF, ON, RANDOM, CYCLE |

### PMT1 (Partial 1)

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 05` | `0000 000a` | PMT1 Partial Switch | 0 - 1 | OFF, ON |
| `00 06` | `0aaa aaaa` | PMT1 Keyboard Range Lower | 0 - 127 | C-1 - UPPER |
| `00 07` | `0aaa aaaa` | PMT1 Keyboard Range Upper | 0 - 127 | LOWER - G9 |
| `00 08` | `0aaa aaaa` | PMT1 Keyboard Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 09` | `0aaa aaaa` | PMT1 Keyboard Fade Width Upper | 0 - 127 | 0 - 127 |
| `00 0A` | `0aaa aaaa` | PMT1 Velocity Range Lower | 1 - 127 | 1 - UPPER |
| `00 0B` | `0aaa aaaa` | PMT1 Velocity Range Upper | 1 - 127 | LOWER - 127 |
| `00 0C` | `0aaa aaaa` | PMT1 Velocity Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 0D` | `0aaa aaaa` | PMT1 Velocity Fade Width Upper | 0 - 127 | 0 - 127 |

### PMT2 (Partial 2)

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 0E` | `0000 000a` | PMT2 Partial Switch | 0 - 1 | OFF, ON |
| `00 0F` | `0aaa aaaa` | PMT2 Keyboard Range Lower | 0 - 127 | C-1 - UPPER |
| `00 10` | `0aaa aaaa` | PMT2 Keyboard Range Upper | 0 - 127 | LOWER - G9 |
| `00 11` | `0aaa aaaa` | PMT2 Keyboard Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 12` | `0aaa aaaa` | PMT2 Keyboard Fade Width Upper | 0 - 127 | 0 - 127 |
| `00 13` | `0aaa aaaa` | PMT2 Velocity Range Lower | 1 - 127 | 1 - UPPER |
| `00 14` | `0aaa aaaa` | PMT2 Velocity Range Upper | 1 - 127 | LOWER - 127 |
| `00 15` | `0aaa aaaa` | PMT2 Velocity Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 16` | `0aaa aaaa` | PMT2 Velocity Fade Width Upper | 0 - 127 | 0 - 127 |

### PMT3 (Partial 3)

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 17` | `0000 000a` | PMT3 Partial Switch | 0 - 1 | OFF, ON |
| `00 18` | `0aaa aaaa` | PMT3 Keyboard Range Lower | 0 - 127 | C-1 - UPPER |
| `00 19` | `0aaa aaaa` | PMT3 Keyboard Range Upper | 0 - 127 | LOWER - G9 |
| `00 1A` | `0aaa aaaa` | PMT3 Keyboard Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 1B` | `0aaa aaaa` | PMT3 Keyboard Fade Width Upper | 0 - 127 | 0 - 127 |
| `00 1C` | `0aaa aaaa` | PMT3 Velocity Range Lower | 1 - 127 | 1 - UPPER |
| `00 1D` | `0aaa aaaa` | PMT3 Velocity Range Upper | 1 - 127 | LOWER - 127 |
| `00 1E` | `0aaa aaaa` | PMT3 Velocity Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 1F` | `0aaa aaaa` | PMT3 Velocity Fade Width Upper | 0 - 127 | 0 - 127 |

### PMT4 (Partial 4)

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 20` | `0000 000a` | PMT4 Partial Switch | 0 - 1 | OFF, ON |
| `00 21` | `0aaa aaaa` | PMT4 Keyboard Range Lower | 0 - 127 | C-1 - UPPER |
| `00 22` | `0aaa aaaa` | PMT4 Keyboard Range Upper | 0 - 127 | LOWER - G9 |
| `00 23` | `0aaa aaaa` | PMT4 Keyboard Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 24` | `0aaa aaaa` | PMT4 Keyboard Fade Width Upper | 0 - 127 | 0 - 127 |
| `00 25` | `0aaa aaaa` | PMT4 Velocity Range Lower | 1 - 127 | 1 - UPPER |
| `00 26` | `0aaa aaaa` | PMT4 Velocity Range Upper | 1 - 127 | LOWER - 127 |
| `00 27` | `0aaa aaaa` | PMT4 Velocity Fade Width Lower | 0 - 127 | 0 - 127 |
| `00 28` | `0aaa aaaa` | PMT4 Velocity Fade Width Upper | 0 - 127 | 0 - 127 |

---

## PCM Synth Tone Partial

Offset: `00 20 00` (Partial 1), `00 22 00` (Partial 2), `00 24 00` (Partial 3), `00 26 00` (Partial 4)

Total size per partial: `00 00 01 1A`

All four partials share identical parameter layout. The offsets below are relative to each partial's base address.

### General

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 00` | `0aaa aaaa` | Partial Level | 0 - 127 | 0 - 127 |
| `00 01` | `0aaa aaaa` | Partial Coarse Tune | 16 - 112 | -48 - +48 |
| `00 02` | `0aaa aaaa` | Partial Fine Tune | 14 - 114 | -50 - +50 |
| `00 03` | `000a aaaa` | Partial Random Pitch Depth | 0 - 30 | 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000, 1100, 1200 |
| `00 04` | `0aaa aaaa` | Partial Pan | 0 - 127 | L64 - 63R |
| `00 05` | `000a aaaa` | Partial Pan Keyfollow | 54 - 74 | -100 - +100 |
| `00 06` | `00aa aaaa` | Partial Random Pan Depth | 0 - 63 | 0 - 63 |
| `00 07` | `0aaa aaaa` | Partial Alternate Pan Depth | 1 - 127 | L63 - 63R |
| `00 08` | `0000 000a` | Partial Env Mode | 0 - 1 | NO-SUS, SUSTAIN |
| `00 09` | `0000 00aa` | Partial Delay Mode | 0 - 3 | NORMAL, HOLD, KEY-OFF-NORMAL, KEY-OFF-DECAY |
| `00 0A` # | `0000 aaaa` `0000 bbbb` | Partial Delay Time | 0 - 149 | 0 - 127, MUSICAL-NOTES |

### Output

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 0C` | `0aaa aaaa` | Partial Output Level | 0 - 127 | 0 - 127 |
| `00 0F` | `0aaa aaaa` | Partial Chorus Send Level | 0 - 127 | 0 - 127 |
| `00 10` | `0aaa aaaa` | Partial Reverb Send Level | 0 - 127 | 0 - 127 |

### Receive Switches

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 12` | `0000 000a` | Partial Receive Bender | 0 - 1 | OFF, ON |
| `00 13` | `0000 000a` | Partial Receive Expression | 0 - 1 | OFF, ON |
| `00 14` | `0000 000a` | Partial Receive Hold-1 | 0 - 1 | OFF, ON |
| `00 16` | `0000 000a` | Partial Redamper Switch | 0 - 1 | OFF, ON |

### Partial Control Switches

Each control (1-4) has 4 switches corresponding to partial targets. Values: OFF (0), ON (1), REVERSE (2).

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 17` | `0000 00aa` | Partial Control 1 Switch 1 | 0 - 2 | OFF, ON, REVERSE |
| `00 18` | `0000 00aa` | Partial Control 1 Switch 2 | 0 - 2 | OFF, ON, REVERSE |
| `00 19` | `0000 00aa` | Partial Control 1 Switch 3 | 0 - 2 | OFF, ON, REVERSE |
| `00 1A` | `0000 00aa` | Partial Control 1 Switch 4 | 0 - 2 | OFF, ON, REVERSE |
| `00 1B` | `0000 00aa` | Partial Control 2 Switch 1 | 0 - 2 | OFF, ON, REVERSE |
| `00 1C` | `0000 00aa` | Partial Control 2 Switch 2 | 0 - 2 | OFF, ON, REVERSE |
| `00 1D` | `0000 00aa` | Partial Control 2 Switch 3 | 0 - 2 | OFF, ON, REVERSE |
| `00 1E` | `0000 00aa` | Partial Control 2 Switch 4 | 0 - 2 | OFF, ON, REVERSE |
| `00 1F` | `0000 00aa` | Partial Control 3 Switch 1 | 0 - 2 | OFF, ON, REVERSE |
| `00 20` | `0000 00aa` | Partial Control 3 Switch 2 | 0 - 2 | OFF, ON, REVERSE |
| `00 21` | `0000 00aa` | Partial Control 3 Switch 3 | 0 - 2 | OFF, ON, REVERSE |
| `00 22` | `0000 00aa` | Partial Control 3 Switch 4 | 0 - 2 | OFF, ON, REVERSE |
| `00 23` | `0000 00aa` | Partial Control 4 Switch 1 | 0 - 2 | OFF, ON, REVERSE |
| `00 24` | `0000 00aa` | Partial Control 4 Switch 2 | 0 - 2 | OFF, ON, REVERSE |
| `00 25` | `0000 00aa` | Partial Control 4 Switch 3 | 0 - 2 | OFF, ON, REVERSE |
| `00 26` | `0000 00aa` | Partial Control 4 Switch 4 | 0 - 2 | OFF, ON, REVERSE |

### Wave

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 27` | `0000 00aa` | Wave Group Type | 0 - 3 | INT, SRX, ---, --- |
| `00 28` # | `0000 aaaa` x4 | Wave Group ID | 0 - 16384 | OFF, 1 - 16384 |
| `00 2C` # | `0000 aaaa` x4 | Wave Number L (Mono) | 0 - 16384 | OFF, 1 - 16384 |
| `00 30` # | `0000 aaaa` x4 | Wave Number R | 0 - 16384 | OFF, 1 - 16384 |
| `00 34` | `0000 00aa` | Wave Gain | 0 - 3 | -6, 0, +6, +12 [dB] |
| `00 35` | `0000 000a` | Wave FXM Switch | 0 - 1 | OFF, ON |
| `00 36` | `0000 00aa` | Wave FXM Color | 0 - 3 | 1 - 4 |
| `00 37` | `000a aaaa` | Wave FXM Depth | 0 - 16 | 0 - 16 |
| `00 38` | `0000 000a` | Wave Tempo Sync | 0 - 1 | OFF, ON |
| `00 39` | `00aa aaaa` | Wave Pitch Keyfollow | 44 - 84 | -200 - +200 |

### Pitch Envelope

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 3A` | `000a aaaa` | Pitch Env Depth | 52 - 76 | -12 - +12 |
| `00 3B` | `0aaa aaaa` | Pitch Env Velocity Sens | 1 - 127 | -63 - +63 |
| `00 3C` | `0aaa aaaa` | Pitch Env Time 1 Velocity Sens | 1 - 127 | -63 - +63 |
| `00 3D` | `0aaa aaaa` | Pitch Env Time 4 Velocity Sens | 1 - 127 | -63 - +63 |
| `00 3E` | `000a aaaa` | Pitch Env Time Keyfollow | 54 - 74 | -100 - +100 |
| `00 3F` | `0aaa aaaa` | Pitch Env Time 1 | 0 - 127 | 0 - 127 |
| `00 40` | `0aaa aaaa` | Pitch Env Time 2 | 0 - 127 | 0 - 127 |
| `00 41` | `0aaa aaaa` | Pitch Env Time 3 | 0 - 127 | 0 - 127 |
| `00 42` | `0aaa aaaa` | Pitch Env Time 4 | 0 - 127 | 0 - 127 |
| `00 43` | `0aaa aaaa` | Pitch Env Level 0 | 1 - 127 | -63 - +63 |
| `00 44` | `0aaa aaaa` | Pitch Env Level 1 | 1 - 127 | -63 - +63 |
| `00 45` | `0aaa aaaa` | Pitch Env Level 2 | 1 - 127 | -63 - +63 |
| `00 46` | `0aaa aaaa` | Pitch Env Level 3 | 1 - 127 | -63 - +63 |
| `00 47` | `0aaa aaaa` | Pitch Env Level 4 | 1 - 127 | -63 - +63 |

### TVF (Time Variant Filter)

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 48` | `0000 0aaa` | TVF Filter Type | 0 - 6 | OFF, LPF, BPF, HPF, PKG, LPF2, LPF3 |
| `00 49` | `0aaa aaaa` | TVF Cutoff Frequency | 0 - 127 | 0 - 127 |
| `00 4A` | `00aa aaaa` | TVF Cutoff Keyfollow | 44 - 84 | -200 - +200 |
| `00 4B` | `0000 0aaa` | TVF Cutoff Velocity Curve | 0 - 7 | FIXED, 1 - 7 |
| `00 4C` | `0aaa aaaa` | TVF Cutoff Velocity Sens | 1 - 127 | -63 - +63 |
| `00 4D` | `0aaa aaaa` | TVF Resonance | 0 - 127 | 0 - 127 |
| `00 4E` | `0aaa aaaa` | TVF Resonance Velocity Sens | 1 - 127 | -63 - +63 |
| `00 4F` | `0aaa aaaa` | TVF Env Depth | 1 - 127 | -63 - +63 |
| `00 50` | `0000 0aaa` | TVF Env Velocity Curve | 0 - 7 | FIXED, 1 - 7 |
| `00 51` | `0aaa aaaa` | TVF Env Velocity Sens | 1 - 127 | -63 - +63 |
| `00 52` | `0aaa aaaa` | TVF Env Time 1 Velocity Sens | 1 - 127 | -63 - +63 |
| `00 53` | `0aaa aaaa` | TVF Env Time 4 Velocity Sens | 1 - 127 | -63 - +63 |
| `00 54` | `000a aaaa` | TVF Env Time Keyfollow | 54 - 74 | -100 - +100 |
| `00 55` | `0aaa aaaa` | TVF Env Time 1 | 0 - 127 | 0 - 127 |
| `00 56` | `0aaa aaaa` | TVF Env Time 2 | 0 - 127 | 0 - 127 |
| `00 57` | `0aaa aaaa` | TVF Env Time 3 | 0 - 127 | 0 - 127 |
| `00 58` | `0aaa aaaa` | TVF Env Time 4 | 0 - 127 | 0 - 127 |
| `00 59` | `0aaa aaaa` | TVF Env Level 0 | 0 - 127 | 0 - 127 |
| `00 5A` | `0aaa aaaa` | TVF Env Level 1 | 0 - 127 | 0 - 127 |
| `00 5B` | `0aaa aaaa` | TVF Env Level 2 | 0 - 127 | 0 - 127 |
| `00 5C` | `0aaa aaaa` | TVF Env Level 3 | 0 - 127 | 0 - 127 |
| `00 5D` | `0aaa aaaa` | TVF Env Level 4 | 0 - 127 | 0 - 127 |

### TVA (Time Variant Amplifier)

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 5E` | `000a aaaa` | Bias Level | 54 - 74 | -100 - +100 |
| `00 5F` | `0aaa aaaa` | Bias Position | 0 - 127 | C-1 - G9 |
| `00 60` | `0000 00aa` | Bias Direction | 0 - 3 | LOWER, UPPER, LOWER&UPPER, ALL |
| `00 61` | `0000 0aaa` | TVA Level Velocity Curve | 0 - 7 | FIXED, 1 - 7 |
| `00 62` | `0aaa aaaa` | TVA Level Velocity Sens | 1 - 127 | -63 - +63 |
| `00 63` | `0aaa aaaa` | TVA Env Time 1 Velocity Sens | 1 - 127 | -63 - +63 |
| `00 64` | `0aaa aaaa` | TVA Env Time 4 Velocity Sens | 1 - 127 | -63 - +63 |
| `00 65` | `000a aaaa` | TVA Env Time Keyfollow | 54 - 74 | -100 - +100 |
| `00 66` | `0aaa aaaa` | TVA Env Time 1 | 0 - 127 | 0 - 127 |
| `00 67` | `0aaa aaaa` | TVA Env Time 2 | 0 - 127 | 0 - 127 |
| `00 68` | `0aaa aaaa` | TVA Env Time 3 | 0 - 127 | 0 - 127 |
| `00 69` | `0aaa aaaa` | TVA Env Time 4 | 0 - 127 | 0 - 127 |
| `00 6A` | `0aaa aaaa` | TVA Env Level 1 | 0 - 127 | 0 - 127 |
| `00 6B` | `0aaa aaaa` | TVA Env Level 2 | 0 - 127 | 0 - 127 |
| `00 6C` | `0aaa aaaa` | TVA Env Level 3 | 0 - 127 | 0 - 127 |

### LFO1

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 6D` | `0000 aaaa` | LFO1 Waveform | 0 - 12 | SIN, TRI, SAW-UP, SAW-DW, SQR, RND, BEND-UP, BEND-DW, TRP, S&H, CHS, VSIN, STEP |
| `00 6E` # | `0000 aaaa` `0000 bbbb` | LFO1 Rate | 0 - 149 | 0 - 127, MUSICAL-NOTES |
| `00 70` | `0000 0aaa` | LFO1 Offset | 0 - 4 | -100, -50, 0, +50, +100 |
| `00 71` | `0aaa aaaa` | LFO1 Rate Detune | 0 - 127 | 0 - 127 |
| `00 72` | `0aaa aaaa` | LFO1 Delay Time | 0 - 127 | 0 - 127 |
| `00 73` | `000a aaaa` | LFO1 Delay Time Keyfollow | 54 - 74 | -100 - +100 |
| `00 74` | `0000 00aa` | LFO1 Fade Mode | 0 - 3 | ON-IN, ON-OUT, OFF-IN, OFF-OUT |
| `00 75` | `0aaa aaaa` | LFO1 Fade Time | 0 - 127 | 0 - 127 |
| `00 76` | `0000 000a` | LFO1 Key Trigger | 0 - 1 | OFF, ON |
| `00 77` | `0aaa aaaa` | LFO1 Pitch Depth | 1 - 127 | -63 - +63 |
| `00 78` | `0aaa aaaa` | LFO1 TVF Depth | 1 - 127 | -63 - +63 |
| `00 79` | `0aaa aaaa` | LFO1 TVA Depth | 1 - 127 | -63 - +63 |
| `00 7A` | `0aaa aaaa` | LFO1 Pan Depth | 1 - 127 | -63 - +63 |

### LFO2

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 7B` | `0000 aaaa` | LFO2 Waveform | 0 - 12 | SIN, TRI, SAW-UP, SAW-DW, SQR, RND, BEND-UP, BEND-DW, TRP, S&H, CHS, VSIN, STEP |
| `00 7C` # | `0000 aaaa` `0000 bbbb` | LFO2 Rate | 0 - 149 | 0 - 127, MUSICAL-NOTES |
| `00 7E` | `0000 0aaa` | LFO2 Offset | 0 - 4 | -100, -50, 0, +50, +100 |
| `00 7F` | `0aaa aaaa` | LFO2 Rate Detune | 0 - 127 | 0 - 127 |
| `01 00` | `0aaa aaaa` | LFO2 Delay Time | 0 - 127 | 0 - 127 |
| `01 01` | `000a aaaa` | LFO2 Delay Time Keyfollow | 54 - 74 | -100 - +100 |
| `01 02` | `0000 00aa` | LFO2 Fade Mode | 0 - 3 | ON-IN, ON-OUT, OFF-IN, OFF-OUT |
| `01 03` | `0aaa aaaa` | LFO2 Fade Time | 0 - 127 | 0 - 127 |
| `01 04` | `0000 000a` | LFO2 Key Trigger | 0 - 1 | OFF, ON |
| `01 05` | `0aaa aaaa` | LFO2 Pitch Depth | 1 - 127 | -63 - +63 |
| `01 06` | `0aaa aaaa` | LFO2 TVF Depth | 1 - 127 | -63 - +63 |
| `01 07` | `0aaa aaaa` | LFO2 TVA Depth | 1 - 127 | -63 - +63 |
| `01 08` | `0aaa aaaa` | LFO2 Pan Depth | 1 - 127 | -63 - +63 |

### LFO Step Sequencer

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `01 09` | `0000 aaaa` | LFO Step Type | 0 - 1 | 0 - 1 |
| `01 0A` | `0aaa aaaa` | LFO Step 1 | 28 - 100 | -36 - +36 |
| `01 0B` | `0aaa aaaa` | LFO Step 2 | 28 - 100 | -36 - +36 |
| `01 0C` | `0aaa aaaa` | LFO Step 3 | 28 - 100 | -36 - +36 |
| `01 0D` | `0aaa aaaa` | LFO Step 4 | 28 - 100 | -36 - +36 |
| `01 0E` | `0aaa aaaa` | LFO Step 5 | 28 - 100 | -36 - +36 |
| `01 0F` | `0aaa aaaa` | LFO Step 6 | 28 - 100 | -36 - +36 |
| `01 10` | `0aaa aaaa` | LFO Step 7 | 28 - 100 | -36 - +36 |
| `01 11` | `0aaa aaaa` | LFO Step 8 | 28 - 100 | -36 - +36 |
| `01 12` | `0aaa aaaa` | LFO Step 9 | 28 - 100 | -36 - +36 |
| `01 13` | `0aaa aaaa` | LFO Step 10 | 28 - 100 | -36 - +36 |
| `01 14` | `0aaa aaaa` | LFO Step 11 | 28 - 100 | -36 - +36 |
| `01 15` | `0aaa aaaa` | LFO Step 12 | 28 - 100 | -36 - +36 |
| `01 16` | `0aaa aaaa` | LFO Step 13 | 28 - 100 | -36 - +36 |
| `01 17` | `0aaa aaaa` | LFO Step 14 | 28 - 100 | -36 - +36 |
| `01 18` | `0aaa aaaa` | LFO Step 15 | 28 - 100 | -36 - +36 |
| `01 19` | `0aaa aaaa` | LFO Step 16 | 28 - 100 | -36 - +36 |

---

## PCM Synth Tone Common 2

Offset: `00 30 00` | Total size: `00 00 00 3C`

| Offset | Bits | Parameter | Range | Display |
|---|---|---|---|---|
| `00 10` | `0aaa aaaa` | Tone Category | 0 - 127 | 0 - 127 |
| `00 11` # | `0000 aaaa` `0000 bbbb` | Tone Number | 0 - 255 | 0 - 255 |
| `00 13` | `0000 0aaa` | Phrase Octave Shift | 61 - 67 | -3 - +3 |
| `00 33` | `0000 000a` | TFX Switch | 0 - 1 | OFF, ON |
| `00 38` # | `0000 aaaa` x4 | Phrase Number | 0 - 65535 | 0 - 65535 |
