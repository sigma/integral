# INTEGRA-7 Wave Names: Source and Shape (research for #98)

Answers [#98](https://github.com/sigma/integral/issues/98) (parent map
[#97](https://github.com/sigma/integral/issues/97)): where the name of every PCM **Wave**
(internal and SRX-01..12) can come from, and what the data looks like.

## TL;DR

- **Roland's own INTEGRA-7 Editor ships every Wave name list** as JavaScript arrays, one file
  per wave group: `wave_list_xv5080.js` (INT), `wave_list_srx01.js` … `wave_list_srx12.js`,
  and `wave_list_synth.js` (SN Synth PCM OSC). They are in the Editor Ver.2 plug-in bundle
  (`Contents/Resources/html/js/view/preset/`), and a public mirror is in
  [SambaGodschynski/integra7-editor-web](https://github.com/SambaGodschynski/integra7-editor-web/tree/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset).
  **Verified:** the installed Editor 2.0.5 (`jp.co.roland.INTEGRA7EditorV2`) and the mirror
  evaluate to identical arrays.
- **Shape:** a flat array of 12-character, space-padded names. **Array index = Wave Number.**
  Index 0 is `"OFF"`. There is no category, no L/R flag and no other metadata.
- **Counts** (waves, excluding OFF): INT 1083. SRX-01 719, -02 48, -03 522, -04 196,
  -05 818, -06 512, -07 860, -08 950, -09 873, -10 261, -11 40, -12 168 (6,967 SRX waves).
  SN Synth PCM 450. Every SRX count matches the "Waveform: N waveforms" line in Roland's
  *Sound List*.
- **Wave Group ID:** INT is Wave Group **Type** 0. Real Integra tones and Roland's own
  conversions store Group ID **1** for INT (some store 0), so readers must accept any Group ID
  when the Type is 0, and writers use 1 (see [INT Group ID](#int-group-id)). `n` = SRX-`n`
  for n = 1..12 (with Wave Group Type 1). The ID names the SRX **board**, not the expansion
  slot.
- **Stereo:** there is no structural pairing. The convention is L at Wave Number `n` and R at
  `n+1`, marked only by an `L`/`R` token in the name. Wave Number R is set separately.
- **SN Synth PCM OSC uses a different list** of 450 waves (1–450). Only 74 of its names also
  appear in the INT list.
- **Licensing:** Roland marks the files "Copyright 2014 Roland Corporation. All rights
  reserved." The mirror has no licence. Shipping the lists in this MIT repo needs a decision
  (see [Licensing](#licensing-and-redistribution)).
- **The Sound List PDF has no Wave list.** It lists Tones and gives only the waveform count
  per SRX. **No device query** for Wave names is known (see [Device query](#device-query)).

## Sources (highest trust first)

| # | Source | What it gives |
|---|--------|---------------|
| P1 | Roland *INTEGRA-7 Editor* Ver.2.0.5 for Mac, installed at `/Library/Audio/Plug-Ins/VST3/Roland/INTEGRA7EditorV2.vst3` ([Roland download page](https://www.roland.com/au/support/by_product/integra-7/updates_drivers/)) | The Wave name arrays, the mapping from Group ID to list, how the Wave Number is displayed. Verified locally. |
| P2 | [SambaGodschynski/integra7-editor-web @ d5b9193](https://github.com/SambaGodschynski/integra7-editor-web/tree/d5b9193874e2a0e3261a231562dff856c685f9fe) | A public mirror of P1's HTML/JS. It has CRLF line endings, but the arrays are identical to P1 (verified by evaluating both). Use it for linkable line numbers. |
| P3 | Roland *INTEGRA-7 Parameter Guide* e01 ([PDF](https://static.roland.com/assets/media/pdf/INTEGRA-7_Parameter_Guide_e01_W.pdf)) | Wave Group / Wave No. semantics (p. 50, p. 67), SN-S Wave Number 1–450 (p. 34), and two named examples that pin the numbering. |
| P4 | Roland *INTEGRA-7 Sound List* je01 ([PDF](https://static.roland.com/assets/media/pdf/INTEGRA-7_Sound_List_je01_W.pdf)) | Waveform count per SRX (SRX pages 30–57). **No per-wave names.** |
| P5 | `docs/midi/06-pcm-synth-tone.md` `00 27`–`00 30`, `docs/midi/07-pcm-drum-kit.md` `00 22`–`00 2B`, `docs/midi/08-supernatural-synth-tone.md` `00 35` | Raw parameter ranges (Wave Number `0`–`16384`, `0` = OFF). |
| S1 | [shimpe/Integra7AuralAlchemist @ d40f43d](https://github.com/shimpe/Integra7AuralAlchemist/tree/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Assets) (GPL-3.0) | Third-party CSV copies of the same lists (`PartialWaveForms_{INT,SRX1..12}.csv`, `PcmWavenumbers.csv`). Secondary: altered text, and an off-by-one for SRX (below). |

## Findings

### 1. Where the names live (P1, P2)

Each file assigns one array:

| File (P2 link) | Global | Entries incl. OFF |
|---|---|---|
| [`wave_list_xv5080.js`](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset/wave_list_xv5080.js) | `globals.parameter.wave.xv5080` | 1084 |
| [`wave_list_srx01.js`](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset/wave_list_srx01.js) … `wave_list_srx12.js` | `globals.parameter.wave.srx01` … `srx12` | 720, 49, 523, 197, 819, 513, 861, 951, 874, 262, 41, 169 |
| [`wave_list_synth.js`](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset/wave_list_synth.js) | `globals.parameter.wave.snSynth` | 450 (no OFF) |
| [`pcm_wave_name.js`](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/preset/pcm_wave_name.js) | `globals.parameter.wavePreset.pcmWaveName` | 450, identical to `snSynth` |

The INT list is called `xv5080`. The INTEGRA-7 internal wave ROM is the XV-5080 set, and the
first entries match it (`StGrand pA L`, `StGrand pA R`, …).

`wave_list_synth.js` starts with a commented-out block labelled `// ROM 9`, lines 10–111
(`MGSAW_FHD`, `TB303SAW FHD`, …). These are not part of the live array, and the editor never
shows them. The live array starts at line 112 with `"001 JP-8 Saw    "`.

Counts were taken by running the files in a Node `vm` context, not by text matching.

Sample (INT, `wave_list_xv5080.js` lines 8–14):

```
"OFF         ", "StGrand pA L", "StGrand pA R", "StGrand pB L",
"StGrand pB R", "StGrand pC L", "StGrand pC R", ...
```

Sample (SN Synth, `wave_list_synth.js` line 112 onward): `"001 JP-8 Saw    "`,
`"002 Syn Saw Wave"`, … `"450 SynthFx 2   "`. These have a 3-digit number prefix and
16-character padding.

### 2. Wave Number numbering: index = Wave Number, 0 = OFF (verified)

- **Editor (P1, P2):** the PCM-S Wave Number slider runs `0 … wavelist.length-1`, and its
  label is `value + ' ' + wavelist[value]`
  ([`scene_tone_pcms.js` L610–636](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_tone_pcms.js#L610-L636)).
  PCM-D does the same
  ([`scene_tone_pcmd.js` L174–223](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_tone_pcmd.js#L174-L223)).
  So the raw parameter value is used directly as the array index.
- **Parameter Guide (P3), pp. 50/67:** Wave Tempo Sync examples "SRX05 3:080:BladeBtL" and
  "SRX08 5:75:BoomRvBel". In the Roland arrays, `srx05[3]` = `080:BladeBtL` and
  `srx08[5]` = `75:BoomRvBel` (`wave_list_srx05.js` L12, `wave_list_srx08.js` L13). This
  independently confirms index = Wave Number, with OFF at 0, for SRX.
- This matches P5: Wave Number `0` = OFF, `1`–`16384` are waves.

### 3. Wave Group Type / Wave Group ID (verified from P1/P2; P3 agrees)

- `model_stringer.js` enumerates the Group ID display as
  `"INT"or"SRX01"or…"SRX12"`. PCM-S partials:
  [L2724](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L2724).
  PCM-D WMT:
  [L3136–3137](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L3136-L3137).
  Index 0 is INT, and `n` is SRX`n`. This is the editor's list index, not what tones
  store for INT (see [INT Group ID](#int-group-id)).
- When the Group selector changes, the editor writes Group Type `0` if the list index is `0`
  and `1` otherwise. The index itself is written as the ID
  ([`scene_tone_pcms.js` L683–695](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_tone_pcms.js#L683-L695)).
  Group ID `n` selects `wave.srx0n`
  ([L559–606](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_tone_pcms.js#L559-L606)).
- SRX options are hidden unless board `n` is in `curExpansion`, the four loaded slots
  ([L663–668](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_tone_pcms.js#L663-L668)).
  So the ID is the **SRX board number** (1..12), independent of which slot holds it. The
  "only if an expansion is loaded" note in P3 p. 50 agrees.
- Group Type values 2–3 are `---` (reserved) in P5. The editor never writes them.

| Group Type | Group ID | List | Waves | Sound List title (P4) |
|---|---|---|---|---|
| 0 INT | any; 1 written (see below) | `xv5080` | 1083 | (internal) |
| 1 SRX | 1 | `srx01` | 719 | SRX-01 Dynamic Drum Kits |
| 1 SRX | 2 | `srx02` | 48 | SRX-02 Concert Piano |
| 1 SRX | 3 | `srx03` | 522 | SRX-03 Studio SRX |
| 1 SRX | 4 | `srx04` | 196 | SRX-04 Symphonique Strings |
| 1 SRX | 5 | `srx05` | 818 | SRX-05 Supreme Dance |
| 1 SRX | 6 | `srx06` | 512 | SRX-06 Complete Orchestra |
| 1 SRX | 7 | `srx07` | 860 | SRX-07 Ultimate Keys |
| 1 SRX | 8 | `srx08` | 950 | SRX-08 Platinum Trax |
| 1 SRX | 9 | `srx09` | 873 | SRX-09 World Collection |
| 1 SRX | 10 | `srx10` | 261 | SRX-10 Big Brass Ensemble |
| 1 SRX | 11 | `srx11` | 40 | SRX-11 Complete Piano |
| 1 SRX | 12 | `srx12` | 168 | SRX-12 Classic EPs |

The SRX counts equal the "• Waveform: N waveforms" line on each SRX page of P4 (verified for
all 12). P4 gives no count for INT. 1083 comes from P1 only.

#### INT Group ID

The editor's list index puts INT at 0, but that is not device data. Stored tones say
otherwise:

- Real Integra tones store Group ID 1 on every active INT partial
  ([`docs/svd/06-pcm-synth.md`](../svd/06-pcm-synth.md), real-tone checks).
- Roland's own XV conversions store 1 on 5,896 of 6,315 INT tones and 0 on the other 419
  ([`map-check-roland-pairs.md`](map-check-roland-pairs.md)).

So the device accepts both, and 1 is what Roland writes. Integral reads Type 0 as INT
whatever the ID, keeps a stored ID as is, and writes 1
([#207](https://github.com/sigma/integral/issues/207)). Whether 0 and 1 are fully
equivalent on the device is untested.

### 4. Stereo pairs → Wave Number L / R

- P3 (p. 50/67): "When in monaural mode, only the left side (L) is specified. When in stereo,
  the right side (R) is also specified. If you specify only the right side (R), there will
  be no sound." L and R are independent numbers in the **same** group. There is one Group
  Type/ID per partial/WMT.
- The lists carry **no pairing metadata**. Stereo samples appear as adjacent entries whose
  names differ only by an `L`/`R` token. The token is a suffix in INT/SRX-01/-03/-04/-05/-10/-11
  (`StGrand pA L`, `StGrand pA R`) and mid-name in SRX-02/-06/-07 (`Grand1 L p A`,
  `Grand1 R p A`; `StGrand L pA`, `StGrand R pA`).
- Adjacent `…L`/`…R` suffix pairs found by name comparison: INT 70, SRX-01 301, -03 167,
  -04 98, -05 101, -06 14, -08 2, -10 123, -11 20. SRX-02/-07 use the mid-name form, which is
  not counted here. Pairs do not always start on an odd number. **Inferred, not
  device-verified:** a stereo pick is L = `n`, R = `n+1`. A picker can offer "stereo" only by
  this naming heuristic, and Roland's editor makes no attempt (two independent sliders).

### 5. SN Synth PCM OSC: a separate 450-entry list (verified)

- P3 p. 34: SN-S OSC "Wave Number 1–450 … valid only if PCM is selected for OSC Wave".
- `model_stringer.js` declares `SHPT_WAV_NUML` as `"1"to"450"`
  ([L3832–3833](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L3832-L3833)),
  displayed through `globals.parameter.wave.snSynth`
  ([L4392](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L4392),
  [L4535–4541](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L4535-L4541)).
  Raw `n` maps to the entry `"nnn Name"`, so entry `n-1` (0-based) of the array. **Inferred** from
  the number prefix and the 1–450 range. Not checked on the device.
- It is not a subset of INT: only 74 of 450 names also occur in `xv5080`. There is no Group
  Type/ID: one list, no SRX.

### 6. Category metadata

None in any source. The arrays are bare names, and P3/P4 attach no category to Waves. Names
carry informal hints that are **not** reliable fields: `L`/`R`, dynamics (`pA`, `mf`, `ff`),
multisample zone letters (`A`/`B`/`C`), `REV …` (reversed), and a `BPM:` prefix on SRX
phrase loops (`080:BladeBtL`, used by Wave Tempo Sync, P3). SRX-09 tempo prefixes are
space-padded (`" 78:GamelnP2"`). Any grouping a picker shows (by Group, then by name prefix)
would have to be derived by us.

### 7. Device query

No device query for Wave names is documented (P5), and `docs/midi/99-undocumented.md` lists
none (its catalog queries at `0F 00 03 02` / `0F 00 04 02` return Studio Set and Tone names
only). Roland's editor has the Wave names hard-coded rather than reading them over MIDI.
**Inference (not proven):** the device exposes no Wave-name query. A capture of the iPad
editor's PCM Wave page (procedure in `99-undocumented.md`) would settle it.

### 8. Third-party copies: shimpe/Integra7AuralAlchemist (S1)

S1 ships the same 13 lists as CSV, added in commit
[4dc4fe7](https://github.com/shimpe/Integra7AuralAlchemist/commit/4dc4fe7842) and described as
"reverse-engineered". Compared with P1:

- The row counts match for every group. The INT CSV has an `"Off"` row 0. **The SRX CSVs
  have no OFF row**, but
  [`WaveformBanks.ParseWaveCsv`](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/WaveformBanks.cs#L49-L63)
  numbers every file from 0. So S1 shows SRX names **one off** (its Wave 3 in SRX-05 is
  `080:BladeBtR`, while P3 says `BladeBtL`). Do not copy S1's numbering.
- The text is altered: whitespace is collapsed, some apostrophes became typographic quotes
  (`You’re DJ M`), and some names differ. SRX-08 #637 is `Snare MENU1` in S1 but
  `Snare MENU10` in Roland's list. INT #28–42 are `SA Rhodes 1A` / `Dyn Rhd mp A` in S1 but
  `SA EP 1 A` / `Dyn EP mp A` in Roland's list. **Open:** which form the INTEGRA-7's own
  display shows (check on the device).
- `PcmWavenumbers.csv` (450 rows) matches `snSynth` except for 26 spelling differences.
- Licence GPL-3.0, so it is not usable in this MIT repo in any case.

## Licensing and redistribution

- P1/P2 files: "Copyright 2014 Roland Corporation. All rights reserved." Editor bundle:
  "Copyright © 2020 Roland Corporation." The P2 mirror has **no licence file**
  (GitHub reports `NOASSERTION`), so it grants nothing beyond Roland's terms.
- S1 is GPL-3.0, which is incompatible with relicensing under this repo's MIT.
- Precedent in this repo: `docs/sounds/*.json` already ships Tone names extracted from
  Roland's Sound List, and `docs/research/fx-param-layout.md` relies on the same editor JS.
  Whether a bare list of short sample names is protectable is a legal question this note does
  not answer. **Options for #97:** (a) generate the table at build time from a user-supplied
  Roland Editor install, (b) commit the list as factual data, as was done for Tones, or
  (c) ship numbers only and load names from the user's editor at run time. This is a
  decision for the PRD, not settled here.

## Recommended data shape for `integral-core` (input to #99–#101)

Per wave group: `(group_type, group_id) → [name; count]`, with Wave Number = index + 1
(index 0 of Roland's array is OFF). Names are at most 12 ASCII characters, right-trimmed. The
SN-S list is a separate 450-entry table keyed by Wave Number 1–450. Total size is about
8.5k names × ≤12 bytes, roughly 100 KB raw. Stereo is a naming heuristic, not data.

## Open questions

1. Does the device display `SA EP 1 A` (Roland editor) or `SA Rhodes 1A` (S1) for INT
   #28–42? Read a tone using those waves on the front panel.
2. Device-verify SN-S raw `n` ↔ `"nnn"` (expected: raw 1 = `JP-8 Saw`).
3. Is there a Wave-name query? Capture the iPad editor on the PCM Wave page.
4. Licensing route for shipping the names (see above).
