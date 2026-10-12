import { afterEach, describe, expect, it } from "vitest";
import { SnsPartialField } from "../../pkg/integral_wasm.js";
import { SyncedRateKnob } from "./SyncedRateKnob";
import { cleanup, click, render } from "../test/dom";

afterEach(cleanup);

function knob(tempoSync: number) {
  const writes: [SnsPartialField, number][] = [];
  const { container } = render(
    <SyncedRateKnob label="Rate" field={SnsPartialField.LfoRate} rate={81} tempoSync={tempoSync} note={11}
      onChange={(f, v) => writes.push([f, v])} />,
  );
  const button = (label: string) =>
    [...container.querySelectorAll("button")].find((b) => b.textContent === label)!;
  return { container, writes, button };
}

describe("SyncedRateKnob", () => {
  it("edits the rate in Hz mode", () => {
    const { container, button } = knob(0);
    expect(button("Hz").getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector("[role=slider]")!.getAttribute("aria-valuenow")).toBe("81");
  });

  it("edits the note in note mode and shows its name", () => {
    const { container, button } = knob(1);
    expect(button("♪").getAttribute("aria-pressed")).toBe("true");
    expect(container.querySelector("[role=slider]")!.getAttribute("aria-valuemax")).toBe("19");
    expect(container.textContent).toContain("1/4");
  });

  it("toggles through the tempo-sync switch field", () => {
    const { writes, button } = knob(0);
    click(button("♪"));
    expect(writes).toEqual([[SnsPartialField.LfoTempoSync, 1]]);
  });
});
