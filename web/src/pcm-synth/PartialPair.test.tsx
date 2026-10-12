import { afterEach, describe, expect, it } from "vitest";
import { WasmDeviceState, pcms_partial_size } from "../../pkg/integral_wasm.js";
import { PartialPair, type PairPartial } from "./PartialRow";
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
  const { container } = render(<PartialPair first={0} structure={structure} upper={half()} lower={half()} />);
  // A box is a panel headed `label`, directly in a grid cell placed by PartialPair.
  const spans = (label: string) =>
    [...container.querySelectorAll("div")]
      .filter((d) => d.firstElementChild?.textContent === label && d.parentElement?.style.gridColumn)
      .map((d) => d.parentElement!.style.gridRow);
  return { container, spans };
}

describe("PartialPair", () => {
  it("keeps independent partials (Structure 1) in their own rows, each with its OUTPUT", () => {
    const { spans, container } = layout(0);
    expect(spans("FILTER")).toEqual(["1", "2"]);
    expect(spans("OUTPUT")).toEqual(["1", "2"]);
    expect(container.textContent).not.toContain("Follows");
  });

  it("runs a merged pair's blocks after the join across both rows, with one OUTPUT", () => {
    // Structure 2: WG1→TVA1 and WG2 mix, then TVF1 → TVF2 → TVA2.
    const { spans, container } = layout(1);
    expect(spans("FILTER")).toEqual(["1 / span 2", "1 / span 2"]);
    expect(spans("AMP")).toEqual(["1", "1 / span 2"]);
    expect(spans("OUTPUT")).toEqual(["1 / span 2"]);
    expect(spans("CONTROL")).toEqual(["1", "2"]);
    // Partial 1's pan modulation follows partial 2.
    expect(container.querySelector('[title*="Follows partial 2"]')).not.toBeNull();
  });
});
