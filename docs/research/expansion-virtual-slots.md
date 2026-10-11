# Expansion Virtual Slots: Read and Load SysEx

Research for [#93](https://github.com/sigma/integral/issues/93), part of the
[Expansions map (#92)](https://github.com/sigma/integral/issues/92).

**Answer:** yes. Two undocumented `RQ1` messages in the `0F` editor/control
region are known: one reads the four slots, the other loads them. Roland's own
editor code uses both, and two open-source editors use them too. Neither has
been checked on our device yet (see [To verify on hardware](#to-verify-on-hardware)).

## Sources and trust levels

| Tag | Source | What it is |
|-----|--------|------------|
| **[OM]** | [INTEGRA-7 Owner's Manual](https://static.roland.com/assets/media/pdf/INTEGRA-7_OM.pdf), pp. 14–15, 27, 29–30, 48, 60 | Roland primary. Front-panel behaviour only; no SysEx. |
| **[RJS]** | Roland's INTEGRA-7 Editor JavaScript (headers say `Copyright 2014 Roland Corporation`), as republished in [SambaGodschynski/integra7-editor-web @ d5b9193](https://github.com/SambaGodschynski/integra7-editor-web/tree/d5b9193874e2a0e3261a231562dff856c685f9fe) | Roland's own editor code. The README says the Mac/PC editor "is just a web page embedded into a VST-Plugin" and that the repo reuses those sources. Closest thing to primary for the protocol. |
| **[AA]** | [shimpe/Integra7AuralAlchemist @ d40f43d](https://github.com/shimpe/Integra7AuralAlchemist/tree/d40f43d42fcaed9eb6e04b99b309025356dcf217) (C#) | Open-source editor. Its code comments record timings measured on a real unit (2026-07-30). |
| **[MJ]** | [mj15003/integra7frontend @ 1ca38e3](https://github.com/mj15003/integra7frontend/tree/1ca38e33340760c1ce756361c2d0f41bab6538a3) (C++/Qt) | Open-source editor. Load only. |
| **[RC1]** | Roland Clan, ["Load SRX boards when opening different songs?"](http://web.archive.org/web/2025/http://forums.rolandclan.com/viewtopic.php?f=54&t=48081), post by PauloF, 2014-03-16 | Forum. First public capture, sniffed from Roland's Mac editor. |
| **[RC2]** | Roland Clan, ["Load expansion card via MIDI?"](http://web.archive.org/web/20250119011213/http://forums.rolandclan.com/viewtopic.php?t=49393), posts by hobbit (2014-12-04, 2014-12-06) and JunoJohn (2014-12-05) | Forum. Corrects the checksum in RC1 and gives load timings. |

The live forum is behind mod_security (HTTP 406 to scripts), so the links go to
the Wayback Machine copies.

## Load: `RQ1 0F 00 30 00`

As with the catalog query (see
[99-undocumented.md §1](../midi/99-undocumented.md#1-presetuser-name-catalog-query)),
this is an **RQ1 whose 4-byte size field carries a payload**. Here the payload
is one slot value for each of slots A to D:

```
F0 41 <dev> 00 00 64 11  0F 00 30 00  <A> <B> <C> <D>  <sum> F7
```

The checksum is the standard Roland sum over `0F 00 30 00 A B C D`.

- **[RJS]** `REQ_LOAD_EXP_ADDR = 0x0F003000`, and `loadExpansion(a,b,c,d)`
  packs `size = a<<24 | b<<16 | c<<8 | d` and sends `com.rq1(...)`
  ([scene_base_midi_controller.js L26](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_base_midi_controller.js#L26),
  [L590–L611](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_base_midi_controller.js#L590-L611)).
  `rq1()` builds `F0 41 <dev> 00 00 64 11 <addr4> <size4> <sum> F7`
  ([communicator.js L124–L150](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/communicator/communicator.js#L124-L150)).
- **[AA]** `MakeLoadSrxMsg`: `payload = [0x0F, 0x00, 0x30, 0x00, slot1, slot2, slot3, slot4]`
  sent with `COMMAND_DATAREQ` (0x11) plus a checksum
  ([Integra7SysexHelpers.cs L70–L75](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/Integra7SysexHelpers.cs#L70-L75)).
- **[MJ]** `uint8_t VirtualSlots[8] = {0x0F, 0x00, 0x30, 0x00, 0, 0, 0, 0};`,
  sent through `DataRequest` (command `0x11`)
  ([integra7device.h L457](https://github.com/mj15003/integra7frontend/blob/1ca38e33340760c1ce756361c2d0f41bab6538a3/src/integra7device.h#L457),
  [integra7device.cpp L662–L669](https://github.com/mj15003/integra7frontend/blob/1ca38e33340760c1ce756361c2d0f41bab6538a3/src/integra7device.cpp#L662-L669),
  [L152–L157](https://github.com/mj15003/integra7frontend/blob/1ca38e33340760c1ce756361c2d0f41bab6538a3/src/integra7device.cpp#L152-L157)).
- **[RC1]/[RC2]** quote the same bytes. RC1 ended every message with
  `30 10 F7`. RC2 (hobbit) explains that the `10` was a MIDI-monitor line
  offset copied by mistake and `30` was a single checksum applied to every
  message, and notes that the device seemed to accept the wrong checksum anyway.

### Slot value table

All four code sources agree, and so do RC1/RC2:

| Value | Title | Value | Title |
|-------|-------|-------|-------|
| `00` | OFF (empty) | `0D` | ExSN1 Ethnic |
| `01`–`0C` | SRX-01 … SRX-12 | `0E` | ExSN2 Wood Winds |
| | | `0F` | ExSN3 Session |
| | | `10` | ExSN4 A.Guitar |
| | | `11` | ExSN5 Brass |
| | | `12` | ExSN6 SFX |
| | | `13` | ExPCM (HQ GM2 + HQ PCM), takes all four slots |

Evidence: the order of the `expArray` names in **[RJS]** with `value - 1` sent
([scene_exp.js L18–L39, L96–L106](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_exp.js#L18-L39));
the `SrxIdForLoad` enum in **[AA]**
([Integra7SysexHelpers.cs L10–L32](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/Integra7SysexHelpers.cs#L10-L32));
the order of `ExpansionNames()` in **[MJ]**, passed by combo index
([integra7device.h L135–L145](https://github.com/mj15003/integra7frontend/blob/1ca38e33340760c1ce756361c2d0f41bab6538a3/src/integra7device.h#L135-L145)).

### Examples (device ID `10`; checksums computed here)

| Intent | SysEx |
|--------|-------|
| Unload all | `F0 41 10 00 00 64 11 0F 00 30 00 00 00 00 00 41 F7` (identical to RC2) |
| SRX-01..04 into A..D | `F0 41 10 00 00 64 11 0F 00 30 00 01 02 03 04 37 F7` (identical to RC2) |
| ExSN1, SRX-02, SRX-06, empty | `F0 41 10 00 00 64 11 0F 00 30 00 0D 02 06 00 2C F7` |
| ExPCM | `F0 41 10 00 00 64 11 0F 00 30 00 13 13 13 13 75 F7` |

For ExPCM, **[MJ]** forces all four combos to index 19
([integra7mainwindow.cpp L6449–L6463](https://github.com/mj15003/integra7frontend/blob/1ca38e33340760c1ce756361c2d0f41bab6538a3/src/integra7mainwindow.cpp#L6449-L6463)).
RC1 says `13` "or higher" applies ExPCM to all four slots. **[RJS]** disables
the other three selects but does not rewrite their values. Sending `13 13 13 13`
is the safe choice.

### Completion

**[RJS]** registers the load's reply handler on `REQ_LOAD_EXP_END_ADDR = 0x0F003002`.
It also defines `REQ_LOAD_EXP_START_ADDR = 0x0F003001`, which it never uses
([L27–L28](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_base_midi_controller.js#L27-L28),
[L603–L608](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_base_midi_controller.js#L603-L608)).
So Roland's editor expects the device to send a **DT1 at `0F 00 30 02` when the
load is done**. It then re-reads the slots. A 20-second safety timer closes the
"LOADING..." dialog if that DT1 never arrives
([scene_exp.js L120–L141](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_exp.js#L120-L141)).

**[AA]** says the opposite: "a load is neither acknowledged nor reported"
([SeedInstrument.cs L144–L145](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/SeedInstrument.cs#L144-L145)).
Its reply matcher only listens for the slot-read address, though, so it may
never have seen the DT1. **Unresolved; verify with `integral monitor`.**

## Read: `RQ1 0F 00 00 10`, size 0

```
-> F0 41 <dev> 00 00 64 11  0F 00 00 10  00 00 00 00  <sum> F7
<- F0 41 <dev> 00 00 64 12  0F 00 00 10  <A> <B> <C> <D>  <sum> F7
```

Example with device `10`: `F0 41 10 00 00 64 11 0F 00 00 10 00 00 00 00 61 F7`.

- **[RJS]** `REQ_READ_EXP_ADDR = 0x0F000010`. `readExpansion` sends an RQ1 with
  `size = 0` and parses the first four data bytes of the DT1 that comes back to
  the same address into `curExpansion`
  ([L30](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_base_midi_controller.js#L30),
  [L613–L633](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_base_midi_controller.js#L613-L633)).
  Roland's editor reads the slots on connect, on device change, before the
  Expansion screen opens, after every load, and when it restores a saved DAW
  state (startup.js
  [L80–L92](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/startup.js#L80-L92):
  it reloads the saved slots only if they differ from the current ones).
- **[AA]** `MakeAskLoadedSrxMsg`: `[0x0F, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00]`
  with RQ1, taking slots from reply bytes 11–14
  ([Integra7SysexHelpers.cs L77–L82](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/Integra7SysexHelpers.cs#L77-L82),
  [Integra7Api.cs L201–L209](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/Integra7Api.cs#L201-L209)).

The reply uses the same value table, with one difference: **ExPCM reads back as
`13 14 15 16`** (19, 20, 21, 22), not `13 13 13 13`.

- **[AA]** measured it: the instrument was "answering (19,20,21,22) when it had
  just been asked for (11,0,0,0)", meaning the outgoing ExPCM loadout
  ([SeedInstrument.cs L164](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/SeedInstrument.cs#L164)).
- **[RJS]** agrees: `var ExPCM_SLOT1 = 20; // db = 19`, and any value `>= 19`
  is shown as ExPCM
  ([scene_exp.js L146–L160](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_exp.js#L146-L160),
  [ui_midi_def.js L91–L100 `isExPCMExist`](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/ui_midi_def.js#L91-L100)).

## Slot count and placement constraints

- **Four slots, A–D.** **[OM]** p. 14: "four EXPANSION Virtual Slots (Slot A–D).
  (In other words, sound data for up to four titles can be used simultaneously.)"
  p. 27 adds: "You can load four titles of SRX or SuperNATURAL sound data into
  these virtual slots."
- **SRX and ExSN take one slot each; ExPCM takes all four.** **[OM]** p. 14
  ("ExPCM uses all four slots") and p. 60 (specifications): "The SRX Series and the
  Expansion SuperNATURAL Sounds use one virtual slot per title. The Expansion
  Hi-Quality PCM Sounds use all four virtual slots."
- **Any title can go in any slot.** **[OM]** says "Choose one title of sound data
  for each slot", and none of the code sources restricts titles by slot. There
  are no size limits beyond ExPCM.
- **No duplicates (in the UI).** **[RJS]** `selectable()` disables a title in
  the other three selects once one slot holds it
  ([scene_exp.js L74–L88](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/view/scene_exp.js#L74-L88)).
  What the device does if sent a duplicate is unknown.
- **A loaded title opens its bank.** Tones become selectable under the title's
  bank select (SRX MSB 93/92, ExSN MSB 89/88, ExPCM MSB 97/96; see
  [03-bank-select-tables.md](../midi/03-bank-select-tables.md)), whichever slot
  holds it. [OM] p. 15 adds that loading ExPCM changes the "GM2" bank to "GM2#".
- **Loaded state is volatile.** **[OM]** p. 27 lists the slots as "temporary
  memory". The System parameters "Startup Expansion SlotA–SlotD" (OFF or a title)
  load titles at power-on (**[OM]** pp. 15, 48). Holding **[EXIT]** at power-on
  skips that load (p. 29 shortcut table). No SysEx address is known for the Startup
  Exp Slot parameters: they are not in the documented System Common block
  (`02 00 00 00`, size `2F`), nor in the [RJS] System map, which ends at `00 2E`.

## Load duration

| Claim | Source | Trust |
|-------|--------|-------|
| ~5 s per slot, ~20 s for all four; a slot that already holds the requested title is not reloaded | **[RC2]** JunoJohn | forum |
| One title 5 s, some about 13 s, ExPCM 18.7 s (stable at 23.3 s) | **[AA]** [SeedInstrument.cs L37–L62](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/SeedInstrument.cs#L37-L62) | measured, recorded in code |
| Device answers nothing for 6–13 s during a load | **[AA]** [TestSeedSettling.cs L5–L11](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Tests/TestSeedSettling.cs#L5-L11) | measured |
| Re-sending the current loadout causes no load and no quiet period | **[AA]** `Verdict` comment, [SeedInstrument.cs L120–L141](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/SeedInstrument.cs#L120-L141) | measured; matches RC2 |
| Roland editor gives up waiting after 20 s | **[RJS]** scene_exp.js L141 | code |

## Side effects

**Verified in code or by measurement:**

- **The device stops answering during a load.** RQ1s get no reply until the load
  finishes ([AA], `AnsweredWithin`). While idle it answers the slot read in
  2–5 ms.
- **A load request sent while a load is running is silently discarded** ([AA]
  [SeedInstrument.cs L80–L86](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/Models/Services/SeedInstrument.cs#L80-L86)).
  Don't queue loads; wait for idle.
- **Stale read just after a load request.** For the first 3–29 ms after the
  request, the slot read still returns the *old* loadout ([AA] SeedInstrument.cs L161–L169).
  Poll after a delay, and treat "answered, then silent, then answered" as the
  completion signal.
- **The loaded set changes which tones exist.** Roland's editor rebuilds its bank
  lists from `curExpansion` and calls `toneUpdate()` after a load ([RJS]
  scene_exp.js L120–L128). [AA] re-reads every part after a load
  ([MainWindowViewModel.cs L1925–L1934](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Src/ViewModels/MainWindowViewModel.cs#L1925-L1934)).
  The temporary tone area of a part whose bank is not loaded does not answer
  RQ1 at all ([AA] SeedInstrument.cs `CaptureAsync` catch comment). **[OM]**
  p. 30: a Studio Set using an unloaded expansion tone plays silence.
- **Restore order matters:** load the slots before restoring a Studio Set that
  references them ([AA]
  [TestSeedRun.cs L185–L198](https://github.com/shimpe/Integra7AuralAlchemist/blob/d40f43d42fcaed9eb6e04b99b309025356dcf217/Tests/TestSeedRun.cs#L185-L198)).
- **No restart needed.** Every source loads at runtime. [OM] confirms the
  front-panel LOAD runs after an "Are you sure?" prompt.

**Not established by any source:**

- Whether sound cuts during a load (the device being busy suggests it may, but
  nobody documents it).
- Whether the Temporary Studio Set is reset or left alone. [AA] snapshots and
  restores the Studio Set around its sweep, but because the sweep changes part
  tones itself, not because of the load.
- Whether the device sends any unsolicited message when someone loads from the
  front panel. Roland's editor doesn't listen for one; it re-reads on connect
  and on screen entry.

## Related lead: Studio Set Common `00 50`–`00 53`

[05-studio-set.md](../midi/05-studio-set.md) marks Studio Set Common offsets
`00 50`–`00 53` as reserved. The [RJS] parameter map names them
`NEFC_EXP_SLOT1_ROMID` … `NEFC_EXP_SLOT4_ROMID`, "Expansion Slot 1–4 RomID",
range 0–127
([model_excmap.js L219–L222](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_excmap.js#L219-L222)).
A matching unused `NESC_EXP_SLOT1..4_ROMID` (System) appears in the string table
([model_stringer.js L166–L169](https://github.com/SambaGodschynski/integra7-editor-web/blob/d5b9193874e2a0e3261a231562dff856c685f9fe/js/model/model_stringer.js#L166-L169)),
which probably corresponds to the Startup Exp Slot parameters. This *suggests*
that each Studio Set records the slots it expects, but no source says the device
uses those bytes. **Hypothesis only.**

## To verify on hardware

1. Read: `F0 41 10 00 00 64 11 0F 00 00 10 00 00 00 00 61 F7`. Expect a DT1 at
   `0F 00 00 10` with 4 bytes that match the front-panel EXPANSION screen.
2. Load a single changed slot. With `integral monitor`, check for a DT1 at
   `0F 00 30 02` (and `0F 00 30 01`) to settle [RJS] vs [AA]. Time the load,
   and listen for a sound cut on a held note.
3. After a load, read Studio Set Common and a part's tone to check whether the
   Temporary Studio Set was touched.
4. Read Studio Set Common `18 00 00 50`, size 4, across Studio Sets saved with
   different slot loadouts.
5. Send a duplicate title (e.g. `01 01 00 00`) and see what the device keeps.

Step 4 means reading user memory, not switching
Studio Sets ([99-undocumented.md §2](../midi/99-undocumented.md#2-user-memory-address-space)).
Load commands change device state, so run them only with the owner's consent.
