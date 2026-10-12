import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { KitLevelFader } from "./KitLevelFader";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

describe("KitLevelFader", () => {
  it("reports Kit Level edits", () => {
    const onKitLevelChange = vi.fn();
    const { container } = render(
      <KitLevelFader kitLevel={100} onChange={onKitLevelChange} />,
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
