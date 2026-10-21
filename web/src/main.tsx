import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { PwaUpdateBanner } from "./PwaUpdateBanner";
import { initWasm } from "./midi";
import { pluginPorts, resizePluginWindow, windowBridge } from "./plugin";

// Inside the plugin's webview the page reaches the device through the plugin
// transport; in a browser, through Web MIDI.
const bridge = windowBridge(window);

// The plugin window is sized from the Grid at zoom 1 (ADR 0004). The Grid
// reads the device spec from core, so it waits for the wasm core.
if (bridge) {
  // A wasm core that fails to load is reported by App.
  initWasm().then(() => resizePluginWindow(bridge, 1, window.screen), () => {});
}

// Offline support: PwaUpdateBanner registers the service worker. In prompt
// mode a new deploy only takes over on the reload the banner offers, so a
// running page never mixes its JS with another build's WASM — and no open
// page reloads mid-session. The plugin serves its own embedded build and
// has no service worker.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {!bridge && <PwaUpdateBanner />}
    <App openPorts={bridge ? () => pluginPorts(bridge) : undefined} />
  </StrictMode>,
);
