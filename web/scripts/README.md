# web/scripts

## Overview

Node build checks that run against a finished `vite build` output. They
check what is emitted, not the source: one for the PWA build that GitHub
Pages and local use serve, one for the copy the plugin embeds
([ADR 0004](../../docs/adr/0004-plugin-ui-in-a-webview.md)). Each one is a
plain ES module with no dependencies outside the Node standard library.

## Interface

- `check-pwa-build.mjs` — `node scripts/check-pwa-build.mjs <outDir> <base>`.
  Checks the PWA invariants of
  [#202](https://github.com/sigma/integral/issues/202):
  - `manifest.webmanifest`'s `start_url` and `scope` both equal `<base>`.
  - `sw.js` precaches the WASM binary and every hashed `.js`, `.css` and
    `.wasm` file in `assets/`.
- `check-plugin-build.mjs` — `node scripts/check-plugin-build.mjs <outDir>`.
  Checks a `vite build --mode plugin` output
  ([#365](https://github.com/sigma/integral/issues/365)):
  - No `sw.js`, `registerSW.js` or `manifest.webmanifest` is emitted, and
    `index.html` does not link the manifest.
  - `assets/` holds a `.wasm` file.
  - `index.html` loads its `index-*.js` entry from `/assets/`, so it uses
    base `/`.

## Invariants

- Exit codes: `0` when the build passes, `1` when any check fails (each
  failure is listed on stderr), `2` on a usage error.
- `check-pwa-build.mjs` reads the precache list by running `sw.js` in a
  `node:vm` context against a stub Workbox that records only
  `precacheAndRoute`. It never pattern-matches the minified source, so keep
  it that way rather than adding a regex.
- The rules mirror `vite.config.ts`. The PWA build uses base `/integral/`
  only when `GITHUB_PAGES=true` and `/` otherwise. The plugin mode turns
  `VitePWA` off and writes to `dist-plugin`. If either config changes, change
  the matching script with it.

## Side Effects

- Read the given build directory from disk. They write nothing.
- `check-pwa-build.mjs` runs the built `sw.js` inside a sandboxed `vm`
  context.

## Verification

- `just check-pwa`: builds `dist/check-root` (base `/`) and
  `dist/check-pages` (base `/integral/`), then runs `check-pwa-build.mjs` on
  each.
- `just build-web-plugin`: builds `dist-plugin`, then runs
  `check-plugin-build.mjs`.
