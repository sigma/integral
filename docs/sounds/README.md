# INTEGRA-7 Factory Sound List

Structured JSON files containing all factory preset tones for the Roland
INTEGRA-7, extracted from the official Sound List PDF (v1.00).

## Files

| File | Description | Count |
|------|-------------|-------|
| `sn-acoustic-preset.json` | SuperNATURAL Acoustic Tone presets (MSB 89, LSB 64-65) | 256 |
| `sn-synth-preset.json` | SuperNATURAL Synth Tone presets (MSB 95, LSB 64-72) | 1109 |
| `sn-drum-preset.json` | SuperNATURAL Drum Kit presets (MSB 88, LSB 64) | 26 |
| `pcm-synth-preset.json` | PCM Synth Tone presets (MSB 87, LSB 64-70) | 896 |
| `pcm-drum-preset.json` | PCM Drum Kit presets (MSB 86, LSB 64) | 14 |
| `gm2.json` | GM2 Tones (MSB 121) and GM2 Drum Kits (MSB 120) | 263 |
| `exsn.json` | ExSN expansion tones (ExSN1-ExSN6) | 115 |
| `expcm.json` | ExPCM expansion tones (SRX-01 through SRX-12, HQ GM2/PCM) | 3314 |

## Entry Format

Each JSON file contains an array of objects with these fields:

```json
{
  "number": 1,
  "name": "Full Grand 1",
  "msb": 89,
  "lsb": 64,
  "pc": 1,
  "category": "Ac.Piano"
}
```

- **number** -- sequential tone number within its bank section (from the PDF)
- **name** -- tone name (max 12 characters on the device)
- **msb** -- Bank Select MSB value
- **lsb** -- Bank Select LSB value
- **pc** -- Program Change number (1-128)
- **category** -- tone category (see below)

Expansion files (`exsn.json`, `expcm.json`) include an additional **bank**
field identifying the expansion pack (e.g. "ExSN1: Ethnic",
"SRX-04: Symphonique Strings").

## Tone Categories

Categories match the 36 values defined in `TONE_CATEGORY_NAMES`
(`crates/integral-core/src/svd.rs`):

Ac.Piano, E.Piano, Organ, Other Keyboards, Accordion/Harmonica,
Bell/Mallet, Ac.Guitar, E.Guitar, Dist.Guitar, Ac.Bass, E.Bass,
Synth Bass, Plucked/Stroke, Strings, Brass, Wind, Flute, Sax,
Recorder, Vox/Choir, Synth Lead, Synth Brass, Synth Pad/Strings,
Synth Bellpad, Synth PolyKey, FX, Synth Seq/Pop, Phrase, Pulsating,
Beat&Groove, Hit, Sound FX, Drums, Percussion, Combination.

Drum kit entries use the category "Drum".

## Bank Select Reference

See `crates/integral-core/src/tone_banks.rs` and `docs/midi/03-bank-select-tables.md`
for the full MSB/LSB mapping.

## Source

Roland INTEGRA-7 Sound List (PDF), version 1.00.
Copyright 2012 Roland Corporation.
