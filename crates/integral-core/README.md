# integral-core

## Overview

The portable Rust library behind every Integral front end. It holds the
INTEGRA-7 SysEx engine, the address maps, the device specification layer, the
mixer and tone state, the Librarian, and the SVD file format. It does no I/O
and never reads a clock, so it builds for native targets and for
`wasm32-unknown-unknown`. The users are `integral-cli` (with the `serde` and
`smf` features) and `integral-wasm` (with `serde` and `sequencer`).
`integral-vst` does not depend on it.

## Interface

- [`./src/README.md`](./src/README.md): the full module contract
  (`DeviceState`, `sysex`, `params`, `device_spec`, tone engines, effects,
  SVD, Expansions, Waves).
- Cargo features:
  - `serde` derives `Serialize` and `Deserialize` on the state and Librarian
    types.
  - `smf` reads Packs from Standard MIDI Files (adds `midly`).
  - `sequencer` adds the step sequencer and its SMF codec (implies `smf`). It
    is off by default so it never ships in the VST.
- `build.rs`: the build script generates the data tables listed under Side
  Effects.

## Invariants

- `integral-core` must compile for `wasm32-unknown-unknown`.
- Dependencies stay minimal: `thiserror`, plus `serde` and `midly` behind
  their features.
- `build.rs` checks the Wave data:
  - Each `docs/waves/*.json` must hold exactly the Wave count listed in
    `docs/waves/README.md`.
  - Wave Numbers must run from 1 to N with no gaps.

  If either check fails, the build panics.
- Device constants live in the spec layer (`device_spec`, `param_registry`,
  tone bank tables). Front ends must consume them, not copy them.

## Side Effects

- Build time: `build.rs` reads these files:
  - `docs/sounds/*.json` (presets, GM2, ExSN, ExPCM, categories)
  - `docs/waves/*.json` and `docs/waves/README.md`
  - `docs/sn-drum-insts/insts.json` and `docs/sn-acoustic-insts/insts.json`
  - `docs/research/jv-wave-map.json` and `docs/research/foreign-fx-map.json`

  It writes the generated `*_data.rs` tables to `OUT_DIR` and emits
  `rerun-if-changed` for each input.
- Runtime: none. The library has no I/O.

## Verification

- Run: `cargo test -p integral-core`
- Run: `just build-wasm`
- Repo gate: `just check`
