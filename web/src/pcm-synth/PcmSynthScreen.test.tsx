import { afterEach, describe, expect, it } from "vitest";
import {
  ParameterFamily,
  PcmsPartialField,
  WasmDeviceState,
  pcms_partial_size,
  pcms_partial_spec,
  pcms_pmt_size,
} from "../../pkg/integral_wasm.js";
import { CELL } from "../synth-ui/grid";
import { familyVar } from "../synth-ui";
import { LANE_Y } from "../layouts/pcm-synth/pair";
import { ROW_H } from "../layouts/pcm-synth/sections";
import { PAIR_Y } from "../layouts/pcm-synth/screen";
import { PcmSynthScreen } from "./PcmSynthScreen";
import type { PcmSynthPartial, PcmSynthPmt } from "./types";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

/** The byte length of a block from its 7-bit RQ1 size. */
const blockLength = (size: Uint8Array) => Array.from(size).reduce((n, b) => n * 128 + b, 0);

/** A partial parsed from an all-zero block, enough to lay out. */
function partial(): PcmSynthPartial {
  return new WasmDeviceState(0x10).applyPcmsPartial(new Uint8Array(blockLength(pcms_partial_size()))) as PcmSynthPartial;
}

/** A PMT with every partial on, pair 1 & 2 on Structure `s12` and pair 3 & 4 on `s34`. */
function pmt(s12: number, s34: number): PcmSynthPmt {
  const p = new WasmDeviceState(0x10).applyPcmsPmt(new Uint8Array(blockLength(pcms_pmt_size()))) as PcmSynthPmt;
  return {
    ...p,
    structureType12: s12,
    structureType34: s34,
    partialEntries: p.partialEntries.map((e) => ({ ...e, partialSwitch: 1 })),
  };
}

const noop = () => {};

function layout(s12: number, s34 = 0) {
  const { container } = render(
    <PcmSynthScreen
      common={null}
      pmt={pmt(s12, s34)}
      partials={[0, 1, 2, 3].map(partial)}
      mfx={null}
      mfxHandlers={{ onTypeChange: noop, onHeaderParam: noop, onNibParam: noop }}
      onCommon={noop}
      onPmt={noop}
      onPmtEntry={noop}
      onPartial={noop}
    />,
  );
  // Every Section is drawn from its layout, and carries its title.
  const placementOf = (d: Element) => d.closest<HTMLElement>("[data-section]")!;
  const boxes = (label: string) => [...container.querySelectorAll(`[data-grid-section="${label}"]`)];
  /** The screen ids of the Sections showing `label`. */
  const sections = (label: string) => boxes(label).map((d) => placementOf(d).dataset.section);
  /** The top row, in Cells, of each Section showing `label`. */
  const rows = (label: string) => boxes(label).map((d) => parseFloat(placementOf(d).style.top) / CELL);
  const arrows = () => container.querySelectorAll("[data-screen-silkscreen] path[data-arrow]").length;
  return { container, sections, rows, arrows };
}

describe("PcmSynthScreen", () => {
  it("keeps independent partials (Structure 1) in their own rows, each with its OUTPUT", () => {
    const { sections, rows, container } = layout(0);
    // Each row is one partial: no numbers needed.
    expect(container.textContent).not.toContain("FILTER 1");
    expect(sections("FILTER")).toEqual(["p12.upper.tvf", "p12.lower.tvf", "p34.upper.tvf", "p34.lower.tvf"]);
    expect(rows("OUTPUT")).toEqual([PAIR_Y[0], PAIR_Y[0] + LANE_Y.lower, PAIR_Y[1], PAIR_Y[1] + LANE_Y.lower]);
    expect(container.textContent).not.toContain("Follows");
  });

  it("runs a merged pair's blocks after the join across both rows, with one OUTPUT", () => {
    // Structure 2: WG1→TVA1 and WG2 mix, then TVF1 → TVF2 → TVA2.
    const { sections, rows, container } = layout(1);
    const middle = PAIR_Y[0] + ROW_H / 2;
    for (const label of ["FILTER 1", "FILTER 2", "AMP 2", "OUTPUT 2"]) expect(rows(label), label).toEqual([middle]);
    expect(sections("OUTPUT 1")).toEqual([]);
    expect(rows("AMP 1")).toEqual([PAIR_Y[0]]);
    expect(rows("CTRL 1")).toEqual([PAIR_Y[0]]);
    // The WAVE boxes head both lanes in the same column.
    expect(sections("WAVE 1")).toEqual(["p12.upper.wave"]);
    expect(sections("WAVE 2")).toEqual(["p12.lower.wave"]);
    // The LFOs name the partial whose depths they send.
    expect(sections("LFO1 · 1")).toEqual(["p12.upper.lfo1"]);
    expect(sections("LFO2 · 2")).toEqual(["p12.lower.lfo2"]);
    // Partial 1's pan modulation follows partial 2.
    expect(container.querySelector('[title*="Follows partial 2"]')).not.toBeNull();
  });

  it("puts every partial's LFOs and WAVE in shared columns", () => {
    const { container } = layout(1, 0);
    const left = (id: string) => container.querySelector<HTMLElement>(`[data-section="${id}"]`)!.style.left;
    for (const box of ["lfo1", "lfo2", "wave"]) {
      const ids = ["p12.upper", "p12.lower", "p34.upper", "p34.lower"].map((lane) => `${lane}.${box}`);
      expect(new Set(ids.map(left)), box).toEqual(new Set([left(ids[0]!)]));
    }
  });

  it("switches each partial from its own Section", () => {
    const { container } = layout(0);
    const titles = [...container.querySelectorAll("[data-grid-section]")].map((d) => d.getAttribute("data-grid-section"));
    expect(titles.filter((t) => t?.startsWith("Partial"))).toEqual(["Partial 1", "Partial 2", "Partial 3", "Partial 4"]);
  });
});

describe("PcmSynthScreen ring modulator", () => {
  it("draws the ring modulator as its own Section, centred on both lanes", () => {
    for (const structure of [4, 5, 6, 7, 8, 9]) {
      const { sections, rows } = layout(structure);
      expect(sections("RING"), `type ${structure + 1}`).toEqual(["p12.ring"]);
      // RING's 3 content rows and title, centred on the pair's two lanes.
      expect(rows("RING")).toEqual([PAIR_Y[0] + ROW_H - 2]);
    }
    expect(layout(1).sections("RING")).toEqual([]);
  });

  it("sends partial 2 dry into the mix too, in types 6, 8 and 10", () => {
    // Type 6 over type 5: partial 2's own arrow to the merged block, and the + they meet at.
    expect(layout(5).arrows() - layout(4).arrows()).toBe(2);
  });
});

describe("PcmSynthScreen booster", () => {
  it("shows a BOOST Section only where the Structure boosts", () => {
    const boxes = (structure: number) => layout(structure).sections("BOOST");
    expect(boxes(0)).toEqual([]);
    expect(boxes(1)).toEqual([]);
    // Type 3 between the filters, type 4 where the lanes meet: both after the join.
    expect(boxes(2)).toEqual(["p12.boost"]);
    expect(boxes(3)).toEqual(["p12.boost"]);
    expect(layout(3).rows("BOOST")).toEqual([PAIR_Y[0] + ROW_H / 2]);
  });
});

describe("PcmSynthScreen arrows", () => {
  it("draws only the OUTPUT arrows into MFX for independent pairs, summed at one +", () => {
    const { arrows, container } = layout(0, 0);
    // Four bodies and the lead from their shared glyph.
    expect(arrows()).toBe(5);
    expect([...container.querySelectorAll("[data-glyph]")].map((g) => g.textContent)).toEqual(["+"]);
  });
});

describe("PcmSynthScreen families", () => {
  it("colours a knob by its field's family, which may differ from its box's", () => {
    const { container } = layout(0);
    const amp = container.querySelector<HTMLElement>('[data-grid-section="AMP"]')!;
    const border = amp.firstElementChild as HTMLElement;
    expect(border.style.getPropertyValue("--section-accent")).toBe(familyVar(ParameterFamily.Amp));
    const pan = amp.querySelector('[role="slider"][aria-label="Pan"]')!;
    const lit = [...pan.querySelectorAll("circle")].map((c) => c.style.fill);
    expect(lit).toContain(familyVar(pcms_partial_spec(PcmsPartialField.Pan).family));
    expect(pcms_partial_spec(PcmsPartialField.Pan).family).toBe(ParameterFamily.Time);
  });
});
