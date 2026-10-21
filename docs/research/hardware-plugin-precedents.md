# Hardware-Synth Plugin Precedents

Research for [#337](https://github.com/sigma/integral/issues/337), part of the
[DAW integration map (#336)](https://github.com/sigma/integral/issues/336).

**Answer, in short:**
- **Device link:** only one precedent runs the link in a separate
  process: Elektron's Overbridge Engine. Plugins reach it over local TCP.
  Every other precedent opens the hardware port from inside the plugin.
  Port contention with the DAW is their most documented failure.
- **Instance granularity:** every precedent uses **one instance per device**.
  Access forbids a second Virus Control instance outright. No precedent uses
  one instance per part, per drum key or per effect. The one exception is a
  standalone effects *unit*, Elektron's Analog Heat.
- **Parameters:** no source documents how parameters whose meaning changes
  with a type are named. Access simply excludes algorithm-switching
  parameters from automation.
- **Recall:** the common pattern stores the full state in the DAW project
  and pushes it on load. The better designs ask the user which side wins
  when the device was edited in the meantime.

Anything not confirmed from a primary source is marked **UNCONFIRMED**.

## Sources

| Tag | Source | Trust |
|-----|--------|-------|
| **[VUM]** | [Virus TI User Manual](http://nic.vajn.icu/PDF/audio-gear/AccessVirus/Virus_TI_UM.pdf) (Access / Kemper Digital, third-party mirror) | Vendor manual, mirrored |
| **[VQS]** | [Virus TI Quickstart](http://nic.vajn.icu/PDF/audio-gear/AccessVirus/Virus_TI_Quickstart.pdf) (mirror) | Vendor manual, mirrored |
| **[VSG]** | [Virus TI Setup Guide Rev 1, 2010](http://web.archive.org/web/20170922051342/http://virus.info/api/download/dlid/389/n/Virus%20TI%20Setup%20Guide.pdf) (virus.info via Wayback) | Vendor |
| **[VCL]** | [TIOS 2.7.1 change log](http://web.archive.org/web/20081121010437/http://www.virus.info/events/2008-08/OS271PB5_change_log.pdf) (Wayback) | Vendor |
| **[VTI]** | [virus.info Total Integration page](http://web.archive.org/web/20260510173831/https://www.virus.info/virusti/totalintegration) (Wayback) | Vendor |
| **[VKVR]** | [TIOS 3.0.4–3.1.0 changelog reprint on KVR](https://www.kvraudio.com/news-print.php?id=12306) | Secondary reprint |
| **[OBM]** | [Overbridge User Manual, 2026-04-23](https://www.elektron.se/wp-content/uploads/2026/04/Overbridge-User-Manual_ENG_260423.pdf) | Vendor |
| **[OBRN]** | [Overbridge release notes](https://www.elektron.se/release-notes/overbridge-release-notes), [support page](https://www.elektron.se/support-downloads/overbridge) | Vendor |
| **[OBKB]** | Elektron KB: [Engine does not start](https://support.elektron.se/support/solutions/articles/43000571670-overbridge-engine-does-not-start), [connection problems](https://support.elektron.se/support/solutions/articles/43000573600-overbridge-elektron-device-connection-problems) | Vendor |
| **[YXS]** | [MOTIF XS Editor VST manual](https://usa.yamaha.com/files/download/other_assets/8/325408/motifxs_editorvst_en_om_a1.pdf) | Vendor |
| **[YXF]** | [MOTIF XF Editor VST manual](https://usa.yamaha.com/files/download/other_assets/3/323563/motifxf_editor_vst_en_om_a0.pdf) | Vendor |
| **[YMOXF]** | [MOXF Editor manual](https://ca.yamaha.com/files/download/other_assets/2/330412/moxf6moxf8_editor_en_om_a0.pdf) | Vendor |
| **[YSM]** | [Studio Manager V2 manual](https://usa.yamaha.com/files/download/other_assets/1/335501/smv2_en_om_e0.pdf) | Vendor |
| **[YMC]** | [MONTAGE apps page](https://usa.yamaha.com/products/music_production/synthesizers/montage/apps.html), [What is MONTAGE Connect](https://yamahasynth.com/learn/montage-series-synthesizers/what-is-montage-connect/), [MONTAGE + Cubase workflow](https://yamahasynth.com/learn/montage-series-synthesizers/montage-cubase-pro-9-workflow/), [instance-limit thread](https://yamahasynth.com/community/montage-series-synthesizers/limits-on-instances-of-montage-connect-in-cubase-9), [bulk-interrupt thread](https://yamahasynth.com/community/montage-series-synthesizers/now-transmitting-bulk-midi-data-interrupting-montage-playback-of-logic-pro-x-midi-track) | Vendor site. The forum answers are by "Bad Mister" (Phil Clendeninn, long-time Yamaha product specialist). The fetched pages don't label him as staff. |
| **[YESP]** | [MONTAGE M Expanded Softsynth Plugin manual v3.0.1](https://br.yamaha.com/files/download/other_assets/0/2232310/ESP_manual_En_v301_D0.pdf) | Vendor |
| **[CT]** | [RomanKubiak/ctrlr @ 8aa00d8](https://github.com/RomanKubiak/ctrlr/tree/8aa00d82127acda42ad9ac9b7b479461e9436aa4) | Source code |
| **[ST]** | [OB-X8 PlugSE guide](https://www.soundtower.com/plugse/obx8/manual/OBX8_PlugSE_instructions.pdf), [Prophet 6 PlugSE](https://www.soundtower.com/plugse/prophet6/page.html), [PlugSE in Logic](https://www.soundtower.com/plugse/toraiz/fx_plugse_logic.html) | Vendor |
| **[MQ]** | Midi Quest 13 help: [instrument plugins](https://www.soundquest.org/Help/MidiQuest13/plugin-instrumentplugins.htm), [MFX/VST settings](https://www.soundquest.org/Help/MidiQuest13/mqmfxvstsettings.htm), [Plugins.ini](https://www.soundquest.org/Help/MidiQuest13/manually_creating_plug-in_file.htm); [product details](https://www.squest.com/Products/MidiQuest13/Details.html) | Vendor |
| **[RIE]** | [INTEGRA-7 Editor Ver.2 Owner's Manual](https://static.roland.com/assets/media/pdf/INTEGRA-7_Editor_Ver2_OM.pdf); [2013 press release](https://www.roland.com/us/company/press_releases/2013/1689/); [Mac download page / known issues](https://tw.roland.com/support/by_product/integra-7/updates_drivers/502220); [Motional Surround Editor download](https://www.roland.com/au/support/by_product/integra-7/updates_drivers/b54a7402-7510-47a0-a22b-9aef38d895ba/) | Vendor |
| **[RPO]** | [SH-101 PLUG-OUT Owner's Manual](https://r2.gear4music.com/media/67/672546/download_672546.pdf) (retailer mirror of the manual listed on [roland.com](https://www.roland.com/us/support/by_product/rc_sh-101/owners_manuals/)) | Vendor manual, mirrored |

## Comparison

| Precedent | Device link owner | MIDI path (notes / control) | Instance granularity | Host parameters | Recall | Effects |
|---|---|---|---|---|---|---|
| Access Virus TI (Virus Control) | In-plugin, over a custom USB driver | Host → plugin → USB, sample-accurate | **One per device, enforced**; 16 parts inside | "Most" automatable; algorithm switches excluded | Project holds everything; pushed on load with progress bar | Inside each part's patch |
| Elektron Overbridge 2 | **Separate Engine process**; plugins reach it on local TCP | Host → plugin → Engine → USB | One per device (machine-select menu) | Sound, FX and macro parameters automatable; count UNCONFIRMED | Opt-in "Total Recall"; 3-way conflict prompt on device | Pages in device plugin; Analog Heat (an FX unit) has its own plugin |
| Yamaha MOTIF XS/XF/MOXF Editor VST | In-plugin, opens the port itself (exclusive on Windows) | Notes on a normal Cubase MIDI track; plugin owns the control port | One per device; all 16 parts | Not documented; effectively none | Saved in project; **Auto Sync** window asks direction | Inside the editor |
| Yamaha Studio Manager V2 | **Host-level app** started with Cubase, holding several editors | Exposes a port that is shared between notes and editor control | One editor per device | Not documented | Saved with project; Recall asks To/From Hardware; optional on load | One editor per device (FX boxes too) |
| Yamaha MONTAGE Connect | In-plugin, USB | Notes on DAW tracks; plugin bulk-dumps | **One per device**: "You can only run one instance" | Automation lanes present but "not operational" | Bulk read on save, bulk dump to edit buffer on open | Part of the Performance |
| Yamaha MONTAGE M ESP | In-plugin, opens Port 4 | Port 4 must be disabled in the DAW | One per Performance; parts 9–16 silent | 128 MIDI-learn slots inside the plugin | Project saves the edit buffer only; transfer is manual | Inside the plugin |
| Ctrlr | In-plugin (per panel) | Both, by per-panel flags (from/to host, H2D, D2H thru) | One panel per device | Fixed index table (`vstIndex`), 64 by default; unmapped slots `undefined_N` | XML state; push on load is opt-in | One plugin type |
| SoundTower PlugSE | In-plugin, direct ports | Instrument variant forwards host notes; Control variant sends none | Per device/channel; many Control instances allowed | OB-X8: "more than a thousand" (NRPN) | Bank saved in project, reloaded on open | No separate FX plugins |
| Sound Quest Midi Quest | In-plugin, direct ports (needs multi-client drivers) | VST: direct ("VST 2.0 … not support SysX"); MFX: via host | One plugin per instrument model | "Hundreds or even thousands" via Automation Manager | "Total Recall" on load, optional; send triggers with spacing | Not separate (UNCONFIRMED) |
| Roland INTEGRA-7 Editor Ver.2 | In-plugin (VSTi/AU) | **Separate USB port**: DAW uses "INTEGRA-7", editor uses "INTEGRA-7 CTRL" | One per device | UNCONFIRMED | Device-centric (READ/WRITE); some DAW "total recall" exists | Tabs in the editor |
| Roland PLUG-OUT (SH-101, SYSTEM-1/8) | In-plugin; MIDI settings shared by all instances | Dedicated "… CTRL" port, "Do not use this port from your DAW" | Soft synth per instance | UNCONFIRMED | Explicit SEND/GET | n/a |

## Per question

### Device link: who owns it, several instances, several hosts

- **Overbridge is the one out-of-process precedent.** "The Overbridge Engine
  handles all USB communication between your connected Elektron devices and
  the Overbridge… plugins and standalone editors. Engine must run, for the
  plugins and standalone editors to function" [OBM §2.3].
  - Plugins reach it over local TCP. Port 46000 is fixed, each device uses
    460n0–460n2, and another user session's Engine blocks the port [OBKB].
  - Standalone editors use the same Engine [OBM §7].
  - The device OS and Overbridge versions "must match", with a warning on
    mismatch [OBM §5.2]. The support page lists the minimum device OS for
    each release [OBRN].
- **Yamaha Studio Manager V2** is a halfway case: one host-level application
  that "starts running when Cubase is started" and holds the editors for
  several devices [YSM]. It is not a background service, though.
- **Everything else is in-plugin** and opens the device port directly:
  - Virus Control [VUM p.53]
  - MOTIF Editor VST [YXS p.40]
  - MONTAGE Connect and ESP [YMC, YESP p.7]
  - Ctrlr (`CtrlrPanel::sendMidi`) [CT]
  - PlugSE [ST]
  - Midi Quest [MQ]
  - Roland's INTEGRA-7 Editor [RIE p.10]
- **Several instances:**
  - Access: "it is not possible to open more than one instance… you should
    not attempt to do so" [VUM p.55].
  - Bad Mister on MONTAGE Connect: "You can only run one instance" [YMC].
  - PLUG-OUT shares its MIDI settings across instances [RPO p.3, p.12].
  - Overbridge selects a device per instance [OBM §6.10]. Two instances on
    one device: UNCONFIRMED (user posts only).
- **Several hosts:** only Yamaha documents it, as a hazard: "If you try to
  access ESP from multiple DAWs at once, user data … may become corrupted"
  [YESP p.4]. Elsewhere it isn't documented, and port exclusivity prevents
  it in practice.

### MIDI path

- **Notes are host-scheduled everywhere.** Even Virus and Overbridge, which
  tunnel everything over USB, take notes from the host track into the plugin
  so the host's delay compensation applies [VQS; OBM §8]. Yamaha and Roland
  leave notes on an ordinary DAW MIDI track to the hardware [YXS p.4; RIE].
- **Control traffic (SysEx) bypasses the host** in every SysEx editor. The
  reasons given:
  - Sound Quest: "the VST 2.0 spec does not currently support SysX data" [MQ].
  - Roland: "You must configure so that a DAW software does not use the same
    MIDI device that VSTi plug-in editor uses" [RIE, Motional Surround page].
  - Ctrlr can route through the host, but it is off by default and a
    reported bug duplicates SysEx in the plugin builds
    ([ctrlr#33](https://github.com/RomanKubiak/ctrlr/issues/33)).
- **Port contention is the recurring failure:**
  - "most of the VST hosts are set to grab all MIDI ports on startup … many
    MIDI interfaces … are not multi-client" [ST].
  - Sound Quest requires multi-client drivers [MQ].
  - MOXF has "Port Open Error … quit any application software which may be
    using the … Port" [YMOXF p.57].
  - ESP: uncheck Port 4 in the DAW [YESP p.7].
  - Overbridge: disable USB-MIDI to the device in Ableton [OBM App. A].
- **Roland's answer for the INTEGRA-7 is a second USB port.** The editor talks
  on "INTEGRA-7 CTRL" while the DAW keeps "INTEGRA-7" [RIE p.10]. This needs
  the device's USB Driver setting on VENDER [RIE download pages]. Whether the
  CTRL port exists with the GENERIC (class-compliant) driver: UNCONFIRMED.

### Instance granularity

- **One instance per device, without exception, for multitimbral synths.**
  - Virus: one VC for all 16 parts, with per-part outputs [VSG p.16, p.18].
  - MOTIF: one editor for all 16 parts [YXS p.3].
  - MONTAGE Connect: one per Performance [YMC].
  - Roland INTEGRA-7 Editor: one window with a part-select area [RIE p.3].
  - Overbridge: one per device, with per-track outputs [OBM §6.11].
- No precedent has per-part, per-drum-key or per-effect instances. Ableton's
  Drum Rack style has no hardware precedent among these. The only effect
  plugin is Analog Heat, which is a separate hardware effects unit [OBM;
  OBRN].

### Parameters

- **Counts:**
  - Virus: "most of the parameters"; VST3 groups them by part and needs no
    mapping [VUM p.56; VKVR].
  - Overbridge: all sound, FX and macro parameters [OBM §6.1.2].
  - PlugSE OB-X8: "more than a thousand" [ST].
  - Midi Quest: "hundreds or even thousands" [MQ].
  - Ctrlr: a fixed table, 64 by default, configurable [CT
    `CtrlrProcessor.cpp` l.28, l.78–83].
  - Yamaha exposes effectively none. MONTAGE Connect's lanes are "not
    operational" [YMC]; ESP has only an internal 128-slot MIDI learn
    [YESP p.18].
- **Parameters whose meaning changes:** no source documents naming per type
  or generic slots.
  - Access *excludes* "oscillator types which switch algorithms" from
    automation, because changing them mid-note causes artefacts [VSG p.20].
  - Overbridge's editor shows different parameters per machine [OBM §6.3.1].
    How the host names react: UNCONFIRMED. Its release notes fixed machine
    changes not showing "until the initial synchronization was completed"
    [OBRN].
  - Ctrlr's index table has no mechanism for it [CT].
- **Automation writes the device's edit buffer.** Overbridge warns the user to
  save the kit on the device [OBM §6.1.2].

### Recall

- **The project holds the state and pushes it on load:**
  - Virus pushes everything with a progress bar ("Configuring Virus
    Control"). Access sells it as removing "sysex dumps and sound mismatch
    problems", even if onboard RAM/ROM sounds change [VSG p.15; VTI].
  - MONTAGE Connect bulk-reads on save and bulk-dumps to the edit buffer on
    open [YMC].
  - Midi Quest's "Total Recall" is optional, with send triggers and a
    spacing between SysEx sends [MQ].
  - Ctrlr has an opt-in "Send MIDI snapshot after panel load" [CT].
- **Conflict handling when the device changed in the meantime:**
  - Overbridge prompts on the device: USE PLUGIN STATE / USE DEVICE STATE /
    CANCEL RECALL. Recall is opt-in, with optional sync on save [OBM §6.1.5].
  - Yamaha's Auto Sync and Studio Manager Recall ask the direction (To/From
    Hardware) and can run automatically on load [YXS p.41; YSM p.5, p.7].
    Yamaha warns that auto-start "will cause unexpected data loss" [YXS p.41].
  - Studio Manager also lets the user choose which data items take part in
    recall [YSM p.8].
- **Roland is device-centric:** READ from the device, WRITE to device memory,
  and edits are lost on Studio Set change [RIE p.2, p.11–12]. A Ver.2.0.4
  known issue mentions "the next total recall", so some project recall
  exists. Its scope is UNCONFIRMED.
- **Pitfalls:**
  - Bulk dumps interrupting playback [YMC thread].
  - Cubase Auto Save breaking the editor's state [YMOXF p.12].
  - "VST3 plug-ins were not saving their state reliably" [VKVR].

### Effects as separate plugins

No precedent models a synth's internal effects as separate plugins. They are
pages or tabs of the device plugin [VUM p.70; OBM §6.2–6.9; YXS; RIE p.3].
The one exception is a dedicated effects *device*, Analog Heat, which gets an
insert-effect plugin [OBM; OBRN]. Studio Manager likewise has a separate editor
per FX *device* [YSM].

### Known host problems

- **Port contention between DAW and plugin**, worst on Windows
  non-multi-client drivers [ST; MQ; YMOXF; YESP; RIE].
- **Offline bounce:** "Only real time bouncing will work" [VSG p.19]; Cubase
  "Fast Bounce" must be off [VSG p.14]. Overbridge: UNCONFIRMED.
- **Latency compensation:**
  - Pro Tools LE needs manual adjustment [VSG p.13].
  - Overbridge's sequencer drifts unless its sync is used, and song-position
    sync is "best effort" [OBM §8.1].
- **Host support narrowed to a list:**
  - MOTIF Editor VST runs in Cubase only [YXS; YXF].
  - MONTAGE Connect runs in Cubase, Live and Logic only [YMC].
  - Overbridge: "Not all DAWs are equally supported" [OBM §3.3.1].
- **SysEx through the host:** Ctrlr's duplicated SysEx in plugin builds
  ([ctrlr#33](https://github.com/RomanKubiak/ctrlr/issues/33)). Sonar
  doesn't give port names to plugins [MQ].
- **Device offline:**
  - The device must be on before the host starts [VSG p.16].
  - "The plugin only works for as long as your … device remains connected"
    [OBM §6].
  - MOTIF drops to OFFLINE when unplugged [YXS p.12].

## What this means for the map

- **[#341 device owner](https://github.com/sigma/integral/issues/341):**
  - Overbridge validates the leaning toward a separate service: one Engine
    owns the link, plugins and standalone editors are clients over local
    TCP, and the device OS version is pinned to the Engine version.
  - Every in-plugin precedent suffers port contention. A service that owns
    the port removes it.
  - Roland's CTRL/main port split suggests an alternative to tunnelling
    notes through a virtual port: the DAW keeps "INTEGRA-7" for notes, and
    the service owns "INTEGRA-7 CTRL" for SysEx. That depends on the VENDER
    driver mode; GENERIC is UNCONFIRMED. It also doesn't serve the wireless
    link, which the Bridge already owns.
- **[#342 plugin family](https://github.com/sigma/integral/issues/342):** per
  Part, per drum key and per effect instances have **no precedent**. Every
  vendor ships one instance per device and refuses or warns against a second.
  The per-Part design is new ground and needs its own justification, for
  example per-track automation lanes and outputs. Effect plugins only have
  precedent for separate effect *hardware*.
- **[#343 parameters](https://github.com/sigma/integral/issues/343):**
  exhaustive exposure has precedent at the thousand-parameter scale
  (PlugSE, Midi Quest, Virus). No primary source answers the question of
  parameters whose meaning changes with a type. The one documented policy,
  from Access, is to exclude algorithm switches from automation. This ticket
  must decide it from first principles.
- **[#344 recall](https://github.com/sigma/integral/issues/344):**
  - Push-on-load is the norm.
  - Overbridge's three-way prompt (plugin state / device state / cancel)
    and Yamaha's direction dialog are the two models for a device edited
    meanwhile.
  - Throttle and space the dump: Midi Quest's tick separation; Yamaha's
    playback-interrupting dumps are the failure case.
  - Don't make recall silent and destructive by default; Yamaha warns
    against exactly that.
