# integral-wasm

## Overview

WASM bindings that expose `integral-core` (with the `serde` and `sequencer`
features) to the TypeScript/React frontend in `web/`. The crate builds as
`cdylib` + `rlib` with `wasm-bindgen`, `serde-wasm-bindgen` and `serde_json`.
The React layer is a thin view, and state flows from this core.

## Subdirectories & Boundaries

- src: [`./src/README.md`](./src/README.md) - `WasmDeviceState` and the
  pull-based SysEx queue, spec and catalog exports, the Librarian, Expansion
  loading, the sequencer, relevance and Display bindings.

## Verification

- Run: `cargo test -p integral-wasm`
- Run: `just build-wasm`
- Run: `just test-web`
