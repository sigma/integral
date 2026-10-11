// Component tests run against the real wasm core (built by `just pack-wasm`).
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { initSync } from "./pkg/integral_wasm.js";

initSync({
  module: readFileSync(join(import.meta.dirname, "pkg/integral_wasm_bg.wasm")),
});

// Let React's act() flush updates in tests.
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
