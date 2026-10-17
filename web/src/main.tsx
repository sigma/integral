import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { PwaUpdateBanner } from "./PwaUpdateBanner";

// Offline support: PwaUpdateBanner registers the service worker. In prompt
// mode a new deploy only takes over on the reload the banner offers, so a
// running page never mixes its JS with another build's WASM — and no open
// page reloads mid-session.
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PwaUpdateBanner />
    <App />
  </StrictMode>,
);
