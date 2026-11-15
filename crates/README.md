# crates

## Overview

The Rust workspace members (edition 2024, see the root `Cargo.toml`). The
portable core library holds all INTEGRA-7 protocol, state and device spec
logic. Three front ends sit on it:

- the CLI, which uses the core directly
- the WASM bindings, which use the core directly
- the DAW plugin, which embeds the web build (and therefore the WASM core)
  and does not link the core itself

```mermaid
graph TD
  core[integral-core]
  cli[integral-cli] --> core
  wasm[integral-wasm] --> core
  web[web/ React UI] --> wasm
  assets[integral-web-assets] -. embeds web/dist-plugin .-> web
  vst[integral-vst] --> assets
  service[integral-device-service] --> assets
```

## Subdirectories & Boundaries

- integral-core: [`./integral-core/README.md`](./integral-core/README.md) -
  the SysEx engine (`DeviceState`, `sysex`, `params`), the device
  specification layer, the tone and effect parameter maps, the SVD format,
  the Librarian, Expansions, Waves and the optional sequencer. It does no I/O
  and must compile for `wasm32-unknown-unknown`.
- integral-cli: [`./integral-cli/README.md`](./integral-cli/README.md) - the
  `integral` binary. It covers device diagnostics, raw SysEx, the monitor,
  the Device service launcher, Pack import and export, and Expansion and Wave tools.
  It does MIDI I/O through `midir`.
- integral-device-service: [`./integral-device-service/README.md`](./integral-device-service/README.md) -
  the Device service: owns the device link and the one `DeviceState`, and
  exposes a virtual note port, and serves the web app beside its WebSocket.
  Native only; `integral-cli bridge` launches it.
- integral-vst: [`./integral-vst/README.md`](./integral-vst/README.md) - the
  `nih-plug` VST3/CLAP plugin and the `integral-app` standalone. It runs a
  macOS webview editor that serves the embedded web UI and links it to MIDI
  ports directly.
- integral-web-assets: [`./integral-web-assets/README.md`](./integral-web-assets/README.md) -
  the web build `web/dist-plugin`, embedded at build time, with `serve(path)`
  for the plugin editor and the Device service.
- integral-wasm: [`./integral-wasm/README.md`](./integral-wasm/README.md) -
  `wasm-bindgen` wrappers for the web frontend. JS pulls outbound SysEx and
  owns the clock.

## Verification

- Run: `cargo test --workspace` (`just test`)
- Run: `just build-wasm`
- Repo gate: `just check`
