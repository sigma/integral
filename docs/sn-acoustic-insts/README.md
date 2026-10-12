# INTEGRA-7 SN Acoustic Instrument List

`insts.json` names every instrument an SN Acoustic tone can play, as listed by
Roland's INTEGRA-7 Editor: 127 instruments.

| Bank | Count |
|------|-------|
| `INT` (internal) | 77 |
| `ExSN1` … `ExSN5` (SuperNATURAL expansions 1–5) | 10, 15, 8, 6, 11 |

## Entry Format

```json
{ "number": 1, "name": "Concert Grand", "category": "Ac.Piano", "bank": "INT",
  "lsb": 64, "pc": 0, "modify": [] }
```

- **number** — position in Roland's list (1…127), for the picker; not a
  device value.
- **name** — instrument name, spelled as in Roland's Editor.
- **category** — Roland's category (19 of them: Ac.Piano, E.Piano, …).
- **bank** — `INT`, or the `ExSN1`…`ExSN5` expansion it comes from.
- **lsb**, **pc** — the pair that identifies the instrument; unique across
  the list. The editor writes them as the tone's Inst Variation (`lsb`) and
  Inst Number (`pc`), Common `00 20` and `00 21`. That pairing is inferred from
  the list's patterns, not documented; see `sn_acoustic::sna_offsets`.
- **modify** — `[index, name]` of the instrument's Modify Parameters, where
  Roland's Editor names them (`mod1`…). Absent for instruments the Editor
  gives a page of their own (e.g. Concert Grand, TW Organ). Not used yet.

## Source

Roland INTEGRA-7 Editor Ver.2, `Contents/Resources/html/js/view/preset/sna_inst.js`
(`globals.parameter.snaInstPreset`), "Copyright 2014 Roland Corporation",
extracted like the Wave lists (see `../waves/README.md`) from the public mirror
[SambaGodschynski/integra7-editor-web @ d5b9193](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset/sna_inst.js)
by evaluating the file in a Node `vm` context.
