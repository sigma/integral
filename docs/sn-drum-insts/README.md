# INTEGRA-7 SN Drum Kit Instrument List

`insts.json` names every **Inst** an SN Drum Kit key can play (its Inst
Number, `00 00` of an SN Drum Kit Note), as listed by Roland's INTEGRA-7
Editor. Inst Number 0 (OFF) is not stored.

| Bank | Inst Numbers | Count |
|------|--------------|-------|
| `INT` (internal) | 1–168 | 168 |
| `ExSN6` (SuperNATURAL expansion 6) | 169–294 | 126 |

## Entry Format

```json
{ "number": 1, "name": "Studio Kick", "type": "Kick", "bank": "INT",
  "width": true, "amb": true, "vari": "Flam/Buzz" }
```

- **number** — Inst Number, running 1…294 with no gaps.
- **name** — Inst name, spelled exactly as in Roland's Editor.
- **type** — Roland's category: Kick, Snare, Tom, Hi-Hat, Ride, Crash,
  Percussion, Other or SFX.
- **bank** — `INT`, or `ExSN6` when the Inst comes from that expansion.
- **vari** — the Variation families the Inst supports, where present, slash-
  separated: `Flam` (FLAM1-3), `Buzz` (BUZZ1-3), `Roll` (ROLL), e.g.
  `Flam/Buzz/Roll`. The build splits it into families (failing on an unknown
  one), which limit the editor's Variation list.
- **width**, **amb** — kept as in the source, where present: whether the Inst
  has Stereo Width and Ambience. The editor doesn't use them yet.

## Source

Roland INTEGRA-7 Editor Ver.2, `Contents/Resources/html/js/view/preset/snd_inst.js`
(`globals.parameter.sndInstPreset`), "Copyright 2014 Roland Corporation",
extracted like the Wave lists (see `../waves/README.md`) from the public mirror
[SambaGodschynski/integra7-editor-web @ d5b9193](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset/snd_inst.js)
by evaluating the file in a Node `vm` context. The OFF entry (number 0) is dropped.
