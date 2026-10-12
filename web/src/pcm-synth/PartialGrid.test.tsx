import { afterEach, describe, expect, it } from "vitest";
import { WasmDeviceState, pcms_partial_size } from "../../pkg/integral_wasm.js";
import { PartialGrid, type PairPartial } from "./PartialRow";
import type { PcmSynthPartial } from "./types";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

/** A partial parsed from an all-zero block, enough to lay out. */
function partial(): PcmSynthPartial {
  const size = Array.from(pcms_partial_size()).reduce((n, b) => n * 128 + b, 0);
  return new WasmDeviceState(0x10).applyPcmsPartial(new Uint8Array(size)) as PcmSynthPartial;
}

const half = (): PairPartial => ({ partial: partial(), isOn: true, onParam: () => {}, onSwitch: () => {} });

function layout(structure: number) {
  const { container } = render(
    <PartialGrid pairs={[{ first: 0, structure, upper: half(), lower: half() }]} />,
  );
  // A box is a SectionPanel (it sets its accent as the top border) headed `label`;
  // its block is the nearest element placed on the grid.
  const panels = (label: string) =>
    [...container.querySelectorAll("div")].filter(
      (d) => d.style.borderTopColor && d.firstElementChild?.textContent === label,
    );
  const block = (d: HTMLElement) => d.closest<HTMLElement>("[data-node]")!;
  const spans = (label: string) => panels(label).map((d) => block(d).style.gridRow);
  /** Names of the blocks holding each box headed `label`. */
  const blocks = (label: string) => panels(label).map((d) => block(d).dataset.node);
  return { container, spans, blocks };
}

describe("PartialGrid", () => {
  it("keeps independent partials (Structure 1) in their own rows, each with its OUTPUT", () => {
    const { spans, container } = layout(0);
    // Each row is one partial: no numbers needed.
    expect(container.textContent).not.toContain("FILTER 1");
    expect(spans("FILTER")).toEqual(["1", "2"]);
    expect(spans("OUTPUT")).toEqual(["1", "2"]);
    expect(container.textContent).not.toContain("Follows");
  });

  it("runs a merged pair's blocks after the join across both rows, with one OUTPUT", () => {
    // Structure 2: WG1→TVA1 and WG2 mix, then TVF1 → TVF2 → TVA2.
    const { spans, blocks, container } = layout(1);
    // FILTER 1, FILTER 2, AMP 2 and OUTPUT 2 form one block (named for partial 2's
    // output, 0-based like the MFX arrows); AMP 1 its own.
    expect(new Set(["FILTER 1", "FILTER 2", "AMP 2", "OUTPUT 2"].flatMap(blocks))).toEqual(new Set(["out-1"]));
    expect(blocks("AMP 1")).toEqual(["0-pre-upper"]);
    // The WAVE boxes head both lanes in the same column.
    expect(blocks("WAVE 1")).toEqual(["0-head-upper"]);
    expect(blocks("WAVE 2")).toEqual(["0-head-lower"]);
    expect(spans("FILTER 1")).toEqual(["1 / span 2"]);
    expect(spans("FILTER 2")).toEqual(["1 / span 2"]);
    expect(spans("AMP 1")).toEqual(["1"]);
    expect(spans("AMP 2")).toEqual(["1 / span 2"]);
    expect(spans("OUTPUT 2")).toEqual(["1 / span 2"]);
    expect(spans("CONTROL 1")).toEqual(["1"]);
    // LFO depth faders name the box they target.
    expect(container.querySelector('[role="slider"][aria-label="FLT 1"]')).not.toBeNull();
    expect(container.querySelector('[role="slider"][aria-label="FLT 2"]')).not.toBeNull();
    // Partial 1's pan modulation follows partial 2.
    expect(container.querySelector('[title*="Follows partial 2"]')).not.toBeNull();
  });
});

describe("PartialGrid with both pairs", () => {
  it("puts every partial's LFOs and WAVE in shared columns", () => {
    const { container } = render(
      <PartialGrid pairs={[
        { first: 0, structure: 1, upper: half(), lower: half() },
        { first: 2, structure: 0, upper: half(), lower: half() },
      ]} />,
    );
    const placed = (prefix: string) =>
      [...container.querySelectorAll<HTMLElement>("[data-node]")]
        .filter((d) => d.dataset.node!.includes(prefix))
        .map((d) => `${d.style.gridRow}:${d.style.gridColumn}`);
    expect(placed("-lfo-")).toEqual(["1:2", "2:2", "3:2", "4:2"]);
    expect(placed("-head-")).toEqual(["1:3", "2:3", "3:3", "4:3"]);
  });
});
