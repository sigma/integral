import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { registerSW } from "virtual:pwa-register";
import { App } from "./App";
import { FamilyPrototypeBar } from "./familyColourPrototype";

// Offline support. In autoUpdate mode a new deploy's service worker activates
// at once and this reloads the page, so a running page never mixes its JS with
// another build's WASM.
registerSW({ immediate: true });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
    <FamilyPrototypeBar />
  </StrictMode>,
);
