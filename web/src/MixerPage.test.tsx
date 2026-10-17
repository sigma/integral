import { afterEach, describe, expect, it } from "vitest";
import { MixerPage } from "./MixerPage";
import type { UseMixerResult } from "./useMixer";
import type { UseExpansionsResult } from "./useExpansions";
import { defaultMixerState } from "./types";
import { cleanup, render } from "./test/dom";

// jsdom lacks the ResizeObserver the FX strips' EQ plot measures with.
globalThis.ResizeObserver ??= class {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
};

afterEach(cleanup);

const expansions = {
  unloadedExpansionOf: () => null,
} as unknown as UseExpansionsResult;

/** The mixer page with the given failed block labels. */
function page(failedBlocks: string[]): { el: HTMLElement; reloaded: () => boolean } {
  let reloaded = false;
  const mixer = {
    state: { ...defaultMixerState(), loading: false, failedBlocks },
    reload: () => {
      reloaded = true;
    },
  } as unknown as UseMixerResult;
  return {
    el: render(<MixerPage mixer={mixer} expansions={expansions} />).container,
    reloaded: () => reloaded,
  };
}

describe("MixerPage", () => {
  it("warns about blocks whose load failed, with a retry", () => {
    const { el, reloaded } = page(["Chorus", "Part 4"]);
    expect(el.textContent).toContain("2 blocks failed to load");
    el.querySelector("button")!.click();
    expect(reloaded()).toBe(true);
  });

  it("warns nothing while every block loaded", () => {
    expect(page([]).el.textContent).not.toContain("failed to load");
  });
});
