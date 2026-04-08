# SuperNATURAL Synth Tone Parameters

Base offset within Temporary Tone: `01 00 00`.

The SuperNATURAL Synth Tone is organized into three sub-blocks: Common,
Common MFX, and up to three Partials.

---

## SN Synth Tone Common (offset `00 00 00`)

Total size: `00 00 00 40`.

| Offset  | Bits        | Parameter              | Range       | Display            |
|---------|-------------|------------------------|-------------|--------------------|
| `00 00` | `0aaa aaaa` | Tone Name 1            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 01` | `0aaa aaaa` | Tone Name 2            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 02` | `0aaa aaaa` | Tone Name 3            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 03` | `0aaa aaaa` | Tone Name 4            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 04` | `0aaa aaaa` | Tone Name 5            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 05` | `0aaa aaaa` | Tone Name 6            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 06` | `0aaa aaaa` | Tone Name 7            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 07` | `0aaa aaaa` | Tone Name 8            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 08` | `0aaa aaaa` | Tone Name 9            | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 09` | `0aaa aaaa` | Tone Name 10           | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 0A` | `0aaa aaaa` | Tone Name 11           | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 0B` | `0aaa aaaa` | Tone Name 12           | 32 -- 127   | 32 -- 127 (ASCII)  |
| `00 0C` | `0aaa aaaa` | Tone Level             | 0 -- 127    | 0 -- 127           |
| `00 0D` | `0000 aaaa` | (reserve) \<\*> #      | --          | --                 |
|         | `0000 bbbb` |                        |             |                    |
|         | `0000 cccc` |                        |             |                    |
| `00 10` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 11` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 12` | `0000 000a` | Portamento Switch      | 0 -- 1      | OFF, ON            |
| `00 13` | `0aaa aaaa` | Portamento Time        | 0 -- 127    | 0 -- 127           |
| `00 14` | `0000 00aa` | Mono Switch            | 0 -- 1      | OFF, ON            |
| `00 15` | `0000 0aaa` | Octave Shift           | 61 -- 67    | -3 -- +3           |
| `00 16` | `000a aaaa` | Pitch Bend Range Up    | 0 -- 24     | 0 -- 24            |
| `00 17` | `000a aaaa` | Pitch Bend Range Down  | 0 -- 24     | 0 -- 24            |
| `00 18` | `0000 0aaa` | (reserve) \<\*>        | --          | --                 |
| `00 19` | `0000 000a` | Partial1 Switch        | 0 -- 1      | OFF, ON            |
| `00 1A` | `0000 000a` | Partial1 Select        | 0 -- 1      | OFF, ON            |
| `00 1B` | `0000 000a` | Partial2 Switch        | 0 -- 1      | OFF, ON            |
| `00 1C` | `0000 000a` | Partial2 Select        | 0 -- 1      | OFF, ON            |
| `00 1D` | `0000 000a` | Partial3 Switch        | 0 -- 1      | OFF, ON            |
| `00 1E` | `0000 000a` | Partial3 Select        | 0 -- 1      | OFF, ON            |
| `00 1F` | `0000 00aa` | RING Switch            | 0 -- 2      | OFF, ---, ON       |
| `00 20` | `0000 000a` | TFX Switch             | 0 -- 1      | OFF, ON            |
| `00 21` | `0000 00aa` | (reserve) \<\*>        | --          | --                 |
| `00 22` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 23` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 24` | `00aa aaaa` | (reserve) \<\*>        | --          | --                 |
| `00 25` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 26` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 27` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 28` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 29` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 2A` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 2B` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 2C` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 2D` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 2E` | `0000 000a` | Unison Switch          | 0 -- 1      | OFF, ON            |
| `00 2F` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 30` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 31` | `0000 000a` | Portamento Mode        | 0 -- 1      | NORMAL, LEGATO     |
| `00 32` | `0000 000a` | Legato Switch          | 0 -- 1      | OFF, ON            |
| `00 33` | `0000 000a` | (reserve) \<\*>        | --          | --                 |
| `00 34` | `0aaa aaaa` | Analog Feel            | 0 -- 127    | 0 -- 127           |
| `00 35` | `0aaa aaaa` | Wave Shape             | 0 -- 127    | 0 -- 127           |
| `00 36` | `0aaa aaaa` | Tone Category          | 0 -- 127    | 0 -- 127           |
| `00 37` | `0000 aaaa` | Phrase Number #        | 0 -- 65535  | 0 -- 65535         |
|         | `0000 bbbb` |                        |             |                    |
|         | `0000 cccc` |                        |             |                    |
|         | `0000 dddd` |                        |             |                    |
| `00 3B` | `0000 0aaa` | Phrase Octave Shift    | 61 -- 67    | -3 -- +3           |
| `00 3C` | `0000 00aa` | Unison Size            | 0 -- 3      | 2, 4, 6, 8         |
| `00 3D` | `0aaa aaaa` | (reserve) \<\*>        | --          | --                 |
| `00 3E` | `0aaa aaaa` | (reserve) \<\*>        | --          | --                 |
| `00 3F` | `0aaa aaaa` | (reserve) \<\*>        | --          | --                 |

---

## SN Synth Tone MFX (offset `00 02 00`)

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

---

## SN Synth Tone Partial (offsets `00 20 00`, `00 21 00`, `00 22 00`)

Total size per partial: `00 00 00 3D`.

Three identical partial blocks are addressed at:
- **Partial 1:** offset `00 20 00`
- **Partial 2:** offset `00 21 00`
- **Partial 3:** offset `00 22 00`

| Offset  | Bits        | Parameter                           | Range      | Display                                              |
|---------|-------------|-------------------------------------|------------|------------------------------------------------------|
| `00 00` | `0000 0aaa` | OSC Wave                            | 0 -- 7     | SAW, SQR, PW-SQR, TRI, SINE, NOISE, SUPER-SAW, PCM  |
| `00 01` | `00aa aaaa` | OSC Wave Variation                  | 0 -- 2     | A, B, C                                              |
| `00 02` | `0000 00aa` | (reserve) \<\*>                     | --         | --                                                   |
| `00 03` | `00aa aaaa` | OSC Pitch                           | 40 -- 88   | -24 -- +24                                           |
| `00 04` | `0aaa aaaa` | OSC Detune                          | 14 -- 114  | -50 -- +50                                           |
| `00 05` | `0aaa aaaa` | OSC Pulse Width Mod Depth           | 0 -- 127   | 0 -- 127                                            |
| `00 06` | `0aaa aaaa` | OSC Pulse Width                     | 0 -- 127   | 0 -- 127                                            |
| `00 07` | `0aaa aaaa` | OSC Pitch Env Attack Time           | 0 -- 127   | 0 -- 127                                            |
| `00 08` | `0aaa aaaa` | OSC Pitch Env Decay                 | 0 -- 127   | 0 -- 127                                            |
| `00 09` | `0aaa aaaa` | OSC Pitch Env Depth                 | 1 -- 127   | -63 -- +63                                           |
| `00 0A` | `0000 0aaa` | FILTER Mode                         | 0 -- 7     | BYPASS, LPF, HPF, BPF, PKG, LPF2, LPF3, LPF4       |
| `00 0B` | `0000 000a` | FILTER Slope                        | 0 -- 1     | -12, -24 dB                                         |
| `00 0C` | `0aaa aaaa` | FILTER Cutoff                       | 0 -- 127   | 0 -- 127                                            |
| `00 0D` | `00aa aaaa` | FILTER Cutoff Keyfollow             | 54 -- 74   | -100 -- +100                                         |
| `00 0E` | `0aaa aaaa` | FILTER Env Velocity Sens            | 1 -- 127   | -63 -- +63                                           |
| `00 0F` | `0aaa aaaa` | FILTER Resonance                    | 0 -- 127   | 0 -- 127                                            |
| `00 10` | `0aaa aaaa` | FILTER Env Attack Time              | 0 -- 127   | 0 -- 127                                            |
| `00 11` | `0aaa aaaa` | FILTER Env Decay Time               | 0 -- 127   | 0 -- 127                                            |
| `00 12` | `0aaa aaaa` | FILTER Env Sustain Level            | 0 -- 127   | 0 -- 127                                            |
| `00 13` | `0aaa aaaa` | FILTER Env Release Time             | 0 -- 127   | 0 -- 127                                            |
| `00 14` | `0aaa aaaa` | FILTER Env Depth                    | 1 -- 127   | -63 -- +63                                           |
| `00 15` | `0aaa aaaa` | AMP Level                           | 0 -- 127   | 0 -- 127                                            |
| `00 16` | `0aaa aaaa` | AMP Level Velocity Sens             | 1 -- 127   | -63 -- +63                                           |
| `00 17` | `0aaa aaaa` | AMP Env Attack Time                 | 0 -- 127   | 0 -- 127                                            |
| `00 18` | `0aaa aaaa` | AMP Env Decay Time                  | 0 -- 127   | 0 -- 127                                            |
| `00 19` | `0aaa aaaa` | AMP Env Sustain Level               | 0 -- 127   | 0 -- 127                                            |
| `00 1A` | `0aaa aaaa` | AMP Env Release Time                | 0 -- 127   | 0 -- 127                                            |
| `00 1B` | `0aaa aaaa` | AMP Pan                             | 0 -- 127   | L64 -- 63R                                           |
| `00 1C` | `0000 0aaa` | LFO Shape                           | 0 -- 5     | TRI, SIN, SAW, SQR, S&H, RND                        |
| `00 1D` | `0aaa aaaa` | LFO Rate                            | 0 -- 127   | 0 -- 127                                            |
| `00 1E` | `0000 000a` | LFO Tempo Sync Switch               | 0 -- 1     | OFF, ON                                              |
| `00 1F` | `000a aaaa` | LFO Tempo Sync Note                 | 0 -- 19    | 16, 12, 8, 4, 2, 1, 3/4, 2/3, 1/2, 3/8, 1/3, 1/4, 3/16, 1/6, 1/8, 3/32, 1/12, 1/16, 1/24, 1/32 |
| `00 20` | `0aaa aaaa` | LFO Fade Time                       | 0 -- 127   | 0 -- 127                                            |
| `00 21` | `0000 000a` | LFO Key Trigger                     | 0 -- 1     | OFF, ON                                              |
| `00 22` | `0aaa aaaa` | LFO Pitch Depth                     | 1 -- 127   | -63 -- +63                                           |
| `00 23` | `0aaa aaaa` | LFO Filter Depth                    | 1 -- 127   | -63 -- +63                                           |
| `00 24` | `0aaa aaaa` | LFO Amp Depth                       | 1 -- 127   | -63 -- +63                                           |
| `00 25` | `0aaa aaaa` | LFO Pan Depth                       | 1 -- 127   | -63 -- +63                                           |
| `00 26` | `0000 0aaa` | Modulation LFO Shape                | 0 -- 5     | TRI, SIN, SAW, SQR, S&H, RND                        |
| `00 27` | `0aaa aaaa` | Modulation LFO Rate                 | 0 -- 127   | 0 -- 127                                            |
| `00 28` | `0000 000a` | Modulation LFO Tempo Sync Switch    | 0 -- 1     | OFF, ON                                              |
| `00 29` | `000a aaaa` | Modulation LFO Tempo Sync Note      | 0 -- 19    | 16, 12, 8, 4, 2, 1, 3/4, 2/3, 1/2, 3/8, 1/3, 1/4, 3/16, 1/6, 1/8, 3/32, 1/12, 1/16, 1/24, 1/32 |
| `00 2A` | `0aaa aaaa` | OSC Pulse Width Shift               | 0 -- 127   | 0 -- 127                                            |
| `00 2B` | `0000 000a` | (reserve) \<\*>                     | --         | --                                                   |
| `00 2C` | `0aaa aaaa` | Modulation LFO Pitch Depth          | 1 -- 127   | -63 -- +63                                           |
| `00 2D` | `0aaa aaaa` | Modulation LFO Filter Depth         | 1 -- 127   | -63 -- +63                                           |
| `00 2E` | `0aaa aaaa` | Modulation LFO Amp Depth            | 1 -- 127   | -63 -- +63                                           |
| `00 2F` | `0aaa aaaa` | Modulation LFO Pan Depth            | 1 -- 127   | -63 -- +63                                           |
| `00 30` | `0aaa aaaa` | Cutoff Aftertouch Sens              | 1 -- 127   | -63 -- +63                                           |
| `00 31` | `0aaa aaaa` | Level Aftertouch Sens               | 1 -- 127   | -63 -- +63                                           |
| `00 32` | `0aaa aaaa` | (reserve) \<\*>                     | --         | --                                                   |
| `00 33` | `0aaa aaaa` | (reserve) \<\*>                     | --         | --                                                   |
| `00 34` | `0000 00aa` | Wave Gain                           | 0 -- 3     | -6, 0, +6, +12 dB                                   |
| `00 35` | `0000 aaaa` | Wave Number #                       | 0 -- 16384 | OFF, 1 -- 16384                                      |
|         | `0000 bbbb` |                                     |            |                                                      |
|         | `0000 cccc` |                                     |            |                                                      |
|         | `0000 dddd` |                                     |            |                                                      |
| `00 39` | `0aaa aaaa` | HPF Cutoff                          | 0 -- 127   | 0 -- 127                                            |
| `00 3A` | `0aaa aaaa` | Super Saw Detune                    | 0 -- 127   | 0 -- 127                                            |
| `00 3B` | `0aaa aaaa` | Modulation LFO Rate Control         | 1 -- 127   | -63 -- +63                                           |
| `00 3C` | `000a aaaa` | AMP Level Keyfollow                 | 54 -- 74   | -100 -- +100                                         |
