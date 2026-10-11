# JV/XP Patch SysEx Format vs the PCM Synth Tone (research for #194)

Answers [#194](https://github.com/sigma/integral/issues/194), part of the Foreign patch map
[#192](https://github.com/sigma/integral/issues/192). It covers the SysEx format of a JV-1080,
JV-2080, JV-1010 or XP-30/50/60/80 **Patch**, where each parameter lands on the INTEGRA-7 PCM
Synth tone ([`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md)), and how the format
checks out against the owner's local corpus.

Terms follow `CONTEXT.md`: **Wave**, **Wave Group**, **Wave Number**, **Foreign patch**,
**Conversion report**. Wave identity is [#195](https://github.com/sigma/integral/issues/195)'s
job. EFX, Chorus and Reverb are [#196](https://github.com/sigma/integral/issues/196)'s job. This
note covers only where those fields sit in the dump.

## TL;DR

- **One format for all of them.** Every model in scope uses Roland Model ID **`6A`** and the same
  Patch layout: a **Patch Common** block at `+00 00` and four **Patch Tone** blocks at
  `+10 00 / 12 00 / 14 00 / 16 00`. A Tone is `0x81` (129) data bytes over the address span
  `00 00`–`01 00`. The models differ only in how long **Patch Common** is (`0x48` / `0x49` /
  `0x4A`) and in a few enumerations (P1–P6).
- **The model shows in the bytes, not the Model ID.** Patch Common length tells the families
  apart: `0x48` = JV-1080, `0x49` = XP-50/60/80 (adds Clock Source), `0x4A` = JV-2080, JV-1010,
  XP-30 (adds Clock Source and Patch Category). Performance Common length narrows it further
  (`0x40` JV-1080, `0x42` XP-60/80/JV-1010/XP-30, `0x44` JV-2080).
- **Most of a Tone maps 1:1 or with a fixed offset.** Of the 71 rows in the Tone table, 34 are
  identical (same range and meaning). 12 differ by a constant (`+1` for signed ±63 values, `+16`
  coarse tune, `+14` fine tune, `+52` pitch-env depth). 7 are keyfollow tables that convert
  exactly (`i = 64 + table[v]/10`). The other 18 need a table, a rescale, a structural transform,
  or have no target.
- **Losses that can't be avoided:** per-Patch Chorus/Reverb, Patch Tempo, EFX/Tone Output Assign
  (MIX bypasses EFX), Volume Control Switch, Pan Control Switch KEY-ON/OFF, Hold/Peak, Clock
  Source, and the delay mode KEY-INTERVAL/PLAYMATE. CLOCK-, TAP- and TEMPO-SYNC delays and LFO
  External Sync become note values that follow the Studio Set tempo
  ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)).
- **Two structural transforms are needed:**
  1. **Controllers.** A JV Tone has its own 3 × 4 destinations and depths. The Integra has 4
     common Matrix Controls with per-partial ON/OFF/REVERSE switches. On the corpus, 80 % of
     patches fit in Matrix Controls 1–3, 15 % more fit by borrowing Matrix Control 4, and **5 %
     (135 of 2,626) overflow**.
  2. **Ranges move to the PMT.** Per-tone key and velocity ranges move into the PMT, and the one
     JV Velocity Cross Fade value has to feed both Integra fade widths.
- **The sensitivity scales don't match.** Velocity sensitivities are **−100…+150 %** on the JV
  but **±63** on the Integra. Roland publishes no formula, but its own JV → XV conversions map
  ±100 % to **±32** and stretch +100…+150 % over +32…+63 by a table
  ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)).
- **Corpus check:** 29,969 DT1 messages, all Model `6A`, all device ID `10`. They hold
  **2,626 distinct real Patches** (4,623 with repeats). Every byte of every one is within the
  documented ranges. 9 `.SYX` files are damaged by CR-LF line-ending conversion.

## Sources

| # | Source | What it gives |
|---|--------|---------------|
| P1 | Roland *JV-1080 Owner's Manual*, MIDI Implementation, pp. 149–155; Waveform List pp. 122–123 ([PDF](https://static.roland.com/assets/media/pdf/JV-1080_OM.pdf), scanned) | Model ID 6A, RQ1/DT1, full address map; Patch Common `0x48`, Tone `0x101`; data-card area `20/21 xx`; INT-A 255 + INT-B 193 Waves |
| P2 | Roland *JV-2080 Owner's Manual*, Parameter address map, pp. 188–190 ([PDF](https://static.roland.com/assets/media/pdf/JV-2080_OM.pdf), scanned) | Patch Common `0x4A` (Clock Source `48`, Patch Category `49`), Performance Common `0x44` (EFX-A/B/C), Part `0x1A`, Delay Mode 0–7 |
| P3 | Roland *XP-60/XP-80 Owner's Manual*, Parameter address map, pp. 221–224 ([PDF](https://static.roland.com/assets/media/pdf/XP-60_80_OM.pdf), scanned) | Patch Common `0x49` (Clock Source PATCH/SEQUENCER), Performance Common `0x42`, Part `0x19`, Delay Mode 0–7 |
| P4 | Roland *XP-50 Owner's Manual*, pp. 145–146 ([PDF](https://static.roland.com/assets/media/pdf/XP-50_OM.pdf), scanned) | Patch Common `0x49`, Delay Mode 0–6 (no TEMPO-SYNC), Rhythm Note `0x3A` |
| P5 | Roland *JV-1010 Owner's Manual*, MIDI Implementation, pp. 72–80 ([PDF](https://static.roland.com/assets/media/pdf/JV-1010_OM.pdf), text) | Same map; `< >` marks values only the JV-1080/2080/XP accept; Wave Group table incl. XP-A(Session) = EXP ID 9 |
| P6 | Roland *XP-30 Owner's Manual*, pp. 69, 75–76 (parameter descriptions), 190–196 (map) ([PDF](https://static.roland.com/assets/media/pdf/XP-30_OM.pdf), text) | Cross Fade meaning, "control source 1 is fixed at Modulation", Delay Mode meanings; XP-B(Orch.) = EXP ID 2, XP-C(Techno) = EXP ID 11 |
| I1 | [`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md) (from the INTEGRA-7 MIDI Implementation v1.00) | Target layout |
| C1 | Owner's corpus `~/Downloads/JV-1080_SP/` (204 `.SYX`, 275 `.MID`), read-only | Checks lengths, ranges, enumerations, framing |

P1–P6 are primary. The scanned manuals (P1–P4) were read from page images. Their OCR was used
only to find the right pages. For the JV-1010 and XP-30 (P5, P6), Roland ships a text layer.
Below, a "JV" claim with no further tag means all six manuals agree on it.

## Framing

### Messages

| Message | Bytes | Notes |
|---|---|---|
| DT1 | `F0 41 dev 6A 12 aa bb cc dd <data…> sum F7` | `dev` 00–1F (unit# − 1). The default is `10`, and the whole corpus uses `10`. |
| RQ1 | `F0 41 dev 6A 11 aa bb cc dd ss tt uu vv sum F7` | 4-byte size. The JV-1080 only receives RQ1 (P1 p. 149). |
| Checksum | `(128 − (Σ address + data) mod 128) mod 128` | Same rule as the Integra. |

- **One DT1 per block.** In the corpus a Patch is always 5 DT1s: Common (72/73/74 data bytes)
  followed by Tone 1–4 (129 data bytes each, in order). Blocks are never split and never merged.
  The only other Patch-area DT1s are 1-byte parameter edits (e.g. `RDPART*.SYX`, `12STR-P*.SYX`).
- **7-bit addressing inside a block.** Tone data byte *k* is at address offset `(k >> 7, k & 7F)`.
  Byte 128 is offset `01 00` (Reverb Send Level). The Integra partial (`0x11A`) works the same way.
- **Nibblized fields (`#`)** take two bytes, high nibble first: Tone **Wave Number** (`03`–`04`,
  0–254) and Common **Patch Tempo** (`2C`–`2D`, 20–250). There are no 4-nibble fields; the
  Integra's Wave Group ID and Wave Number fields are 4-nibble.
- **Containers.** `.SYX` files hold back-to-back raw messages. `.MID` files hold the same
  messages as SMF `F0` events (corpus: 12,796 `F0` events, all complete in one event, with no
  `F7` continuation packets). 187 of the 275 `.MID` files carry no SysEx at all.

### Where a Patch lives (P1 p. 154–155, P2 p. 188, P3 p. 221, P5 p. 74)

| Start address | Area | Patch slot |
|---|---|---|
| `03 00 00 00` | Patch-mode Temporary Patch | — |
| `02 0p 00 00`, p = 0–8, A–F | Performance-mode Temporary Patch of Part p+1 | `02 09` is the **Temporary Rhythm Setup**, not a patch |
| `11 pp 00 00`, pp = 00–7F | User Patch USER:001–128 | pp + 1 |
| `21 pp 00 00` | Data-card Patch CARD:001–128 | **JV-1080 only** |
| `10 00`–`10 1F 00 00` | User Performance 1–32 | — |
| `10 40` / `10 41 00 00` | User Rhythm Setup 1 / 2 | — |
| `20 00 / 20 40 xx xx` | Data-card Performance / Rhythm | JV-1080 only |
| `01 00 00 00` | Temporary Performance | — |
| `00 00 00 00` | System | — |

The address says nothing about the source model. A reader should treat User, Temporary and
Card Patches the same way: each one is a Patch, and the slot number is only a hint for the name.

### Identifying the source model

The Model ID (`6A`) is shared by the JV-1080, JV-2080, JV-1010, XP-30, XP-50, XP-60 and XP-80.
(The XV series uses `00 10`; see [#193](https://github.com/sigma/integral/issues/193).) The
families differ by block length:

| Block | `0x48`/`0x40`/`0x13` | `0x49` | `0x4A` | `0x42`/`0x19` | `0x44`/`0x1A` |
|---|---|---|---|---|---|
| Patch Common | **JV-1080** (P1) | **XP-50/60/80** (P3, P4) | **JV-2080, JV-1010, XP-30** (P2, P5, P6) | | |
| Performance Common / Part | JV-1080 | | | XP-60/80, JV-1010, XP-30 | JV-2080 |
| System Common | JV-1080 `0x28` | | | JV-1010/XP-30 `0x66` | JV-2080 `0x62` |

Other cues:

- A Wave Group Type 2 (EXP) with ID 9, 2 or 11 plus a `0x4A` Common hints at a JV-1010 or XP-30.
  On those models the built-in XP-A/B/C sets are addressed as EXP IDs 9/2/11 (P5, P6). This is
  ambiguous with a JV-2080 holding SR-JV80-09/02/11 boards, and it doesn't matter for Wave
  mapping: same ID, same content (to confirm in #195).
- A Delay Mode of 7 (TEMPO-SYNC) rules out the JV-1080 and XP-50.
- Data-card addresses mean a JV-1080.

A reader should **detect by Patch Common length** and record the family in the Conversion
report. It should not refuse a shorter or longer Common: the parameter map is a prefix-compatible
superset (P5's `< >` convention).

## Model differences that matter

| Field | JV-1080 (P1) | XP-50 (P4) | XP-60/80 (P3) | JV-2080 (P2) | JV-1010 / XP-30 (P5, P6) |
|---|---|---|---|---|---|
| Patch Common length | `0x48` | `0x49` | `0x49` | `0x4A` | `0x4A` |
| Common `48` Clock Source | — | PATCH, SEQUENCER | PATCH, SEQUENCER | PATCH, SYSTEM | PATCH, SYSTEM |
| Common `49` Patch Category | — | — | — | 0–38 | 0–38 |
| Tone `09` Delay Mode | 0–6: NORMAL, HOLD, **KEY-INTERVAL**, CLOCK-SYNC, TAP-SYNC, KEY-OFF-NORMAL, KEY-OFF-DECAY | 0–6, index 2 = **PLAYMATE** | 0–7, + TEMPO-SYNC | 0–7, + TEMPO-SYNC | 0–7 (TAP-SYNC ignored) |
| Common `19` EFX Output Assign | MIX, OUTPUT1, OUTPUT2 | | MIX, DIR, \<OUTPUT-2> | MIX, DIR1, DIR2 | MIX, \<DIRECT-1>, \<DIRECT-2> |
| Tone `7D` Output Assign | MIX, EFX, OUTPUT1, OUTPUT2 | MIX, EFX, \<OUTPUT-1>, \<OUTPUT-2> | | MIX, EFX, DIR1, DIR2 | MIX, EFX, \<DIRECT-1>, \<DIRECT-2> |
| Wave Group Type 1 (PCM card) | yes | yes | yes | yes | ignored |
| Internal Waves | INT-A 1–255, INT-B 1–193 (448) | same ROM | same ROM | same ROM | same + XP-A/B/C as EXP |

KEY-INTERVAL (JV-1080) and PLAYMATE (later models) sit at the same index. They look like the same
mode renamed, but this is **not confirmed**. P6 defines PLAYMATE; P1 has no description in its map
pages. Neither has an Integra equivalent, so the question doesn't change the conversion: Roland
converts both to NORMAL with Delay Time 0 ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)).

## Parameter map: Patch Common → PCM Synth Tone

Notation: `v` = JV raw byte, `i` = Integra raw byte. Targets: **C** = Common `00 00 00`,
**MFX** = `00 02 00`, **PMT** = `00 10 00`, **C2** = Common 2 `00 30 00`. "Identical" means the
same raw range and meaning.

| JV off. | JV parameter (range) | Integra target | Mapping |
|---|---|---|---|
| `00`–`0B` | Patch Name 1–12 (32–127) | C `00`–`0B` Tone Name 1–12 | identical |
| `0C` | EFX Type (0–39) | MFX `00` MFX Type | by table (#196) |
| `0D`–`18` | EFX Parameter 1–12 (0–127) | MFX Parameter 1–32 | by table (#196) |
| `19` | EFX Output Assign | — | **none** (the Integra has no tone-level output routing) |
| `1A` | EFX Mix Out Send Level | — | none at tone level; may fold into an MFX level parameter (#196) |
| `1B` | EFX Chorus Send Level | MFX `02` | identical |
| `1C` | EFX Reverb Send Level | MFX `03` | identical |
| `1D`, `1F` | EFX Control Source 1/2 (OFF, SYS-CTRL1, SYS-CTRL2, MOD, BREATH, FOOT, VOLUME, PAN, EXPRESSION, BEND, AFT) | MFX `05`, `07` Control 1/2 Source | by index (the source list is indexed by CC number): SYS-CTRL1→SYS2 (99), SYS-CTRL2→SYS3 (100), MOD→CC01, BREATH→CC02, FOOT→CC04, VOLUME→CC07, PAN→CC10, EXPRESSION→CC11, BEND→96, AFT→97 (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `1E`, `20` | EFX Control Depth 1/2 (0–126 = ±63) | MFX `06`, `08` Control Sens | `i = v + 1` |
| — | (EFX control destination is fixed per type) | MFX `0D`–`10` Control Assign | from the EFX type (#196) |
| `21`–`26` | Chorus Level, Rate, Depth, Pre-Delay, Feedback, Output | — | **none** in a tone; Studio Set Chorus (#196) |
| `27`–`2B` | Reverb Type, Level, Time, HF Damp, Delay Feedback | — | **none** in a tone; Studio Set Reverb (#196) |
| `2C`–`2D`# | Patch Tempo (20–250) | — | **none** (tempo belongs to the Studio Set) |
| `2E` | Patch Level | C `0E` Tone Level | identical |
| `2F` | Patch Pan | C `0F` Tone Pan | identical |
| `30` | Analog Feel | C `15` | identical |
| `31` | Bend Range Up (0–12) | C `29` (0–48) | identical value |
| `32` | Bend Range Down (0–48) | C `2A` | identical |
| `33` | Key Assign Mode (POLY, SOLO) | C `16` Mono/Poly (MONO, POLY) | `i = 1 − v` |
| `34` | Solo Legato | C `17` Legato Switch | identical |
| `35`–`39` | Portamento Switch, Mode, Type, Start, Time | C `19`–`1D` | identical |
| — | Control 1 source: **fixed to Modulation** (P6 p. 75) | Matrix Control source (allocated by the controller transform, below) | CC01 |
| `3A`, `3B` | Patch Control Source 2/3 (OFF, SYS-CTRL1/2, MOD, BREATH, FOOT, VOLUME, PAN, EXPRESSION, BEND, AFT, LFO1, LFO2, VELOCITY, KEYFOLLOW, PLAYMATE) | Matrix Control source (allocated by the controller transform, below) | by index as above: SYS-CTRL1→CTRL2 (99), SYS-CTRL2→CTRL3 (100), MOD→CC01, BEND→96, AFT→97, VELOCITY→102, KEYFOLLOW→103, LFO1→105, LFO2→106; **PLAYMATE → none** (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `3C`–`3F` | EFX / Control 1–3 Hold/Peak | — | **none** |
| `40` | Velocity Range Switch | PMT `04` PMT Velocity Control | PMT Velocity Control is always ON. Switch OFF → every tone's velocity range is 1–127 with fades 0, discarding the JV ranges the JV ignores (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `41` | Octave Shift (0–6) | C `13` (61–67) | `i = v + 61` |
| `42` | Stretch Tune Depth | C `14` | identical |
| `43` | Voice Priority (LAST, LOUDEST) | C `10` | identical |
| `44`, `46` | Structure Type 1&2 / 3&4 (0–9) | PMT `00`, `02` | identical |
| `45`, `47` | Booster 1&2 / 3&4 (0–3) | PMT `01`, `03` | identical |
| `48` | Clock Source (XP, JV-2080+) | — | **none** |
| `49` | Patch Category (0–38, JV-2080/1010/XP-30) | C2 `10` Tone Category | via the XV Patch Category of the same index, as Roland converts it ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |

Integra Common fields with no JV source, and their defaults: Coarse/Fine Tune 64, Legato
Retrigger ON (as Roland converts it, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)),
Portamento defaults, Cutoff/Resonance/Attack/Release/Velocity offsets 64, PMT Control Switch,
Matrix Control 4 (OFF unless the controller packing needs it), MFX Control 3–4 OFF.

## Parameter map: Patch Tone n → PCM Synth Partial n (+ PMT n)

Targets are Partial n offsets unless they say PMT. `T15` = the 16-step keyfollow table
`−100,−70,−50,−30,−10,0,+10,+20,+30,+40,+50,+70,+100,+120,+150,+200`. `T14` = the 15-step
table `−100,−70,−50,−40,−30,−20,−10,0,+10,+20,+30,+40,+50,+70,+100` (P1 p. 153).

| JV off. | JV parameter (range) | Integra target | Mapping |
|---|---|---|---|
| `00` | Tone Switch | PMT n Partial Switch | identical |
| `01` | Wave Group Type (0 INT, 1 PCM card, 2 EXP) | `27` Wave Group Type (INT, SRX) | via the Wave map (#195) |
| `02` | Wave Group ID (INT: 1 = INT-A, 2 = INT-B; PCM: card ID; EXP: SR-JV80 board no.) | `28`# Wave Group ID | via the Wave map (#195) |
| `03`–`04`# | Wave Number (0–254 = Wave 1–255) | `2C`# Wave Number L (0 = OFF) | via the Wave map; note JV raw *v* = Wave *v+1*. `30`# Wave Number R = 0 |
| `05` | Wave Gain (−6, 0, +6, +12) | `34` | identical |
| `06` | FXM Switch | `35` | identical |
| `07` | FXM Color (1–4) | `36` | identical |
| `08` | FXM Depth (0–15 = 1–16) | `37` (0–16) | `i = v + 1` |
| `09` | Tone Delay Mode | `09` Delay Mode (NORMAL, HOLD, KEY-OFF-NORMAL, KEY-OFF-DECAY) | NORMAL→0, HOLD→1, KEY-OFF-N→2, KEY-OFF-D→3, Delay Time copied; CLOCK-SYNC, TAP-SYNC → NORMAL with a note-valued Delay Time (128–149); **KEY-INTERVAL/PLAYMATE → NORMAL with Delay Time 0** (reported) (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)). TEMPO-SYNC (no pair shows it) is read as CLOCK-SYNC, a chosen stand-in |
| `0A` | Tone Delay Time (0–127) | `0A`# Delay Time (0–149) | identical for 0–127 (write as two nibbles); a note value when the Delay Mode is synced (see `09`) |
| `0B` | Velocity Cross Fade (0–127) | PMT n Velocity Fade Width Lower **and** Upper | the JV value fades at whichever edge of the range the velocity crosses (P6 p. 69), so copy it to both widths; Roland does the same, and only when the Velocity Range Switch is ON (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `0C`, `0D` | Velocity Range Lower/Upper (1–127) | PMT n Velocity Range Lower/Upper | identical when the Velocity Range Switch is ON; 1–127 when it is OFF (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `0E`, `0F` | Keyboard Range Lower/Upper | PMT n Keyboard Range Lower/Upper | identical; Keyboard Fade Widths = 0 |
| `10` | Redamper Control Switch | `16` Redamper Switch | identical |
| `11` | Volume Control Switch (CC7) | — | **none**: Receive Expression (`13`) stays ON; OFF is reported (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `12` | Hold-1 Control Switch | `14` Receive Hold-1 | identical |
| `13` | Pitch Bend Control Switch | `12` Receive Bender | identical |
| `14` | Pan Control Switch (OFF, CONTINUOUS, KEY-ON) | `15` (reserve; the XV's Receive Pan Mode) | KEY-ON→1, CONTINUOUS and OFF→0 (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)). The Integra doesn't read the reserve, so KEY-ON and OFF are reported |
| `15`–`2C` | Controller 1–3 Destination 1–4 (0–18) and Depth 1–4 (0–126) | Common Matrix Control 1–3 Dest/Sens + `17`–`26` Partial Control *c* Switch *k* | **structural** (see below). Destination indices 0–18 are the same lists (OFF, PCH, CUT, RES, LEV, PAN, MIX→DRY, CHO, REV, PL1/2→PIT-LFO1/2, FL1/2→TVF-LFO1/2, AL1/2→TVA-LFO1/2, pL1/2→PAN-LFO1/2, L1R/L2R→LFO1/2-RATE); `sens = v + 1` |
| `2D`, `35` | LFO1/2 Waveform (TRI, SIN, SAW, SQR, TRP, S&H, RND, CHS) | `6D`, `7B` | TRI→1, SIN→0, SAW→2 (SAW-UP), SQR→4, TRP→8, S&H→9, RND→5, CHS→10 (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `2E`, `36` | LFO Key Sync | `76`, `01 04` Key Trigger | identical |
| `2F`, `37` | LFO Rate (0–127) | `6E`#, `7C`# (0–149) | identical for 0–127; a note value (128–149) when External Sync is CLOCK or TAP (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `30`, `38` | LFO Offset (−100…+100, 5 steps) | `70`, `7E` | identical |
| `31`, `39` | LFO Delay Time | `72`, `01 00` | identical |
| `32`, `3A` | LFO Fade Mode (ON-IN, ON-OUT, OFF-IN, OFF-OUT) | `74`, `01 02` | identical |
| `33`, `3B` | LFO Fade Time | `75`, `01 03` | identical |
| `34`, `3C` | LFO External Sync (OFF, CLOCK, TAP) | (via LFO Rate) | CLOCK, TAP → the LFO Rate becomes a note value 128–149, by the same table as the synced Delay Time (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)). It then follows the Studio Set tempo |
| `3D` | Coarse Tune (0–96) | `01` (16–112) | `i = v + 16` |
| `3E` | Fine Tune (0–100) | `02` (14–114) | `i = v + 14` |
| `3F` | Random Pitch Depth (0–30) | `03` | identical (same 31-step table) |
| `40` | Pitch Keyfollow (`T15`) | `39` Wave Pitch Keyfollow (44–84 = ±200 step 10) | `i = 64 + T15[v]/10` (exact) |
| `41` | P-Env Depth (0–24) | `3A` (52–76) | `i = v + 52` |
| `42` | P-Env Velocity Sens (0–125 = −100…+150 %) | `3B` (±63) | `i = 64 + trunc((v − 50) × 16/25)` for `v` ≤ 105 (±100 % = ±32), then Roland's table up to 127 for `v` 106–125 (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `43`, `44` | P-Env Velocity Time 1/4 (`T14` %) | `3C`, `3D` (±63) | `i = 64 + trunc(T14[v] × 63/100)` (±100 % = ±63) (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `45` | P-Env Time Keyfollow (`T14`) | `3E` (54–74 = ±100 step 10) | `i = 64 + T14[v]/10` (exact) |
| `46`–`49` | P-Env Time 1–4 | `3F`–`42` | identical |
| `4A`–`4D` | P-Env Level 1–4 (0–126) | `44`–`47` (1–127) | `i = v + 1`; `43` Level 0 = 64 (JV starts at 0) (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| `4E`, `4F` | Pitch LFO1/2 Depth | `77`, `01 05` | `i = v + 1` |
| `50` | Filter Type (OFF, LPF, BPF, HPF, PKG) | `48` (adds LPF2, LPF3) | identical |
| `51` | Cutoff Frequency | `49` | identical |
| `52` | Cutoff Keyfollow (`T15`) | `4A` (44–84) | `i = 64 + T15[v]/10` (exact) |
| `53` | Resonance | `4D` | identical |
| `54` | Resonance Velocity Sens (%) | `4E` | as P-Env Velocity Sens |
| `55` | F-Env Depth (0–126) | `4F` | `i = v + 1` |
| `56` | F-Env Velocity Curve (0–6 = 1–7) | `50` (FIXED, 1–7) | `i = v + 1` |
| `57` | F-Env Velocity Sens (%) | `51` | as P-Env Velocity Sens |
| `58`, `59` | F-Env Velocity Time 1/4 (`T14`) | `52`, `53` | as P-Env Velocity Time |
| `5A` | F-Env Time Keyfollow (`T14`) | `54` | `i = 64 + T14[v]/10` (exact) |
| `5B`–`5E` | F-Env Time 1–4 | `55`–`58` | identical |
| `5F`–`62` | F-Env Level 1–4 | `5A`–`5D` | identical; `59` Level 0 = 0 (Roland's mapping, [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)) |
| — | (no JV cutoff-velocity control apart from the env) | `4B`, `4C` Cutoff Velocity Curve/Sens | default (sens 64 = 0) |
| `63`, `64` | Filter LFO1/2 Depth | `78`, `01 06` | `i = v + 1` |
| `65` | Tone Level | `00` Partial Level | identical |
| `66` | Bias Direction | `60` | identical |
| `67` | Bias Position | `5F` | identical |
| `68` | Bias Level (`T14`) | `5E` (54–74) | `i = 64 + T14[v]/10` (exact) |
| `69` | Level Env Velocity Curve (0–6) | `61` (FIXED, 1–7) | `i = v + 1` |
| `6A` | Level Env Velocity Sens (%) | `62` | as P-Env Velocity Sens |
| `6B`, `6C` | Level Env Velocity Time 1/4 (`T14`) | `63`, `64` | as P-Env Velocity Time |
| `6D` | Level Env Time Keyfollow (`T14`) | `65` | `i = 64 + T14[v]/10` (exact) |
| `6E`–`71` | Level Env Time 1–4 | `66`–`69` | identical |
| `72`–`74` | Level Env Level 1–3 | `6A`–`6C` | identical |
| `75`, `76` | Amp (Level) LFO1/2 Depth | `79`, `01 07` | `i = v + 1` |
| `77` | Tone Pan | `04` | identical |
| `78` | Pan Keyfollow (`T14`) | `05` (54–74) | `i = 64 + T14[v]/10` (exact) |
| `79` | Random Pan Depth (0–63) | `06` | identical |
| `7A` | Alternate Pan Depth (1–127) | `07` | identical |
| `7B`, `7C` | Pan LFO1/2 Depth | `7A`, `01 08` | `i = v + 1` |
| `7D` | Output Assign (MIX, EFX, DIR/OUT 1–2) | — | **none**: on the Integra every partial goes through the tone MFX, so MIX (EFX bypassed) and direct outs are lost |
| `7E` | Mix/EFX Send Level | `0C` Partial Output Level | identical value |
| `7F` | Chorus Send Level | `0F` | identical |
| `01 00` | Reverb Send Level | `10` | identical |

Integra Partial fields with no JV source, and their defaults: Env Mode = SUSTAIN (JV Patch Tones
have no envelope mode; only Rhythm Notes do), Wave Tempo Sync OFF, LFO Rate Detune 0, LFO Delay
Time Keyfollow 64, LFO Step sequencer unused.

### The controller transform

A JV Tone carries its **own** destinations and depths for Controllers 1–3. On the Integra,
Matrix Control *c* owns one source and four (destination, sens) slots for the whole tone, and
each partial picks per slot OFF, ON or REVERSE (`17`–`26`). A JV patch maps exactly when, for each
controller, the distinct `(destination, |depth|)` pairs across active tones fit the slots: ON for
`+depth`, REVERSE for `−depth`, OFF where a tone lacks the pair. Matrix Control 4 is spare (the JV
has 3 controllers) and can take 4 more slots for any one source.

Corpus fit, over the 2,626 real distinct patches:

| Result | Patches |
|---|---|
| Fits directly (≤ 4 pairs per controller) | 2,107 (80 %) |
| Fits with Matrix Control 4 borrowed | 384 (15 %) |
| Overflows; some pairs must be dropped and reported | 135 (5 %) |

In about half the patches, the active tones disagree on their destinations or depths for some
controller (1,337 for Controller 1), so the converter cannot simply copy Tone 1's settings.

Roland's own conversions pack differently: tone by tone, greedily, with Matrix Controls handed
out in order of first use and shared by controllers with the same source, so Matrix Control 4 is
not a reserved spare. That rule drops routes in 133 of these Patches, and the converter follows it
([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md), section b).

## Corpus survey (`~/Downloads/JV-1080_SP/`)

Containers: 204 `.SYX`, 275 `.MID` (the map's counts are right). The folder also holds 46 other
files (`.SVQ` ×20, `.WRK` ×10, `.LIB` ×6, `.INS` ×3, `.SVD` ×2, `.ARR`, `.CAL`, `.HQX`, `.SV`,
`.SVC`) that were not parsed.

**SysEx messages, by Model ID**

| Manufacturer / Model ID | Messages | Files |
|---|---|---|
| Roland `41`, Model `6A`, DT1 (`12`) | 29,969 | 292 |
| Roland `41`, Model `6A`, RQ1 (`11`) | 0 | 0 |
| Roland, any other Model ID (`00 10` XV, `42` GS, `46` JV-880, …) | **0** | 0 |
| Universal Non-Realtime `7E 7F 09 02` (GM System Off) | 8 | 8 `.MID` |
| Malformed (`F0 1C …` with 8-bit data) | 1 | `64PIANO.SYX` |

Every Roland message uses device ID `10`. 187 `.MID` files contain no SysEx.

**Model-6A dumps by unit**

| Unit | Count | By model fingerprint |
|---|---|---|
| **Patch** (Common + 4 Tones) | 4,623 total, **2,627 distinct** (2,626 real + 1 all-spaces placeholder in `SIGMAPP.SYX`), in 251 files (178 `.SYX`, 73 `.MID`) | `0x48` JV-1080: 2,102 distinct / 152 files · `0x49` XP-50/60/80: 397 / 31 files · `0x4A` JV-2080 family: 128 / 1 file (`2080_PTC.MID`) |
| Patch by area (counts include the 9 CR-LF-damaged ones) | User 3,551 · Patch-mode Temporary 142 · Performance-mode Temporary (per Part) 939 | 864 of the 939 Performance-mode ones are song set-ups inside `.MID` files |
| **Rhythm Setup** (Common `0x0C` + Notes `0x3A`) | 74 Rhythm Commons + 4,733 Rhythm Notes, in 69 files (60 `.MID`, 9 `.SYX`) | Rhythm Note is `0x3A` on every model, so it gives no fingerprint |
| **Performance** | 102 Performance Commons, in 71 files | `0x40` JV-1080: 38 · `0x42` XP/JV-1010: 64 · plus 5 name-only writes |
| **System** | 3 full System Commons (`0x52`, in 3 `.MID`, model unconfirmed), plus 1-byte System edits and Scale Tunes in 21 files | — |

**Range check.** Every byte of the 2,626 real distinct Patches is within the documented JV
range. This checked 73 Common fields, including the nibblized tempo, and 130 Tone fields. The
one failure is the all-`0x20` placeholder. Values in use:

| Field | Observed (active tones) |
|---|---|
| Wave Group (Type, ID) | INT-A 3,752 · INT-B 3,143 · EXP (SR-JV80) 1/2/4/5/6: 30/72/118/96/78 · PCM card 1/3: 3/36 · PCM ID 0 (invalid): 2 (`CHRONO3.MID`) |
| Highest Wave Number | INT-A 255, INT-B 193: matches the 448-Wave ROM (P1 pp. 122–123) |
| Delay Mode | NORMAL 7,170 · HOLD 38 · KEY-INTERVAL/PLAYMATE 12 · CLOCK-SYNC 61 · KEY-OFF-N 44 · KEY-OFF-D 5 · TAP/TEMPO-SYNC 0 |
| LFO External Sync used (LFO1 or LFO2) | 389 of 7,330 tones |
| Tone Output Assign | MIX 4,785 · EFX 2,545 · DIR 0 |
| Pan Control Switch | CONTINUOUS 7,260 · OFF 62 · KEY-ON 8 |
| Velocity Cross Fade ≠ 0 | 76 tones |
| Hold/Peak used | 18 patches |
| Clock Source = SEQUENCER/SYSTEM | 8 patches |

**Damaged files.** 9 `.SYX` files (`50S_SF`, `DIGCHR`, `FUNKANLG`, `LFOKEYS`, `METROPLS`,
`SK20BS`, `SQRDRPS`, `SUBBS`, `THERMIN`) have messages one byte too long with bad checksums. Each
data `0A` became `0D 0A`: the file went through a text-mode (CR-LF) transfer. A reader must
reject messages whose checksum fails. It could repair the damage by dropping a `0D` before `0A`
when that makes the length and checksum right; whether to do that is a reader-design call
([#198](https://github.com/sigma/integral/issues/198)).

No XV-series (`00 10`) data is in this folder.

## Open doubts

1. **Velocity-sensitivity rescale.** *Settled.* The JV gives `−100…+150 %` (raw `2v − 100`) and
   the `T14` time sensitivities in %. The Integra gives ±63 with no unit. Roland publishes no
   correspondence, but its paired JV → XV conversions show one: Velocity Sens maps ±100 % to ±32
   and stretches +100…+150 % over +32…+63 by a table; Velocity Time Sens is
   `64 + trunc(pct × 63/100)` ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md), section a). The
   earlier candidate `64 + clamp(round(pct × 63/100))` would make every sensitivity about twice as
   strong.
2. **LFO "SAW" polarity** is not stated in P1–P6. *Settled:* Roland converts it to SAW-UP
   ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)).
3. **Envelope start levels.** The JV Pitch and TVF envelopes have no Level 0. *Settled:* Roland
   converts them to Pitch L0 = 64 and TVF L0 = 0 ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)).
4. **KEY-INTERVAL vs PLAYMATE**: likely the same mode renamed; unconfirmed. It has no Integra
   target either way; Roland converts it to NORMAL with Delay Time 0.
5. **Volume Control Switch → Receive Expression**: *settled*. Roland doesn't carry it: Receive
   Expression stays ON ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)).
6. **Integra control-source index tails.** `06-pcm-synth-tone.md` gives Matrix Source as 0–109 and
   MFX Control Source as 0–101. The listed names count only to 108 and 100. *Settled:* the lists
   are indexed by CC number, and index 32 (CC32) is a hole, so BEND = 96, AFT = 97, CTRL1–4 /
   SYS1–4 = 98–101 ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md), section b).
7. **XP-50 Performance Common size** and the `0x52` System Common seen in 3 `.MID` files were not
   pinned to a model.
8. **Patch Category → Tone Category**: *settled*. Roland keeps the index as the XV Patch Category
   ([`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md)); only 128 corpus patches carry a category.

## Confidence

- **High**: framing, addresses, block lengths, the per-model fingerprint, and the identical and
  offset mappings. Three scanned manuals and two text manuals agree, and 2,626 real patches check
  out.
- **High** since [`jv-xv-roland-pairs.md`](jv-xv-roland-pairs.md): the controller transform, the
  Cross Fade mapping and the velocity-sensitivity formulas, all read from Roland's own paired
  conversions.
