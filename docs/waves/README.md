# INTEGRA-7 Wave Name Lists

Structured JSON files naming every **Wave** of the Roland INTEGRA-7, one file
per Wave Group, extracted from Roland's INTEGRA-7 Editor.

## Files

| File | Wave Group | Count |
|------|------------|-------|
| `int.json` | INT (internal, XV-5080 wave set) | 1083 |
| `srx-01.json` | SRX-01 | 719 |
| `srx-02.json` | SRX-02 | 48 |
| `srx-03.json` | SRX-03 | 522 |
| `srx-04.json` | SRX-04 | 196 |
| `srx-05.json` | SRX-05 | 818 |
| `srx-06.json` | SRX-06 | 512 |
| `srx-07.json` | SRX-07 | 860 |
| `srx-08.json` | SRX-08 | 950 |
| `srx-09.json` | SRX-09 | 873 |
| `srx-10.json` | SRX-10 | 261 |
| `srx-11.json` | SRX-11 | 40 |
| `srx-12.json` | SRX-12 | 168 |
| `sn-synth.json` | SN Synth (picked by an SN Synth OSC in PCM mode) | 450 |

## Entry Format

Each JSON file contains an array of objects with these fields:

```json
{
  "number": 1,
  "name": "StGrand pA L"
}
```

- **number** -- Wave Number, running 1…N with no gaps. Wave Number 0 (OFF)
  is not stored.
- **name** -- Wave name (max 12 characters), right-trimmed and spelled
  exactly as in Roland's Editor. Leading spaces are Roland's and kept (e.g.
  SRX-09 Wave Number 7, `" 78:GamelnP2"`).

A Stereo pair is not stored: by convention the L Wave is at Wave Number
`n` and the R Wave at `n+1`, marked only by an `L`/`R` in the name.

## Source

Roland INTEGRA-7 Editor Ver.2 wave list arrays
(`Contents/Resources/html/js/view/preset/`), "Copyright 2014 Roland
Corporation":

- `int.json` from `wave_list_xv5080.js`
- `srx-01.json` … `srx-12.json` from `wave_list_srx01.js` … `wave_list_srx12.js`
- `sn-synth.json` from `wave_list_synth.js`. Source names carry a
  `"nnn "` prefix: it is stripped, and `number` is that `nnn`. The
  commented-out `// ROM 9` block is not part of the list.

Extracted from the public mirror
[SambaGodschynski/integra7-editor-web @ d5b9193](https://github.com/SambaGodschynski/integra7-editor-web/tree/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset)
by evaluating the files in a Node `vm` context. The mirror's arrays are
identical to the installed Editor 2.0.5. See
[wave-names research](../research/wave-names.md) for the full analysis.
