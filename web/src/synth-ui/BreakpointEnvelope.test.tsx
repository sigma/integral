import { afterEach, describe, expect, it, vi } from "vitest";
import { BreakpointEnvelope } from "./BreakpointEnvelope";
import { cleanup, render } from "../test/dom";
import { arrowUp, slider } from "../test/toneService";

afterEach(cleanup);

describe("BreakpointEnvelope", () => {
  it("shows a fixed level as a read-only fader", () => {
    const onChange = vi.fn();
    const stage = (label: string) => ({ label, value: 64, min: 0, max: 127, defaultValue: 0, onChange });
    const { container } = render(
      <BreakpointEnvelope
        times={[stage("T1"), stage("T2")]}
        levels={[{ label: "L0", at: 0, text: "0" }, stage("L1"), { label: "L2", at: 0, text: "0" }]}
      />,
    );
    const l0 = slider(container, "L0");
    expect(l0.getAttribute("aria-readonly")).toBe("true");
    arrowUp(l0);
    expect(onChange).not.toHaveBeenCalled();
    arrowUp(slider(container, "L1"));
    expect(onChange).toHaveBeenCalledWith(65);
  });
});
