import type { ReactElement } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { PcmsPartialField, SnsPartialField, pcms_partial_spec, sns_partial_spec } from "../../pkg/integral_wasm.js";
import { FilterScope, SNS_FILTER_RESPONSES } from "./FilterScope";
import { switchOptions } from "../fieldSpec";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function pathOf(type: string, cutoff = 64, resonance = 0): string {
  return drawn(<FilterScope type={type} cutoff={cutoff} resonance={resonance} />);
}

/** The path an SN Synth FILTER draws. */
function snsPathOf(mode: string, slope: string, cutoff = 64, resonance = 0): string {
  return drawn(
    <FilterScope type={mode} slope={slope} cutoff={cutoff} resonance={resonance} responses={SNS_FILTER_RESPONSES} />,
  );
}

/** The path `scope` draws. */
function drawn(scope: ReactElement): string {
  const d = render(scope).container.querySelector("path")?.getAttribute("d") ?? "";
  cleanup();
  return d;
}

/** y of the path's sample nearest x; the path is `M x y L x y …`. */
function yAt(d: string, x: number): number {
  const tokens = d.split(" ");
  let best = { dx: Infinity, y: NaN };
  for (let i = 0; i + 2 < tokens.length; i += 3) {
    const dx = Math.abs(Number(tokens[i + 1]) - x);
    if (dx < best.dx) best = { dx, y: Number(tokens[i + 2]) };
  }
  return best.y;
}

describe("FilterScope", () => {
  it("draws every PCM-S filter type", () => {
    for (const { label } of switchOptions(pcms_partial_spec(PcmsPartialField.TvfFilterType))) {
      expect(pathOf(label), label).toBeTruthy();
    }
  });

  it("follows cutoff and resonance", () => {
    // a low-pass passes the low end and cuts the top (y grows downwards)
    expect(yAt(pathOf("LPF"), 0)).toBeLessThan(yAt(pathOf("LPF"), 1));
    expect(pathOf("LPF", 100)).not.toEqual(pathOf("LPF", 20));
    expect(pathOf("LPF", 64, 127)).not.toEqual(pathOf("LPF", 64, 0));
    // fully cut frequencies draw nothing: a low cutoff low-pass stops short of the right edge
    expect(pathOf("LPF", 10).endsWith(" 1 1")).toBe(false);
    // LPF2 and LPF3 ignore Resonance
    expect(pathOf("LPF2", 64, 127)).toEqual(pathOf("LPF2", 64, 0));
  });

  it("draws every SN-S filter mode at both slopes", () => {
    for (const { label: mode } of switchOptions(sns_partial_spec(SnsPartialField.FilterMode))) {
      for (const { label: slope } of switchOptions(sns_partial_spec(SnsPartialField.FilterSlope))) {
        expect(snsPathOf(mode, slope), `${mode} ${slope}`).toBeTruthy();
      }
    }
  });

  it("steepens an SN-S filter at -24dB, and leaves BYP flat", () => {
    // an octave and a half above the cutoff, -24dB cuts deeper (y grows downwards)
    expect(yAt(snsPathOf("LPF", "-24dB", 32), 0.5)).toBeGreaterThan(yAt(snsPathOf("LPF", "-12dB", 32), 0.5));
    expect(yAt(snsPathOf("HPF", "-24dB", 96), 0.5)).toBeGreaterThan(yAt(snsPathOf("HPF", "-12dB", 96), 0.5));
    expect(snsPathOf("LPF", "-12dB", 64, 127)).not.toEqual(snsPathOf("LPF", "-12dB", 64, 0));
    expect(snsPathOf("LPF", "-12dB", 100)).not.toEqual(snsPathOf("LPF", "-12dB", 20));
    expect(yAt(snsPathOf("BYP", "-24dB"), 0)).toEqual(yAt(snsPathOf("BYP", "-24dB"), 1));
  });
});
