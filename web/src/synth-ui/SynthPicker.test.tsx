import { afterEach, describe, expect, it, vi } from "vitest";
import { SHORT_LIST_MAX, SynthPicker } from "./SynthPicker";
import { FOOTPRINT } from "./grid";
import { expectFillsFootprint, rootOf } from "../test/grid";
import { cleanup, click, option, render, typeInto } from "../test/dom";

afterEach(cleanup);

/** More options than a short list holds, so the popover keeps its search box. */
const insts = [
  { value: 0, label: "OFF" },
  { value: 1, label: "Studio Kick", family: "Kick" },
  { value: 2, label: "Room Kick", family: "Kick" },
  { value: 3, label: "Studio Snare", family: "Snare" },
  ...Array.from({ length: 10 }, (_, i) => ({ value: 4 + i, label: `Tom ${i + 1}`, family: "Tom" })),
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
    expect(listed().slice(0, 4)).toEqual(["OFF", "Studio Kick", "Room Kick", "Studio Snare"]);

    typeInto(document.querySelector("input")!, "studio");
    expect(listed()).toEqual(["Studio Kick", "Studio Snare"]);

    click(option(document, "Studio Snare"));
    expect(onChange).toHaveBeenCalledWith(3);
    // Picking closes the list.
    expect(listed()).toEqual([]);
  });

  it("shows a value without a name as its number", () => {
    const { container } = render(<SynthPicker label="Inst" value={99} options={insts} onChange={() => {}} />);
    expect(container.querySelector("button")!.textContent).toContain("99");
  });

  it("opens a short list as a dropdown, no search box, and writes the option picked", () => {
    const onChange = vi.fn();
    const shapes = ["TRI", "SIN", "SAW", "SQR", "S&H", "RND"].map((label, value) => ({ value, label }));
    const { container } = render(<SynthPicker label="Shape" value={0} options={shapes} onChange={onChange} />);
    click(container.querySelector("button")!);
    expect(document.querySelector("input")).toBeNull();
    expect(listed()).toEqual(["TRI", "SIN", "SAW", "SQR", "S&H", "RND"]);
    // The current option takes the focus the search box would.
    expect(document.activeElement?.textContent).toBe("TRI");

    click(option(document, "SQR"));
    expect(onChange).toHaveBeenCalledWith(3);
    expect(listed()).toEqual([]);
  });

  it("keeps the search box once a list has more than the short-list maximum", () => {
    const at = (n: number) => Array.from({ length: n }, (_, value) => ({ value, label: `Wave ${value}` }));
    const short = render(<SynthPicker label="Wave" value={0} options={at(SHORT_LIST_MAX)} onChange={() => {}} />);
    click(short.container.querySelector("button")!);
    expect(document.querySelector("input")).toBeNull();
    cleanup();

    const long = render(<SynthPicker label="Wave" value={0} options={at(SHORT_LIST_MAX + 1)} onChange={() => {}} />);
    click(long.container.querySelector("button")!);
    expect(document.querySelector("input")).not.toBeNull();
  });
});
