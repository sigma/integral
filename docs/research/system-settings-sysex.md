# System and Front-Panel Settings: Which Are SysEx-Addressable

Research for [#277](https://github.com/sigma/integral/issues/277), part of the
[System/front-panel map (#276)](https://github.com/sigma/integral/issues/276).

**Answer:** only the System Common block `02 00 00 00` (size `2F`) is
addressable. It covers everything in the first bullet of the ticket, plus one
extra: offset `00 2E`, documented as a reserve, is the **USB Audio Sampling
Rate** in Roland's own editor. Every other item is front-panel only: Rx GM/GM2
System On, the Preview mode, notes and velocities, USB Driver, USB-MIDI Thru,
Startup Studio Set and Startup Exp Slot A–D. Roland's editor knows their names
but has no address for them. **Ext Part Source Select is a split case.** The
Owner's Manual puts it in SYSTEM > SOUND. Roland's editor maps an "Audio Input
Source Select" to Studio Set Common `00 4B`, which the MIDI Implementation
documents as a reserve. Nothing here was checked on the device.

All front-panel-only items except the Exp Slots are probably stored in the
undocumented tail of the `SYSa` backup chunk (bits 221–863). Only the Preview
notes and velocities have been located there, and only by inference.

## Sources and trust levels

| Tag | Source | What it is |
|-----|--------|------------|
| **[MI]** | [docs/midi/04-address-map.md](../midi/04-address-map.md), System Common; [docs/midi/05-studio-set.md](../midi/05-studio-set.md), Studio Set Common | Roland MIDI Implementation v1.00, extracted. Primary. |
| **[OM]** | [INTEGRA-7 Owner's Manual](https://static.roland.com/assets/media/pdf/INTEGRA-7_OM.pdf), pp. 10, 15, 47–49 | Roland primary. Lists the SYSTEM menu parameters; says nothing about SysEx. |
| **[RJS]** | Roland's INTEGRA-7 Editor JavaScript (`Copyright 2014 Roland Corporation`), republished in [SambaGodschynski/integra7-editor-web @ d5b9193](https://github.com/SambaGodschynski/integra7-editor-web/tree/d5b9193874e2a0e3261a231562dff856c685f9fe) | Roland's editor code (see [expansion-virtual-slots.md](expansion-virtual-slots.md#sources-and-trust-levels)). The address tables are `js/model/model_excmap.js` and `js/model/model_id.js`. The display-string table is `js/model/model_stringer.js`. |
| **[SVD]** | [docs/svd/10-system.md](../svd/10-system.md) | Our offline decode of the `SYSa` chunk from 18 backups. |
| **[UND]** | [docs/midi/99-undocumented.md](../midi/99-undocumented.md) | Our device-verified undocumented messages. |

## Answer table

"SVD" means the item is stored in a BACKUP `SYSa` entry ([SVD]).

| Item | SysEx | Size / encoding | In SVD backup? | Source |
|------|-------|-----------------|----------------|--------|
| Studio Set Control Channel | `02 00 00 11` | 1 B, `000a aaaa`, 0–16 = ch 1–16, OFF | yes, bits 72–76 | [MI], [RJS] `NESC_PRF_CTRL_CH` |
| System Control 1–4 Source | `02 00 00 20`–`00 23` | 1 B each, 0–97 = OFF, CC01–31, CC33–95, BEND, AFT | yes, bits 175–202 | [MI], [RJS] `NESC_CTRL1..4_SRC` |
| Control Source (Select) | `02 00 00 24` | 1 bit, 0–1 = SYSTEM, STUDIO SET | yes, bit 203 | [MI], [RJS] `NESC_CTRL_SRC` |
| Clock Source | `02 00 00 25` | 1 bit, 0–1 = MIDI, USB | yes, bit 204 | [MI], [RJS] `NESC_CLK_SRC` |
| System Tempo | `02 00 00 26`–`00 27` | 2 nibbles, MSB first, 20–250 | yes, bits 205–212 (8-bit) | [MI], [RJS] `NESC_TEMPO` (`INTEGER2x4`) |
| Tempo Assign Source | `02 00 00 28` | 1 bit, 0–1 = SYSTEM, STUDIO SET | yes, bit 213 | [MI]. [RJS] calls it `NESC_OR_TEMPO` "Tempo Override", OFF/ON: same bit, different label |
| Rx Program Change | `02 00 00 29` | 1 bit, OFF/ON | yes, bit 214 | [MI], [RJS] `NESC_RX_PC` |
| Rx Bank Select | `02 00 00 2A` | 1 bit, OFF/ON | yes, bit 215 | [MI], [RJS] `NESC_RX_BS` |
| 5.1CH Center Speaker Switch | `02 00 00 2B` | 1 bit, OFF/ON | yes, bit 216 | [MI], [RJS] `NESC_RSS_CENTERSP_SW` |
| 5.1CH Sub Woofer Switch | `02 00 00 2C` | 1 bit, OFF/ON | yes, bit 217 | [MI], [RJS] `NESC_RSS_WOOFER_SW` |
| 2CH Output Mode | `02 00 00 2D` | 1 bit, 0–1 = SPEAKER, PHONES | yes, bit 218 | [MI], [RJS] `NESC_RSS_OUTMODE` |
| **USB Audio Sampling Rate** (OM: "Sampling Rate", SOUND) | `02 00 00 2E` *(reserve in [MI])* | 2 bits, 0–2 = 44.1, 48, 96 kHz | yes, bits 219–220 (0 in all 18 backups) | [RJS] `NESC_USB_SAMP_RATE` (`INTEGER1x2`); [MI] marks it `<*>` (ignored on receive) |
| Rx GM System On | front-panel only | OFF/ON | probably, tail (bits 221–863), not located | [OM] p. 49; [RJS] `NESC_RX_GM1` string only, no address |
| Rx GM2 System On | front-panel only | OFF/ON | probably, tail, not located | [OM] p. 49; [RJS] `NESC_RX_GM2` string only |
| Preview Mode | front-panel only | SINGLE, CHORD, PHRASE | probably, inside bits 221–224 (`0110`) | [OM] p. 49; [RJS] `NESC_PVW_MODE` string only; [SVD] |
| Preview 1–4 Note Number | front-panel only | C-1–G9 | yes (inferred), bits 225–280: 7-bit note + 7-bit velocity per pair = 48, 60, 72, 84 / 127 | [OM] p. 49; [RJS] `NESC_PVW1..4_NOTE` string only; [SVD] |
| Preview 1–4 Velocity | front-panel only | OFF, 1–127 | yes (inferred), same pairs | as above |
| Preview **trigger** (not a setting) | `0F 00 20 00` DT1, 1 B: `00` off, `01`–`10` part | fire-and-forget | n/a | [UND] §3, [RJS] `PREVIEW_ADDR` |
| Ext Part Source Select | **Studio Set Common `18 00 00 4B`** per [RJS] *(reserve in [MI])*. No System address. | 1 bit, 0–1 = USB-AUDIO, EXT-AUDIO (OM: USB AUDIO, INPUT) | Studio Set entries carry the bit (`00 4B`, always 0); `SYSa` location unknown | [OM] p. 48 lists it under SYSTEM > SOUND; [RJS] `NEFC_IN_SRC_SELECT` at `0x004B` in the Studio Set Common table; a `NESC_IN_SRC_SELECT` string also exists, with no address |
| USB Driver (VENDER / GENERIC) | front-panel only | 2 values; takes effect after save + power cycle | probably, tail | [OM] p. 48; [RJS] `NESC_USB_MIDI_DRV` string only |
| USB-MIDI Thru | front-panel only | OFF/ON | probably, tail | [OM] pp. 45, 49; [RJS] `NESC_USB_MIDI_THRU` string only |
| USB audio routing | no parameter | fixed: USB out = A (MIX) signal, USB in → Ext Part | n/a | [manual/05-operational.md](../manual/05-operational.md#usb-audio-routing). Only the Sampling Rate and Ext Part Source Select rows above configure it |
| Startup Studio Set | front-panel only | LAST-SET or a Studio Set number | probably, tail | [OM] p. 48; [RJS] strings `NESC_PUP_PRF_BS_MSB/LSB/PC` (0–127 each), no address |
| Startup Exp Slot A–D | front-panel only | OFF or a library name per slot | probably, tail | [OM] pp. 15, 48; [RJS] strings `NESC_EXP_SLOT1..4_ROMID` ("OFF" or 1–127), no address. The runtime slots *are* addressable: read `0F 00 00 10`, load `0F 00 30 00` ([UND] §5) |

## Findings in detail

### System Common is the whole System address space

[MI] defines one System sub-block, System Common at `02 00 00 00`, size `2F`.
[RJS] agrees: its `SYS` node (`model_excmap.js` L1829–L1830) holds only `_SC`,
and the `SC` table (L70–L111) ends at `0x002E`. The editor's SYSTEM screen
(`scene_system.html` L52–L98) binds only SOUND (Level, Tune, Key Shift),
SYNC/TEMPO, MIDI (Control Channel, Rx PC, Rx BS), MOTIONAL SURROUND and CONTROL
parameters, all from that table. If Roland's own editor had an address for
Preview, Rx GM, USB-MIDI Thru or Startup, it would use it. It doesn't.

### The string table names the missing parameters

`model_stringer.js` L92–L169 is a display-string table for every `NESC_*` id.
After `NESC_USB_SAMP_RATE` it lists, in this order: `POWER_UP`, `PVW_MODE`,
`PVW1..4_NOTE`/`_VELO` (interleaved), `TX_EDIT`, `RX_GM1`, `RX_GM2`, `RX_GS`,
`RX_EXC`, `DEV_ID`, `OUTPUT_GAIN`, `SYNC_MODE`, `SYNC_OUT`, `SOFT_THRU`,
`METRO_LEVEL`, `METRO_SND`, `AUTO_POWER`, `IN_SRC_SELECT`, `USB_MIDI_DRV`,
`USB_MIDI_THRU`, `TOP_SCREEN`, `SCRN_SAVE_TIME`, `LCD_CONTRAST`,
`LCD_BRIGHTNESS`, `PUP_PRF_BS_MSB/LSB/PC`, `EXP_SLOT1..4_ROMID`. None of these
ids appears in `model_id.js` or `model_excmap.js`, so none has an address in
Roland's editor.

**Hypothesis (unverified):** the `SYSa` tail follows this order. [SVD] places
4 bits at 221–224 and then four (note, velocity) pairs at 225–280. That fits
`POWER_UP` + `PVW_MODE` followed by `PVW1_NOTE`, `PVW1_VELO`, …. If
`PVW_MODE` is the last 2 bits (`10` = 2), it decodes to PHRASE. The bit widths
of the later fields are unknown, so this order is a lead for the backup-diff
check in [docs/svd/10-system.md](../svd/10-system.md#open-questions--device-check),
not a decode.

### `00 2E` is the Sampling Rate, and [MI] says it's ignored on receive

[MI] documents `02 00 00 2E` as `0000 00aa` (reserve) `<*>`. [RJS] names it
`NESC_USB_SAMP_RATE`, `INTEGER1x2`, 0–2, displayed as 44100/48000/96000 Hz.
That matches [OM] p. 48 ("Sampling Rate 44.1, 48, 96 kHz", SOUND). The 2-bit
width matches [SVD] bits 219–220. The `<*>` mark means a DT1 may be ignored, and
the editor's SYSTEM screen doesn't expose it. Treat it as **readable but
not proven writable**.

### Ext Part Source Select: Studio Set byte or System setting?

[OM] p. 48 lists "Ext Part Source Select" (USB AUDIO, INPUT) under SYSTEM >
SOUND, and pp. 10 and 47 point to it there. [RJS] has
`NEFC_IN_SRC_SELECT` = "Audio Input Source Select" at Studio Set Common `00 4B`
(`model_excmap.js` L214, `model_id.js` L466), 1 bit, `"USB-AUDIO"or"EXT-AUDIO"`
(`model_stringer.js` L723). [MI] marks Studio Set Common `00 4B` as
`0000 000a` (reserve) `<*>`. [SVD] finds it 1 bit wide and always 0 in Studio
Set entries ([docs/svd/05-studio-set.md](../svd/05-studio-set.md)).

The same pattern holds at `00 50`–`00 53` ("Expansion Slot RomID"), which the
device turned out to ignore ([UND] §5). So the editor table is not proof that
`00 4B` works. **Open:** whether a DT1 to `18 00 00 4B` switches the Ext Part
source, and whether it reads back the SYSTEM setting.

### GM System On reception

The INTEGRA-7 receives Universal Non-realtime SysEx ([docs/midi/01-protocol.md](../midi/01-protocol.md)).
The Rx GM / Rx GM2 System On switches gate it ([OM] p. 49), but there is no
address to read or set them. A remote editor can see the result through Setup
`01 00 00 00` Sound Mode (STUDIO, GM1, GM2, GS) in [MI].

### System changes are volatile

[OM] p. 48: "System settings will return to their previous state when you turn
off the unit… you must save the system settings" ([SHIFT] + [EDIT] in the
SYSTEM screen). The only known write request is `0F 00 10 00`, and it targets
User Studio Sets and tones ([UND] §4, [RJS] `WRITE_REQ_ADDR`). No System
equivalent is known. A DT1 to `02 00 00 xx` should therefore be treated as
lasting only until power-off. This is unverified.

### What `integral-core` has today

Only `address::SYSTEM_COMMON` (`crates/integral-core/src/address.rs` L153) and
the preview trigger (`Device::preview_start/stop/toggle`,
`crates/integral-core/src/device.rs` ~L1318–L1356). `param_registry.rs`,
`device_spec.rs` and `svd_specs.rs` have no System Common parameters. The
`SYSa` chunk is carried through as an opaque 108-byte entry (`svd.rs`).

## To verify on hardware

1. RQ1 `02 00 00 00` size `2F`: check that `00 2E` reads 0 and changes after
   setting Sampling Rate on the panel. Then DT1 `00 2E` and see whether the
   panel follows.
2. Toggle Ext Part Source Select on the panel and RQ1 Studio Set Common
   `18 00 00 4B`. Then DT1 `00 4B` and see whether the panel follows.
3. RQ1 `02 00 00 2F` and above, small sizes, to confirm that nothing answers
   past the documented block.
4. Power-cycle after a DT1 to System Common without a panel System Write, to
   confirm the change doesn't persist.
5. Backup-diff the `SYSa` tail one SYSTEM item at a time, as in
   [docs/svd/10-system.md](../svd/10-system.md#open-questions--device-check),
   using the [RJS] string order as the field-order hypothesis.
