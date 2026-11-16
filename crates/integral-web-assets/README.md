# integral-web-assets

## Overview

The web build the plugin editor and the Device service serve: `web/dist-plugin` (`vite build
--mode plugin`: no service worker, base `/`), embedded at build time.

## Interface

- `serve(path)` returns `Served { status, mime, body }` for a request path
  (the URI's path, no query). `/` is `index.html`; a path with an empty, `.`
  or `..` segment, or one not in the build, is a `404`.
- `serve_from(assets, path)` does the same from a given table, for tests.
- `mime(path)` is the `Content-Type` of a file by its extension (WASM is
  `application/wasm`, which `WebAssembly.instantiateStreaming` requires).
- `ASSETS` is the embedded table, sorted by path.

## Invariants

- If `web/dist-plugin` is missing, the table is empty and `index.html` is a
  page saying to run `just build-web-plugin`. The build still succeeds with
  a warning, so `cargo build` and clippy need no Node.

## Side Effects

- Build time: `build.rs` reads `web/dist-plugin`, writes
  `$OUT_DIR/web_assets.rs` (one `include_bytes!` per file), and emits
  `rerun-if-changed`.

## Verification

- Run: `cargo test -p integral-web-assets`
