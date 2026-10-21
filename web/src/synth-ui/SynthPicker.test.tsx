import { afterEach, describe, expect, it, vi } from "vitest";
import { SynthPicker } from "./SynthPicker";
import { FOOTPRINT } from "./grid";
import { expectFillsFootprint, rootOf } from "../test/grid";
import { cleanup, click, render, typeInto } from "../test/dom";

afterEach(cleanup);

const insts = [
  { value: 0, label: "OFF" },
  { value: 1, label: "Studio Kick", family: "Kick" },
  { value: 2, label: "Room Kick", family: "Kick" },
  { value: 3, label: "Studio Snare", family: "Snare" },
];

/** The options the open popover lists, by name. */
function listed(): string[] {
  return [...document.querySelectorAll('[role="option"]')].map((o) => o.textContent ?? "");
}

describe("SynthPicker", () => {
  it("fills 6×1 and shows its name and the current option's", () => {
    const { container } = render(<SynthPicker label="Inst" value={2} options={insts} onChange={() => {}} />);
    const root = rootOf(container);
    expectFillsFootprint(root, FOOTPRINT.picker);
    expect(root.textContent).toContain("Inst");
    expect(container.querySelector("button")!.textContent).toContain("Room Kick");
  });

  it("filters the options by typed text and writes the one picked", () => {
    const onChange = vi.fn();
    const { container } = render(<SynthPicker label="Inst" value={0} options={insts} onChange={onChange} />);
    click(container.querySelector("button")!);
    expect(listed()).toEqual(["OFF", "Studio Kick", "Room Kick", "Studio Snare"]);

    typeInto(document.querySelector("input")!, "studio");
    expect(listed()).toEqual(["Studio Kick", "Studio Snare"]);

    const snare = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find(
      (o) => o.textContent === "Studio Snare",
    )!;
    click(snare);
    expect(onChange).toHaveBeenCalledWith(3);
    // Picking closes the list.
    expect(listed()).toEqual([]);
  });

  it("shows a value without a name as its number", () => {
    const { container } = render(<SynthPicker label="Inst" value={9} options={insts} onChange={() => {}} />);
    expect(container.querySelector("button")!.textContent).toContain("9");
  });
});
