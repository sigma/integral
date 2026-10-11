import { afterEach, describe, expect, it, vi } from "vitest";
import { SurroundField } from "../pkg/integral_wasm.js";
import { SurroundPage } from "./SurroundPage";
import { defaultMixerState } from "./types";
import type { UseMixerResult } from "./useMixer";
import { cleanup, render } from "./test/dom";
import { arrowUp, slider } from "./test/toneService";

afterEach(cleanup);

describe("SurroundPage", () => {
  it("sets the Motional Surround fields it edits", () => {
    const setSurroundParam = vi.fn();
    const state = defaultMixerState();
    const mixer = { state, setSurroundParam } as unknown as UseMixerResult;
    const { container } = render(<SurroundPage mixer={mixer} />);

    arrowUp(slider(container, "Depth"));
    // The Ext Part's Width knob follows the 16 parts' ones.
    const widths = container.querySelectorAll('[role="slider"][aria-label="W"]');
    arrowUp(widths[widths.length - 1]!);

    expect(setSurroundParam.mock.calls).toEqual([
      [SurroundField.Depth, state.surround.depth + 1],
      [SurroundField.ExtPartWidth, state.surround.ext.width + 1],
    ]);
  });
});
