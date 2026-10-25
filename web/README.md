# web

## Overview

The Integral web frontend, built with React 19, TypeScript and Vite. It
ships in two forms:

- **PWA.** Served locally or on GitHub Pages, it reaches the INTEGRA-7
  through Web MIDI and works offline.
- **Plugin.** An embedded copy runs inside the plugin's webview
  ([ADR 0004](../docs/adr/0004-plugin-ui-in-a-webview.md)).

Both are backed by `crates/integral-wasm`, the Rust core compiled to wasm.
`just pack-wasm` copies its `wasm-pack` output into `pkg/`.

## Interface

- [`src`](./src/README.md) — the app: entry point, pages, tone editors, the
  device service and transports, and hooks.
- [`scripts`](./scripts/README.md) — Node checks run on finished builds:
  PWA precache and manifest, and the plugin build's shape.
- `vite.config.ts` — the Vite config:
  - `react()` and `VitePWA` (prompt-mode registration; precaches
    `**/*.{js,css,html,wasm,svg}` up to 8 MiB a file).
  - Base `/integral/` when `GITHUB_PAGES=true`, `/` otherwise.
  - `--mode plugin` turns off the PWA and writes to `dist-plugin`.
  - The dev server may serve `pkg/`.
  - Vitest runs in jsdom with a 20 s test timeout.
- `vitest.setup.ts` — runs `initSync` on the real
  `pkg/integral_wasm_bg.wasm` before the tests, and sets
  `IS_REACT_ACT_ENVIRONMENT`.
- `index.html` — mounts `/src/main.tsx` on `#root`.
- `tsconfig.json` — strict mode, `noUncheckedIndexedAccess`,
  `noUnusedLocals` and `noUnusedParameters`, over `src`.
- `package.json` scripts: `dev`, `build` (`tsc -b && vite build`),
  `preview`, `test` (`vitest run`). Use the `just` targets below, which
  pack the wasm first.

## Invariants

- `pkg/` must hold a current `wasm-pack` build before typechecking, testing,
  building or serving. Every `just` web target that typechecks, tests, builds
  or serves depends on `pack-wasm` (`lint-css`, `web-deps` and `web-icons`
  don't need it). The
  component tests run against the real core, never a mock.
- The PWA build and the plugin build come from one source tree, with
  different modes:
  - The PWA manifest's `start_url` and `scope` equal the Vite base, and every
    hashed JS, CSS and wasm asset is precached.
  - The plugin build has no service worker or manifest, uses base `/`, and
    includes the wasm.

  `scripts/` checks both.
- A new deploy takes over only when the user accepts the reload, so the JS
  and the wasm it loads always come from one build.
- Device, tone and FX data come from the core. The frontend does not
  hardcode them (see [`src`](./src/README.md)).
- Generated and installed directories are not source: `node_modules/`,
  `pkg/`, `dist/`, `dist-plugin/`, `src/wasm/`, and `tsconfig.tsbuildinfo`
  (written by `tsc -b`). `public/` holds static
  assets: `icon.svg`, and the PWA icons that `just web-icons` renders from
  it.

## Side Effects

- Builds write `dist/` (with `dist/check-root` and `dist/check-pages` from
  `just check-pwa`) or `dist-plugin/` (`just build-web-plugin`).
- `just pack-wasm` writes `pkg/`, copying only the files that changed, so a
  running dev server isn't reloaded for nothing.
- `just web-deps` runs `npm ci` when `node_modules` is missing or older than
  `package-lock.json`.
- At runtime, see [`src`](./src/README.md): Web MIDI or plugin IPC,
  `localStorage`, the service worker, file downloads.

## Verification

- Run: `just test-web` (Vitest + jsdom against the packed wasm)
- Run: `just lint-web` (`tsc -b`) and `just lint-css` (Stylelint)
- Run: `just check-pwa` and `just build-web-plugin` (build, then check with
  `scripts/`)
- Dev server: `just dev-web`
