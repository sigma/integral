# integral-vst

## Overview

The DAW plugins (six VST3 plugins from one bundle, via `nih-plug`) and the
`integral-app` standalone. Both embed the web UI in a macOS webview
([ADR 0004](../../docs/adr/0004-plugin-ui-in-a-webview.md)). The plugins
declare the host parameter lists of `integral-core` and connect to the Device
service ([ADR 0006](../../docs/adr/0006-device-service-owns-the-link.md)) as
clients. The embedded page runs the WASM core, and the webview side only serves
the page and passes SysEx between the page and the Device service. The plugins
open no MIDI port.

## Using the plugins in Ableton Live

The plugins are audio effects that pass audio through; they play no notes
([ADR 0007](../../docs/adr/0007-plugins-per-tone-type.md)). Live's External
Instrument plays the notes and returns the INTEGRA-7's audio.

1. Install: `just bundle-plugin`, then copy `target/bundle/Integral.vst3` to
   `~/Library/Audio/Plug-Ins/VST3/` and rescan in Live's Plug-ins preferences.
2. Run the Device service: `just bridge`, or for RTP-MIDI
   `just bridge --link rtp --port <name>`.
3. For each Part, on a MIDI track:
   1. Insert External Instrument in the instrument slot. Set MIDI To to
      `Integral Device Service` and the channel to the Part's receive channel.
      Set Audio From to the inputs the INTEGRA-7 is wired to.
   2. Insert the Tone or Kit plugin for the Part's tone type after it, and
      pick the Part in its editor.
4. Insert Integral Studio Set on the master track for the mixer and effects.

## Interface

- [`./src/README.md`](./src/README.md): the plugin family, the standalone
  entry point and the webview editor.
- `build.rs` embeds `web/dist-plugin` (built by `just build-web-plugin`). It
  writes a sorted `(path, bytes)` table to `$OUT_DIR/web_assets.rs`, which
  `src/webview/assets.rs` serves.
- Build outputs:
  - a `cdylib` and `lib` library (the six plugins; `just bundle-plugin` makes
    `target/bundle/Integral.vst3` from it)
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
- Runtime: see [`./src/README.md`](./src/README.md) (the service socket,
  audio backend, window).

## Verification

- Run: `cargo test -p integral-vst`
- Live runbook ([#463](https://github.com/sigma/integral/issues/463)): with
  Live running and a plugin editor open, only the service holds the device
  port. The service is the one process this prints (exactly one line):
  `pgrep -fl 'integral.* bridge'`. No other process, Live included, opens a
  MIDI port of the device. The plugins have no MIDI code (`midir` is not a
  dependency of this crate), and `integral-app` holds no port either: it
  refuses nih-plug's `--midi-input` and `--midi-output`.
- Run: `just build-plugin` (release, with the web UI embedded)
