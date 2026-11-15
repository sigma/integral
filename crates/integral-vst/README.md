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

### Following Live's tempo

The INTEGRA-7 follows Live through MIDI clock; no plugin sends a tempo
([#465](https://github.com/sigma/integral/issues/465)). Its tempo-synced
effects then follow Live, settling on a new tempo over a few seconds.

1. On the INTEGRA-7, set SYSTEM > SYNC/TEMPO > Sync Mode to SLAVE. It has no
   SysEx address, so only the panel can set it, and it returns to MASTER at
   power-off unless you save the System settings.
2. Send Live's clock to a wired port: 5-pin MIDI IN with SYSTEM Clock Source
   MIDI, or USB COMPUTER with Clock Source USB. In Live's MIDI preferences,
   turn Sync on for that output only. Only one host should send clock
   ([ADR 0006](../../docs/adr/0006-device-service-owns-the-link.md)).

The device ignores clock on the Wireless link, so clock sent to
`Integral Device Service` with `--link wireless` never locks. A Wireless-only
rig keeps the device's own tempo
([#571](https://github.com/sigma/integral/issues/571)). Once clock stops, the
device holds the last tempo it received.

Live opens a plugin's device panel empty: each plugin declares more than 64
host parameters, and Live leaves such a list for you to fill
([#526](https://github.com/sigma/integral/issues/526)). To add parameters,
click Live's Configure button and touch a control in the plugin's editor; the
parameter stays on the panel after Configure is turned off.

## Interface

- [`./src/README.md`](./src/README.md): the plugin family, the standalone
  entry point and the webview editor.
- `integral-web-assets` embeds `web/dist-plugin` (built by
  `just build-web-plugin`), and `src/webview/assets.rs` serves it.
- Build outputs:
  - a `cdylib` and `lib` library (the six plugins; `just bundle-plugin` makes
    `target/bundle/Integral.vst3` from it)
  - the `integral-app` binary (the standalone)

## Invariants

- If `web/dist-plugin` is missing, the editor shows a page explaining how to
  build the UI (see `integral-web-assets`).
- `wry` is pinned to an exact version (`=0.57.0`), because the editor relies
  on its platform behaviour. Bump it on purpose and re-check in a host.
- `baseview` uses nih-plug's source and revision, so the workspace `[patch]`
  applies and only one `baseview` is built.
- The webview editor dependencies are macOS only.

## Side Effects

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
