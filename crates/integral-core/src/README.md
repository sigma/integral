# integral-core/src

## Overview

The portable INTEGRA-7 engine. It builds and parses Roland SysEx, holds the
device address map and the device specification layer, and keeps the mixer
state. It also carries the per-engine tone parameter maps, effect tables, the
SVD backup format, the Expansion and Wave catalogs, the Librarian, and an
optional step sequencer.

Nothing here does I/O or reads a clock. Hosts (`integral-cli`,
`integral-wasm`) pass in bytes and monotonic `now_ms` and get bytes and
actions back. The crate must build for `wasm32-unknown-unknown`.

## Interface

### Protocol and device state

- `lib.rs` is the crate root. It declares the modules and defines
  `ToneParseError`. `sequencer` is compiled only with the `sequencer`
  feature.
- `address.rs` defines `Address` and `DataSize`, two 4-byte values with
  7 bits per byte and carry between bytes. It also holds the
  `studio_set_part` and `studio_set_common` helpers and the base constants.
- `sysex.rs`:
  - `build_dt1` and `build_rq1` build messages, and `checksum` computes the
    Roland checksum.
  - `parse_dt1` returns a `Dt1Message` or a `Dt1ParseError`.
  - `identity_request` and `parse_identity_reply` handle the Identity
    exchange. `DeviceIdentity::is_integra7` checks the reply.
- `params.rs` holds the mixer, FX, EQ, Comp+EQ and surround addresses and
  sizes, plus `ToneType`, `tone_type_from_bank_msb`, `temporary_tone_base`
  and `part_address`.
- `device.rs`: `DeviceState`, the state machine that every host drives.
  - `send_dt1` merges a queued DT1 with a later one to the same address.
    `send_raw` only appends to the queue.
  - `drain(now_ms)` returns the next message once the throttle allows it.
  - `handle_dt1` applies an incoming DT1 to the state.
  - `load_mixer` and `load_sns_tone` start tracked reads that are retried.
  - `start_expansion_load` and `next_load_action` run an Expansion load.
  - `set_mixer_field` and the typed `set_*` setters edit parameters.
- `device_spec.rs`: the device specification layer. It holds
  `INTEGRA7: DeviceSpec` (part count and other device constants), the
  `USER_MEMORY` layouts (`UserMemorySpec`, `BlockRun`) and `KeyRange`.
- `midi_bridge.rs`: `RequestTracker` matches RQ1 requests to their DT1
  replies and expires the old ones.
- `wireless.rs`: the WNA1100-RL wireless link protocol, with no I/O.
  - `rddp_probe` and `parse_rddp_reply` do RDDP discovery on UDP port 9314.
  - `Frame` and `FrameDecoder` handle session framing.
  - `pack_usb_midi`, `Reassembler` and `encode_midi` handle USB-MIDI packing.
  - `Session` and `Reconnect` handle the session timers and reconnection.
- `catalog.rs`, `tone_catalog.rs`, `initialize.rs` and `user_write.rs` build
  the undocumented commands:
  - catalog name queries at `0F 00 03 02` and `0F 00 04 02`, with
    `ToneCatalog` paging through the results
  - Init of a Temporary area at `0F 00 18 TT`
  - User write at `0F 00 10 00`
- `tone_banks.rs` holds `TONE_BANK_GROUPS`, `find_bank` and `user_bank_for`.
  `factory_catalog.rs` provides `factory_tone`, generated at build time.
- `parse_helpers.rs` decodes and encodes names and nibbles. `bitstream.rs`
  provides `BitReader` and `BitWriter`. `block_field.rs` defines the
  `BlockField` trait. `family.rs` defines `ParameterFamily` and its `Rgb`
  palette.
- `sequencer.rs` (`sequencer` feature only) defines `Note`, `Track`,
  `Pattern`, `TimedMidi` and `Sequencer` (`due`, `stop`, `panic`).
  [`./sequencer/README.md`](./sequencer/README.md) covers the SMF codec.

### Tone and effect parameters

- `param_registry.rs`:
  - `ParamDef` and `ParamFormat` describe a parameter and its display
    format, and `format_value` formats a value.
  - `FieldCatalog` and `CatalogOption` are the named value lists.
  - `MixerField` and `MixerScope` describe a mixer parameter, and
    `mixer_field_address` and `mixer_param` resolve it.
- `field_spec.rs`: `FieldSpec` holds the min, max, default, format and family
  of a field. Per-block lookups are named `pcms_*_spec`, `pcmd_*_spec`,
  `sns_*_spec`, `sna_common_spec` and `snd_*_spec`. `pcms_partial_sync` and
  `sns_partial_sync` describe tempo sync: `sns_partial_sync` pairs a
  switch field with a note field, and `pcms_partial_sync` returns an
  in-range sync (labels and a note offset).
- Each engine module holds an address map, field enums, state structs, and
  parse and encode functions:
  - `pcm_synth.rs`: PCM Synth Tone (`pcms_*`).
  - `pcm_drum.rs`: PCM Drum Kit (`pcmd_*`, keys 21-108, 4 WMT layers).
  - `sn_synth.rs`: SN Synth Tone (`sns_*`). `SnSynthToneBlocks` assembles
    the blocks.
  - `sn_acoustic.rs`: SN Acoustic Tone (`sna_*`). `sna_inst_params.rs` adds
    the per-instrument Modify Parameters.
  - `sn_drum.rs`: SN Drum Kit (`snd_*`, keys 27-88).
  - `pcm_structure.rs`: `routing` for PCM Structure Types.
- Effects:
  - `mfx.rs`: `mfx_address`, `tone_mfx_block_address`, `parse_mfx_block`,
    `encode_mfx_block` and `encode_mfx_param`.
  - `mfx_params.rs`: the generated tables for all 68 MFX types, read through
    `mfx_type_def`.
  - `fx_params.rs`: `FxParamDef` for the chorus and reverb parameters.
- UI support:
  - `display.rs`: `display_binding` maps a `DisplayKind` and a `DisplayHost`
    to the `DisplaySlot`s a Display reads.
  - `relevance.rs`: `Relevance` (`Active`, `SwitchedOff`, `Irrelevant`),
    computed by the `*_relevance` predicates.
  - `lfo_rates.rs` (`pcm_lfo_hz`, `sn_lfo_hz`) and `velocity_curve.rs`
    (`velocity_curve`).

### Files, Expansions and Waves

- `svd.rs`:
  - `SvdFile` provides `parse`, `write` and `sound`, and `SvdChunk<K>` holds
    one chunk.
  - `ChunkType` and `SOUND_CHUNKS` describe the chunk types.
  - `SvdFamily::of` tells INTEGRA-7 files from XV files, and `parse_xv` and
    `XvChunkType` read the XV ones.
  - `SvdError` is the error type.
- `svd_convert.rs`: `svd_to_sysex`, `sysex_to_svd`, `pack_sections`,
  `unpack_sections` and `split_common_mfx`, driven by the tables in
  `svd_specs.rs` (`SvdToneSpec`, `SvdSection`, `ParamBits`).
- `expansions.rs`: `EXPANSIONS` (19 entries), `by_load_value`, `by_name` and
  `expansion_of_bank`.
- `expansion_slots.rs`: `ExpansionSlots`, `build_read_request` and
  `parse_read_reply`.
- `expansion_load.rs`: `plan` and `LoadPlan`, plus the `LoadMachine` with its
  `LoadAction` and `LoadOutcome`.
- `mixer_load.rs`: `MixerBlock` lists the mixer reads (`initial`, `request`,
  `apply`).
- `waves.rs`: `Wave`, `WaveGroup`, `waves`, `name`, `families` and
  `stereo_partner`.

### Subdirectories

- [`./librarian/README.md`](./librarian/README.md): Packs, export, import
  planning and storage, and Foreign file conversion.
- [`./state/README.md`](./state/README.md): the `MixerState` data model and
  the dump parsers.
- [`./sequencer/README.md`](./sequencer/README.md): the Pattern SMF codec
  (`sequencer` feature).

## Invariants

- Messages:
  - `build_dt1` and `build_rq1` always write Roland ID `0x41` and Model ID
    `00 00 64`.
  - `parse_dt1` rejects a wrong model, a wrong manufacturer, a non-DT1
    command, missing framing, a message under 14 bytes, and a checksum
    mismatch.
  - The checksum covers the address plus the data (DT1) or the size (RQ1):
    `(128 - sum % 128) % 128`.
- Device ID is a parameter everywhere. Identity Request uses broadcast
  `0x7F`. Neither `parse_dt1` nor `DeviceState` filters by device ID.
- `DeviceState` send queue:
  - Throttle: `THROTTLE_MS = 40.0` (stricter than the documented 20 ms
    minimum), and the first send goes out at once.
  - DT1s merge on key `dt1:{address}`. Raw sends never merge.
  - The queue is held while an Expansion load runs.
- `DeviceState` incoming DT1s:
  - An incoming DT1 that matches a local send within `ECHO_SUPPRESS_MS = 150`
    counts as an echo and is ignored. The window starts at the first drain
    after the send was queued, which can be before that DT1 itself leaves
    ([#439](https://github.com/sigma/integral/issues/439)). Block replies are
    never treated as echoes.
- `DeviceState` tracked reads are resent after 2000 ms and abandoned after
  2 retries.
- No code here enforces the 256-byte SysEx packet limit. Callers rely on
  block sizes from the address map staying under it
  ([#436](https://github.com/sigma/integral/issues/436)).
- Address bytes stay within `0x00-0x7F`, and arithmetic carries into the next
  byte. `DataSize::from_value` rejects values of `2^28` or more.
- Device constants (part count 16, drum key ranges, Expansion slots) come
  from `device_spec.rs` and the engine maps. Views and commands must not copy
  them. Exceptions inside core: `handle_dt1` loops over a literal `0..16`,
  and `state::NUM_PARTS` is its own `16`
  ([#438](https://github.com/sigma/integral/issues/438)). `Pattern::new` takes `DeviceSpec::part_count`.
- Field specs:
  - Each spec function matches its field enum exhaustively, so a new field
    fails to build until it has a spec.
  - Defaults come from each engine's INIT block.
- Drum keys outside the range give `KeyOutOfRange`. `pcmd_wmt_byte`
  const-asserts `wmt < 4`.
- MFX:
  - Every tone type has its MFX block at offset `00 02 00`.
  - The params are 32 nibblized 4-byte values, stored as raw minus 32768.
  - There are exactly 68 MFX types.
- SVD:
  - Parsing rejects a bad `SVD1` signature, the wrong model (`MI69` for the
    INTEGRA-7), unknown chunks, and truncated files.
  - Empty chunks are still written with a zone header.
- ExPCM fills all four Expansion slots. It loads as `13 13 13 13` and reads
  back as `13 14 15 16`.
- `Relevance::SwitchedOff` fields stay editable. Relevance rules live here,
  never in UI layouts.
- Known doc drift: the `device.rs` module doc example calls
  `handle_incoming`, which does not exist. The entry point is `handle_dt1`
  ([#437](https://github.com/sigma/integral/issues/437)).

## Side Effects

- None at runtime. All mutable state lives in values the caller owns
  (`DeviceState`, `RequestTracker`, `ToneCatalog`, `Session`, `LoadMachine`,
  `Sequencer`).
- Static data is immutable. A few `OnceLock` caches are filled lazily, once
  (INIT blocks, catalog name lists).
- Several modules `include!` tables that [`../build.rs`](../build.rs)
  generates in `OUT_DIR`: the factory catalog, tone categories, Waves, SN
  Drum and SN Acoustic instruments, and the Foreign Wave and effect maps.

## Verification

- Run: `cargo test -p integral-core`
- Run: `cargo test -p integral-core --all-features`
- Run: `just build-wasm` (wasm32 must compile)
