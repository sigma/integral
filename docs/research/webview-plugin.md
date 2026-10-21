# Webview-hosted plugin UI

Research for [#327](https://github.com/sigma/integral/issues/327) (map:
[#326](https://github.com/sigma/integral/issues/326)). Question: can the
nih-plug plugin embed the existing web UI (React + `integral-wasm`) in a
webview instead of rendering with vizia?

Researched 2026-10-21. Sources are linked inline. Statements marked
*inference* or *unverified* have no primary source behind them.

## Verdict

**Viable with caveats.** Commercial frameworks ship webview plugin UIs
(JUCE 8, Beamer). The pieces we need all exist: a child webview, a custom
asset protocol, an IPC channel and zoom. The caveats:

1. No nih-plug webview crate is released or production-ready. We would
   depend on a fork, pinned by rev, or write about 500 lines of our own
   on top of wry.
2. Keyboard focus between webview and host is a known, partly unsolved
   problem on every framework.
3. A risk that is **not specific to the webview**: whether hosts forward
   plugin SysEx output to the hardware port at all. This decides whether
   *any* plugin UI can reach the device, so it should be its own question
   on the map (see [Implications](#implications-for-the-map)).

## Our starting point

- `crates/integral-vst/Cargo.toml` takes `nih_plug` and `nih_plug_vizia`
  from git with no rev. `Cargo.lock` resolves `nih_plug` to
  [`28b149e`](https://github.com/robbert-vdh/nih-plug/commit/28b149ec4d62757d0b448809148a0c3ca6e09a95)
  (2025-09-07). Upstream HEAD is `de42101` (2026-09-03).
- The workspace `Cargo.toml` patches baseview to `3e12973` through a
  `www.github.com` source, for the macOS standalone null-window fix
  ([RustAudio/baseview#204](https://github.com/RustAudio/baseview/pull/204)).
- The plugin already owns a Rust `DeviceState` in `SharedState`.
  - `process()` feeds incoming DT1 into `DeviceState`.
  - It drains outgoing SysEx through `context.send_event(NoteEvent::MidiSysEx)`
    (`crates/integral-vst/src/lib.rs`).
  - So SysEx reaches the device through **the host's MIDI routing**, not a
    port the plugin opens.
- The web UI talks to the device as follows:
  - `web/src/midi.ts` calls `navigator.requestMIDIAccess({ sysex: true })`
    and pairs the input and output ports by name.
  - `IntegraService` (`web/src/integra.ts`) owns a `WasmDeviceState`.
  - A 5 ms `setInterval` calls `device.drain(now)` and writes the result to
    `MIDIOutput.send`.
  - Replies arrive through the `midimessage` listener.
  - Pacing lives in the core: `THROTTLE_MS = 40.0` in
    `crates/integral-core/src/device.rs`. That is 40 ms, not the 20 ms
    minimum in `CLAUDE.md`.
- Web bundle size (`web/dist/assets`): about 1.5 MB in total. The JS is
  492 KB, the CSS 53 KB and the WASM 1.0 MB.

## 1. Webview options for nih-plug

| Option | Licence | State | nih-plug / baseview | Notes |
|---|---|---|---|---|
| [httnn/nih-plug-webview](https://github.com/httnn/nih-plug-webview) (was maxjvh) | ISC | Last commit 2025-01-06. README: "work in progress, not production-ready", tested on macOS only | Both from git with **no rev** ([Cargo.toml](https://github.com/httnn/nih-plug-webview/blob/main/Cargo.toml)). wry 0.35.1 | No `set_scale_factor` on Windows or Linux. Open issues: Windows 11 freeze ([#11](https://github.com/httnn/nih-plug-webview/issues/11)), FL Studio crash ([#15](https://github.com/httnn/nih-plug-webview/issues/15)), Bitwig crash ([#16](https://github.com/httnn/nih-plug-webview/issues/16)). The wry bump in [PR #19](https://github.com/httnn/nih-plug-webview/pull/19) is unmerged. Eats Escape because Escape crashes Live on macOS |
| [FreqlabHQ fork](https://github.com/FreqlabHQ/nih-plug-webview) | ISC | Active (2026-10-19) | nih-plug **`28b149e`, our exact rev**. baseview `237d323`, which is not our `3e12973` | Vendors a patched wry 0.53.5. Prefixes the ObjC classes `NihPlugWry*` so that two wry users in one process don't collide. Handles an NSView with no window yet |
| [toiglak fork](https://github.com/toiglak/nih-plug-webview) | ISC | v0.2.0, last commit 2026-02-26 | Pins nih-plug `17ba595e`. **No baseview**: wry `build_as_child` plus its own reparenting | JS API `window.plugin.send/listen/resize` ([webview.d.ts](https://github.com/toiglak/nih-plug-webview/blob/main/webview.d.ts)). Serves assets over a custom protocol. Sets a WebView2 data dir through `WebContext`. Implements `set_scale_factor`. Most complete, but one maintainer |
| Own editor on [wry](https://docs.rs/wry/latest/wry/) 0.57 | Apache-2.0/MIT | Released 2026-09-08 | Implement `nih_plug::Editor` directly with `WebViewBuilder::build_as_child` | `build_as_child` works on macOS, Windows and X11 only. IPC is `window.ipc.postMessage` one way and `evaluate_script` the other. Assets come from `with_(asynchronous_)custom_protocol` |
| [Beamer](https://github.com/helpermedia/beamer) | MIT | Active | Not nih-plug | Has AU and a webview-first UI. Shown for contrast only |

**Compatibility with our pin.**
- Only the FreqlabHQ fork matches our nih-plug rev.
- Its baseview rev (`237d323`) differs from our patched `3e12973`. The two
  would have to be reconciled. *Unverified*: whether our `[patch]` would
  override its rev cleanly.
- The toiglak approach (no baseview) avoids the baseview question. It also
  bypasses the `nih_plug_vizia` → baseview stack that #220 runs into.
- Writing our own editor on wry lets us choose the pins.

**IPC shape.** Every option uses the same two primitives: a JS → Rust string
message and a Rust → JS `evaluate_script`. Binary SysEx has to be encoded,
as a JSON array or base64.

## 2. Host compatibility (macOS WKWebView, Windows WebView2)

**Keyboard and focus.** This is the main known breakage, and it is the same
for JUCE.
- On macOS the webview takes keys away from Live and Logic, so the space
  bar no longer reaches the transport. Forwarding keys to the next
  responder fixes this only partly
  ([JUCE forum](https://forum.juce.com/t/webview-and-keyboard-input-propagation-issue-to-the-host/62439)).
- Windows has the same problem, with no JS workaround
  ([JUCE forum](https://forum.juce.com/t/fixing-webview-keyboard-focus-issue/63987);
  [wry#1761](https://github.com/tauri-apps/wry/issues/1761)).
- Logic on Apple Silicon can lose focus permanently after a resize
  ([JUCE forum](https://forum.juce.com/t/bug-m1-apple-silicon-keyboard-focus-lost-permanently-in-logic-when-plugin-is-resized/51292)).
- JUCE overrides `performKeyEquivalent:` to make Cmd-C/V/X work
  ([source](https://github.com/juce-framework/JUCE/blob/master/modules/juce_gui_extra/native/juce_WebBrowserComponent_mac.mm)).
- Our UI is mouse-driven and has little text entry: names and number
  fields. That limits the exposure.

**Multiple instances.**
- On macOS, `WKProcessPool` has had no effect since macOS 12
  ([Apple](https://developer.apple.com/documentation/webkit/wkprocesspool)).
  The real hazard is wry's ObjC class names colliding when two wry users
  share a host process, which is why FreqlabHQ added class prefixes.
- On Windows the WebView2 user data folder defaults to a folder next to the
  host `.exe`, which is often under Program Files. That gives "Access
  Denied", so a plugin **must** set its own folder
  ([Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/user-data-folder);
  [JUCE `withUserDataFolder`](https://github.com/juce-framework/JUCE/blob/master/modules/juce_gui_extra/misc/juce_WebBrowserComponent.h)).
- Instances that share a folder share one browser process.

**WebView2 runtime.**
- The Evergreen runtime ships with Windows 11 and is on "the vast majority
  of Windows 10 devices".
- Microsoft still advises a presence check
  ([distribution](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)).

**Per-host status.**
- **Logic:** cannot load nih-plug plugins, because nih-plug has no AU and
  Logic has no VST3 or CLAP. This is not webview-specific: it already
  applies to the vizia plugin.
- **Ableton Live:** the Escape crash on macOS is in the httnn README.
- **Bitwig:** a crash on macOS release, fixed by a wry rollback
  ([#16](https://github.com/httnn/nih-plug-webview/issues/16)).
- **Reaper:** no webview-specific reports found.
- No vendor statements from Logic, Live, Bitwig or Reaper, and no HiDPI
  reports specific to these hosts (*unverified*).

**Evidence that it can be done.**
- JUCE 8 officially supports plugin webview UIs. It provides
  `withNativeFunction`, `withResourceProvider` and `Backend::webview2`
  ([docs](https://docs.juce.com/master/classWebBrowserComponent_1_1Options.html)),
  with `examples/Plugins/WebViewPluginDemo.h` as the example.
- It serves resources from `juce://juce.backend/` on macOS and from
  `https://juce.backend/` on Windows
  ([source](https://github.com/juce-framework/JUCE/blob/master/modules/juce_gui_extra/misc/juce_WebBrowserComponent.cpp)).

## 3. Reaching the device: the IPC bridge

**Web MIDI.**
- WKWebView has none: WebKit declines to implement the Web MIDI API
  ([WebKit](https://webkit.org/tracking-prevention/)).
- WebView2 has Web MIDI. SysEx needs a `PermissionRequested` handler that
  allows `MidiSystemExclusiveMessages`
  ([Microsoft](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2permissionkind)).
- In a plugin the port belongs to the host's routing anyway. Even on
  Windows, the page should not open the port itself.

**Bridge design.** The seam already exists. `IntegraService` writes through a
`MidiPortPair` (`output.send` and the `midimessage` listener).
1. Put a small transport interface behind it, with two implementations:
   - Web MIDI in the browser.
   - IPC in the plugin: send means `ipc.postMessage(bytes)`, and receive is
     a JS callback that the editor calls with `evaluate_script`.
2. Keep the throttle where it is. `WasmDeviceState.drain()` in the page
   already paces sends at `THROTTLE_MS`.
3. The plugin forwards the bytes as they come:
   - The editor's IPC handler pushes them onto a lock-free queue.
   - `process()` emits them through `context.send_event`.
   - Incoming SysEx from `process()` goes back to the editor thread and is
     handed to the page with `evaluate_script`.
4. Do not run two `DeviceState`s that both throttle. The plugin's own
   `DeviceState` then only queues raw bytes, or it moves into the page. The
   prototype should choose one.

**Latency.**
- The only published measurement is about 3.6 ms, later 2.6 ms, per
  WKWebView round trip on a 2015 iPad
  ([1](https://blog.persistent.info/2015/01/wkwebview-communication-latency.html),
  [2](https://blog.persistent.info/2015/08/wkwebview-communication-latency.html)).
  No WebView2 numbers were found.
- `process()` adds up to one audio buffer of delay (about 10.7 ms at 512
  samples and 48 kHz), and it stops entirely while the host has the plugin
  suspended.
- Both are well under the 40 ms core throttle (*inference*). The spike
  should measure the round trip.

**Host SysEx routing.** This risk is **not webview-specific**.
- nih-plug emits SysEx output in both wrappers at our rev:
  - VST3 sends a `DataEvent` of type `kMidiSysEx`.
  - CLAP sends a `clap_event_midi_sysex`.
  - Both are gated on `MIDI_OUTPUT >= Basic`
    ([wrappers](https://github.com/robbert-vdh/nih-plug/tree/master/src/wrapper)).
- Whether the host forwards that SysEx to a hardware port depends on the
  host:
  - **Ableton:** documents SysEx only through Max for Live
    ([Ableton](https://help.ableton.com/hc/en-us/articles/360003148640-SysEx-support);
    the page was fetched through a search summary).
  - **Reaper:** routes plugin MIDI to a hardware output
    ([wiki](https://www.cockos.com/wiki/index.php/Recording_MIDI)), but SysEx
    is unverified.
  - **Bitwig:** unverified.
- The alternative is to open the port from the plugin with midir, bypassing
  the host. CoreMIDI is multi-client. Classic WinMM ports are often
  single-client, so the plugin may lose to the DAW for the port
  (*inference*).

## 4. Loading the WASM

- `WebAssembly.instantiateStreaming` requires `Content-Type:
  application/wasm`
  ([MDN](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/instantiateStreaming_static)).
  wasm-bindgen's `init()` uses it when it is available.
- A CSP must allow `'wasm-unsafe-eval'` in `script-src`
  ([MDN](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src)).
  WASM does not need a secure context.
- **Avoid `file://`.**
  - WebView2 gives each file its own origin and no secure context
    ([Microsoft](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/working-with-local-content)).
  - The WKWebView behaviour with `file://` is *unverified*.
- **Use a custom protocol.**
  - wry's `with_custom_protocol` appears as `<scheme>://` on macOS (a
    `WKURLSchemeHandler`) and as `http(s)://<scheme>.<path>` on Windows
    ([wry](https://docs.rs/wry/latest/wry/struct.WebViewBuilder.html)).
  - Serve `web/dist` embedded in the binary (`include_dir!` or similar)
    with explicit MIME types.
  - This is how JUCE does it.
- The bundle is about 1.5 MB, which is fine to embed.
- Things to turn off in the plugin build:
  - The PWA service worker (`vite-plugin-pwa`) is meaningless there and may
    fail on a custom scheme.
  - The Vite `base` must be `/`.

## 5. Scale factor and resizing

nih-plug:
- `Editor::size()` is in logical pixels.
- `set_scale_factor` is never called on macOS. On Windows and Linux the
  editor multiplies by it.
- `GuiContext::request_resize()` asks the host to adopt `size()` and may be
  refused. "Host->Plugin resizing has not been implemented yet"
  ([editor.rs](https://github.com/robbert-vdh/nih-plug/blob/master/src/editor.rs),
  [gui.rs](https://github.com/robbert-vdh/nih-plug/blob/master/src/context/gui.rs)).

The page:
- It sees its window through `innerWidth`/`innerHeight` and
  `devicePixelRatio`, which the engine derives from the view's backing
  scale (*inference*).
- Uniform zoom (the standing decision) can be applied in either of two
  places:
  - **Natively**, with WKWebView `pageZoom` ("equivalent to setting the CSS
    zoom property", macOS 11+,
    [Apple](https://developer.apple.com/documentation/webkit/wkwebview/pagezoom))
    or WebView2 `ZoomFactor`
    ([Microsoft](https://learn.microsoft.com/en-us/dotnet/api/microsoft.web.webview2.core.corewebview2controller.rasterizationscale)).
    wry exposes both through `WebView::zoom`.
  - **In the page**, with CSS `zoom` or `transform: scale` on the Grid root,
    computed from the window size. This is the same code path as the
    browser, so prefer it.
- Plugin-initiated resize means JS → IPC → `request_resize` → `set_bounds`
  on the child webview, which is what toiglak's `window.plugin.resize`
  does.
- Because host-initiated resize isn't implemented in nih-plug, the window
  size is effectively chosen by the plugin. The user picks a zoom step, as
  the vizia plugin would have to.

## Implications for the map

- **For the prototype ([#328](https://github.com/sigma/integral/issues/328)):**
  - Build the spike as our own `Editor` on wry 0.57, or on the toiglak or
    FreqlabHQ fork pinned by rev.
  - Serve the embedded `web/dist` over a custom protocol.
  - Add a transport seam in `IntegraService`.
  - Test in Reaper first, because it is the most likely to route plugin
    SysEx to hardware.
  - Measure the IPC round trip.
  - Check that keyboard input in a name field and Escape do not reach or
    crash the host.
- **New question for the map:** does any target host route plugin SysEx
  output to the INTEGRA-7?
  - If not, the plugin has to open the MIDI port itself (midir), with the
    Windows single-client caveat.
  - This applies whichever renderer wins.
- Logic is unreachable without an AU wrapper, whichever renderer is chosen.
- Throttle mismatch: the core uses 40 ms but `CLAUDE.md` states a 20 ms
  minimum. They don't conflict (40 ≥ 20), but the ticket's "20 ms" should
  be read as the core's 40 ms.
