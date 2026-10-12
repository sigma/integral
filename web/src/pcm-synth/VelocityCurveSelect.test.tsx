import { afterEach, describe, expect, it } from "vitest";
import { VelocityCurveSelect } from "./VelocityCurveSelect";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

const OPTIONS = ["FIXED", "1", "2", "3", "4", "5", "6", "7"].map((label, value) => ({ value, label }));

function curvePath(value: number): string {
  const { container } = render(
    <VelocityCurveSelect label="Vel Crv" value={value} options={OPTIONS} onChange={() => {}} color="#68c" />,
  );
  const d = container.querySelector("path")!.getAttribute("d")!;
  cleanup();
  return d;
}

describe("VelocityCurveSelect", () => {
  it("draws FIXED flat at full effect and curve 1 as a straight rise", () => {
    expect(curvePath(0)).toMatch(/^M 0 0 .* L 1 0$/);
    expect(curvePath(1)).toMatch(/^M 0 1 .* L 0\.5 0\.5 .* L 1 0$/);
  });

  it("draws a different shape for each curve", () => {
    const paths = OPTIONS.map((o) => curvePath(o.value));
    expect(new Set(paths).size).toBe(OPTIONS.length);
  });
});
