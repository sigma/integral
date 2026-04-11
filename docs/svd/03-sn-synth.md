# SN Synth Tone — SVD Mapping (SHPa)

Chunk type: `SHPa` | Entry size: **280 bytes** | Model: `MI69`

This mapping has been validated against the Synth Legends expansion pack
and the physical INTEGRA-7 device via SysEx RQ1 reads.

## Entry Layout

| Byte Range | Section          | Bits Used | Padded Bytes |
|------------|------------------|-----------|--------------|
| 0–29       | Common           | 228       | 30           |
| 30–107     | MFX              | 618       | 78           |
| 108–153    | Partial 1        | 350       | 46           |
| 154–199    | Partial 2        | 350       | 46           |
| 200–245    | Partial 3        | 350       | 46           |
| 246        | End marker       | —         | 1 (`0x0E`)   |
| 247–279    | Zero padding     | —         | 33           |
| **Total**  |                  |           | **280**      |

**Note:** Common and MFX are **separate** sections, each independently
byte-aligned. This was confirmed by device validation — the MFX Type byte
aligns correctly at byte offset 30, not at the bit position implied by
concatenating Common + MFX bits.

## Section 1: Common (228 bits → 30 bytes)

### Common Parameters (228 bits)

Source: `docs/midi/08-supernatural-synth-tone.md` — SN Synth Tone Common
(`00 00 00`, size `00 00 00 40`).

| SysEx Offset | Bits | Parameter                |
|--------------|------|--------------------------|
| `00 00`      | 7    | Tone Name 1 (ASCII)      |
| `00 01`      | 7    | Tone Name 2              |
| `00 02`      | 7    | Tone Name 3              |
| `00 03`      | 7    | Tone Name 4              |
| `00 04`      | 7    | Tone Name 5              |
| `00 05`      | 7    | Tone Name 6              |
| `00 06`      | 7    | Tone Name 7              |
| `00 07`      | 7    | Tone Name 8              |
| `00 08`      | 7    | Tone Name 9              |
| `00 09`      | 7    | Tone Name 10             |
| `00 0A`      | 7    | Tone Name 11             |
| `00 0B`      | 7    | Tone Name 12             |
| `00 0C`      | 7    | Tone Level               |
| `00 0D`      | 4×3  | (reserve, nibblized)     |
| `00 10`      | 1    | (reserve)                |
| `00 11`      | 1    | (reserve)                |
| `00 12`      | 1    | Portamento Switch        |
| `00 13`      | 7    | Portamento Time          |
| `00 14`      | 2    | Mono Switch              |
| `00 15`      | 3    | Octave Shift             |
| `00 16`      | 5    | Pitch Bend Range Up      |
| `00 17`      | 5    | Pitch Bend Range Down    |
| `00 18`      | 3    | (reserve)                |
| `00 19`      | 1    | Partial 1 Switch         |
| `00 1A`      | 1    | Partial 1 Select         |
| `00 1B`      | 1    | Partial 2 Switch         |
| `00 1C`      | 1    | Partial 2 Select         |
| `00 1D`      | 1    | Partial 3 Switch         |
| `00 1E`      | 1    | Partial 3 Select         |
| `00 1F`      | 2    | RING Switch              |
| `00 20`      | 1    | TFX Switch               |
| `00 21`      | 2    | (reserve)                |
| `00 22`      | 1    | (reserve)                |
| `00 23`      | 1    | (reserve)                |
| `00 24`      | 6    | (reserve)                |
| `00 25`      | 1    | (reserve)                |
| `00 26`      | 1    | (reserve)                |
| `00 27`      | 1    | (reserve)                |
| `00 28`      | 1    | (reserve)                |
| `00 29`      | 1    | (reserve)                |
| `00 2A`      | 1    | (reserve)                |
| `00 2B`      | 1    | (reserve)                |
| `00 2C`      | 1    | (reserve)                |
| `00 2D`      | 1    | (reserve)                |
| `00 2E`      | 1    | Unison Switch            |
| `00 2F`      | 1    | (reserve)                |
| `00 30`      | 1    | (reserve)                |
| `00 31`      | 1    | Portamento Mode          |
| `00 32`      | 1    | Legato Switch            |
| `00 33`      | 1    | (reserve)                |
| `00 34`      | 7    | Analog Feel              |
| `00 35`      | 7    | Wave Shape               |
| `00 36`      | 7    | Tone Category            |
| `00 37`      | 4×4  | Phrase Number (nibblized) |
| `00 3B`      | 3    | Phrase Octave Shift      |
| `00 3C`      | 2    | Unison Size              |
| `00 3D`      | 7    | (reserve)                |
| `00 3E`      | 7    | (reserve)                |
| `00 3F`      | 7    | (reserve)                |

**Total Common: 228 bits → 30 bytes (padded)**

## Section 2: MFX (618 bits → 78 bytes)

### MFX Parameters (618 bits)

Source: `docs/midi/08-supernatural-synth-tone.md` — SN Synth Tone MFX
(`00 02 00`, size `00 00 01 11`).

| SysEx Offset | Bits | Parameter                |
|--------------|------|--------------------------|
| `00 00`      | 7    | MFX Type                 |
| `00 01`      | 7    | (reserve)                |
| `00 02`      | 7    | MFX Chorus Send Level    |
| `00 03`      | 7    | MFX Reverb Send Level    |
| `00 04`      | 2    | (reserve)                |
| `00 05`      | 7    | MFX Control 1 Source     |
| `00 06`      | 7    | MFX Control 1 Sens       |
| `00 07`      | 7    | MFX Control 2 Source     |
| `00 08`      | 7    | MFX Control 2 Sens       |
| `00 09`      | 7    | MFX Control 3 Source     |
| `00 0A`      | 7    | MFX Control 3 Sens       |
| `00 0B`      | 7    | MFX Control 4 Source     |
| `00 0C`      | 7    | MFX Control 4 Sens       |
| `00 0D`      | 5    | MFX Control Assign 1     |
| `00 0E`      | 5    | MFX Control Assign 2     |
| `00 0F`      | 5    | MFX Control Assign 3     |
| `00 10`      | 5    | MFX Control Assign 4     |
| `00 11`      | 4×4  | MFX Parameter 1 (nibb.)  |
| `00 15`      | 4×4  | MFX Parameter 2          |
| `00 19`      | 4×4  | MFX Parameter 3          |
| `00 1D`      | 4×4  | MFX Parameter 4          |
| `00 21`      | 4×4  | MFX Parameter 5          |
| `00 25`      | 4×4  | MFX Parameter 6          |
| `00 29`      | 4×4  | MFX Parameter 7          |
| `00 2D`      | 4×4  | MFX Parameter 8          |
| `00 31`      | 4×4  | MFX Parameter 9          |
| `00 35`      | 4×4  | MFX Parameter 10         |
| `00 39`      | 4×4  | MFX Parameter 11         |
| `00 3D`      | 4×4  | MFX Parameter 12         |
| `00 41`      | 4×4  | MFX Parameter 13         |
| `00 45`      | 4×4  | MFX Parameter 14         |
| `00 49`      | 4×4  | MFX Parameter 15         |
| `00 4D`      | 4×4  | MFX Parameter 16         |
| `00 51`      | 4×4  | MFX Parameter 17         |
| `00 55`      | 4×4  | MFX Parameter 18         |
| `00 59`      | 4×4  | MFX Parameter 19         |
| `00 5D`      | 4×4  | MFX Parameter 20         |
| `00 61`      | 4×4  | MFX Parameter 21         |
| `00 65`      | 4×4  | MFX Parameter 22         |
| `00 69`      | 4×4  | MFX Parameter 23         |
| `00 6D`      | 4×4  | MFX Parameter 24         |
| `00 71`      | 4×4  | MFX Parameter 25         |
| `00 75`      | 4×4  | MFX Parameter 26         |
| `00 79`      | 4×4  | MFX Parameter 27         |
| `00 7D`      | 4×4  | MFX Parameter 28         |
| `01 01`      | 4×4  | MFX Parameter 29         |
| `01 05`      | 4×4  | MFX Parameter 30         |
| `01 09`      | 4×4  | MFX Parameter 31         |
| `01 0D`      | 4×4  | MFX Parameter 32         |

**Total MFX: 618 bits → 78 bytes (padded)**

## Sections 3–5: Partials (350 bits → 46 bytes each)

Source: `docs/midi/08-supernatural-synth-tone.md` — SN Synth Tone Partial
(offsets `00 20 00`, `00 21 00`, `00 22 00`; size `00 00 00 3D` each).

Three identical partial blocks, each independently byte-aligned.

| SysEx Offset | Bits | Parameter                     |
|--------------|------|-------------------------------|
| `00 00`      | 3    | OSC Wave                      |
| `00 01`      | 6    | OSC Wave Variation            |
| `00 02`      | 2    | (reserve)                     |
| `00 03`      | 6    | OSC Pitch                     |
| `00 04`      | 7    | OSC Detune                    |
| `00 05`      | 7    | OSC Pulse Width Mod Depth     |
| `00 06`      | 7    | OSC Pulse Width               |
| `00 07`      | 7    | OSC Pitch Env Attack Time     |
| `00 08`      | 7    | OSC Pitch Env Decay           |
| `00 09`      | 7    | OSC Pitch Env Depth           |
| `00 0A`      | 3    | FILTER Mode                   |
| `00 0B`      | 1    | FILTER Slope                  |
| `00 0C`      | 7    | FILTER Cutoff                 |
| `00 0D`      | 6    | FILTER Cutoff Keyfollow       |
| `00 0E`      | 7    | FILTER Env Velocity Sens      |
| `00 0F`      | 7    | FILTER Resonance              |
| `00 10`      | 7    | FILTER Env Attack Time        |
| `00 11`      | 7    | FILTER Env Decay Time         |
| `00 12`      | 7    | FILTER Env Sustain Level      |
| `00 13`      | 7    | FILTER Env Release Time       |
| `00 14`      | 7    | FILTER Env Depth              |
| `00 15`      | 7    | AMP Level                     |
| `00 16`      | 7    | AMP Level Velocity Sens       |
| `00 17`      | 7    | AMP Env Attack Time           |
| `00 18`      | 7    | AMP Env Decay Time            |
| `00 19`      | 7    | AMP Env Sustain Level         |
| `00 1A`      | 7    | AMP Env Release Time          |
| `00 1B`      | 7    | AMP Pan                       |
| `00 1C`      | 3    | LFO Shape                     |
| `00 1D`      | 7    | LFO Rate                      |
| `00 1E`      | 1    | LFO Tempo Sync Switch         |
| `00 1F`      | 5    | LFO Tempo Sync Note           |
| `00 20`      | 7    | LFO Fade Time                 |
| `00 21`      | 1    | LFO Key Trigger               |
| `00 22`      | 7    | LFO Pitch Depth               |
| `00 23`      | 7    | LFO Filter Depth              |
| `00 24`      | 7    | LFO Amp Depth                 |
| `00 25`      | 7    | LFO Pan Depth                 |
| `00 26`      | 3    | Modulation LFO Shape          |
| `00 27`      | 7    | Modulation LFO Rate           |
| `00 28`      | 1    | Modulation LFO Tempo Sync     |
| `00 29`      | 5    | Modulation LFO Tempo Sync Note|
| `00 2A`      | 7    | OSC Pulse Width Shift         |
| `00 2B`      | 1    | (reserve)                     |
| `00 2C`      | 7    | Mod LFO Pitch Depth           |
| `00 2D`      | 7    | Mod LFO Filter Depth          |
| `00 2E`      | 7    | Mod LFO Amp Depth             |
| `00 2F`      | 7    | Mod LFO Pan Depth             |
| `00 30`      | 7    | Cutoff Aftertouch Sens        |
| `00 31`      | 7    | Level Aftertouch Sens         |
| `00 32`      | 7    | (reserve)                     |
| `00 33`      | 7    | (reserve)                     |
| `00 34`      | 2    | Wave Gain                     |
| `00 35`      | 4×4  | Wave Number (nibblized)       |
| `00 39`      | 7    | HPF Cutoff                    |
| `00 3A`      | 7    | Super Saw Detune              |
| `00 3B`      | 7    | Mod LFO Rate Control          |
| `00 3C`      | 5    | AMP Level Keyfollow           |

**Total Partial: 350 bits → padded to 368 bits = 46 bytes**

## Validation Evidence

1. **Patch names**: All 128 Synth Legends patches decode correctly as 7-bit
   ASCII from the first 84 bits of each entry (e.g., "SL-JP8 1",
   "SL-TB Saw 1", "SL-D50 1")
2. **Device comparison**: `svd-validate` reads each SysEx block from the
   physical device and compares byte-by-byte with the SVD decode.
   MFX (145 bytes) and all unsigned params match exactly.
3. **Section boundaries**: Common (30B) + MFX (78B) + 3 Partials (46B each)
   confirmed by MFX Type byte appearing at offset 30
4. **Signed param bias**: 11 bytes (out of 336 total) differ due to the
   bias encoding for params with non-zero range minimums.
   Formula: `sysex = svd + 64 - 2^(bits-1)`
5. **End marker**: Every entry has `0x0E` at byte 246
6. **Total size**: 30 + 78 + 46×3 + 1 + 33 = 280 bytes
