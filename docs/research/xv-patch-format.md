# XV-series Patch SysEx Format vs the PCM Synth Tone (research for #193)

Answers [#193](https://github.com/sigma/integral/issues/193) (parent map
[#192](https://github.com/sigma/integral/issues/192)). Covers the SysEx layout of an
XV-5080 / XV-3080 / XV-5050 / XV-2020 / XV-88 **Patch**, and how each of its parameters
lands on the INTEGRA-7 PCM Synth tone
([`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md)). The full
parameter-by-parameter table is in the sibling
[`xv-patch-format.csv`](xv-patch-format.csv) (one row per XV parameter, plus the
Integra-only parameters).

## TL;DR

- **The INTEGRA-7 PCM Synth tone is the XV Patch with a few fields renamed to reserves.**
  Patch Common, Patch Common MFX, Patch TMT and Patch Tone 1–4 sit at the same offsets
  (`00 00 00`, `00 02 00`, `00 10 00`, `00 20 00`–`00 26 00`) and have the same bit widths
  and value ranges. Of their 293 parameters, **278 copy byte for byte**, 4 need
  remapping, and 11 have no Integra equivalent (they become Integra reserves).
- Several Integra "reserves" that carry data in saved files are just XV parameters:
  Common `00 0C` = Patch Category, `00 1F` = Patch Tempo (120 seen), `00 27` = Patch
  Output Assign (13 = TONE), MFX `00 01` = MFX Dry Send (127), and Partial `00 0D`/`00 0E`/`00 11`
  = the MFX-path Chorus/Reverb sends and the Tone Output Assign.
- **The only real conversion work** is: Wave Group Type/ID (SR-JV80 and SAMPLE groups),
  MFX Type numbering ([#196](https://github.com/sigma/integral/issues/196)), the Patch-level
  Chorus and Reverb blocks (they have no home in a PCM Synth tone), Patch Category →
  Tone Category, and filling the Integra-only fields (Common 2, LFO step sequencer).
- **Waves:** the XV INT Wave table is the Integra INT table. Wave Numbers 1–1083 name the same
  Waves. 15 names differ, and only because "Rhodes" became "EP" (`docs/waves/int.json`).
- **Framing:** Model ID `00 10` (two bytes), DT1 `F0 41 dev 00 10 12 aa bb cc dd … sum F7`.
  A Patch dump is **9 DT1 messages, one per block, 1056 bytes in total**.
- **The source model cannot be identified from a Patch dump.** All five XV models use Model ID
  `00 10` and the same Patch layout. They differ only in which effect type values are legal,
  and only the XV-5080 has MULTI-PARTIAL Patches and the SAMPLE group. A reader can only say
  "XV family". Some values narrow the candidate models (see [Model identification](#4-identifying-the-source-model)).
- **MULTI-PARTIAL Patches (XV-5080 only) cannot convert.** They play user samples from RAM.

## Sources (highest trust first)

| # | Source | What it gives |
|---|--------|---------------|
| P1 | Roland *XV-5080 Owner's Manual* ([PDF](https://static.roland.com/assets/media/pdf/XV-5080_OM.pdf), [roland.com](https://www.roland.com/us/support/by_product/xv-5080/owners_manuals/)), MIDI Implementation pp. 282–304 | The SysEx format (p. 288), Parameter Address Map (pp. 290–297), the checksum example (p. 303). Also the effect routing (pp. 24–27), Patch categories (p. 20), Multi-Partial Patches (p. 126), Wave Group (p. 136), Data Transfer (pp. 203–204) and the Waveform List (pp. 277–281). |
| P2 | Roland *XV-3080 Owner's Manual* ([PDF](https://static.roland.com/assets/media/pdf/XV-3080_OM.pdf)), MIDI Implementation, PDF pp. 178–187 | Model ID, address map, Patch blocks. |
| P3 | Roland *XV-5050 Owner's Manual* ([PDF](https://static.roland.com/assets/media/pdf/XV-5050_OM.pdf)), PDF pp. 159–168 | Same. |
| P4 | Roland *XV-2020 Owner's Manual* ([PDF](https://static.roland.com/assets/media/pdf/XV-2020_OM.pdf)), PDF pp. 143–152 | Same. |
| P5 | Roland *XV-88 Owner's Manual* ([PDF](https://static.roland.com/assets/media/pdf/XV-88_OM.pdf)), PDF pp. 275–286 | Same. Marks the XV-5080-only fields with `<XV-5080>`. |
| P6 | [`docs/midi/06-pcm-synth-tone.md`](../midi/06-pcm-synth-tone.md), [`docs/svd/06-pcm-synth.md`](../svd/06-pcm-synth.md#undocumented-and-reserve-fields-with-content) | The Integra side, and the values observed in Integra reserves. |
| P7 | [`docs/waves/int.json`](../waves/int.json), [`docs/research/wave-names.md`](wave-names.md) | Integra INT Wave names; Wave Group Type/ID semantics on the Integra. |
| S1 | JSynthLib XV-5080 driver, [`RolandXV5080PatchDriver.java` @ 772349b](https://github.com/jpcaruana/jsynthlib/blob/772349b2324c57154b59a42bd2e68a2cdf5e10fc/src/main/java/org/jsynthlib/drivers/roland/xv5080/RolandXV5080PatchDriver.java#L19-L22) | Secondary. Confirms the 9-message, 1056-byte Patch dump against real hardware. |

The manuals were compared by machine. Each model's Patch tables were extracted from the PDF
text and diffed per (block, offset) against the XV-5080, then against `06-pcm-synth-tone.md`.
The scratch scripts were not kept.

## 1. Message framing

From P1 p. 288 (identical in P2–P5):

```
DT1:  F0 41 dev 00 10 12 aa bb cc dd <data…> sum F7
RQ1:  F0 41 dev 00 10 11 aa bb cc dd ss tt uu vv sum F7
```

- `dev` is the device ID, `00`–`1F`, default `10`. A reader must ignore it.
- Model ID is the two bytes **`00 10`**. The JV-1080 uses the single byte `6A`, the Integra uses
  `00 00 64`.
- The checksum is the same Roland sum as on the Integra: address and data bytes, then
  `(128 - sum % 128) % 128`. The worked example is on p. 303.
- Addresses and sizes are 7-bit per byte. A size of `00 00 01 11` is 145 bytes.
- "Data larger than 256 bytes will be divided into packets of 256 bytes or less […] at an
  interval of about 20 ms" (p. 288). No Patch block exceeds 145 bytes, so **each block is
  one DT1**.
- Values marked `#` are nibblized, high nibble first: Patch Tempo, Delay Time, LFO Rate,
  Wave Group ID, Wave Numbers, and the MFX/Chorus/Reverb parameters. They use the same
  encoding and the same offset bias as the Integra (MFX parameters are `12768`–`52768` for
  `-20000`–`+20000`).

### Patch blocks

| Offset | Block | Size (hex) | Bytes | DT1 length | Integra block at the same offset |
|---|---|---|---|---|---|
| `00 00 00` | Patch Common | `00 00 00 4F` | 79 | 91 | PCM Synth Tone Common (`00 50`, adds one reserve at `00 4F`) |
| `00 02 00` | Patch Common MFX | `00 00 01 11` | 145 | 157 | PCM Synth Tone Common MFX (same size) |
| `00 04 00` | Patch Common Chorus | `00 00 00 34` | 52 | 64 | — (unused on the Integra) |
| `00 06 00` | Patch Common Reverb | `00 00 00 53` | 83 | 95 | — (unused on the Integra) |
| `00 10 00` | Patch TMT (Tone Mix Table) | `00 00 00 29` | 41 | 53 | PCM Synth Tone PMT (same size) |
| `00 20 00`, `00 22 00`, `00 24 00`, `00 26 00` | Patch Tone 1–4 | `00 00 01 09` | 137 | 149 | PCM Synth Tone Partial 1–4 (`01 1A`, adds the LFO step sequencer at `01 09`–`01 19`) |
| `00 30 00` + (key−21)×`00 00 20`, keys 21–108 | Patch Split Key (XV-5080 only) | `00 00 00 0C` | 12 | 24 | **Collides** with PCM Synth Tone Common 2 at `00 30 00` |

A 4-tone Patch dump is the first 9 blocks: 948 data bytes + 9 × 12 header bytes = **1056
bytes**. JSynthLib (S1) uses exactly these 9 message sizes (91, 157, 64, 95, 53, 149 × 4).

**Trap:** an XV-5080 block at offset `00 30 00` is a Split Key (12 bytes), not an Integra
Common 2 (60 bytes). A reader that maps offsets straight through must ignore the Split Key
blocks. In practice it should reject any Patch whose Tone Type is MULTI-PARTIAL.

### User and Temporary areas

Top-level map (P1 p. 290; P2 p. 181, P3 p. 162, P4 p. 146, P5 p. 279):

| Address | Area |
|---|---|
| `1F 00 00 00` | Temporary Patch (Patch mode) |
| `11 00 00 00` + (n−1) × `00 20 00 00` | Temporary Patch of Performance part n: parts 1–32 on the XV-5080, 1–16 on the others. 7-bit carry, so part 5 is `12 00 00 00`. Part 32 is `18 60 00 00`. |
| same + `00 10 00 00` | Temporary Rhythm of that part (out of scope) |
| `30 00 00 00` + (n−1) × `00 01 00 00` | User Patch 001–128 (all five models) |
| `40 00 00 00` + (n−1) × `00 10 00 00` | User Rhythm 1–4 (out of scope) |
| `20 00 00 00` + (n−1) × `00 01 00 00` | User Performance 01–64 (out of scope) |

Every Patch block offset has a first byte of `00`. So, for any Patch DT1:

- address bytes 0–1 name the Patch slot (`30 nn` for User Patch nn+1, `1F 00` for the Patch-mode
  temp, `11 00`…`18 60` for Performance part temps);
- byte 2 names the block (`00`, `02`, `04`, `06`, `10`, `20`, `22`, `24`, `26`, or `30`–`45`
  for Split Keys);
- byte 3 plus the data length gives the bytes inside the block.

To group a stream of DT1s into Patches, group by bytes 0–1. A new Patch starts when that pair
changes, or when the Common block arrives again. A dump of the User bank holds 128 such groups.
The device's Data Transfer (P1 pp. 203–204) can send USER 001–128 or TEMP.

The XV-5050 and XV-2020 also have a `01 00 00 00` Setup block, and put System at
`02 00 00 00` rather than `00 00 00 00`. This only matters for System dumps.

## 2. Parameter mapping, XV → Integra

Status values in the CSV:

- **identical**: same offset, bit width and range. Copy the byte (or nibbles).
- **rescaled**: the value needs a mapping, given below.
- **no equivalent**: dropped. Report it in the Conversion report only when it differs from
  the neutral value (see the notes).

Per block:

| XV block | Params | Identical | Rescaled | No equivalent |
|---|---|---|---|---|
| Patch Common | 78 | 72 | 1 (Category) | 5 |
| Patch Common MFX | 49 | 46 | 1 (MFX Type) | 2 |
| Patch Common Chorus | 16 | 0 | 0 | 16 |
| Patch Common Reverb | 23 | 0 | 0 | 23 |
| Patch TMT | 41 | 41 | 0 | 0 |
| Patch Tone (×4) | 125 | 119 | 2 (Wave Group Type/ID) | 4 |
| Patch Split Key (×88, XV-5080) | 9 | 0 | 0 | 9 |

"Identical" covers some values that only line up on the Integra side:

- TMT Velocity Control: XV 0–2 (OFF, ON, RANDOM) is a subset of Integra 0–3 (adds CYCLE).
- LFO1/LFO2 Waveform: XV 0–10 is a subset of Integra 0–12 (adds VSIN, STEP).
- Matrix Control Source and MFX Control Source: XV `SYS1`–`SYS4` (98–101) land on Integra
  `CTRL1`–`CTRL4` (98–101). Both are indirections to controller numbers set outside the
  tone (XV System Control 1–4, Integra Studio Set Tone Control 1–4). The value copies, but the
  physical controller behind it depends on the target's setup.
- Matrix Control Destination: the range is `0`–`33` on both. XV values 30–33 are `MFX1`–`MFX4`.
  The Integra table names only `OFF`…`FXM` (0–29) and leaves 30–33 unnamed. Treat 30–33 as
  "no equivalent" until a device check (see open doubts).
- MFX Parameters 1–32 and MFX Control Assign 1–4: the layout copies, but their meaning depends
  on MFX Type. That is [#196](https://github.com/sigma/integral/issues/196)'s job.

### Every non-identical XV parameter

| XV block / offset | XV parameter (range) | Integra at that offset | Status | Conversion |
|---|---|---|---|---|
| Common `00 0C` | Patch Category (0–127) | reserve | rescaled | Look up Integra Tone Category (Common 2 `00 10`) in the [category table](#6-patch-category--tone-category). Also copy the raw byte into reserve `00 0C`, where Integra files keep it. |
| Common `00 0D` | Tone Type (4TONES, MULTI-PARTIAL) | reserve | no equivalent | 4TONES: convert. MULTI-PARTIAL: **reject the Patch**, because its sound is RAM samples (P1 p. 126). |
| Common `00 1E` | Patch Clock Source (PATCH, SYSTEM) | reserve | no equivalent | The Integra follows the Studio Set tempo. Report it when PATCH and the tone uses tempo sync (Wave Tempo Sync, LFO Rate or Delay Time ≥ 128 = note values). |
| Common `00 1F` # | Patch Tempo (20–250) | reserve (8-bit) | no equivalent | Copy verbatim. The Integra reserve holds the same value: 120 and 100 are seen in Integra files. |
| Common `00 21` | One Shot Mode (OFF, ON) | reserve | no equivalent | Report it when ON. |
| Common `00 27` | Patch Output Assign (MFX, A–D, 1–8, TONE) | reserve | no equivalent | Write 13 (TONE), the value Integra files always carry. |
| MFX `00 00` | MFX Type (0–127; 90 types on XV-5080/5050, 63 on XV-3080/88, 40 on XV-2020) | MFX Type (0–67) | rescaled | Type table from [#196](https://github.com/sigma/integral/issues/196). |
| MFX `00 01` | MFX Dry Send Level | reserve | no equivalent | Write 127, as Integra files do. Report it when the XV value is not 127. |
| MFX `00 04` | MFX Output Assign (A–D) | reserve | no equivalent | Drop. These are physical output jacks. |
| Chorus `00 00`–`00 33` | Chorus Type (OFF, CHORUS, DELAY; +GM2 CHORUS on XV-5050), Level, Output Assign, Output Select, Parameters 1–12 | — | no equivalent | A PCM Synth tone has no Chorus or Reverb. Where they land (Part sends, Studio Set effects, or dropped) is [#196](https://github.com/sigma/integral/issues/196)'s call. |
| Reverb `00 00`–`00 52` | Reverb Type (OFF, REVERB, SRV ROOM/HALL/PLATE; +GM2 REVERB on XV-5050), Level, Output Assign, Parameters 1–20 | — | no equivalent | As Chorus. |
| Tone `00 0D` | Tone Chorus Send Level (MFX) | reserve | no equivalent | See below. |
| Tone `00 0E` | Tone Reverb Send Level (MFX) | reserve | no equivalent | See below. |
| Tone `00 0F` | Tone Chorus Send Level (non MFX) | Partial Chorus Send Level | identical* | See below. |
| Tone `00 10` | Tone Reverb Send Level (non MFX) | Partial Reverb Send Level | identical* | See below. |
| Tone `00 11` | Tone Output Assign (MFX, A–D, 1–8) | reserve | no equivalent | Drop, but read it first to choose the send pair (below). |
| Tone `00 15` | Tone Receive Pan Mode (CONTINUOUS, KEY-ON) | reserve | no equivalent | Report it when KEY-ON. |
| Tone `00 27` | Wave Group Type (INT-WAVE, SRJV80-EXP, SRX-EXP, SAMPLE) | Wave Group Type (INT, SRX, —, —) | rescaled | `0 → 0`; `2 → 1`; `1` (SR-JV80) → Wave mapping from [#195](https://github.com/sigma/integral/issues/195), else OFF; `3` (SAMPLE, XV-5080 only) → OFF. |
| Tone `00 28` # | Wave Group ID (0–16384) | Wave Group ID | rescaled | INT: write 0 (Integra convention, [wave-names.md](wave-names.md)). SRX: the board number 1–12, *assumed* to be the same on the XV (unverified). SR-JV80: from #195. |
| Split Key (all) | Partial Number, Assign Type, Mute Group, sends, Output Assign | — (Common 2 sits here) | no equivalent | Only present in MULTI-PARTIAL Patches, which are rejected. |

\* **The two pairs of sends.** XV Tones store two Chorus/Reverb send pairs. The "(MFX)" pair
is used when Tone Output Assign = MFX; the "(non MFX)" pair is used when it is A–D or 1–8.
The panel shows one "Chorus Send Level" and switches between them (P1 pp. 24–26). The
Integra kept the "(non MFX)" offsets as its Partial sends and turned the "(MFX)" offsets into
reserves, which still hold 53, 60, 72 or 127 in real Integra tones. A byte-for-byte copy is
therefore the documented, offset-faithful mapping. A *sound*-faithful reader would instead take
the pair that the XV Output Assign selected. **Recommendation:** when Tone Output Assign = MFX
(the common case), write the "(MFX)" pair into Integra `00 0F`/`00 10`, keep both originals in
the reserves, and report it if the pairs differ. Settle this with a device check (open doubt 2).

## 3. Integra-only parameters (reverse direction)

An XV Patch has nothing for these. The reader must fill them:

| Integra block / offset | Parameter | Fill with |
|---|---|---|
| Common `00 4F` | reserve | 0 |
| Common 2 `00 00`–`00 0F` | 16-character long name (reserve) | The 12-character XV name, space-padded |
| Common 2 `00 10` | Tone Category | From XV Patch Category ([table](#6-patch-category--tone-category)) |
| Common 2 `00 11` # | Tone Number | 0 |
| Common 2 `00 13` | Phrase Octave Shift | 64 (0) |
| Common 2 `00 33` | TFX Switch | 0 (open: check what Integra INIT uses) |
| Common 2 `00 38` # | Phrase Number | 0 |
| Partial `01 09` | LFO Step Type | 0 |
| Partial `01 0A`–`01 19` | LFO Step 1–16 | 64 (0) |

Integra-only *values* inside shared parameters cannot come from an XV Patch: PMT Velocity
Control CYCLE, LFO waveforms VSIN and STEP, and SRX Wave Groups the XV never had (none: the
SRX-01..12 boards are the same).

## 4. Identifying the source model

Every XV model's DT1 carries Model ID `00 10` (P1 p. 288; P2–P5, "The model ID of the
exclusive messages used by this instrument is 00H 10H"). The Patch address map, block
sizes, and every Patch parameter's offset, bit width and range are **identical across all
five models**. The machine diff found differences only in:

| Parameter | XV-5080 | XV-3080 | XV-5050 | XV-2020 | XV-88 |
|---|---|---|---|---|---|
| MFX types | 90 | 63 | 90 | 40 (range `0–40`) | 63 |
| Chorus Type | 0–2 | 0–2 | 0–3 (+GM2 CHORUS) | 0–1 (OFF, CHORUS) | 0–2 |
| Reverb Type | 0–4 | 0–4 | 0–5 (+GM2 REVERB) | 0–1 (OFF, REVERB) | 0–4 |
| Tone Type MULTI-PARTIAL, Wave Group SAMPLE, Split Key | yes | marked `<XV-5080>` | marked `<*>` (ignored) | marked `<*>` | marked `<XV-5080>` |
| SR-JV80 Wave Group | yes | yes | no slots | no slots | yes |
| Performance parts | 32 | 16 | 16 | 16 | 16 |

So from the bytes alone a reader can say **"XV family"** (`F0 41 dev 00 10 12` followed by
Patch block addresses). Some values only narrow the candidates: Tone Type MULTI-PARTIAL, SAMPLE
waves or Split Key blocks mean XV-5080; Chorus Type 3 or Reverb Type 5 mean XV-5050; an MFX Type
of 63 or more means XV-5080 or XV-5050. Because the Patch format is shared, conversion does
not need the exact model. Only the MFX Type numbering might, if the models number their
types differently (#196 should check).

Whether other XV-engine products (e.g. the 2001 Fantom) also use `00 10` was not checked. The
RD-700 does not: its manual gives Model ID `00 43`. A reader should accept `00 10` and then
check that the addresses fit the XV Patch map.

## 5. Waves

- P1's Waveform List (pp. 277–281) has 1083 entries. Compared by number against
  `docs/waves/int.json`, **all 1083 Wave Numbers match**. Only 15 names differ, because
  "Rhodes"/"Rhd" was renamed "EP": Wave Numbers 28–42 (`SA Rhodes 1A` → `SA EP 1 A`,
  `Dyn Rhd mp A` → `Dyn EP mp A`, …).
- The XV-3080, XV-5050, XV-2020 and XV-88 lists contain every `int.json` name apart from
  the same 15. Those lists were checked by name presence, not by position.
- So an XV `INT-WAVE` Wave Number copies unchanged to Integra INT. This confirms the map's
  standing decision.
- SRX boards are the same products on both (SRX-01..12). The XV manuals do not document the
  Wave Group ID values (board number, slot, or something else). See open doubt 3.

## 6. Patch Category → Tone Category

P1 p. 20 lists 38 categories plus NO ASSIGN. The MIDI Implementation gives only the range
`0–127`. Numbering them in list order (0 = NO ASSIGN) is **inferred**. Three Integra tones
carry this legacy byte in their reserve `00 0C` (P6), and their values fit the inferred order:
Full Stops 6 = ORG, Retro Lead 22 = HLD, D-50 Stack 4 = BEL.

| XV | Abbr | Integra Tone Category | | XV | Abbr | Integra Tone Category |
|---|---|---|---|---|---|---|
| 0 | — | 0 No assign | | 20 | SBR | 35 Synth Brass |
| 1 | PNO | 1 Ac.Piano | | 21 | SAX | 30 Sax |
| 2 | EP | 5 E.Piano | | 22 | HLD | 34 Synth Lead |
| 3 | KEY | 10 Other Keyboards | | 23 | SLD | 34 Synth Lead |
| 4 | BEL | 14 Bell/Mallet | | 24 | TEK | 40 Synth Seq/Pop (closest) |
| 5 | MLT | 14 Bell/Mallet | | 25 | PLS | 42 Pulsating |
| 6 | ORG | 6 Organ | | 26 | FX | 39 FX |
| 7 | ACD | 12 Accordion/Harmonica | | 27 | SYN | 38 Synth PolyKey |
| 8 | HRM | 12 Accordion/Harmonica | | 28 | BPD | 36 Synth Pad/Strings |
| 9 | AGT | 16 Ac.Guitar | | 29 | SPD | 36 Synth Pad/Strings |
| 10 | EGT | 17 E.Guitar | | 30 | VOX | 32 Vox/Choir |
| 11 | DGT | 18 Dist.Guitar | | 31 | PLK | 22 Plucked/Stroke |
| 12 | BS | 20 E.Bass (XV merges Ac/E bass) | | 32 | ETH | 22 Plucked/Stroke (closest) |
| 13 | SBS | 21 Synth Bass | | 33 | FRT | 22 Plucked/Stroke |
| 14 | STR | 24 Strings | | 34 | PRC | 47 Percussion |
| 15 | ORC | 24 Strings (closest) | | 35 | SFX | 45 Sound FX |
| 16 | HIT | 44 Hit | | 36 | BTS | 43 Beat&Groove |
| 17 | WND | 28 Wind | | 37 | DRM | 46 Drums |
| 18 | FLT | 29 Flute | | 38 | CMB | 48 Combination |
| 19 | BRS | 26 Brass | | | | |

Integra IDs come from `docs/sounds/categories.json`, via `web/src/categories.ts`. The
"closest" rows are judgement calls and are not information loss: the category only drives
browsing.

## Open doubts and device checks

1. **Category numbering** is inferred from list order and three data points. *Check:* one real
   XV dump per category, or the XV-5080 Patch List PDF next to a factory-bank dump.
2. **Which send pair the Integra uses.** The Integra's Partial Chorus/Reverb Send sit at the XV
   "(non MFX)" offsets, but every Integra partial appears to pass through the tone MFX. *Check:* on the
   Integra, set Partial `00 0D` to 0 and `00 0F` to 127 (and the reverse), and listen for
   chorus on a dry tone with MFX THRU.
3. **XV Wave Group ID values** are undocumented in all five manuals. Possibilities: INT = 0 or 1,
   SRX = board number (as on the Integra) or slot, SR-JV80 = board number 1–19. *Check:* a
   real XV-5080 dump of a Patch on an SRX and on an SR-JV80 board (corpus,
   [#197](https://github.com/sigma/integral/issues/197)).
4. **Matrix Control Destination 30–33** (XV MFX1–MFX4): the Integra's range covers them, but
   its documentation names nothing past FXM. *Check:* send 30 to an Integra Matrix destination
   and read it back / listen.
5. **MFX Type numbering across XV models** (63-type vs 90-type machines): is type n the same
   effect on both? Hand over to [#196](https://github.com/sigma/integral/issues/196).
6. **No real XV dump was checked.** The owner's corpus is JV-only: a scan of
   `~/Downloads/JV-1080_SP` found no `00 10` messages. The 1056-byte, 9-message framing is
   confirmed only by the manuals and by JSynthLib (S1).

## Confidence

- **High**: framing, Model ID, block offsets and sizes, addressing, per-parameter
  identity (machine-diffed against Roland's tables for all five models), the INT Wave
  identity for the XV-5080.
- **Medium**: the meaning of the Integra reserves (backed by the values observed in P6), and
  the INT Wave identity for the other four models (name presence only).
- **Low**: the category numbering, XV Wave Group ID values, and the send-pair choice (open doubts 1–3).
