# librarian::foreign

## Overview

Reads a `Pack` from any supported file: an INTEGRA-7 SVD, an XV-series SVD,
raw SysEx (`.syx`), or the SysEx events of a Standard MIDI File (`.mid`, only
with the `smf` feature). SysEx is split into Roland DT1 frames of any model.
Native INTEGRA-7 DT1s that cover a whole tone or Studio Set become Pack
entries directly. Foreign JV/XP (Model ID `6A`) and XV (Model ID `00 10`)
Patches convert to PCM Synth tones, and their Rhythm Sets to PCM Drum Kits,
using Wave and effect maps generated at build time. Each converted entry gets
a Conversion report, and the file gets a file-level report.

## Interface

- `mod.rs`: `Pack::read` detects the format from the first bytes and routes
  to the SVD, SysEx or SMF reader. `PackRead` holds `format`, `pack`,
  `file_report`, and per-entry `reports`, `sources` and `effects`.
  `PackFormat` is `Svd | XvSvd | Sysex | Midi`, and `label(origin)` gives its
  display name.
- `frame.rs`: Roland framing for any model.
  - `Frame` is a parsed DT1 with a variable-length model, a 4-byte address,
    and the data.
  - `parse` reads one `F0..F7` message and returns `FrameError`
    (`NotDt1 | BadChecksum`) on failure.
  - `split` and `Messages` cut a byte stream into messages and flag
    incomplete bytes.
- `integra.rs`: native INTEGRA-7 DT1s. Finds the Temporary area or User slot
  that the DT1s write and rebuilds the entry blocks.
- `jv.rs`, `jv_rhythm.rs`: convert a JV/XP Patch to a PCM Synth tone and a
  Rhythm Setup (keys 35-98) to a PCM Drum Kit. Both are driven by mapping-row
  tables.
- `xv.rs`, `xv_rhythm.rs`: convert an XV Patch (read as an XV-5080's) and an
  XV Rhythm Set (keys 21-108). Each is a byte copy plus fix rows, from SysEx
  or bit-packed `.svd` entries.
- `pcm_tone.rs`, `pcm_kit.rs`: shared conversion targets. They hold the INIT
  tone or kit, the block layout, the Integra-only reserves, and the wave and
  WMT helpers.
- `fx_map.rs`: the Foreign effect map (JV EFX and XV MFX to INTEGRA-7 MFX).
  Converts parameters, matches control assigns, and detects tempo sync.
- `wave_map.rs`: `ForeignWave`, `ForeignWaveGroup`, and `WaveTarget` with
  `Confidence` (`Low < Medium < High`), mapping JV/XP and SR-JV80 Waves to
  INT or SRX Waves.
- `report.rs`:
  - `ConversionItem` (`WaveUnmapped`, `WaveApproximate`, `EffectDropped`,
    `EffectSubstituted`, `ParamDropped`, `RoutingDropped`, `NotApplied`).
  - `ConversionReport` and `lossy_count`.
  - `FileItem` (`BadChecksum`, `Skipped`, `Duplicates`, `Unreadable`).
  - `Skipped`, `SourceModel`, `ForeignEffect` and `EffectFamily`, and
    `Location`.
- `studio_set_fx.rs`: `StudioSetEffect`, `FxBlock` and `effects_dt1s`. They
  turn Foreign Chorus/Reverb into Temporary Studio Set effects, which are used
  to audition a patch.
- `tests.rs` holds the tests and is not part of the interface.

## Invariants

- DT1 framing:
  - The message is `F0 41`, then a device ID (ignored), then a Model ID made
    of leading `00` bytes and one non-zero byte, then `12`, a 4-byte
    address, at least one data byte, the checksum, and `F7`.
  - A bad checksum gives `BadChecksum`.
- `split`:
  - A new `F0`, or a status byte `>= 0x80` other than `F7`, inside a message
    drops that message and marks the stream incomplete.
  - Stray bytes outside a message also mark it incomplete.
- Block assembly works by address, not by message boundaries:
  - Later bytes overwrite earlier ones.
  - The result is `None` unless every block is fully covered.
- Format detection order:
  - SVD family first: Integra is read natively, XV goes to the XV reader,
    and a JV/XP SVD is rejected.
  - Then a leading `F0` means SysEx, and `MThd` means SMF.
  - Anything else is an unknown format.
- `PackRead.reports`, `sources` and `effects` are indexed like
  `pack.entries`. Native entries have empty reports and effects.
- Duplicates and failures:
  - A converted unit byte-identical to an earlier entry is not added, and is
    counted as a duplicate.
  - Failed conversions are tallied as `Skipped { kind, count }`.
- An entry is clean when its report holds only `NotApplied` items.
- Required blocks:
  - A JV Patch with a Common shorter than the JV-1080's, or with a missing
    block, is `Incomplete`.
  - JV Rhythm needs its Common and at least one Note.
  - XV Rhythm needs its Common, MFX, Chorus and Reverb.
  - XV `.svd`: only Patch entries convert.
- Converted tones carry INTEGRA-7 reserve values: Output Assign TONE (13) and
  MFX Dry Send 127. An effect type with no target leaves the MFX THRU.

## Side Effects

- None at runtime: pure transforms over byte slices. `effects_dt1s` only
  builds DT1 bytes, and the caller sends them.
- Build time: `fx_map.rs` and `wave_map.rs` `include!` `OUT_DIR` data that
  [`build.rs`](../../../build.rs) generates from
  `docs/research/foreign-fx-map.json` and `docs/research/jv-wave-map.json`.

## Verification

- Run: `cargo test -p integral-core --features smf`
