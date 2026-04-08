# Parameter Address Map

The INTEGRA-7 (ModelID = 00H 00H 64H) organizes its parameters into a
hierarchical address space. All addresses are 4 bytes wide and are transmitted
via SysEx Data Set 1 (DT1) and Data Request 1 (RQ1) messages.

**Document conventions:**

- **"#" marked addresses** use nibblized data encoding. Each byte of the
  original value is split into two nibbles sent as separate bytes. For example,
  ABH becomes 0AH 0BH, transmitted in that order.
- **"\<\*>" marked parameters** are ignored on receive; the INTEGRA-7 discards
  these values in incoming DT1 messages.
- **Bit patterns** such as `0000 aaaa` indicate which bits carry data within the
  transmitted byte. Unused bits must be zero.

## Top-Level Address Map

| Start Address   | Description                |
|-----------------|----------------------------|
| `01 00 00 00`   | Setup                      |
| `02 00 00 00`   | System                     |
| `18 00 00 00`   | Temporary Studio Set       |
| `19 00 00 00`   | Temporary Tone (Part 1)    |
| `19 20 00 00`   | Temporary Tone (Part 2)    |
| `19 40 00 00`   | Temporary Tone (Part 3)    |
| `19 60 00 00`   | Temporary Tone (Part 4)    |
| `1A 00 00 00`   | Temporary Tone (Part 5)    |
| `1A 20 00 00`   | Temporary Tone (Part 6)    |
| `1A 40 00 00`   | Temporary Tone (Part 7)    |
| `1A 60 00 00`   | Temporary Tone (Part 8)    |
| `1B 00 00 00`   | Temporary Tone (Part 9)    |
| `1B 20 00 00`   | Temporary Tone (Part 10)   |
| `1B 40 00 00`   | Temporary Tone (Part 11)   |
| `1B 60 00 00`   | Temporary Tone (Part 12)   |
| `1C 00 00 00`   | Temporary Tone (Part 13)   |
| `1C 20 00 00`   | Temporary Tone (Part 14)   |
| `1C 40 00 00`   | Temporary Tone (Part 15)   |
| `1C 60 00 00`   | Temporary Tone (Part 16)   |

Each Temporary Tone block spans `00 20 00 00` in address space (the distance
between consecutive parts). The System block contains a single sub-block
(System Common) at offset `00 00 00`.

## Temporary Tone Sub-Structure

Within each part's Temporary Tone block, the following offsets select the tone
type. Only one tone type is active at a time, determined by the part's Tone Bank
settings in the Studio Set.

| Offset      | Description                      |
|-------------|----------------------------------|
| `00 00 00`  | PCM Synth Tone                   |
| `01 00 00`  | SuperNATURAL Synth Tone          |
| `02 00 00`  | SuperNATURAL Acoustic Tone       |
| `03 00 00`  | SuperNATURAL Drum Kit            |
| `10 00 00`  | PCM Drum Kit                     |

For example, to address the SuperNATURAL Synth Tone for Part 3, combine the
part's start address (`19 40 00 00`) with the tone type offset (`01 00 00`) to
get `19 41 00 00`.

## Setup Parameters

Base address: `01 00 00 00`. Total size: `00 00 00 38`.

| Offset  | Bits          | Description                | Range     | Display Values          |
|---------|---------------|----------------------------|-----------|-------------------------|
| `00 00` | `0000 0aaa`   | Sound Mode                 | 1 -- 4    | STUDIO, GM1, GM2, GS    |
| `00 01` | `0aaa aaaa`   | (reserve) \<\*>            | --        | --                      |
| `00 02` | `0aaa aaaa`   | (reserve) \<\*>            | --        | --                      |
| `00 03` | `0aaa aaaa`   | (reserve) \<\*>            | --        | --                      |
| `00 04` | `0aaa aaaa`   | Studio Set BS MSB (CC# 0)  | 0 -- 127  | 0 -- 127                |
| `00 05` | `0aaa aaaa`   | Studio Set BS LSB (CC# 32) | 0 -- 127  | 0 -- 127                |
| `00 06` | `0aaa aaaa`   | Studio Set PC (PC)         | 0 -- 127  | 0 -- 127                |
| `00 07` | `0aaa aaaa`   | (reserve) \<\*>            | --        | --                      |
|  ...    |  ...          |  ...                       |  ...      |  ...                    |
| `00 2F` | `0aaa aaaa`   | (reserve) \<\*>            | --        | --                      |
| `00 30` | `0000 000a`   | (reserve) \<\*>            | --        | --                      |
|  ...    |  ...          |  ...                       |  ...      |  ...                    |
| `00 36` | `0000 000a`   | (reserve) \<\*>            | --        | --                      |
| `00 37` | `0000 aaaa`   | (reserve)                  | 0 -- 1    | --                      |

The reserve block from `00 07` to `00 2F` occupies 41 consecutive bytes, each
with bit pattern `0aaa aaaa`. The reserve block from `00 30` to `00 36` occupies
7 bytes, each with bit pattern `0000 000a`. All reserve fields marked \<\*> are
ignored on receive.

## System Common Parameters

Base address: `02 00 00 00`. Total size: `00 00 00 2F`.

| Offset  | Bits          | Description                     | Range       | Display Values                                |
|---------|---------------|---------------------------------|-------------|-----------------------------------------------|
| `00 00` | `0000 aaaa`   | Master Tune (byte 1 of 4) #     | 24 -- 2024  | -100.0 -- 100.0 [cent]                        |
| `00 01` | `0000 bbbb`   | Master Tune (byte 2 of 4) #     |             |                                               |
| `00 02` | `0000 cccc`   | Master Tune (byte 3 of 4) #     |             |                                               |
| `00 03` | `0000 dddd`   | Master Tune (byte 4 of 4) #     |             |                                               |
| `00 04` | `00aa aaaa`   | Master Key Shift                | 40 -- 88    | -24 -- +24                                    |
| `00 05` | `0aaa aaaa`   | Master Level                    | 0 -- 127    | 0 -- 127                                      |
| `00 06` | `0000 000a`   | Scale Tune Switch               | 0 -- 1      | OFF, ON                                       |
| `00 07` | `0000 000a`   | (reserve) \<\*>                 | --          | --                                            |
| `00 08` | `0000 000a`   | (reserve) \<\*>                 | --          | --                                            |
| `00 09` | `000a aaaa`   | (reserve) \<\*>                 | --          | --                                            |
|  ...    |  ...          |  ...                            |  ...        |  ...                                          |
| `00 10` | `000a aaaa`   | (reserve) \<\*>                 | --          | --                                            |
| `00 11` | `000a aaaa`   | Studio Set Control Channel      | 0 -- 16     | 1 -- 16, OFF                                  |
| `00 12` | `0aaa aaaa`   | (reserve) \<\*>                 | --          | --                                            |
|  ...    |  ...          |  ...                            |  ...        |  ...                                          |
| `00 1F` | `0aaa aaaa`   | (reserve) \<\*>                 | --          | --                                            |
| `00 20` | `0aaa aaaa`   | System Control 1 Source         | 0 -- 97     | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT    |
| `00 21` | `0aaa aaaa`   | System Control 2 Source         | 0 -- 97     | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT    |
| `00 22` | `0aaa aaaa`   | System Control 3 Source         | 0 -- 97     | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT    |
| `00 23` | `0aaa aaaa`   | System Control 4 Source         | 0 -- 97     | OFF, CC01 -- CC31, CC33 -- CC95, BEND, AFT    |
| `00 24` | `0000 000a`   | Control Source                  | 0 -- 1      | SYSTEM, STUDIO SET                            |
| `00 25` | `0000 000a`   | System Clock Source             | 0 -- 1      | MIDI, USB                                     |
| `00 26` | `0000 aaaa`   | System Tempo (byte 1 of 2) #   | 20 -- 250   | 20 -- 250                                     |
| `00 27` | `0000 bbbb`   | System Tempo (byte 2 of 2) #   |             |                                               |
| `00 28` | `0000 000a`   | Tempo Assign Source             | 0 -- 1      | SYSTEM, STUDIO SET                            |
| `00 29` | `0000 000a`   | Receive Program Change          | 0 -- 1      | OFF, ON                                       |
| `00 2A` | `0000 000a`   | Receive Bank Select             | 0 -- 1      | OFF, ON                                       |
| `00 2B` | `0000 000a`   | 5.1CH Center Speaker Switch     | 0 -- 1      | OFF, ON                                       |
| `00 2C` | `0000 000a`   | 5.1CH Sub Woofer Switch         | 0 -- 1      | OFF, ON                                       |
| `00 2D` | `0000 000a`   | 2CH Output Mode                 | 0 -- 1      | SPEAKER, PHONES                               |
| `00 2E` | `0000 00aa`   | (reserve) \<\*>                 | --          | --                                            |

**Notes on nibblized parameters:**

- **Master Tune** occupies 4 bytes (`00 00` -- `00 03`). The raw 16-bit value
  (range 24 -- 2024, center 1024) is split into 4 nibbles. For example, a value
  of 1024 (0400H) is transmitted as `00 04 00 00`. The displayed value maps
  linearly: 24 = -100.0 cent, 1024 = 0.0 cent, 2024 = +100.0 cent.

- **System Tempo** occupies 2 bytes (`00 26` -- `00 27`). The tempo value
  (range 20 -- 250) is split into 2 nibbles. For example, a tempo of 120 (78H)
  is transmitted as `07 08`.
