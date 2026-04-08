# INTEGRA-7 MIDI Implementation Reference

Extracted from the official Roland INTEGRA-7 MIDI Implementation document
(v1.00, September 2012). Model ID: `00H 00H 64H`.

## Contents

| File | Description |
|------|-------------|
| [01-protocol.md](01-protocol.md) | SysEx protocol (DT1/RQ1), checksum algorithm, transmission rules |
| [02-channel-messages.md](02-channel-messages.md) | Channel voice/mode messages, all CC mappings, RPN table |
| [03-bank-select-tables.md](03-bank-select-tables.md) | Bank Select MSB/LSB/PC for all tone and kit types |
| [04-address-map.md](04-address-map.md) | Top-level address map, Setup, System Common parameters |
| [05-studio-set.md](05-studio-set.md) | Studio Set: Common, Chorus, Reverb, Motional Surround, EQ, MIDI, Parts |
| [06-pcm-synth-tone.md](06-pcm-synth-tone.md) | PCM Synth Tone: Common, MFX, PMT, Partials 1-4, Common 2 |
| [07-pcm-drum-kit.md](07-pcm-drum-kit.md) | PCM Drum Kit: Common, MFX, Comp/EQ, per-key Partials, Common 2 |
| [08-supernatural-synth-tone.md](08-supernatural-synth-tone.md) | SuperNATURAL Synth Tone: Common, MFX, Partials 1-3 |
| [09-supernatural-acoustic-tone.md](09-supernatural-acoustic-tone.md) | SuperNATURAL Acoustic Tone: Common, MFX |
| [10-supernatural-drum-kit.md](10-supernatural-drum-kit.md) | SuperNATURAL Drum Kit: Common, MFX, Comp/EQ, per-key Notes |
| [99-undocumented.md](99-undocumented.md) | Undocumented: catalog queries, user storage addresses |

## Address Space Quick Reference

| Start Address | Block |
|---------------|-------|
| `01 00 00 00` | Setup |
| `02 00 00 00` | System |
| `0F 00 03 02` | Catalog query (undocumented) |
| `10 00 00 00` | User Studio Set storage (undocumented) |
| `18 00 00 00` | Temporary Studio Set |
| `19 00 00 00` | Temporary Tone (Part 1) |
| `19 20 00 00` | Temporary Tone (Part 2) |
| ... | ... |
| `1C 60 00 00` | Temporary Tone (Part 16) |

## Key Protocol Facts

- **Manufacturer ID:** `41H` (Roland)
- **Model ID:** `00H 00H 64H` (INTEGRA-7)
- **DT1 Command:** `12H` — Data Set 1
- **RQ1 Command:** `11H` — Data Request 1
- **Checksum:** `(128 - (sum_of_address_and_data % 128)) % 128`
- **Max packet:** 256 bytes
- **Min interval:** 20 ms between packets
- **Nibblized data:** Values > 7 bits are split into multiple 7-bit bytes (high nibble first)
