# integral-wasm/src

## Overview

A thin `wasm-bindgen` layer over `integral-core` for the web frontend. Its
centre is `WasmDeviceState`, which wraps `DeviceState` and its queue of
outbound SysEx. Rust never calls JS. JS pulls the bytes to send (`drain`) and
pushes in the DT1s it receives (`handleDt1`). The sequencer, the Librarian
machines and the Expansion loader work the same way: JS polls for the next
action and sends the bytes itself.

Structured data crosses the boundary through `serde_wasm_bindgen`. Some static
catalogs cross as JSON strings, and simple types cross as `wasm_bindgen`
enums and structs with getters.

## Interface

Names below are the JS names that `wasm-bindgen` exports.

- `lib.rs`
  - `WasmDeviceState`:
    - `new(device_id)` creates it.
    - `drain(now_ms)` returns the next message to send.
    - `sendRaw(key, bytes)` queues raw SysEx.
    - `handleDt1(addr, data, now_ms)` feeds in a received DT1, taking the
      Part tone selects of dumps the Device service relayed (a tone changed
      on the device), which no read of the state asked for.
    - `readState()` returns the state as a `MixerState`-shaped object.
    - Setters: `setPart*`, `setMixerField`, `setStudioSetField`,
      `setChorusParam`, `setReverbParam` and the per-engine `set*Param`.
    - Block parsers: `applySnaCommon`, `applySndCommon`, `applySndNote`
      and `snsTone`. `reloadSnsTone` and `loadMixer` queue tracked RQ1
      reads.
    - Edits and navigation: `renameTone`, `renameStudioSet`,
      `changePartTone`, `switchStudioSet`, `previewStart`, `previewToggle`,
      `previewPart`. Load status: `mixerLoading`, `mixerFailedBlocks`.
  - SysEx helpers:
    - `identity_request`, `parse_identity_reply` and `DeviceIdentity`
    - `build_rq1`, `parse_dt1` and `Dt1Message`
    - `build_tone_catalog_request`, `build_studio_set_catalog_request` and
      `parse_catalog_entry`
    - `build_tone_write_request`, `build_studio_set_write_request`,
      `parse_write_reply` and `write_timeout_ms`
  - Spec data:
    - `deviceSpecJson`, `factoryTonesJson` and `snaInstsJson`
    - Per-engine block addresses and sizes (`pcms_common_address` …
      `pcmd_partial_size`), `sns_partial_count`, `mfx_block_size`,
      `is_drum_kit_bank`
    - Family colours: `familyColour`, `mfxHeaderFamily`
    - `*_spec`, `WasmFieldSpec` (its `catalog` and `controlSource` name a
      picker's list), `catalog_options` and `mixer_field_spec`
    - `tone_bank_groups`, `WasmToneBank` and `user_tone_bank`
    - `mfx_params`, `mfx_type_names`, `chorus_params`, `reverb_params` and
      `WasmFxParamDef`
- `display.rs`: `display_binding`, `WasmDisplaySlot`, `DisplayKind` and
  `DisplayProp`.
- `relevance.rs`: the `*_relevance` functions and `Relevance`.
- `part_settings.rs`: the Part settings registry (`PartSetting`,
  `PartSettingGroup`, `part_setting_fields`, `part_setting_spec`, `_name`,
  `_label`, `_group`, `part_setting_group_label`) and the
  `WasmDeviceState` methods `partSetting` and `setPartSetting`.
- `routing.rs`: `WasmDeviceState.effectiveRouting()` returns the state's
  `EffectiveRouting` (TypeScript custom section).
- `system.rs`: System Common. `system_fields`, `system_groups`,
  `system_group_label`, `system_field_spec` (`WasmSystemFieldSpec`, with
  `formatValue`, `formatNumber` and `parseValue`), `system_volatile_notice`
  and `parseSystemCommon` (`WasmSystemCommon`, with `studioSetInForce`);
  `system_common_address` and `system_common_size` for `requestData`; the
  SYSTEM-or-STUDIO SET pairs, `SystemOrStudioSet`, `system_pair_selector`
  and `system_control_source_field`; `buildSystemCommonRequest` and
  `setSystemField` on `WasmDeviceState`.
- `expansion.rs`:
  - The catalog and slot helpers: `expansionCatalog`, `stageExpansionSlot`
    and `expansionSlotsAddress`.
  - Load methods on `WasmDeviceState`: `planExpansionLoad`,
    `startExpansionLoad`, `nextLoadAction` and `handleExpansionDt1`.
  - `WasmLoadAction` and `WasmLoadPlan`.
- `librarian.rs`:
  - `librarianTypes` lists the Librarian types.
  - `Pack` (Rust `WasmPack`) reads Packs and writes them with `toSvd`.
  - `planImport` returns an `ImportPlan` (Rust `WasmImportPlan`). Edit it with `assign`,
    `exclude` and `include`.
  - `PackExport` and `PackImport` (Rust `WasmPackExport`,
    `WasmPackImport`) are driven by `nextAction`,
    `onRead`, `onWriteReply`, `onTimeout`, `progress` and `cancel`.
    `nextAction` returns a `LibrarianAction`.
  - The file also carries a TypeScript custom section.
- `initialize.rs`: `Initialization` (Rust `WasmInitialization`) sends one
  Init, built with `studioSet`, `soundCtrl`, `part`, `tone` or `partial`
  (a partial index for a tone, the key number for a kit). It is driven like
  the Librarian machines, by `nextAction`, `onRead`, `onWriteReply`,
  `onTimeout` and `cancel`. `initFinishedAddress` is the address of the
  "Init finished" DT1.
- `sequencer.rs`: `WasmSequencer` with `play`, `due(device, now_ms,
  horizon_ms)` (returns `{tMs, bytes}[]`), `stop`, `panic`, `toSmf`,
  `toSmfOffline` and `loadSmf`.
- `studio_set_settings.rs`: the Studio Set settings registry:
  `StudioSetSetting`, `studio_set_setting_spec`, `studio_set_setting_count`,
  `studio_set_field_name`, `studio_set_field_label` and
  `phase_lock_notice`, and `WasmDeviceState.setStudioSetField`.
- `waves.rs`: `waveGroups`, `WaveGroup` (Rust `WasmWaveGroup`),
  `waveGroupWavesJson`, `waveName` and `waveStereoPartner`.

## Invariants

- Rust never calls into JS. No `js_sys::Function`, `Closure` or `web_sys`
  callbacks are used.
- JS owns the clock: every timed call takes a monotonic `now_ms`.
- Part indexes must be 0-15. Partial indexes are checked for each tone type.
  Out-of-range indexes throw `JsError`.
- `handleDt1` ignores addresses shorter than 4 bytes and uses the first 4
  of a longer one. `onRead` and `onWriteReply` reject any address that is
  not exactly 4 bytes.
- Expansion slots cross as 4 bytes, one per slot A-D (0 = OFF).
  `startExpansionLoad` throws if the slots are unknown, if a load is already
  running, or if the target is invalid.
- Librarian types cross the boundary as CLI names (for example `sn-synth`).
  Packs, plans and machines stay in Rust behind handles.
- serde structs use `camelCase`. Some error paths return `null`, or `"[]"`
  for the JSON helpers, instead of throwing.

## Side Effects

- Setters change the mixer state held in Rust and queue merged, throttled
  SysEx.
- An Expansion load holds the send queue until it finishes.
- `WasmInitialization` resets a Temporary area and announces none of its
  values: JS reads the area back after `Done`.
- `WasmPackImport` writes User memory and the Temporary areas through the
  device, so JS must refresh the mixer state after `Done`.
- `WasmSequencer` keeps the play state and returns each due event only once.
- No I/O or global state. All state lives in Rust handles that JS owns.

## Verification

- Run: `cargo test -p integral-wasm`
- Run: `just build-wasm`
- Run: `just test-web` (Vitest against the packed WASM)
