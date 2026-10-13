# Studio Set Parameters Behind the Missing Part and Common Features

Research for [#278](https://github.com/sigma/integral/issues/278), part of the
[front-panel features map (#276)](https://github.com/sigma/integral/issues/276).
It covers gaps 1–7 and 9 on the map.

**Answer:** every parameter that gaps 1–7 need is documented, with an address
and a range. Core knows only their SVD bit widths. None is in `param_registry`,
`FieldSpec` or `device_spec`, none is parsed by `mixer_load`, and no
`integral-wasm` setter writes one. Gap 9 (Initialization) needs no
client-written defaults: Roland's editor sends an **undocumented RQ1 at
`0F 00 18 0T`** for Studio Set, Sound Ctrl, Part, Tone and Partial Init. Our
device has not been tested with it yet. Roland's editor also lists an `init`
value for every parameter. That list is a fallback source of defaults, and the
SVD INIT entries confirm it.

## Sources and trust levels

| Tag | Source | What it is |
|-----|--------|------------|
| **[MI]** | [`docs/midi/05-studio-set.md`](../midi/05-studio-set.md), [`docs/midi/04-address-map.md`](../midi/04-address-map.md) | Our extract of Roland's MIDI Implementation v1.00. Primary for addresses and ranges. |
| **[PG]** | [`docs/params/01-studio-set.md`](../params/01-studio-set.md), [`docs/manual/04-studio-sets.md`](../manual/04-studio-sets.md) | Our digest of the Parameter Guide and the Owner's Manual. Primary for display values and front-panel behaviour. |
| **[SVD]** | [`docs/svd/05-studio-set.md`](../svd/05-studio-set.md), [`docs/svd/10-system.md`](../svd/10-system.md) | Our decode of 1,482 Studio Set entries from the 18 official packs. "Always *n*" means every entry holds *n*. |
| **[RJS]** | Roland INTEGRA-7 Editor V2, installed at `/Library/Audio/Plug-Ins/VST3/Roland/INTEGRA7EditorV2.vst3/Contents/Resources/html/js/` (`Copyright 2014 Roland Corporation`). Files: `model/model_excmap.js` (`SC`, `FC`, `FM` and `FP` tables), `view/scene_base_midi_controller.js` (Init requests), `communicator/communicator.js` (RQ1 framing), `view/common.js` (dialog handlers). | Roland's own editor code. Primary for undocumented commands, reserve names and `init` values. Not yet checked on our device. |
| **[CODE]** | `crates/integral-core/src/{param_registry,field_spec,device_spec,params,mixer_load,svd_specs}.rs`, `state/parse.rs`, `crates/integral-wasm/src/lib.rs`, `web/src/{integra,useMixer}.ts` at `main` (`3b01efb3`) | What the code models today. |

## How to read the tables

- **Offset** is relative to the block. Part *p* (0-based) is at
  `18 00 (20+p) <offset>` [MI §7]. MIDI channel *c* (0-based) is at
  `18 00 (10+c) 00` [MI §6]. Common is at `18 00 00 <offset>` [MI §1]. System
  Common is at `02 00 00 <offset>` [MI 04].
- **Raw / display** is the SysEx range and what the front panel shows.
  "−64..+63 @64" means raw 0–127 with 64 as zero. All fields are one 7-bit byte
  unless marked `#` (nibblized: 1 nibble per byte, MSB first).
- **Init** is the `init` value in [RJS] `model_excmap.js`. Where [SVD] shows
  one value in every entry, it agrees, unless a row says otherwise.
- **Reg** means "in `param_registry`, `FieldSpec` or `device_spec`". The
  answer is **no** for every row below. `param_registry::mixer_params` holds
  only the 7 mixer fields per part (level, pan, mute, sends, output, Rx
  channel) plus the Common/FX/EQ entries. `FieldSpec` covers tone blocks only.
  `device_spec` knows the Part block only as a 77-byte run
  (`USER_MEMORY`, `device_spec.rs:236`). `svd_specs.rs` (`SS_COMMON_PARAMS`,
  `SS_PART_PARAMS`, MIDI) knows every field's **bit width** for SVD
  conversion, but has no name, range or display.
- **mixer_load**: `MixerBlock::Part` reads Part `00 00`–`00 29`
  (`params::PART_MIXER_SIZE` = `0x2A`), and `state::parse::parse_part_dump`
  keeps only offsets `00`, `06`–`0A`, `25`, `27`–`29`. So a row is either
  **fetched, dropped** (inside `00`–`29`, not parsed) or **not read**.
  `mixer_load` reads no Common byte in `10`–`3E` and no MIDI block.
- **wasm**: no `integral-wasm` setter exists for any row. The only route is
  `IntegraDevice.sendRaw`. `web/src` refers to none of these parameters.

## 1. Part SCALE (+ System tuning)

| Param | Offset | Raw / display | Init | Reg | mixer_load | wasm |
|---|---|---|---|---|---|---|
| Part Scale Tune Type | Part `2B` | 0–8: CUSTOM, EQUAL, JUST-MAJ, JUST-MIN, PYTHAGORE, KIRNBERGE, MEANTONE, WERCKMEIS, ARABIC | 1 (EQUAL) | no | not read | no |
| Part Scale Tune Key | Part `2C` | 0–11: C … B | 0 (C) | no | not read | no |
| Part Scale Tune for C … B | Part `2D`–`38` (12 bytes) | −64..+63 cents @64 | 64 | no | not read | no |
| Master Tune `#` | System `00`–`03` (4 nibbles) | 24–2024 = −100.0..+100.0 cents, 1024 = 0 (440 Hz) | 1024 | no | not read | no |
| Master Key Shift | System `04` | 40–88 = −24..+24 @64 | 64 | no | not read | no |
| Scale Tune Switch | System `06` | 0–1 OFF/ON | 1 | no | not read | no |

Notes:

- The Type is a template that "sets all Scale Tune C–B settings" [PG]. Whether
  the device rewrites `2D`–`38` when Type changes, and announces it, is
  untested. A UI should re-read `2B`–`38` after writing Type.
- Availability by tone type [PG]: not on SN-A Organ or SN-D.
- System `12`–`1F` are "Live Set Scale Tune Type/Key/C…B" in [RJS] (init 1,
  0, 64 × 12). [SVD] shows exactly those values (`00 12` = 1, `00 14`–`00 1F`
  = 64). They are not part of the per-Part scale. Leave them alone.

## 2. Part KBD (keyboard zones)

| Param | Offset | Raw / display | Init | Reg | mixer_load | wasm |
|---|---|---|---|---|---|---|
| Keyboard Range Lower | Part `1D` | 0–127 = C-1..UPPER | 0 | no | fetched, dropped | no |
| Keyboard Range Upper | Part `1E` | 0–127 = LOWER..G9 | 127 | no | fetched, dropped | no |
| Keyboard Fade Width Lower | Part `1F` | 0–127 | 0 | no | fetched, dropped | no |
| Keyboard Fade Width Upper | Part `20` | 0–127 | 0 | no | fetched, dropped | no |
| Velocity Range Lower | Part `21` | 1–127 = 1..UPPER | 1 | no | fetched, dropped | no |
| Velocity Range Upper | Part `22` | 0–127 = LOWER..127 ([RJS] min 1) | 127 | no | fetched, dropped | no |
| Velocity Fade Width Lower | Part `23` | 0–127 | 0 | no | fetched, dropped | no |
| Velocity Fade Width Upper | Part `24` | 0–127 | 0 | no | fetched, dropped | no |
| Part Velocity Sens Offset | Part `1C` | 1–127 = −63..+63 @64 | 64 | no | fetched, dropped | no |
| Part Mono/Poly | Part `0D` | 0–2: MONO, POLY, TONE | 2 | no | fetched, dropped | no |
| Part Legato Switch | Part `0E` | 0–2: OFF, ON, TONE | 2 | no | fetched, dropped | no |
| Velocity Curve Type | Part `43` | 0–4: OFF, 1, 2, 3, 4 | 0 | no | not read | no |

Notes:

- Lower ≤ Upper is a cross-field constraint ("Lower cannot exceed Upper and
  vice versa" [PG]). The device's behaviour on a violating write is untested.
- `velocity_curve.rs` models the PCM tone **V-Curve** (FIXED, 1–7). It is not
  this Part Velocity Curve Type (OFF, 1–4). Don't reuse it.
- Availability [PG]: velocity range/fade, Mono/Poly and Legato are absent on
  SN-A Organ, and Legato is absent on SN-A Other.

## 3. Part PITCH

| Param | Offset | Raw / display | Init | Reg | mixer_load | wasm |
|---|---|---|---|---|---|---|
| Part Octave Shift | Part `1B` | 61–67 = −3..+3 @64 | 64 | no | fetched, dropped | no |
| Part Coarse Tune (RPN 2) | Part `0B` | 16–112 = −48..+48 @64 | 64 | no | fetched, dropped | no |
| Part Fine Tune (RPN 1) | Part `0C` | 14–114 = −50..+50 @64 | 64 | no | fetched, dropped | no |
| Part Pitch Bend Range (RPN 0) | Part `0F` | 0–25 = 0..24, TONE (25) | 25 | no | fetched, dropped | no |
| Part Portamento Switch (CC65) | Part `10` | 0–2: OFF, ON, TONE | 2 | no | fetched, dropped | no |
| Part Portamento Time (CC5) `#` | Part `11`–`12` (2 nibbles) | 0–128 = 0..127, TONE (128) | 128 | no | fetched, dropped | no |

Notes:

- Portamento Time is the only multi-byte Part field. A 1-byte read or write
  at `11` alone is invalid. An RQ1 must not start or end inside it
  ([99-undocumented §2 Read Notes](../midi/99-undocumented.md#read-notes)).
- "TONE" is the top raw value, not a separate flag. A UI needs a sentinel
  option, not a plain number range.
- Availability [PG]: SN-D has none of these. PCM-D has only Bend Range. SN-A
  Organ has no portamento.

## 4. Part OFFSET (Sound Ctrl)

| Param | Offset | Raw / display | Init | Reg | mixer_load | wasm |
|---|---|---|---|---|---|---|
| Part Cutoff Offset (CC74) | Part `13` | −64..+63 @64 | 64 | no | fetched, dropped | no |
| Part Resonance Offset (CC71) | Part `14` | −64..+63 @64 | 64 | no | fetched, dropped | no |
| Part Attack Time Offset (CC73) | Part `15` | −64..+63 @64 | 64 | no | fetched, dropped | no |
| Part Decay Time Offset (CC75) | Part `16` | −64..+63 @64 | 64 | no | fetched, dropped | no |
| Part Release Time Offset (CC72) | Part `17` | −64..+63 @64 | 64 | no | fetched, dropped | no |
| Part Vibrato Rate (CC76) | Part `18` | −64..+63 @64 | 64 | no | fetched, dropped | no |
| Part Vibrato Depth (CC77) | Part `19` | −64..+63 @64 | 64 | no | fetched, dropped | no |
| Part Vibrato Delay (CC78) | Part `1A` | −64..+63 @64 | 64 | no | fetched, dropped | no |

Notes:

- Each one has a CC number. The device probably moves these bytes when it
  receives the CC on the Part's channel, without sending a DT1 (compare
  [99-undocumented §6](../midi/99-undocumented.md#6-temporary-tone-reloads)).
  This is untested, but it means a UI value can go stale.
- Availability [PG] differs by tone type: SN-A Piano/Organ have none, and SN-D
  has only attack, decay and release.

## 5. Part MIDI (Rx Switch and Rx filters)

| Param | Offset | Raw / display | Init | Reg | mixer_load | wasm |
|---|---|---|---|---|---|---|
| Receive Switch | Part `01` | 0–1 OFF/ON | 1 | no (only `params::part::RECEIVE_SWITCH`, unused) | fetched, dropped | no |
| Receive Program Change | Part `39` | 0–1 | 1 | no | not read | no |
| Receive Bank Select | Part `3A` | 0–1 | 1 | no | not read | no |
| Receive Pitch Bend | Part `3B` | 0–1 | 1 | no | not read | no |
| Receive Polyphonic Key Pressure | Part `3C` | 0–1 | 1 | no | not read | no |
| Receive Channel Pressure | Part `3D` | 0–1 | 1 | no | not read | no |
| Receive Modulation | Part `3E` | 0–1 | 1 | no | not read | no |
| Receive Volume | Part `3F` | 0–1 | 1 | no | not read | no |
| Receive Pan | Part `40` | 0–1 | 1 | no | not read | no |
| Receive Expression | Part `41` | 0–1 | 1 | no | not read | no |
| Receive Hold-1 | Part `42` | 0–1 | 1 | no | not read | no |

Note: [RJS] calls `01` "Part Switch" (`NEFP_RX_SW`). The front panel calls it
Rx Switch [PG].

## 6. Voice Reserve

| Param | Offset | Raw / display | Init | Reg | mixer_load | wasm |
|---|---|---|---|---|---|---|
| Voice Reserve 1–16 | Common `18`–`27` (one per Part) | 0–64 = 0..63, FULL (64) | 0 | no | not read | no |

Notes:

- The constraint "All parts cannot total more than 64" [PG] is a cross-field
  rule. What the device does with a write that breaks it (clamp, reject,
  accept) is untested.
- These bytes sit in Common, not in the Part block. A Part-settings surface
  must write Common `18 + p` for Part *p*.
- Every [SVD] entry holds 0 for all 16, so the factory sets reserve nothing.

## 7. Studio Set Common control

| Param | Offset | Raw / display | Init | Reg | mixer_load | wasm |
|---|---|---|---|---|---|---|
| Tone Control 1 Source | Common `39` | 0–97 (see quirk) | [RJS] 0; [SVD] always 1 (CC01) | no | not read | no |
| Tone Control 2 Source | Common `3A` | 0–97 | [RJS] 0; [SVD] always 97 (AFT) | no | not read | no |
| Tone Control 3 Source | Common `3B` | 0–97 | [RJS] 0; [SVD] always 2 (CC02) | no | not read | no |
| Tone Control 4 Source | Common `3C` | 0–97 | [RJS] 0; [SVD] always 4 (CC04) | no | not read | no |
| Studio Set Tempo `#` | Common `3D`–`3E` (2 nibbles) | 20–250 BPM | 120 | no | not read | no |
| Phase Lock (MIDI ch 1–16) | MIDI block `18 00 (10+c) 00` | 0–1 OFF/ON | 0 | no | not read | no |

Notes:

- **Init disagreement:** [RJS] gives Tone Control init 0 (OFF), but all 1,482
  [SVD] entries, INIT sets included, hold 1, 97, 2, 4. Trust [SVD] for "what
  a Studio Set Init produces" until a device check settles it.
- Tone Control sources only act when System `24` Control Source = STUDIO SET
  (raw 1, the default). Studio Set Tempo only acts when System `28` Tempo
  Assign Source = STUDIO SET (raw 1; default 0 = SYSTEM) [MI 04, PG]. Both
  switches are gap 8 (System page). A Common-control UI should show which
  source is active.
- Phase Lock is one 1-byte block per MIDI channel (16 RQ1s, or one RQ1 over
  `18 00 10 00`–`18 00 1F 00` if the reply comes as several DT1s; see the Read
  Notes). It is not available for SN-A organ [PG].

## 9. Initialization

**There is a device command.** Roland's editor sends an RQ1 to `0F 00 18 TT`
whose 4-byte size field carries a payload [RJS `scene_base_midi_controller.js`:
`REQ_INITIALIZE = 0x0F001800`, `INIT_FIXED_DATA = 0x7F7F0000`]. This is the
same "RQ1 with payload" pattern as the catalog query, the write request and
the expansion load (`0F 00 03 02`, `0F 00 10 00`, `0F 00 30 00`).

| Init | TT | Size field (payload) | Notes |
|---|---|---|---|
| Studio Set Init | `00` | `7F 7F 00 00` | whole Temporary Studio Set |
| Sound Ctrl Init | `08` | `7F 7F 00 00` | **no Part byte**, see open questions |
| Part Init | `10` | `7F 7F PP 00` | `PP` = Part, 0-based |
| Tone Init | `20` | `7F 7F PP 00` | Part's Temporary tone (name becomes INIT TONE / INIT KIT) |
| Partial Init | `30` | `7F 7F PP NN` | `NN` = Partial (SN-D/PCM-D: key index). Not offered for SN-A or GM2. |

Framing [RJS `communicator.js` `rq1`]: `F0 41 <dev> 00 00 64 11 0F 00 18 TT
<4-byte size> <checksum> F7`, with the standard Roland checksum over address
and size bytes. The device answers with a DT1 at `0F 00 18 02`. The editor
treats a reply whose first 3 data bytes are `00 00 00` as the result, and a 4th
byte of `00` as success (`isResult`, `isCompleted`).

After an Init the editor **re-reads** the affected blocks itself
(`common.js`: `model.read("PRM-_PRF", "PRM-_FPART<n>")` after Part Init, and
the whole Studio Set plus all Part tones after Studio Set Init). This implies
that the device doesn't announce the reset values, which matches the silent
tone reloads in [99-undocumented §6](../midi/99-undocumented.md#6-temporary-tone-reloads).
Core would follow an Init with a `load_mixer` (or a narrower read) once the
`0F 00 18 02` reply arrives.

### If the client must write defaults

This is the fallback path, if the command fails on our firmware. Defaults
come from three places, in order of trust:

1. **The device itself.** `10 00 00 00` reads back an `INIT LIVE` Studio Set
   ([99-undocumented §2, Not User Memory](../midi/99-undocumented.md#not-user-memory)).
   It is the device's own INIT image, but whether it equals what Studio Set
   Init writes is untested.
2. **[RJS] `init` values**, listed in the tables above. Caveats: Tone Control
   sources (0 vs [SVD] 1/97/2/4), and per-Part fields with one scalar `init`
   for all 16 Parts (e.g. Receive Channel 0, Bank 89/64/0), which can't be
   right for a 16-Part INIT set.
3. **[SVD] INIT entries.** Every User slot named `INIT STUDIO` in the pack
   backups is a full image of the factory INIT Studio Set, reserves included.
   `svd_convert` can already turn one into SysEx blocks.

Sound Ctrl Init's scope is the 8 OFFSET fields (Part `13`–`1A`), all
init 64 [PG, manual §Initialization]. So a client fallback for it is
trivial: one 8-byte DT1 of `40` per Part.

## Reserved bytes that carry data

Writing a whole block must preserve these values. A client-side Init must
write them as listed.

| Where | [MI] says | Data seen | [RJS] name and init |
|---|---|---|---|
| Common `10` | reserve, 7 bits | 127 [SVD] | (not listed) |
| Common `11`–`13` | reserve, 6 bits | 16 [SVD] | MFX1–3 Control Channel, init 16 (OFF) |
| Common `14`–`17` | reserve, 1 bit | 1 [SVD] | MFX Control Src1–4, init 1 |
| Common `38` | reserve, 4 bits | 0 | MFX Structure, init 0 |
| Common `4B` | reserve, 1 bit | 0 | Audio Input Source Select, init 0 |
| Common `50`–`53` | reserve | 0 | Expansion Slot 1–4 RomID (already in [99-undocumented §5](../midi/99-undocumented.md#5-expansion-virtual-slots)) |
| Part `02`–`05` | reserve, 1 bit | 1 [SVD] | Receive Src1–4, init 1 |
| Part `26` | reserve, 7 bits | 127 [SVD] | **Part Dry Send Level**, init 127 |
| Part `2A` | reserve, 2 bits | 0 | (not listed) |
| Part `45`, `47` | reserve, 7 bits | 64 [SVD] | (not listed; they sit between Surround L-R/F-B, probably unused 2nd-axis bytes) |
| System `09`–`10` | reserve | 0, 1, …, 7 [SVD] | Live Set Main / Sub 1–7 Channel |
| System `12`–`1F` | reserve | 1, 0, 64 × 12 [SVD] | Live Set Scale Tune Type / Key / C…B |

Part `26` "Part Dry Send Level" is a real mixer-style control that the MIDI
Implementation hides. It is out of this ticket's scope, but worth a look when
the effects routing overview (gap 10) is planned.

## Device quirks and open questions

- **Tone Control Source value 32 ([#221](https://github.com/sigma/integral/issues/221)).**
  The doc lists OFF, CC01–CC31, CC33–CC95, BEND, AFT. That is 97 labels, but
  the range is 0–97, which is 98 values. Every [SVD] Studio Set holds 97 in
  Tone Control 2, and the System INIT holds 97 in System Control 2.
  `docs/svd/10-system.md` decodes both as AFT. Under a dense index, 97 would
  be out of range (AFT = 96). Under the CC-number index of #200 (32 unused,
  BEND 96, AFT 97), it is AFT. Offline data supports the hole for **both** the
  Studio Set and System lists. A device read-back is still owed.
- **Sound Ctrl Init has no Part byte** in [RJS]. It may act on the device's
  current Part, on all Parts, or on Part 1. Test on the device before
  exposing it per Part. Sending `7F 7F PP 00` is untested.
- **Init commands are unverified on our device.** Before relying on them,
  check the `0F 00 18 TT` request and its `0F 00 18 02` reply with
  `integral raw-send`, and diff a `load_mixer` before and after.
- **Silent changes.** Scale Type templates and CC-driven OFFSET/pitch fields
  may change bytes without a DT1. Re-read after writes that may cascade.
- **Front-panel "Part Init" vs editor "Part Init".** The Owner's Manual digest
  says Part Init "resets one part's settings". [RJS] re-reads both the Part
  block and the Part's tone afterwards, so it may reset the tone reference
  too. Check the scope on the device.
- **`mixer_load` already fetches gaps 2–4 and Rx Switch.** Bytes `01` and
  `0B`–`24` arrive in every `MixerBlock::Part` reply and are dropped. Gaps
  1, 5 (Rx filters), 6 and 7 need new reads: Part `2B`–`43`, Common `18`–`27`
  and `39`–`3E`, the MIDI blocks, and System `00`–`06`. A single 77-byte Part
  read (`00`–`4C`) covers every Part field, including Surround. Today Surround
  is read separately (`PartSurround`).
