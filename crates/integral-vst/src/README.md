# integral-vst/src

## Overview

The nih-plug `Integral` plugin, exported as VST3 and CLAP, and a standalone
binary that wraps the same plugin. The plugin has stereo audio in and out,
passes MIDI through unchanged, and has no automatable parameters. The only
state it persists is the editor size. The editor is the webview UI, available
only on macOS.

## Interface

- `lib.rs`
  - `Integral` is the plugin type, with name `Integral` and vendor `sigma`.
  - CLAP: ID `com.sigma.integral`, with features Utility and NoteEffect.
  - VST3: class ID `IntgrI7CtrlSurf\0`, subcategory Tools.
  - `IntegralParams.editor_size` is the persisted editor size,
    `DEFAULT_EDITOR_SIZE = (1280, 820)`.
  - `Plugin::editor` returns a `WebviewEditor` with a `MidirLink` factory on
    macOS, and `None` on other targets.
  - `Plugin::process` forwards MIDI events and leaves audio untouched.
- `standalone.rs`: the `integral-app` binary, which runs the plugin through
  `nih_export_standalone`. `detect_sample_rate()` reads the sample rate of
  the default `cpal` output device.
- [`./webview/README.md`](./webview/README.md): the editor, the IPC protocol,
  the asset server and the `DeviceLink` seam.

## Invariants

- Audio is 2 in and 2 out. MIDI input and output use
  `MidiConfig::Basic`. Sample-accurate automation is off.
- The plugin has no DAW-automatable parameters: the page edits the device
  directly.
- Device SysEx never goes through the host, because hosts such as Live drop
  plugin SysEx. The editor opens its own MIDI link instead.
- In the standalone, a `--sample-rate` given by the user takes priority.
  Otherwise the detected rate is used.

## Side Effects

- The standalone opens audio and MIDI through nih-plug's backends, and prints
  the detected sample rate to `stderr`.
- On macOS the editor opens MIDI ports directly (see `webview/`).

## Verification

- Run: `cargo test -p integral-vst`
