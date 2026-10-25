# sequencer (SMF codec)

## Overview

Private submodule of [`sequencer`](../sequencer.rs) that saves and loads a
step-sequencer `Pattern` as a Standard MIDI File, per
[ADR 0001](../../../../docs/adr/0001-pattern-persisted-as-smf.md). The whole
`sequencer` module exists only with the `sequencer` Cargo feature (which
enables `smf` and `midly`), so it never ships in the VST.

File layout: SMF type 1, PPQ 96 (one Step, a 16th note, is 24 ticks). Track 0
is a conductor track with the tempo and a 4/4 time signature; then one `MTrk`
per Track, in Part order, named `Part N`.

## Interface

- `smf.rs`
  - `SmfError` (re-exported as `sequencer::SmfError`): `Parse(midly::Error)`,
    `Format`, `Timing`, `TrackCount { expected, found }`,
    `OffGrid { track, tick }`, `Length { track, steps }`,
    `Unpaired { track, pitch }`.
  - `Pattern::to_smf(&self, state: &MixerState) -> Vec<u8>`: encode. Each
    Track's channel comes from the Part's receive channel in `state`; pass
    `MixerState::default()` when no device is connected.
  - `Pattern::from_smf(bytes: &[u8], part_count: u8) -> Result<Pattern, SmfError>`:
    decode a file written by `to_smf`. Track identity comes from `MTrk`
    order; file channels are ignored.

## Invariants

- Only type 1 (`Format::Parallel`) with metrical PPQ 96 is accepted
  (`Format` / `Timing` errors otherwise).
- Track count minus the conductor must equal `part_count` (`TrackCount`).
- Every Note On, Note Off and End-of-Track tick is a multiple of 24
  (`OffGrid`); other events are ignored wherever they fall; a Track's length is the
  tick of its End-of-Track and must be 1..=64 Steps (`Length`).
- Every Note On (velocity > 0) needs a matching Note Off or velocity-0 Note On
  (`Unpaired`). A repeated Note On of an open pitch cuts the open note at
  the retrigger (at least 1 Step); a Note Off with no open note is ignored.
- Encode clamps notes to the Track end and to the next onset of the same
  pitch, and drops notes starting at or after the end; on one tick, Note Off sorts before Note On.
- Tempo is `60_000_000 / bpm` (rounded); decode takes the first Tempo meta in
  track 0, flooring tempo at 1.
- Track numbers in errors are 1-based, matching the `Part N` names.

## Side Effects

- None (pure). Works on in-memory buffers via `midly`; callers do file I/O.

## Verification

- Run: `cargo test -p integral-core --features sequencer`
