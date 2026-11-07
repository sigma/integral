# integral-vst/src/webview

## Overview

The plugin editor runs the web UI in a `wry` webview hosted in a `baseview`
child window. It is macOS only, as described in
[ADR 0004](../../../../docs/adr/0004-plugin-ui-in-a-webview.md). The page is
served from the embedded web build over `integral://localhost/`.

The page and the plugin exchange JSON messages tagged by `type`. The page
posts them with `window.ipc.postMessage`. The plugin replies by running a
script that dispatches `CustomEvent("integral-plugin")`. `Bridge` turns page
messages into calls on a `DeviceLink`, which is `ServiceLink`: a raw SysEx client of the Device service
([ADR 0006](../../../../docs/adr/0006-device-service-owns-the-link.md)). The
plugin opens no MIDI port.

## Interface

- `mod.rs`: `editor` is compiled only with `#[cfg(target_os = "macos")]`.
  `assets`, `bridge`, `ipc` and `link` build on every target.
- `assets.rs`
  - `SCHEME` is `integral`, and `START_URL` is `integral://localhost/`.
  - `CSP` is the page's Content Security Policy.
  - `serve(path)` returns `Served { status, mime, body }`. `serve_from` and
    `mime` are its helpers.
- `ipc.rs`
  - `PageMessage` is sent by the page: `ready`, `open { port }`,
    `send { port, data }`, `resize { width, height }`,
    `bind { part?, key? }` and the host gestures `beginEdit { id }`,
    `performEdit { id, value }` and `endEdit { id }` (`id` is the host
    parameter ID of ADR 0008, `value` the raw value).
  - `PluginMessage` is sent by the plugin: `ports`, `opened`, `openFailed`,
    `receive`, `instance { plugin, binding, status, statusText, notices }`,
    `refused { message }` and `error`.
  - `PLUGIN_EVENT` is the event name. `decode` and `encode` convert messages,
    and `delivery_script` builds the delivery script.
- `gestures.rs`: the page's gestures by host ID (`begin`, `perform`, `end`),
  over the instance's own `family::host::HostEdits` (`ParamBank::edits`, the
  context adapter the follower uses too) and its `ParamBank` for ID to
  position. `family::host::editor_*` keep one gesture per parameter and note
  a drag's values as the device's side gave them (`ParamBank::expect`), so
  the instance does not write them back to the device. This is the plugin's
  gesture entry point.
- `bridge.rs`: `Bridge::new(Box<dyn DeviceLink>, Arc<Instance>)`, with `handle`, `refresh_ports`, `refresh_instance`,
  `take_outgoing` and `take_resize`.
- `link.rs`
  - `trait DeviceLink` has `ports`, `open(port, Inbound)`, `send` and
    `close`.
  - `Inbound` is the callback for what a link tells the page: `Incoming`
    is `Opened`, `OpenFailed`, `Receive` or `Error`. A link answers `open`
    with `Opened` or `OpenFailed`, possibly later, and the bridge sends
    `send` only to a port that has said `Opened`.
  - `LinkError` is the error type.
- `service_link.rs` (unix): `ServiceLink::new(Arc<Instance>)`.
  - It offers one port, `PORT` (the page's `DEVICE_SERVICE_NAME`), while
    the instance's status is `Connected` and the open client, if any, is
    connected. Otherwise `ports` fails with a status text, which the bridge
    tells the page once.
  - `open` connects a second `Client` of its own to the instance's socket
    (the instance's worker owns the instance client's `Events`, and the raw
    setting is per client) and asks for raw SysEx. It says `Opened` at the
    client's first greeting, or `OpenFailed` after 1.5 s. It publishes the
    client's ID as `Instance::editor_client`.
  - `send` queues the bytes with `Client::raw`, into the service's paced
    queue, and fails while the client is disconnected. What the service
    refuses reaches the page as an `error`.
- `editor.rs`: `WebviewEditor::new(size, LinkFactory, Arc<FamilyParams>)` is
  the nih-plug `Editor`.

## Invariants

- Wire strings must match `web/src/plugin.ts`. Unknown fields and byte values
  out of range are rejected (`deny_unknown_fields`).
- WASM is served as `application/wasm`, and the CSP allows
  `'wasm-unsafe-eval'`.
- Nothing is delivered before `ready`. A new `ready` clears the outbox and the
  last port list, then lists the ports again.
- The outbox holds at most `OUTBOX_LIMIT = 8192` messages. When it is full,
  the oldest messages are dropped.
- `send` works only on the open port. On any other port it returns an
  `error` with `NotOpen`. Opening the port that is already open just answers
  `opened`.
- `ports` is sent only when the list changes. `instance` is sent on `ready`
  and when the instance's Binding, status or notices change.
- `bind` goes through `Instance::bind`, so only a Binding the kind accepts
  (`PluginKind::accepts`) is taken; anything else is a `refused` and the old
  Binding stays.
- A host gesture is `beginEdit`, any number of `performEdit`, then `endEdit`,
  per ID. An out-of-order message or an ID the plugin lacks is a `refused` and
  reaches nothing. (`error` is the device link's.) A gesture still begun when the page reloads (`ready`) or
  the bridge is dropped is ended, so the host never waits on one.
- Resize requests outside 200..=16384 logical px per side are rejected
  with an `error`, not clamped. If the host
  refuses, the previous size is restored.
- The bridge does not throttle. The page paces its sends with core's
  `DeviceState`.
- The webview is built on the first `on_frame`, because `wry` panics if the
  view is not in a window yet. A failed build is logged and not retried.
- Only AppKit window handles are supported. Devtools are on only in debug
  builds.

## Side Effects

- Threads:
  - The UI thread runs `on_frame`, which polls ports every 1 s, handles
    resizes and delivers messages with `evaluate_script`. It also runs the
    IPC handler. Both use one `Rc<RefCell<Bridge>>`.
  - The `ServiceLink` forwarding thread pushes `opened`, `receive` and
    `error` messages into an `Arc<Mutex<VecDeque>>` outbox. It exists while
    a port is open.
- `ServiceLink::open` closes any open port, then connects a client to the
  service socket. Closing flushes the client (up to `FLUSH_TIMEOUT`, on the
  UI thread) so the last messages reach the service, which sends them on.
- Gestures call the host's GUI context on the UI thread; `perform` holds the
  value to the parameter's range.
- Resizing writes the persisted `editor_size` and asks the host to resize.
- Dropping the editor handle closes the window. Dropping `Bridge` or
  `ServiceLink` closes the client.

## Verification

- Run: `cargo test -p integral-vst`
