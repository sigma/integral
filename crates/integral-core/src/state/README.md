# state

## Overview

Plain-data runtime mixer state for the INTEGRA-7 (Parts, EQ, Chorus/Reverb,
Motional Surround, Drum Comp+EQ) and the parsers that turn raw SysEx dump
bytes into it. `DeviceState` (in [`device.rs`](../device.rs)) owns a
`MixerState` and updates it from device replies. With the `serde` feature the
types derive `Serialize`/`Deserialize` in camelCase for the WASM bridge.

## Interface

- `mod.rs`
  - `NUM_PARTS` (16).
  - `EqState`: 3-band parametric EQ (enabled, low/mid/high freq and gain, mid Q).
  - `FxState`: Chorus or Reverb block (enabled, type, level, output, output
    assign, `params: Vec<i32>`).
  - `PartState`: level, pan, mute, tone bank/PC, receive channel, sends,
    output assign, tone name and category, `eq`.
  - `SurroundPartState`, `SurroundState`: Motional Surround (16 Parts + Ext).
  - `CompEqUnit`, `DrumCompEqState`: six Comp+EQ units, global enable,
    assigned Part, output assigns.
  - `MixerState`: Studio Set name/PC, master level, solo, `parts[16]`, chorus,
    reverb, ext, master EQ, surround, drum Comp+EQ, Studio Set name cache,
    preview Part.
- `parse.rs`
  - Sizes: `PART_DUMP_SIZE` (0x2A), `PART_EQ_DUMP_SIZE` (8),
    `MASTER_EQ_DUMP_SIZE` (7).
  - `parse_part_dump`, `parse_part_eq_dump`, `parse_master_eq_dump`.
  - `parse_surround_common`, `parse_surround_part`.
  - `parse_comp_eq_unit`, `parse_comp_eq_block`, `encode_comp_eq_block`
    (6 x 14 = 0x54 bytes).
  - `decode_nib_params(data, count)`, `encode_nib_param(i32) -> [u8; 4]`:
    the 4-nibble FX parameter codec (offset 32768).

## Invariants

- Parsers never fail or panic on short input: below the expected size they
  return `Default`; `parse_comp_eq_block` defaults any incomplete unit.
- `parse_master_eq_dump` leaves `enabled` at its default; the caller sets it
  from the separate switch read.
- Switches decode as `byte == 1`.
- Nibble codec: only the low 4 bits of each of 4 bytes count, giving a 16-bit
  value minus 32768. `decode_nib_params` returns fewer than `count` values if
  data runs out; `encode_nib_param` wraps out-of-range values silently.
- `encode_comp_eq_block` is the inverse of `parse_comp_eq_block`.
- `MixerState::default()` makes Part N receive on channel N;
  `DrumCompEqState::default()` assigns Part 10.
- Documented value ranges (level 0-127, pan 64 = centre, EQ gain 0-30) are
  not enforced: fields are raw `u8`.
- Known doc drift: `parse_part_dump`'s comment says a 0x29-byte dump, but it
  requires `PART_DUMP_SIZE` (0x2A) and reads Output Assign at 0x29
  ([#437](https://github.com/sigma/integral/issues/437)).

## Side Effects

- None (pure data and byte-slice transforms).

## Verification

- Run: `cargo test -p integral-core`
