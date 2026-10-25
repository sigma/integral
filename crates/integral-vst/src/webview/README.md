# integral-vst/src/webview

## Overview

The plugin editor runs the web UI in a `wry` webview hosted in a `baseview`
child window. It is macOS only, as described in
[ADR 0004](../../../../docs/adr/0004-plugin-ui-in-a-webview.md). The page is
served from the embedded web build over `integral://localhost/`.

The page and the plugin exchange JSON messages tagged by `type`. The page
posts them with `window.ipc.postMessage`. The plugin replies by running a
script that dispatches `CustomEvent("integral-plugin")`. `Bridge` turns page
messages into calls on a `DeviceLink`, which today is `MidirLink`. The
`DeviceLink` trait is the seam for the out-of-process Device service in
[ADR 0006](../../../../docs/adr/0006-device-service-owns-the-link.md).

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
    `send { port, data }` and `resize { width, height }`.
  - `PluginMessage` is sent by the plugin: `ports`, `opened`, `openFailed`,
    `receive` and `error`.
  - `PLUGIN_EVENT` is the event name. `decode` and `encode` convert messages,
    and `delivery_script` builds the delivery script.
- `bridge.rs`: `Bridge::new(Box<dyn DeviceLink>)`, with `handle`,
  `refresh_ports`, `take_outgoing` and `take_resize`.
- `link.rs`
  - `trait DeviceLink` has `ports`, `open(port, Inbound)`, `send` and
    `close`.
  - `Inbound` is the callback for incoming bytes, and `LinkError` is the
    error type.
  - `MidirLink` is the current implementation. Its ports are the names that
    exist as both an input and an output, and its client name is
    `Integral`.
- `editor.rs`: `WebviewEditor::new(size, LinkFactory)` is the nih-plug
  `Editor`.

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
- `ports` is sent only when the list changes.
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
  - The `midir` input callback runs on its own thread and pushes `receive`
    messages into an `Arc<Mutex<VecDeque>>` outbox.
- `MidirLink::open` closes any open port pair, then opens a new pair by exact
  port name with `Ignore::None`. All SysEx, timing and active sensing
  messages reach the page.
- Resizing writes the persisted `editor_size` and asks the host to resize.
- Dropping the editor handle closes the window. Dropping `Bridge` or
  `MidirLink` closes the ports.

## Verification

- Run: `cargo test -p integral-vst`
