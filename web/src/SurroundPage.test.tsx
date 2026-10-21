import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DisplayKind, MixerField, MixerScopeKind } from "../pkg/integral_wasm.js";
import { SurroundPage } from "./SurroundPage";
import { defaultMixerState, type MixerState } from "./types";
import type { UseMixerResult } from "./useMixer";
import { cleanup, click, render } from "./test/dom";
import { arrowUp, slider } from "./test/toneService";

afterEach(cleanup);

/** The Surround page over `state`, with its writes recorded. */
function draw(state: MixerState = defaultMixerState()) {
  const setMixerField = vi.fn();
  const mixer = { state, setMixerField } as unknown as UseMixerResult;
  const { container } = render(<SurroundPage mixer={mixer} />);
  return { container, setMixerField, state };
}

/** The Section titled `title`. */
function section(container: HTMLElement, title: string): HTMLElement {
  const found = container.querySelector<HTMLElement>(`[data-grid-section="${title}"]`);
  if (!found) throw new Error(`no Section "${title}"`);
  return found;
}

/** Dispatches a pointer event of `type` at `(x, y)` on `target`. */
function pointer(target: Element, type: string, x: number, y: number): void {
  act(() => {
    // jsdom has no PointerEvent; React only reads the type and coordinates.
    const event = new MouseEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y });
    Object.defineProperty(event, "pointerId", { value: 1 });
    target.dispatchEvent(event);
  });
}

describe("SurroundPage", () => {
  it("draws every Control in a Grid Section", () => {
    const { container } = draw();
    const controls = container.querySelectorAll('[role="slider"], [role="radiogroup"], button');
    expect(controls.length).toBeGreaterThan(0);
    for (const control of controls) {
      expect(control.closest("[data-field]")?.closest("[data-grid-section]")).not.toBeNull();
    }
  });

  it("writes the common fields in the Common scope", () => {
    const { container, setMixerField, state } = draw();
    arrowUp(slider(section(container, "Surround"), "Depth"));
    arrowUp(slider(section(container, "Ambience"), "HF Damp"));
    click([...section(container, "Surround").querySelectorAll('[role="radio"]')].find((b) => b.textContent === "Hall 1")!);
    expect(setMixerField.mock.calls).toEqual([
      [MixerScopeKind.Common, 0, MixerField.SurroundDepth, state.surround.depth + 1],
      [MixerScopeKind.Common, 0, MixerField.SurroundAmbienceHfDamp, state.surround.ambienceHfDamp + 1],
      [MixerScopeKind.Common, 0, MixerField.SurroundRoomType, 2],
    ]);
  });

  it("writes each part's controls in its own scope, and the Ext Part's in Ext", () => {
    const { container, setMixerField, state } = draw();
    arrowUp(slider(section(container, "Part 3"), "Amb"));
    arrowUp(slider(section(container, "Ext"), "Width"));
    expect(setMixerField.mock.calls).toEqual([
      [MixerScopeKind.Part, 2, MixerField.SurroundAmbienceSend, state.surround.parts[2]!.ambienceSend + 1],
      [MixerScopeKind.Ext, 0, MixerField.SurroundWidth, state.surround.ext.width + 1],
    ]);
  });

  it("shows the current values", () => {
    const state = defaultMixerState();
    state.surround.parts[4]!.width = 7;
    const { container } = draw(state);
    expect(slider(section(container, "Part 5"), "Width").getAttribute("aria-valuenow")).toBe("7");
  });

  describe("the pad", () => {
    /** The pad's surface, 200 px square at the origin. */
    function pad(container: HTMLElement): HTMLElement {
      const display = container.querySelector(`[data-display="${DisplayKind.SurroundPad}"]`);
      const surface = display?.querySelector<HTMLElement>("[data-surround-pad]");
      if (!surface) throw new Error("no surround pad");
      surface.getBoundingClientRect = () => ({ left: 0, top: 0, width: 200, height: 200 }) as DOMRect;
      return surface;
    }

    /** The dot labelled `label`. */
    function dot(surface: HTMLElement, label: string): HTMLElement {
      const found = [...surface.querySelectorAll<HTMLElement>("[data-surround-dot]")].find(
        (d) => d.textContent === label,
      );
      if (!found) throw new Error(`no dot "${label}"`);
      found.setPointerCapture = () => {};
      return found;
    }

    it("drags a part to a new position", () => {
      const { container, setMixerField } = draw();
      const surface = pad(container);
      pointer(dot(surface, "2"), "pointerdown", 100, 100);
      setMixerField.mockClear();
      pointer(surface, "pointermove", 200, 0);
      pointer(surface, "pointerup", 200, 0);
      expect(setMixerField.mock.calls).toEqual([
        [MixerScopeKind.Part, 1, MixerField.SurroundLr, 127],
        [MixerScopeKind.Part, 1, MixerField.SurroundFb, 127],
      ]);
    });

    it("drags the Ext Part in the Ext scope", () => {
      const { container, setMixerField } = draw();
      const surface = pad(container);
      pointer(dot(surface, "EX"), "pointerdown", 100, 100);
      setMixerField.mockClear();
      pointer(surface, "pointermove", 0, 200);
      expect(setMixerField.mock.calls).toEqual([
        [MixerScopeKind.Ext, 0, MixerField.SurroundLr, 0],
        [MixerScopeKind.Ext, 0, MixerField.SurroundFb, 0],
      ]);
    });

    it("moves the part selected in its Section on a click", () => {
      const { container, setMixerField } = draw();
      click(section(container, "Part 4"));
      const surface = pad(container);
      pointer(surface, "pointerdown", 0, 0);
      expect(setMixerField.mock.calls).toEqual([
        [MixerScopeKind.Part, 3, MixerField.SurroundLr, 0],
        [MixerScopeKind.Part, 3, MixerField.SurroundFb, 127],
      ]);
    });
  });
});
