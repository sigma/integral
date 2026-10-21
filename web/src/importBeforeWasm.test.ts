/**
 * The app starts the wasm core after its modules load (`initWasm` in
 * `midi.ts`), so importing the app or a layout must not call into wasm: a
 * module that did would blank the page in a browser, while the other tests,
 * which start wasm in `vitest.setup.ts`, would not notice.
 */

import { describe, expect, it, vi } from "vitest";

const WASM = "../pkg/integral_wasm.js";

/** Layout modules the barrel and registry may not reach: each must import before wasm too. */
const LAYOUT_MODULES = [
  "./layouts/drum/pcmDrum",
  "./layouts/drum/snDrum",
  "./layouts/pcm-synth/pair",
  "./layouts/pcm-synth/screen",
  "./layouts/pcm-synth/sections",
  "./layouts/sn-acoustic/amp",
  "./layouts/sn-acoustic/common",
  "./layouts/sn-acoustic/fields",
  "./layouts/sn-acoustic/filter",
  "./layouts/sn-acoustic/inst",
  "./layouts/sn-acoustic/screen",
  "./layouts/sn-acoustic/vibrato",
  "./layouts/sn-synth/common",
  "./layouts/sn-synth/partial",
  "./layouts/sn-synth/screen",
  "./sn-synth/displays",
  "./sn-synth/PartialRow",
  "./pcm-synth/PcmSynthScreen",
  "./pcm-synth/PartialSections",
];

describe("importing before wasm starts", () => {
  it("calls no wasm function", async () => {
    vi.resetModules();
    vi.doMock(WASM, async (importOriginal) => {
      const real = await importOriginal<Record<string, unknown>>();
      // Enums stay; every function and class throws, as before `init()`.
      return Object.fromEntries(
        Object.entries(real).map(([name, value]) => [
          name,
          typeof value === "function"
            ? () => {
                throw new Error(`${name} called at import, before wasm starts`);
              }
            : value,
        ]),
      );
    });
    try {
      await expect(import("./layouts")).resolves.toBeDefined();
      await expect(import("./layouts/registry")).resolves.toBeDefined();
      for (const module of LAYOUT_MODULES) await expect(import(module)).resolves.toBeDefined();
      await expect(import("./App")).resolves.toBeDefined();
    } finally {
      vi.doUnmock(WASM);
      vi.resetModules();
    }
  });
});
