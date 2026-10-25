# integral-vst

## Overview

The DAW plugin (VST3 and CLAP, via `nih-plug`) and the `integral-app`
standalone. Both embed the web UI in a macOS webview
([ADR 0004](../../docs/adr/0004-plugin-ui-in-a-webview.md)). The crate does
not depend on `integral-core`. The embedded page runs the WASM core, and this
crate only serves the page and passes MIDI bytes between the page and the
device.

## Interface

- [`./src/README.md`](./src/README.md): the plugin (`Integral`), the
  standalone entry point and the webview editor.
- `build.rs` embeds `web/dist-plugin` (built by `just build-web-plugin`). It
  writes a sorted `(path, bytes)` table to `$OUT_DIR/web_assets.rs`, which
  `src/webview/assets.rs` serves.
- Build outputs:
  - a `cdylib` and `lib` library (the plugin)
  - the `integral-app` binary (the standalone)

## Invariants

- If `web/dist-plugin` is missing, the asset table is empty. The build still
  succeeds with a warning, so `cargo build` and clippy need no Node, and the
  editor shows a page explaining how to build the UI.
- `wry` is pinned to an exact version (`=0.57.0`), because the editor relies
  on its platform behaviour. Bump it on purpose and re-check in a host.
- `baseview` uses nih-plug's source and revision, so the workspace `[patch]`
  applies and only one `baseview` is built.
- The webview editor dependencies are macOS only.

## Side Effects

- Build time: reads `web/dist-plugin`, writes `$OUT_DIR/web_assets.rs`, and
  emits `rerun-if-changed`.
- Runtime: see [`./src/README.md`](./src/README.md) (MIDI ports, audio
  backend, window).

## Verification

- Run: `cargo test -p integral-vst`
- Run: `just build-plugin` (release, with the web UI embedded)
