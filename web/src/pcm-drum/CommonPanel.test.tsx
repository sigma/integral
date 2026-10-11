import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CommonPanel } from "./CommonPanel";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

describe("CommonPanel", () => {
  it("reports Kit Level edits", () => {
    const onKitLevelChange = vi.fn();
    const { container } = render(
      <CommonPanel
        common={{ kitName: "Kit", kitLevel: 100 }}
        common2={null}
        onKitLevelChange={onKitLevelChange}
      />,
    );
    const fader = container.querySelector('[role="slider"][aria-label="Kit Lv"]');
    if (!fader) throw new Error("no Kit Lv fader");

    act(() => {
      fader.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }),
      );
    });

    expect(onKitLevelChange).toHaveBeenCalledWith(101);
  });
});
