# SuperNATURAL Drum Kit Parameters

Base offset within Temporary Tone: `03 00 00`.

The SuperNATURAL Drum Kit is organized into four sub-blocks: Common,
Common MFX, Common Comp/EQ, and per-key Notes.

---

## SN Drum Kit Common (offset `00 00 00`)

Total size: `00 00 00 14`.

| Offset  | Bits        | Parameter          | Range      | Display            |
|---------|-------------|--------------------|------------|--------------------|
| `00 00` | `0aaa aaaa` | Kit Name 1         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 01` | `0aaa aaaa` | Kit Name 2         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 02` | `0aaa aaaa` | Kit Name 3         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 03` | `0aaa aaaa` | Kit Name 4         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 04` | `0aaa aaaa` | Kit Name 5         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 05` | `0aaa aaaa` | Kit Name 6         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 06` | `0aaa aaaa` | Kit Name 7         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 07` | `0aaa aaaa` | Kit Name 8         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 08` | `0aaa aaaa` | Kit Name 9         | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 09` | `0aaa aaaa` | Kit Name 10        | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0A` | `0aaa aaaa` | Kit Name 11        | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0B` | `0aaa aaaa` | Kit Name 12        | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0C` | `0aaa aaaa` | (reserve)          | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0D` | `0aaa aaaa` | (reserve)          | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0E` | `0aaa aaaa` | (reserve)          | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 0F` | `0aaa aaaa` | (reserve)          | 32 -- 127  | 32 -- 127 (ASCII)  |
| `00 10` | `0aaa aaaa` | Kit Level          | 0 -- 127   | 0 -- 127           |
| `00 11` | `0aaa aaaa` | Ambience Level     | 0 -- 127   | 0 -- 127           |
| `00 12` | `0aaa aaaa` | Phrase Number      | 0 -- 127   | 0 -- 127           |
| `00 13` | `0000 000a` | TFX Switch         | 0 -- 1     | OFF, ON            |

---

## SN Drum Kit MFX (offset `00 02 00`)

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

## SN Drum Kit Common Comp/EQ (offset `00 08 00`)

Total size: `00 00 00 54`.

Six output groups (1--6), each with a compressor and 3-band EQ. Structure
is identical to the PCM Drum Kit Common Comp/EQ block.

| Offset  | Bits        | Parameter              | Range     | Display                                                    |
|---------|-------------|------------------------|-----------|------------------------------------------------------------|
| `00 00` | `0000 000a` | Comp1 Switch           | 0 -- 1    | OFF, ON                                                    |
| `00 01` | `000a aaaa` | Comp1 Attack Time      | 0 -- 31   | 0.05, 0.06 ... 1.0, 2.0 ... 50.0 msec                     |
| `00 02` | `000a aaaa` | Comp1 Release Time     | 0 -- 23   | 0.05, 0.07, 0.1, 0.5, 1, 5 ... 2000 msec                  |
| `00 03` | `0aaa aaaa` | Comp1 Threshold        | 0 -- 127  | 0 -- 127                                                   |
| `00 04` | `000a aaaa` | Comp1 Ratio            | 0 -- 19   | 1:1, 2:1 ... 100:1, inf:1                                  |
| `00 05` | `000a aaaa` | Comp1 Output Gain      | 0 -- 24   | 0 -- +24 dB                                                |
| `00 06` | `0000 000a` | EQ1 Switch             | 0 -- 1    | OFF, ON                                                    |
| `00 07` | `0000 000a` | EQ1 Low Freq           | 0 -- 1    | 200, 400 Hz                                                |
| `00 08` | `000a aaaa` | EQ1 Low Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 09` | `000a aaaa` | EQ1 Mid Freq           | 0 -- 16   | 200, 250, 315 ... 8000 Hz                                  |
| `00 0A` | `000a aaaa` | EQ1 Mid Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 0B` | `0000 0aaa` | EQ1 Mid Q              | 0 -- 4    | 0.5, 1.0, 2.0, 4.0, 8.0                                   |
| `00 0C` | `0000 00aa` | EQ1 High Freq          | 0 -- 2    | 2000, 4000, 8000 Hz                                        |
| `00 0D` | `000a aaaa` | EQ1 High Gain          | 0 -- 30   | -15 -- +15 dB                                              |
| `00 0E` | `0000 000a` | Comp2 Switch           | 0 -- 1    | OFF, ON                                                    |
| `00 0F` | `000a aaaa` | Comp2 Attack Time      | 0 -- 31   | 0.05 ... 50.0 msec                                         |
| `00 10` | `000a aaaa` | Comp2 Release Time     | 0 -- 23   | 0.05 ... 2000 msec                                         |
| `00 11` | `0aaa aaaa` | Comp2 Threshold        | 0 -- 127  | 0 -- 127                                                   |
| `00 12` | `000a aaaa` | Comp2 Ratio            | 0 -- 19   | 1:1 ... inf:1                                              |
| `00 13` | `000a aaaa` | Comp2 Output Gain      | 0 -- 24   | 0 -- +24 dB                                                |
| `00 14` | `0000 000a` | EQ2 Switch             | 0 -- 1    | OFF, ON                                                    |
| `00 15` | `0000 000a` | EQ2 Low Freq           | 0 -- 1    | 200, 400 Hz                                                |
| `00 16` | `000a aaaa` | EQ2 Low Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 17` | `000a aaaa` | EQ2 Mid Freq           | 0 -- 16   | 200 ... 8000 Hz                                            |
| `00 18` | `000a aaaa` | EQ2 Mid Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 19` | `0000 0aaa` | EQ2 Mid Q              | 0 -- 4    | 0.5, 1.0, 2.0, 4.0, 8.0                                   |
| `00 1A` | `0000 00aa` | EQ2 High Freq          | 0 -- 2    | 2000, 4000, 8000 Hz                                        |
| `00 1B` | `000a aaaa` | EQ2 High Gain          | 0 -- 30   | -15 -- +15 dB                                              |
| `00 1C` | `0000 000a` | Comp3 Switch           | 0 -- 1    | OFF, ON                                                    |
| `00 1D` | `000a aaaa` | Comp3 Attack Time      | 0 -- 31   | 0.05 ... 50.0 msec                                         |
| `00 1E` | `000a aaaa` | Comp3 Release Time     | 0 -- 23   | 0.05 ... 2000 msec                                         |
| `00 1F` | `0aaa aaaa` | Comp3 Threshold        | 0 -- 127  | 0 -- 127                                                   |
| `00 20` | `000a aaaa` | Comp3 Ratio            | 0 -- 19   | 1:1 ... inf:1                                              |
| `00 21` | `000a aaaa` | Comp3 Output Gain      | 0 -- 24   | 0 -- +24 dB                                                |
| `00 22` | `0000 000a` | EQ3 Switch             | 0 -- 1    | OFF, ON                                                    |
| `00 23` | `0000 000a` | EQ3 Low Freq           | 0 -- 1    | 200, 400 Hz                                                |
| `00 24` | `000a aaaa` | EQ3 Low Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 25` | `000a aaaa` | EQ3 Mid Freq           | 0 -- 16   | 200 ... 8000 Hz                                            |
| `00 26` | `000a aaaa` | EQ3 Mid Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 27` | `0000 0aaa` | EQ3 Mid Q              | 0 -- 4    | 0.5, 1.0, 2.0, 4.0, 8.0                                   |
| `00 28` | `0000 00aa` | EQ3 High Freq          | 0 -- 2    | 2000, 4000, 8000 Hz                                        |
| `00 29` | `000a aaaa` | EQ3 High Gain          | 0 -- 30   | -15 -- +15 dB                                              |
| `00 2A` | `0000 000a` | Comp4 Switch           | 0 -- 1    | OFF, ON                                                    |
| `00 2B` | `000a aaaa` | Comp4 Attack Time      | 0 -- 31   | 0.05 ... 50.0 msec                                         |
| `00 2C` | `000a aaaa` | Comp4 Release Time     | 0 -- 23   | 0.05 ... 2000 msec                                         |
| `00 2D` | `0aaa aaaa` | Comp4 Threshold        | 0 -- 127  | 0 -- 127                                                   |
| `00 2E` | `000a aaaa` | Comp4 Ratio            | 0 -- 19   | 1:1 ... inf:1                                              |
| `00 2F` | `000a aaaa` | Comp4 Output Gain      | 0 -- 24   | 0 -- +24 dB                                                |
| `00 30` | `0000 000a` | EQ4 Switch             | 0 -- 1    | OFF, ON                                                    |
| `00 31` | `0000 000a` | EQ4 Low Freq           | 0 -- 1    | 200, 400 Hz                                                |
| `00 32` | `000a aaaa` | EQ4 Low Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 33` | `000a aaaa` | EQ4 Mid Freq           | 0 -- 16   | 200 ... 8000 Hz                                            |
| `00 34` | `000a aaaa` | EQ4 Mid Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 35` | `0000 0aaa` | EQ4 Mid Q              | 0 -- 4    | 0.5, 1.0, 2.0, 4.0, 8.0                                   |
| `00 36` | `0000 00aa` | EQ4 High Freq          | 0 -- 2    | 2000, 4000, 8000 Hz                                        |
| `00 37` | `000a aaaa` | EQ4 High Gain          | 0 -- 30   | -15 -- +15 dB                                              |
| `00 38` | `0000 000a` | Comp5 Switch           | 0 -- 1    | OFF, ON                                                    |
| `00 39` | `000a aaaa` | Comp5 Attack Time      | 0 -- 31   | 0.05 ... 50.0 msec                                         |
| `00 3A` | `000a aaaa` | Comp5 Release Time     | 0 -- 23   | 0.05 ... 2000 msec                                         |
| `00 3B` | `0aaa aaaa` | Comp5 Threshold        | 0 -- 127  | 0 -- 127                                                   |
| `00 3C` | `000a aaaa` | Comp5 Ratio            | 0 -- 19   | 1:1 ... inf:1                                              |
| `00 3D` | `000a aaaa` | Comp5 Output Gain      | 0 -- 24   | 0 -- +24 dB                                                |
| `00 3E` | `0000 000a` | EQ5 Switch             | 0 -- 1    | OFF, ON                                                    |
| `00 3F` | `0000 000a` | EQ5 Low Freq           | 0 -- 1    | 200, 400 Hz                                                |
| `00 40` | `000a aaaa` | EQ5 Low Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 41` | `000a aaaa` | EQ5 Mid Freq           | 0 -- 16   | 200 ... 8000 Hz                                            |
| `00 42` | `000a aaaa` | EQ5 Mid Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 43` | `0000 0aaa` | EQ5 Mid Q              | 0 -- 4    | 0.5, 1.0, 2.0, 4.0, 8.0                                   |
| `00 44` | `0000 00aa` | EQ5 High Freq          | 0 -- 2    | 2000, 4000, 8000 Hz                                        |
| `00 45` | `000a aaaa` | EQ5 High Gain          | 0 -- 30   | -15 -- +15 dB                                              |
| `00 46` | `0000 000a` | Comp6 Switch           | 0 -- 1    | OFF, ON                                                    |
| `00 47` | `000a aaaa` | Comp6 Attack Time      | 0 -- 31   | 0.05 ... 50.0 msec                                         |
| `00 48` | `000a aaaa` | Comp6 Release Time     | 0 -- 23   | 0.05 ... 2000 msec                                         |
| `00 49` | `0aaa aaaa` | Comp6 Threshold        | 0 -- 127  | 0 -- 127                                                   |
| `00 4A` | `000a aaaa` | Comp6 Ratio            | 0 -- 19   | 1:1 ... inf:1                                              |
| `00 4B` | `000a aaaa` | Comp6 Output Gain      | 0 -- 24   | 0 -- +24 dB                                                |
| `00 4C` | `0000 000a` | EQ6 Switch             | 0 -- 1    | OFF, ON                                                    |
| `00 4D` | `0000 000a` | EQ6 Low Freq           | 0 -- 1    | 200, 400 Hz                                                |
| `00 4E` | `000a aaaa` | EQ6 Low Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 4F` | `000a aaaa` | EQ6 Mid Freq           | 0 -- 16   | 200 ... 8000 Hz                                            |
| `00 50` | `000a aaaa` | EQ6 Mid Gain           | 0 -- 30   | -15 -- +15 dB                                              |
| `00 51` | `0000 0aaa` | EQ6 Mid Q              | 0 -- 4    | 0.5, 1.0, 2.0, 4.0, 8.0                                   |
| `00 52` | `0000 00aa` | EQ6 High Freq          | 0 -- 2    | 2000, 4000, 8000 Hz                                        |
| `00 53` | `000a aaaa` | EQ6 High Gain          | 0 -- 30   | -15 -- +15 dB                                              |

---

## SN Drum Kit Note (offsets `00 10 00` through `00 4D 00`)

Total size per note: `00 00 00 13`.

There are 62 note slots addressed individually. Key #27 (D#1) starts at
offset `00 10 00` within the SN Drum Kit block. Each subsequent key is
offset by `00 01 00` from the previous, so Key #28 is at `00 11 00`,
Key #29 at `00 12 00`, and so on up to Key #88 (E6) at `00 4D 00`.

| Offset  | Bits        | Parameter          | Range      | Display                                                    |
|---------|-------------|--------------------|------------|------------------------------------------------------------|
| `00 00` | `0000 aaaa` | Inst Number #      | 0 -- 512   | 0 -- 512                                                   |
|         | `0000 bbbb` |                    |            |                                                            |
|         | `0000 cccc` |                    |            |                                                            |
|         | `0000 dddd` |                    |            |                                                            |
| `00 04` | `0aaa aaaa` | Level              | 0 -- 127   | 0 -- 127                                                   |
| `00 05` | `0aaa aaaa` | Pan                | 0 -- 127   | L64 -- 63R                                                 |
| `00 06` | `0aaa aaaa` | Chorus Send Level  | 0 -- 127   | 0 -- 127                                                   |
| `00 07` | `0aaa aaaa` | Reverb Send Level  | 0 -- 127   | 0 -- 127                                                   |
| `00 08` | `0000 aaaa` | Tune #             | 8 -- 248   | -1200 -- +1200                                             |
|         | `0000 bbbb` |                    |            |                                                            |
|         | `0000 cccc` |                    |            |                                                            |
|         | `0000 dddd` |                    |            |                                                            |
| `00 0C` | `0aaa aaaa` | Attack             | 0 -- 100   | 0 -- 100%                                                  |
| `00 0D` | `0aaa aaaa` | Decay              | 1 -- 64    | -63 -- 0                                                   |
| `00 0E` | `000a aaaa` | Brilliance         | 49 -- 76   | -15 -- +12                                                 |
| `00 0F` | `0aaa aaaa` | Variation          | 0 -- 7     | OFF, FLAM1, FLAM2, FLAM3, BUZZ1, BUZZ2, BUZZ3, ROLL       |
| `00 10` | `00aa aaaa` | Dynamic Range      | 0 -- 63    | 0 -- 63                                                    |
| `00 11` | `0aaa aaaa` | Stereo Width       | 0 -- 127   | 0 -- 127                                                   |
| `00 12` | `0000 0aaa` | Output Assign      | 0 -- 6     | PART, COMP+EQ1 ... COMP+EQ6                                |
