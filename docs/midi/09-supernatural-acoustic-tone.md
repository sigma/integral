# SuperNATURAL Acoustic Tone Parameters

Base offset within Temporary Tone: `02 00 00`.

The SuperNATURAL Acoustic Tone is organized into two sub-blocks: Common and
Common MFX.

---

## SN Acoustic Tone Common (offset `00 00 00`)

Total size: `00 00 00 46`.

| Offset  | Bits        | Parameter                | Range      | Display            |
|---------|-------------|--------------------------|------------|--------------------|
| `00 00` | `0aaa aaaa` | Tone Name 1              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 01` | `0aaa aaaa` | Tone Name 2              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 02` | `0aaa aaaa` | Tone Name 3              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 03` | `0aaa aaaa` | Tone Name 4              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 04` | `0aaa aaaa` | Tone Name 5              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 05` | `0aaa aaaa` | Tone Name 6              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 06` | `0aaa aaaa` | Tone Name 7              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 07` | `0aaa aaaa` | Tone Name 8              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 08` | `0aaa aaaa` | Tone Name 9              | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 09` | `0aaa aaaa` | Tone Name 10             | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0A` | `0aaa aaaa` | Tone Name 11             | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0B` | `0aaa aaaa` | Tone Name 12             | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0C` | `0aaa aaaa` | (reserve)                | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0D` | `0aaa aaaa` | (reserve)                | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0E` | `0aaa aaaa` | (reserve)                | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0F` | `0aaa aaaa` | (reserve)                | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 10` | `0aaa aaaa` | Tone Level               | 0 -- 127   | 0 -- 127           |
| `00 11` | `0000 000a` | Mono/Poly                | 0 -- 1     | MONO, POLY         |
| `00 12` | `0aaa aaaa` | Portamento Time Offset   | 0 -- 127   | -64 -- +63         |
| `00 13` | `0aaa aaaa` | Cutoff Offset            | 0 -- 127   | -64 -- +63         |
| `00 14` | `0aaa aaaa` | Resonance Offset         | 0 -- 127   | -64 -- +63         |
| `00 15` | `0aaa aaaa` | Attack Time Offset       | 0 -- 127   | -64 -- +63         |
| `00 16` | `0aaa aaaa` | Release Time Offset      | 0 -- 127   | -64 -- +63         |
| `00 17` | `0aaa aaaa` | Vibrato Rate             | 0 -- 127   | -64 -- +63         |
| `00 18` | `0aaa aaaa` | Vibrato Depth            | 0 -- 127   | -64 -- +63         |
| `00 19` | `0aaa aaaa` | Vibrato Delay            | 0 -- 127   | -64 -- +63         |
| `00 1A` | `0000 0aaa` | Octave Shift             | 61 -- 67   | -3 -- +3           |
| `00 1B` | `0aaa aaaa` | Category                 | 0 -- 127   | 0 -- 127           |
| `00 1C` | `0000 aaaa` | Phrase Number #          | 0 -- 255   | 0 -- 255           |
|         | `0000 bbbb` |                          |            |                    |
| `00 1E` | `0000 0aaa` | Phrase Octave Shift      | 61 -- 67   | -3 -- +3           |
| `00 1F` | `0000 000a` | TFX Switch               | 0 -- 1     | OFF, ON            |
| `00 20` | `0aaa aaaa` | Inst Variation           | 0 -- 127   | 0 -- 127           |
| `00 21` | `0aaa aaaa` | Inst Number              | 0 -- 127   | 0 -- 127           |
| `00 22` | `0aaa aaaa` | Modify Parameter 1       | 0 -- 127   | 0 -- 127           |
| `00 23` | `0aaa aaaa` | Modify Parameter 2       | 0 -- 127   | 0 -- 127           |
| `00 24` | `0aaa aaaa` | Modify Parameter 3       | 0 -- 127   | 0 -- 127           |
| `00 25` | `0aaa aaaa` | Modify Parameter 4       | 0 -- 127   | 0 -- 127           |
| `00 26` | `0aaa aaaa` | Modify Parameter 5       | 0 -- 127   | 0 -- 127           |
| `00 27` | `0aaa aaaa` | Modify Parameter 6       | 0 -- 127   | 0 -- 127           |
| `00 28` | `0aaa aaaa` | Modify Parameter 7       | 0 -- 127   | 0 -- 127           |
| `00 29` | `0aaa aaaa` | Modify Parameter 8       | 0 -- 127   | 0 -- 127           |
| `00 2A` | `0aaa aaaa` | Modify Parameter 9       | 0 -- 127   | 0 -- 127           |
| `00 2B` | `0aaa aaaa` | Modify Parameter 10      | 0 -- 127   | 0 -- 127           |
| `00 2C` | `0aaa aaaa` | Modify Parameter 11      | 0 -- 127   | 0 -- 127           |
| `00 2D` | `0aaa aaaa` | Modify Parameter 12      | 0 -- 127   | 0 -- 127           |
| `00 2E` | `0aaa aaaa` | Modify Parameter 13      | 0 -- 127   | 0 -- 127           |
| `00 2F` | `0aaa aaaa` | Modify Parameter 14      | 0 -- 127   | 0 -- 127           |
| `00 30` | `0aaa aaaa` | Modify Parameter 15      | 0 -- 127   | 0 -- 127           |
| `00 31` | `0aaa aaaa` | Modify Parameter 16      | 0 -- 127   | 0 -- 127           |
| `00 32` | `0aaa aaaa` | Modify Parameter 17      | 0 -- 127   | 0 -- 127           |
| `00 33` | `0aaa aaaa` | Modify Parameter 18      | 0 -- 127   | 0 -- 127           |
| `00 34` | `0aaa aaaa` | Modify Parameter 19      | 0 -- 127   | 0 -- 127           |
| `00 35` | `0aaa aaaa` | Modify Parameter 20      | 0 -- 127   | 0 -- 127           |
| `00 36` | `0aaa aaaa` | Modify Parameter 21      | 0 -- 127   | 0 -- 127           |
| `00 37` | `0aaa aaaa` | Modify Parameter 22      | 0 -- 127   | 0 -- 127           |
| `00 38` | `0aaa aaaa` | Modify Parameter 23      | 0 -- 127   | 0 -- 127           |
| `00 39` | `0aaa aaaa` | Modify Parameter 24      | 0 -- 127   | 0 -- 127           |
| `00 3A` | `0aaa aaaa` | Modify Parameter 25      | 0 -- 127   | 0 -- 127           |
| `00 3B` | `0aaa aaaa` | Modify Parameter 26      | 0 -- 127   | 0 -- 127           |
| `00 3C` | `0aaa aaaa` | Modify Parameter 27      | 0 -- 127   | 0 -- 127           |
| `00 3D` | `0aaa aaaa` | Modify Parameter 28      | 0 -- 127   | 0 -- 127           |
| `00 3E` | `0aaa aaaa` | Modify Parameter 29      | 0 -- 127   | 0 -- 127           |
| `00 3F` | `0aaa aaaa` | Modify Parameter 30      | 0 -- 127   | 0 -- 127           |
| `00 40` | `0aaa aaaa` | Modify Parameter 31      | 0 -- 127   | 0 -- 127           |
| `00 41` | `0aaa aaaa` | Modify Parameter 32      | 0 -- 127   | 0 -- 127           |
| `00 42` | `0aaa aaaa` | (reserve) \<\*>          | --         | --                 |
| `00 43` | `0aaa aaaa` | (reserve) \<\*>          | --         | --                 |
| `00 44` | `0aaa aaaa` | (reserve) \<\*>          | --         | --                 |
| `00 45` | `0aaa aaaa` | (reserve) \<\*>          | --         | --                 |

---

## SN Acoustic Tone MFX (offset `00 02 00`)

Total size: `00 00 01 11`.

| Offset  | Bits        | Parameter              | Range          | Display                                          |
|---------|-------------|------------------------|----------------|--------------------------------------------------|
| `00 00` | `0aaa aaaa` | MFX Type               | 0 -- 67        | (see MFX Type list)                              |
| `00 01` | `0aaa aaaa` | (reserve) \<\*>        | --             | --                                               |
| `00 02` | `0aaa aaaa` | MFX Chorus Send Level  | 0 -- 127       | 0 -- 127                                         |
| `00 03` | `0aaa aaaa` | MFX Reverb Send Level  | 0 -- 127       | 0 -- 127                                         |
| `00 04` | `0000 00aa` | (reserve) \<\*>        | --             | --                                               |
| `00 05` | `0aaa aaaa` | MFX Control 1 Source   | 0 -- 101       | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT, SYS1 -- SYS4 |
| `00 06` | `0aaa aaaa` | MFX Control 1 Sens     | 1 -- 127       | -63 -- +63                                       |
| `00 07` | `0aaa aaaa` | MFX Control 2 Source   | 0 -- 101       | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT, SYS1 -- SYS4 |
| `00 08` | `0aaa aaaa` | MFX Control 2 Sens     | 1 -- 127       | -63 -- +63                                       |
| `00 09` | `0aaa aaaa` | MFX Control 3 Source   | 0 -- 101       | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT, SYS1 -- SYS4 |
| `00 0A` | `0aaa aaaa` | MFX Control 3 Sens     | 1 -- 127       | -63 -- +63                                       |
| `00 0B` | `0aaa aaaa` | MFX Control 4 Source   | 0 -- 101       | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT, SYS1 -- SYS4 |
| `00 0C` | `0aaa aaaa` | MFX Control 4 Sens     | 1 -- 127       | -63 -- +63                                       |
| `00 0D` | `000a aaaa` | MFX Control Assign 1   | 0 -- 16        | OFF, 1 -- 16                                     |
| `00 0E` | `000a aaaa` | MFX Control Assign 2   | 0 -- 16        | OFF, 1 -- 16                                     |
| `00 0F` | `000a aaaa` | MFX Control Assign 3   | 0 -- 16        | OFF, 1 -- 16                                     |
| `00 10` | `000a aaaa` | MFX Control Assign 4   | 0 -- 16        | OFF, 1 -- 16                                     |
| `00 11` | `0000 aaaa` | MFX Parameter 1 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 15` | `0000 aaaa` | MFX Parameter 2 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 19` | `0000 aaaa` | MFX Parameter 3 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 1D` | `0000 aaaa` | MFX Parameter 4 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 21` | `0000 aaaa` | MFX Parameter 5 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 25` | `0000 aaaa` | MFX Parameter 6 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 29` | `0000 aaaa` | MFX Parameter 7 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 2D` | `0000 aaaa` | MFX Parameter 8 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 31` | `0000 aaaa` | MFX Parameter 9 #      | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 35` | `0000 aaaa` | MFX Parameter 10 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 39` | `0000 aaaa` | MFX Parameter 11 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 3D` | `0000 aaaa` | MFX Parameter 12 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 41` | `0000 aaaa` | MFX Parameter 13 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 45` | `0000 aaaa` | MFX Parameter 14 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 49` | `0000 aaaa` | MFX Parameter 15 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 4D` | `0000 aaaa` | MFX Parameter 16 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 51` | `0000 aaaa` | MFX Parameter 17 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 55` | `0000 aaaa` | MFX Parameter 18 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 59` | `0000 aaaa` | MFX Parameter 19 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 5D` | `0000 aaaa` | MFX Parameter 20 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 61` | `0000 aaaa` | MFX Parameter 21 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 65` | `0000 aaaa` | MFX Parameter 22 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 69` | `0000 aaaa` | MFX Parameter 23 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 6D` | `0000 aaaa` | MFX Parameter 24 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 71` | `0000 aaaa` | MFX Parameter 25 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 75` | `0000 aaaa` | MFX Parameter 26 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 79` | `0000 aaaa` | MFX Parameter 27 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `00 7D` | `0000 aaaa` | MFX Parameter 28 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `01 01` | `0000 aaaa` | MFX Parameter 29 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `01 05` | `0000 aaaa` | MFX Parameter 30 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `01 09` | `0000 aaaa` | MFX Parameter 31 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
| `01 0D` | `0000 aaaa` | MFX Parameter 32 #     | 12768 -- 52768 | -20000 -- +20000                                 |
|         | `0000 bbbb` |                        |                |                                                  |
|         | `0000 cccc` |                        |                |                                                  |
|         | `0000 dddd` |                        |                |                                                  |
