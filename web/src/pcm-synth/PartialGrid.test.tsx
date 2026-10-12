import { afterEach, describe, expect, it } from "vitest";
import { PcmsPmtField, WasmDeviceState, pcms_partial_size } from "../../pkg/integral_wasm.js";
import { PartialGrid, type PairPartial } from "./PartialRow";
import type { PcmSynthPartial } from "./types";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

/** A partial parsed from an all-zero block, enough to lay out. */
function partial(): PcmSynthPartial {
  const size = Array.from(pcms_partial_size()).reduce((n, b) => n * 128 + b, 0);
  return new WasmDeviceState(0x10).applyPcmsPartial(new Uint8Array(size)) as PcmSynthPartial;
}

/** A pair's booster props, for the pair starting at partial index `first`. */
const boost = (first: number) => ({
  booster: 0,
  onBooster: () => {},
  boosterField: first ? PcmsPmtField.Booster34 : PcmsPmtField.Booster12,
});

const half = (): PairPartial => ({ partial: partial(), isOn: true, onParam: () => {}, onSwitch: () => {} });

function layout(structure: number) {
  const { container } = render(
    <PartialGrid pairs={[{ first: 0, structure, upper: half(), lower: half(), ...boost(0) }]} />,
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
    expect(spans("CTRL 1")).toEqual(["1"]);
    // LFO depth faders name the box they target.
    expect(container.querySelector('[role="slider"][aria-label="FLT 1"]')).not.toBeNull();
    expect(container.querySelector('[role="slider"][aria-label="FLT 2"]')).not.toBeNull();
    // Partial 1's pan modulation follows partial 2.
    expect(container.querySelector('[title*="Follows partial 2"]')).not.toBeNull();
  });
});

describe("PartialGrid ring modulator", () => {
  it("draws the ring modulator as its own box between the lanes and the merged block", () => {
    for (const structure of [4, 5, 6, 7, 8, 9]) {
      const { blocks, spans } = layout(structure);
      expect(blocks("RING"), `type ${structure + 1}`).toEqual(["0-ring"]);
      expect(spans("RING")).toEqual(["1 / span 2"]);
    }
    expect(layout(1).blocks("RING")).toEqual([]);
  });

  it("sends partial 2 dry into the mix too, in types 6, 8 and 10", () => {
    const arrows = (structure: number) => layout(structure).container.querySelectorAll("path[marker-end]").length;
    // Type 6 over type 5: partial 2's own arrow to the merged block, and the + they meet at.
    expect(arrows(5) - arrows(4)).toBe(2);
  });
});

describe("PartialGrid booster", () => {
  it("shows a BOOST box only where the Structure boosts", () => {
    const boxes = (structure: number) => layout(structure).blocks("BOOST");
    expect(boxes(0)).toEqual([]);
    expect(boxes(1)).toEqual([]);
    // Type 3 between the filters, type 4 where the lanes meet: both in the merged block.
    expect(boxes(2)).toEqual(["out-1"]);
    expect(boxes(3)).toEqual(["out-1"]);
  });
});

describe("PartialGrid with both pairs", () => {
  it("puts every partial's LFOs and WAVE in shared columns", () => {
    const { container } = render(
      <PartialGrid pairs={[
        { first: 0, structure: 1, upper: half(), lower: half(), ...boost(0) },
        { first: 2, structure: 0, upper: half(), lower: half(), ...boost(2) },
      ]} />,
    );
    const placed = (prefix: string) =>
      [...container.querySelectorAll<HTMLElement>("[data-node]")]
        .filter((d) => d.dataset.node!.includes(prefix))
        .map((d) => `${d.style.gridRow}:${d.style.gridColumn}`);
    expect(placed("-lfo-")).toEqual(["1:1", "2:1", "3:1", "4:1"]);
    expect(placed("-head-")).toEqual(["1:2", "2:2", "3:2", "4:2"]);
    // Each pair's path has its own columns: pair 1-2's merged block follows its
    // AMP 1 block, whatever the width of pair 3-4's blocks.
    const node = (name: string) => container.querySelector<HTMLElement>(`[data-node="${name}"]`)!;
    expect(node("out-1").style.gridColumn).toBe("2");
    expect(node("out-2").style.gridColumn).toBe("1");
    expect(node("out-1").parentElement).not.toBe(node("out-2").parentElement);
  });
});
