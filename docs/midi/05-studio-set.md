# Studio Set Parameters

Base address: **18 00 00 00**

The Studio Set is the top-level performance container in the INTEGRA-7. It holds
global mix settings, effects routing, per-channel MIDI configuration, 16 parts
with full tone/mix/controller setup, and per-part EQ.

---

## 1. Studio Set Common

Offset from base: **00 00 00**
Total size: **00 00 00 54**

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0aaa aaaa | Studio Set Name 1 | 32 - 127 | ASCII |
| 00 01 | 0aaa aaaa | Studio Set Name 2 | 32 - 127 | ASCII |
| 00 02 | 0aaa aaaa | Studio Set Name 3 | 32 - 127 | ASCII |
| 00 03 | 0aaa aaaa | Studio Set Name 4 | 32 - 127 | ASCII |
| 00 04 | 0aaa aaaa | Studio Set Name 5 | 32 - 127 | ASCII |
| 00 05 | 0aaa aaaa | Studio Set Name 6 | 32 - 127 | ASCII |
| 00 06 | 0aaa aaaa | Studio Set Name 7 | 32 - 127 | ASCII |
| 00 07 | 0aaa aaaa | Studio Set Name 8 | 32 - 127 | ASCII |
| 00 08 | 0aaa aaaa | Studio Set Name 9 | 32 - 127 | ASCII |
| 00 09 | 0aaa aaaa | Studio Set Name 10 | 32 - 127 | ASCII |
| 00 0A | 0aaa aaaa | Studio Set Name 11 | 32 - 127 | ASCII |
| 00 0B | 0aaa aaaa | Studio Set Name 12 | 32 - 127 | ASCII |
| 00 0C | 0aaa aaaa | Studio Set Name 13 | 32 - 127 | ASCII |
| 00 0D | 0aaa aaaa | Studio Set Name 14 | 32 - 127 | ASCII |
| 00 0E | 0aaa aaaa | Studio Set Name 15 | 32 - 127 | ASCII |
| 00 0F | 0aaa aaaa | Studio Set Name 16 | 32 - 127 | ASCII |
| 00 18 | 0aaa aaaa | Voice Reserve 1 | 0 - 64 | 0 - 63, FULL |
| 00 19 | 0aaa aaaa | Voice Reserve 2 | 0 - 64 | 0 - 63, FULL |
| 00 1A | 0aaa aaaa | Voice Reserve 3 | 0 - 64 | 0 - 63, FULL |
| 00 1B | 0aaa aaaa | Voice Reserve 4 | 0 - 64 | 0 - 63, FULL |
| 00 1C | 0aaa aaaa | Voice Reserve 5 | 0 - 64 | 0 - 63, FULL |
| 00 1D | 0aaa aaaa | Voice Reserve 6 | 0 - 64 | 0 - 63, FULL |
| 00 1E | 0aaa aaaa | Voice Reserve 7 | 0 - 64 | 0 - 63, FULL |
| 00 1F | 0aaa aaaa | Voice Reserve 8 | 0 - 64 | 0 - 63, FULL |
| 00 20 | 0aaa aaaa | Voice Reserve 9 | 0 - 64 | 0 - 63, FULL |
| 00 21 | 0aaa aaaa | Voice Reserve 10 | 0 - 64 | 0 - 63, FULL |
| 00 22 | 0aaa aaaa | Voice Reserve 11 | 0 - 64 | 0 - 63, FULL |
| 00 23 | 0aaa aaaa | Voice Reserve 12 | 0 - 64 | 0 - 63, FULL |
| 00 24 | 0aaa aaaa | Voice Reserve 13 | 0 - 64 | 0 - 63, FULL |
| 00 25 | 0aaa aaaa | Voice Reserve 14 | 0 - 64 | 0 - 63, FULL |
| 00 26 | 0aaa aaaa | Voice Reserve 15 | 0 - 64 | 0 - 63, FULL |
| 00 27 | 0aaa aaaa | Voice Reserve 16 | 0 - 64 | 0 - 63, FULL |
| 00 39 | 0aaa aaaa | Tone Control 1 Source | 0 - 97 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT |
| 00 3A | 0aaa aaaa | Tone Control 2 Source | 0 - 97 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT |
| 00 3B | 0aaa aaaa | Tone Control 3 Source | 0 - 97 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT |
| 00 3C | 0aaa aaaa | Tone Control 4 Source | 0 - 97 | OFF, CC01 - CC31, CC33 - CC95, BEND, AFT |
| 00 3D | 0000 aaaa 0000 bbbb | Studio Set Tempo (#) | 20 - 250 | 20 - 250 |
| 00 3F | 000a aaaa | Solo Part | 0 - 16 | OFF, 1 - 16 |
| 00 40 | 0000 000a | Reverb Switch | 0 - 1 | OFF, ON |
| 00 41 | 0000 000a | Chorus Switch | 0 - 1 | OFF, ON |
| 00 42 | 0000 000a | Master EQ Switch | 0 - 1 | OFF, ON |
| 00 43 | 0000 000a | Drum Comp/EQ Switch | 0 - 1 | OFF, ON |
| 00 44 | 0000 aaaa | Drum Comp/EQ Part | 0 - 15 | 1 - 16 |
| 00 45 | 0000 aaaa | Drum Comp/EQ 1 Output Assign | 0 - 12 | PART, A, B, C, D, 1, 2, 3, 4, 5, 6, 7, 8 |
| 00 46 | 0000 aaaa | Drum Comp/EQ 2 Output Assign | 0 - 12 | PART, A, B, C, D, 1, 2, 3, 4, 5, 6, 7, 8 |
| 00 47 | 0000 aaaa | Drum Comp/EQ 3 Output Assign | 0 - 12 | PART, A, B, C, D, 1, 2, 3, 4, 5, 6, 7, 8 |
| 00 48 | 0000 aaaa | Drum Comp/EQ 4 Output Assign | 0 - 12 | PART, A, B, C, D, 1, 2, 3, 4, 5, 6, 7, 8 |
| 00 49 | 0000 aaaa | Drum Comp/EQ 5 Output Assign | 0 - 12 | PART, A, B, C, D, 1, 2, 3, 4, 5, 6, 7, 8 |
| 00 4A | 0000 aaaa | Drum Comp/EQ 6 Output Assign | 0 - 12 | PART, A, B, C, D, 1, 2, 3, 4, 5, 6, 7, 8 |
| 00 4C | 0aaa aaaa | Ext Part Level | 0 - 127 | 0 - 127 |
| 00 4D | 0aaa aaaa | Ext Part Chorus Send Level | 0 - 127 | 0 - 127 |
| 00 4E | 0aaa aaaa | Ext Part Reverb Send Level | 0 - 127 | 0 - 127 |
| 00 4F | 0000 000a | Ext Part Mute Switch | 0 - 1 | OFF, ON |

> **Note:** Offsets 00 10 - 00 17, 00 28 - 00 38, 00 4B, and 00 50 - 00 53 are
> reserved.

---

## 2. Studio Set Common Chorus

Offset from base: **00 04 00**
Total size: **00 00 00 54**

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0000 aaaa | Chorus Type | 0 - 3 | 0 - 3 |
| 00 01 | 0aaa aaaa | Chorus Level | 0 - 127 | 0 - 127 |
| 00 03 | 0000 00aa | Chorus Output Select | 0 - 2 | MAIN, REV, MAIN+REV |
| 00 04 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 1 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 08 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 2 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 0C | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 3 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 10 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 4 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 14 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 5 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 18 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 6 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 1C | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 7 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 20 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 8 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 24 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 9 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 28 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 10 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 2C | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 11 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 30 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 12 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 34 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 13 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 38 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 14 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 3C | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 15 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 40 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 16 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 44 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 17 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 48 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 18 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 4C | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 19 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 50 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Chorus Parameter 20 (#) | 12768 - 52768 | -20000 - +20000 |

> **Note:** Offset 00 02 is reserved. All Chorus Parameters are nibblized
> 4-byte values (marked with #).

---

## 3. Studio Set Common Reverb

Offset from base: **00 06 00**
Total size: **00 00 00 63**

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0000 aaaa | Reverb Type | 0 - 6 | 0 - 6 |
| 00 01 | 0aaa aaaa | Reverb Level | 0 - 127 | 0 - 127 |
| 00 02 | 0000 00aa | Reverb Output Assign | 0 - 3 | A, B, C, D |
| 00 03 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 1 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 07 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 2 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 0B | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 3 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 0F | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 4 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 13 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 5 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 17 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 6 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 1B | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 7 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 1F | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 8 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 23 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 9 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 27 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 10 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 2B | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 11 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 2F | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 12 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 33 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 13 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 37 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 14 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 3B | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 15 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 3F | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 16 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 43 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 17 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 47 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 18 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 4B | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 19 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 4F | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 20 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 53 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 21 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 57 | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 22 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 5B | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 23 (#) | 12768 - 52768 | -20000 - +20000 |
| 00 5F | 0000 aaaa 0000 bbbb 0000 cccc 0000 dddd | Reverb Parameter 24 (#) | 12768 - 52768 | -20000 - +20000 |

> **Note:** All Reverb Parameters are nibblized 4-byte values (marked with #).

---

## 4. Studio Set Common Motional Surround

Offset from base: **00 08 00**
Total size: **00 00 00 10**

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0000 000a | Motional Surround Switch | 0 - 1 | OFF, ON |
| 00 01 | 0000 00aa | Room Type | 0 - 3 | ROOM1, ROOM2, HALL1, HALL2 |
| 00 02 | 0aaa aaaa | Ambience Level | 0 - 127 | 0 - 127 |
| 00 03 | 0aaa aaaa | Room Size | 0 - 2 | SMALL, MEDIUM, LARGE |
| 00 04 | 0aaa aaaa | Ambience Time | 0 - 100 | 0 - 100 |
| 00 05 | 0aaa aaaa | Ambience Density | 0 - 100 | 0 - 100 |
| 00 06 | 0aaa aaaa | Ambience HF Damp | 0 - 100 | 0 - 100 |
| 00 07 | 0aaa aaaa | Ext Part L-R | 0 - 127 | -64 - +63 |
| 00 08 | 0aaa aaaa | Ext Part F-B | 0 - 127 | -64 - +63 |
| 00 09 | 00aa aaaa | Ext Part Width | 0 - 32 | 0 - 32 |
| 00 0A | 0aaa aaaa | Ext Part Ambience Send Level | 0 - 127 | 0 - 127 |
| 00 0B | 000a aaaa | Ext Part Control Channel | 0 - 16 | 1 - 16, OFF |
| 00 0C | 0aaa aaaa | Motional Surround Depth | 0 - 100 | 0 - 100 |

> **Note:** Offsets 00 0D - 00 0F are reserved.

---

## 5. Studio Set Master EQ

Offset from base: **00 09 00**
Total size: **00 00 00 07**

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0000 000a | EQ Low Freq | 0 - 1 | 200, 400 Hz |
| 00 01 | 000a aaaa | EQ Low Gain | 0 - 30 | -15 - +15 dB |
| 00 02 | 000a aaaa | EQ Mid Freq | 0 - 16 | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz |
| 00 03 | 000a aaaa | EQ Mid Gain | 0 - 30 | -15 - +15 dB |
| 00 04 | 0000 0aaa | EQ Mid Q | 0 - 4 | 0.5, 1.0, 2.0, 4.0, 8.0 |
| 00 05 | 0000 00aa | EQ High Freq | 0 - 2 | 2000, 4000, 8000 Hz |
| 00 06 | 000a aaaa | EQ High Gain | 0 - 30 | -15 - +15 dB |

---

## 6. Studio Set MIDI (per channel)

Offset from base: **00 10 00** through **00 1F 00** (channels 1 - 16)
Total size per channel: **00 00 00 01**

Channel *n* (1-16) is at offset **00 {0F+n} 00**, i.e.:

| Channel | Offset |
|---------|--------|
| 1 | 00 10 00 |
| 2 | 00 11 00 |
| 3 | 00 12 00 |
| 4 | 00 13 00 |
| 5 | 00 14 00 |
| 6 | 00 15 00 |
| 7 | 00 16 00 |
| 8 | 00 17 00 |
| 9 | 00 18 00 |
| 10 | 00 19 00 |
| 11 | 00 1A 00 |
| 12 | 00 1B 00 |
| 13 | 00 1C 00 |
| 14 | 00 1D 00 |
| 15 | 00 1E 00 |
| 16 | 00 1F 00 |

Each channel block contains a single parameter:

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0000 000a | Phase Lock | 0 - 1 | OFF, ON |

---

## 7. Studio Set Part (per part)

Offset from base: **00 20 00** through **00 2F 00** (parts 1 - 16)
Total size per part: **00 00 00 4D**

Part *n* (1-16) is at offset **00 {1F+n} 00**, i.e.:

| Part | Offset |
|------|--------|
| 1 | 00 20 00 |
| 2 | 00 21 00 |
| 3 | 00 22 00 |
| 4 | 00 23 00 |
| 5 | 00 24 00 |
| 6 | 00 25 00 |
| 7 | 00 26 00 |
| 8 | 00 27 00 |
| 9 | 00 28 00 |
| 10 | 00 29 00 |
| 11 | 00 2A 00 |
| 12 | 00 2B 00 |
| 13 | 00 2C 00 |
| 14 | 00 2D 00 |
| 15 | 00 2E 00 |
| 16 | 00 2F 00 |

Each part block contains:

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0000 aaaa | Receive Channel | 0 - 15 | 1 - 16 |
| 00 01 | 0000 000a | Receive Switch | 0 - 1 | OFF, ON |
| 00 06 | 0aaa aaaa | Tone Bank Select MSB (CC# 0) | 0 - 127 | 0 - 127 |
| 00 07 | 0aaa aaaa | Tone Bank Select LSB (CC# 32) | 0 - 127 | 0 - 127 |
| 00 08 | 0aaa aaaa | Tone Program Number (PC) | 0 - 127 | 0 - 127 |
| 00 09 | 0aaa aaaa | Part Level (CC# 7) | 0 - 127 | 0 - 127 |
| 00 0A | 0aaa aaaa | Part Pan (CC# 10) | 0 - 127 | L64 - 63R |
| 00 0B | 0aaa aaaa | Part Coarse Tune (RPN# 2) | 16 - 112 | -48 - +48 |
| 00 0C | 0aaa aaaa | Part Fine Tune (RPN# 1) | 14 - 114 | -50 - +50 |
| 00 0D | 0000 00aa | Part Mono/Poly | 0 - 2 | MONO, POLY, TONE |
| 00 0E | 0000 00aa | Part Legato Switch (CC# 68) | 0 - 2 | OFF, ON, TONE |
| 00 0F | 000a aaaa | Part Pitch Bend Range (RPN# 0) | 0 - 25 | 0 - 24, TONE |
| 00 10 | 0000 00aa | Part Portamento Switch (CC# 65) | 0 - 2 | OFF, ON, TONE |
| 00 11 | 0000 aaaa 0000 bbbb | Part Portamento Time (CC# 5) (#) | 0 - 128 | 0 - 127, TONE |
| 00 13 | 0aaa aaaa | Part Cutoff Offset (CC# 74) | 0 - 127 | -64 - +63 |
| 00 14 | 0aaa aaaa | Part Resonance Offset (CC# 71) | 0 - 127 | -64 - +63 |
| 00 15 | 0aaa aaaa | Part Attack Time Offset (CC# 73) | 0 - 127 | -64 - +63 |
| 00 16 | 0aaa aaaa | Part Decay Time Offset (CC# 75) | 0 - 127 | -64 - +63 |
| 00 17 | 0aaa aaaa | Part Release Time Offset (CC# 72) | 0 - 127 | -64 - +63 |
| 00 18 | 0aaa aaaa | Part Vibrato Rate (CC# 76) | 0 - 127 | -64 - +63 |
| 00 19 | 0aaa aaaa | Part Vibrato Depth (CC# 77) | 0 - 127 | -64 - +63 |
| 00 1A | 0aaa aaaa | Part Vibrato Delay (CC# 78) | 0 - 127 | -64 - +63 |
| 00 1B | 0000 0aaa | Part Octave Shift | 61 - 67 | -3 - +3 |
| 00 1C | 0aaa aaaa | Part Velocity Sens Offset | 1 - 127 | -63 - +63 |
| 00 1D | 0aaa aaaa | Keyboard Range Lower | 0 - 127 | C-1 - UPPER |
| 00 1E | 0aaa aaaa | Keyboard Range Upper | 0 - 127 | LOWER - G9 |
| 00 1F | 0aaa aaaa | Keyboard Fade Width Lower | 0 - 127 | 0 - 127 |
| 00 20 | 0aaa aaaa | Keyboard Fade Width Upper | 0 - 127 | 0 - 127 |
| 00 21 | 0aaa aaaa | Velocity Range Lower | 1 - 127 | 1 - UPPER |
| 00 22 | 0aaa aaaa | Velocity Range Upper | 0 - 127 | LOWER - 127 |
| 00 23 | 0aaa aaaa | Velocity Fade Width Lower | 0 - 127 | 0 - 127 |
| 00 24 | 0aaa aaaa | Velocity Fade Width Upper | 0 - 127 | 0 - 127 |
| 00 25 | 0000 000a | Mute Switch | 0 - 1 | OFF, MUTE |
| 00 27 | 0aaa aaaa | Part Chorus Send Level (CC# 93) | 0 - 127 | 0 - 127 |
| 00 28 | 0aaa aaaa | Part Reverb Send Level (CC# 91) | 0 - 127 | 0 - 127 |
| 00 29 | 0000 aaaa | Part Output Assign | 0 - 11 | A, B, C, D, 1, 2, 3, 4, 5, 6, 7, 8 |
| 00 2B | 0aaa aaaa | Part Scale Tune Type | 0 - 8 | CUSTOM, EQUAL, JUST-MAJ, JUST-MIN, PYTHAGORE, KIRNBERGE, MEANTONE, WERCKMEIS, ARABIC |
| 00 2C | 0aaa aaaa | Part Scale Tune Key | 0 - 11 | C, C#, D, D#, E, F, F#, G, G#, A, A#, B |
| 00 2D | 0aaa aaaa | Part Scale Tune for C | 0 - 127 | -64 - +63 |
| 00 2E | 0aaa aaaa | Part Scale Tune for C# | 0 - 127 | -64 - +63 |
| 00 2F | 0aaa aaaa | Part Scale Tune for D | 0 - 127 | -64 - +63 |
| 00 30 | 0aaa aaaa | Part Scale Tune for D# | 0 - 127 | -64 - +63 |
| 00 31 | 0aaa aaaa | Part Scale Tune for E | 0 - 127 | -64 - +63 |
| 00 32 | 0aaa aaaa | Part Scale Tune for F | 0 - 127 | -64 - +63 |
| 00 33 | 0aaa aaaa | Part Scale Tune for F# | 0 - 127 | -64 - +63 |
| 00 34 | 0aaa aaaa | Part Scale Tune for G | 0 - 127 | -64 - +63 |
| 00 35 | 0aaa aaaa | Part Scale Tune for G# | 0 - 127 | -64 - +63 |
| 00 36 | 0aaa aaaa | Part Scale Tune for A | 0 - 127 | -64 - +63 |
| 00 37 | 0aaa aaaa | Part Scale Tune for A# | 0 - 127 | -64 - +63 |
| 00 38 | 0aaa aaaa | Part Scale Tune for B | 0 - 127 | -64 - +63 |
| 00 39 | 0000 000a | Receive Program Change | 0 - 1 | OFF, ON |
| 00 3A | 0000 000a | Receive Bank Select | 0 - 1 | OFF, ON |
| 00 3B | 0000 000a | Receive Pitch Bend | 0 - 1 | OFF, ON |
| 00 3C | 0000 000a | Receive Polyphonic Key Pressure | 0 - 1 | OFF, ON |
| 00 3D | 0000 000a | Receive Channel Pressure | 0 - 1 | OFF, ON |
| 00 3E | 0000 000a | Receive Modulation | 0 - 1 | OFF, ON |
| 00 3F | 0000 000a | Receive Volume | 0 - 1 | OFF, ON |
| 00 40 | 0000 000a | Receive Pan | 0 - 1 | OFF, ON |
| 00 41 | 0000 000a | Receive Expression | 0 - 1 | OFF, ON |
| 00 42 | 0000 000a | Receive Hold-1 | 0 - 1 | OFF, ON |
| 00 43 | 0000 0aaa | Velocity Curve Type | 0 - 4 | OFF, 1, 2, 3, 4 |
| 00 44 | 0aaa aaaa | Motional Surround L-R | 0 - 127 | -64 - +63 |
| 00 46 | 0aaa aaaa | Motional Surround F-B | 0 - 127 | -64 - +63 |
| 00 48 | 00aa aaaa | Motional Surround Width | 0 - 32 | 0 - 32 |
| 00 49 | 0aaa aaaa | Motional Surround Ambience Send Level | 0 - 127 | 0 - 127 |

> **Note:** Offsets 00 02 - 00 05, 00 26, 00 2A, 00 45, 00 47, and
> 00 4A - 00 4C are reserved. Portamento Time at 00 11 is a nibblized
> 2-byte value (marked with #).

---

## 8. Studio Set Part EQ (per part)

Offset from base: **00 50 00** through **00 5F 00** (parts 1 - 16)
Total size per part: **00 00 00 08**

Part *n* (1-16) is at offset **00 {4F+n} 00**, i.e.:

| Part | Offset |
|------|--------|
| 1 | 00 50 00 |
| 2 | 00 51 00 |
| 3 | 00 52 00 |
| 4 | 00 53 00 |
| 5 | 00 54 00 |
| 6 | 00 55 00 |
| 7 | 00 56 00 |
| 8 | 00 57 00 |
| 9 | 00 58 00 |
| 10 | 00 59 00 |
| 11 | 00 5A 00 |
| 12 | 00 5B 00 |
| 13 | 00 5C 00 |
| 14 | 00 5D 00 |
| 15 | 00 5E 00 |
| 16 | 00 5F 00 |

Each part EQ block contains:

| Offset | Bits | Parameter | Range | Display |
|--------|------|-----------|-------|---------|
| 00 00 | 0000 000a | EQ Switch | 0 - 1 | OFF, ON |
| 00 01 | 0000 000a | EQ Low Freq | 0 - 1 | 200, 400 Hz |
| 00 02 | 000a aaaa | EQ Low Gain | 0 - 30 | -15 - +15 dB |
| 00 03 | 000a aaaa | EQ Mid Freq | 0 - 16 | 200, 250, 315, 400, 500, 630, 800, 1000, 1250, 1600, 2000, 2500, 3150, 4000, 5000, 6300, 8000 Hz |
| 00 04 | 000a aaaa | EQ Mid Gain | 0 - 30 | -15 - +15 dB |
| 00 05 | 0000 0aaa | EQ Mid Q | 0 - 4 | 0.5, 1.0, 2.0, 4.0, 8.0 |
| 00 06 | 0000 00aa | EQ High Freq | 0 - 2 | 2000, 4000, 8000 Hz |
| 00 07 | 000a aaaa | EQ High Gain | 0 - 30 | -15 - +15 dB |
