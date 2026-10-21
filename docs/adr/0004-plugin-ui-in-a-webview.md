# The plugin renders the web UI in a webview

The VST3/CLAP plugin's editor is a webview that runs the web app. It does not have its own vizia views. The editor is our own nih-plug `Editor` on wry and baseview. It loads the built `web/dist` from the plugin over a custom protocol, and the page reaches the device through a plugin transport behind the web app's existing port interface (`MidiPortPair`). Web and plugin therefore share one renderer as well as one layout definition. Decided in [Run the web UI inside the plugin](https://github.com/sigma/integral/issues/328), after the spike ran the unchanged web app in Ableton Live 12 with a 1 ms IPC round trip.

## Consequences

- **The vizia views go.** `crates/integral-vst/src/views/` and `theme.css` are deleted when the webview editor lands. The second, hand-ported UI and its drift (#220) go with them, and #220 closes as superseded at that point.
- **Native rendering of layout definitions was rejected.** It would mean two renderers for every Section and every Display, kept in step by hand, and the web is where features land first.
- **No `nih-plug-webview` crate.** The upstream crate is unmaintained, and the forks pin a different baseview from ours. Our editor is small, but it owns one platform quirk: the host's view has no window when the editor spawns, so the webview is built on baseview's first frame.
- **The device link is not the host's MIDI routing.** Live does not pass plugin SysEx through, so the plugin opens the device port itself. Who owns that port across plugins and the web app is decided on the DAW map ([Who owns the device link?](https://github.com/sigma/integral/issues/341)).
- **The page talks through an explicit plugin transport, not a fake `navigator.requestMIDIAccess`.** It paces its sends itself, as on the web, and the Rust side does not throttle a second time.
- **Hosts own the keyboard in places.** Live takes Escape and closes the plugin view, so no plugin UI depends on Escape, and every dialog has a visible Cancel. Typing in text fields works.
- **Window size comes from the Grid.** The plugin sizes its window as the Grid's size × zoom (`request_resize`), and zoom is the only size control.
- **Closing the editor destroys the page.** On reopen, the page reconnects to the last port and re-reads device state, as on first load. Keeping the session in the plugin process is deferred until reopen latency matters.
- **Windows is unproven.** It needs a WebView2 user data folder outside the DAW's directory, and the spike ran on macOS only.
