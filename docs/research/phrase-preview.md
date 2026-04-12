# Phrase Preview Research (i7-kd0.1)

## Finding: Undocumented Preview SysEx at `0F 00 20 00`

**Verified working** on the INTEGRA-7 via `integral raw-send`.

### Command Format (standard DT1)

```
F0 41 <dev> 00 00 64 12 0F 00 20 00 <part> <checksum> F7
```

| Data byte | Meaning |
|-----------|---------|
| `00` | Preview OFF |
| `01`–`10` | Preview parts 1–16 |

**Important:** Must send OFF (`00`) between switching parts.

### Verified Commands (device ID `10`)

| Function | SysEx |
|----------|-------|
| Preview OFF | `F0 41 10 00 00 64 12 0F 00 20 00 00 51 F7` |
| Preview part 1 | `F0 41 10 00 00 64 12 0F 00 20 00 01 50 F7` |
| Preview part 2 | `F0 41 10 00 00 64 12 0F 00 20 00 02 4F F7` |
| ... | pattern continues |
| Preview part 16 | `F0 41 10 00 00 64 12 0F 00 20 00 10 41 F7` |

### Address Space Context

Lives in the `0F` undocumented "editor/control" region:

| Address | Function |
|---------|----------|
| `0F 00 03 02` | Studio Set catalog query |
| `0F 00 04 02` | Tone catalog query |
| `0F 00 20 00` | **Preview trigger** |

### How It Works

Simulates pressing the hardware VOLUME knob. The device plays the built-in
phrase for the tone on the specified part, using the tone's **Phrase Number**
parameter. Preview mode (SINGLE/CHORD/PHRASE) follows the System setting.

### Source

Discovered by "Wonderer" on Roland Clan Forums via systematic address
scanning. Confirmed working on our device 2026-04-12.

## Current Webapp State

The preview button (`web/src/TopBar.tsx:65`) currently sends a single
Note On (C4, velocity 100, 500ms) via `web/src/useMixer.ts:540-550`.
This should be replaced with the SysEx preview command.

## Per-Tone Phrase Parameters (Reference)

Each tone type stores its own phrase number and octave shift:

| Tone Type    | Parameter         | SysEx Offset    | Range   | Code |
|-------------|-------------------|-----------------|---------|------|
| SN Synth    | Phrase Number     | `00 37` (4-nib) | 0-65535 | `sn_synth.rs:119` |
| SN Synth    | Phrase Oct Shift  | `00 3B`         | -3/+3   | `sn_synth.rs:121` |
| SN Acoustic | Phrase Number     | `00 1C` (2-nib) | 0-255   | — |
| SN Acoustic | Phrase Oct Shift  | `00 1E`         | -3/+3   | — |
| PCM Synth   | Phrase Number     | `00 38` (4-nib) | 0-65535 | `pcm_synth.rs:722` |
| PCM Synth   | Phrase Oct Shift  | `00 13`         | -3/+3   | `pcm_synth.rs:717` |
| PCM Drum    | Phrase Number     | `00 10` (2-nib) | 0-255   | `pcm_drum.rs:408` |
| SN Drum     | Phrase Number     | `00 12`         | 0-127   | — |

## Implementation Plan (i7-kd0.2 / i7-kd0.3)

### integral-core changes
- Add `preview::start(part: u8)` and `preview::stop()` functions that
  build the DT1 messages at `0F 00 20 00`.
- Wire into `Device` so it can be called from the WASM layer.

### webapp changes (`web/src/useMixer.ts:540-550`)
- Replace `sendNoteOn`/`sendNoteOff` with the SysEx preview commands.
- Send `preview::start(selectedPart + 1)`, then `preview::stop()` on a
  second press or after a timeout.

### Code paths to modify
- `web/src/TopBar.tsx:65` — preview button
- `web/src/useMixer.ts:540-550` — preview callback
- `web/src/MixerPage.tsx:49` — passes `mixer.preview`
