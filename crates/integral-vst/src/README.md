# integral-vst/src

## Overview

The six nih-plug plugins of the family
([ADR 0007](../../../docs/adr/0007-plugins-per-tone-type.md)), exported as VST3
only, and a standalone binary that wraps the Studio Set plugin. They are one
generic plugin, `Family<K>`, over a `Kind` that names the plugin. The editor is
the webview UI, available only on macOS.

## Interface

- `lib.rs`: the type aliases `StudioSetPlugin`, `SnSynthPlugin`,
  `SnAcousticPlugin`, `PcmSynthPlugin`, `SnDrumPlugin` and `PcmDrumPlugin`,
  and the one `nih_export_vst3!` of all six, in `PluginKind::ALL` order.
- `family/`
  - `kind.rs`: `Kind` (the `PluginKind`, the host name and the VST3 class ID
    of a plugin) and its six implementations. Class IDs are `IntgrI7` and a
    tag; they identify the plugin in saved projects and never change.
  - `params.rs`: `FamilyParams` declares the kind's host list as `IntParam`s
    over the raw range, with value text and string-to-value from the list's
    `HostFormat`, no smoothing, and the group from `HostParam.group`. It also
    saves the editor size (`editor-size`) and the Binding (`binding`).
  - `binding.rs`: `BindingCell`, an `Option<Binding>` in one atomic, so the
    audio thread can read it. Saved as JSON (`null`, `{"part":2}` or
    `{"part":2,"key":36}`); a saved Binding the plugin cannot have is ignored.
  - `instance.rs`: `Instance`, what the plugin, the connection worker and
    the editor share: the Binding, the `Status`, and whether the host has
    restored the state.
  - `status.rs`: `Status`: "Device service not running", "Device offline",
    "Connected", and "Device service version mismatch" (the service speaks
    another protocol version or lacks `Bind` and `Claim`; not retried).
  - `link.rs` (unix): `Connection` is the Device service client and a worker
    thread; `Tracker` is the sans-IO state machine that maps client events to
    the status and to `Bind` and `Claim` requests.
- `standalone.rs`: the `integral-app` binary, which runs the Studio Set plugin
  through `nih_export_standalone`. `detect_sample_rate()` reads the sample
  rate of the default `cpal` output device.
- [`./webview/README.md`](./webview/README.md): the editor, the IPC protocol,
  the asset server and the `DeviceLink` seam.

## Invariants

- Audio is 2 in and 2 out and passes through untouched. The plugins have no
  host MIDI in or out: notes go to the service's note port. Sample-accurate
  automation is off.
- `process()` only marks the instance as settled (a wait-free store). The
  connection runs on its own threads.
- **Adoption.** A new Tone or Kit plugin claims a free target from the
  service (`Request::Claim`, atomic: the service picks the first free Part,
  preferring one that holds a tone of the plugin's type, and binds the client
  to it), but only once settled (`Instance::settle`). The host restores state
  after creating the instance, so settling happens at a restore
  (`deserialize_fields`), the first `process()`, or when the editor window
  opens (`Editor::spawn`). It does **not** happen in `Plugin::editor()`:
  nih-plug calls that while it builds the wrapper, before any `setState`.
  An instance with a restored Binding keeps it. `Instance::was_restored` says
  a state with a valid Binding was restored. The recall (ADR 0009, #458) keys
  on `was_restored()`, not on `is_settled()`, which a fresh instance reaches too. The
  Studio Set plugin is bound from the start, to `Binding::ANY`. A key of a Kit
  plugin is never claimed, it is chosen. What a plugin may be bound to is
  `PluginKind::accepts` in core.
- Every Binding the instance holds is announced to the service
  (`Request::Bind`, and in the `Hello` after a reconnect). A service that
  refuses `Bind` or `Claim` as malformed is too old: the status becomes
  `ServiceIncompatible` rather than the instance waiting for a target.
- The connection starts in `initialize()`, not on construction, so a host that
  only scans the plugins connects nothing.
- Device SysEx never goes through the host, because hosts such as Live drop
  plugin SysEx. The editor still opens its own MIDI link, until it goes through
  the service.
- In the standalone, a `--sample-rate` given by the user takes priority.
  Otherwise the detected rate is used.

## Side Effects

- Each initialized instance connects to the Device service socket under the
  user's temp directory (creating the private `Integral` directory), retrying
  while the service is away.
- The standalone opens audio and MIDI through nih-plug's backends, and prints
  the detected sample rate to `stderr`.
- On macOS the editor opens MIDI ports directly (see `webview/`).

## Verification

- Run: `cargo test -p integral-vst`. `tests/connection.rs` runs instances
  against a real service on a fake link.
- Run: `just plugin-test load`, which loads every class of the bundle in the
  test host and round-trips its state. Loading activates the plugins, which
  connect to the user's Device service: the host refuses to run while one is
  listening, unless given `--allow-service`.
- The bundle is one `Integral.vst3` with six classes; hosts list each class.
  The test host picks one by name (`--class`).
