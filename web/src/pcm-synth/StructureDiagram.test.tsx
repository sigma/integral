import { afterEach, describe, expect, it } from "vitest";
import { StructureDiagram } from "./StructureDiagram";
import { cleanup, render } from "../test/dom";

afterEach(cleanup);

function labels(structure: number, first = 1): string[] {
  const { container } = render(<StructureDiagram structure={structure} first={first} />);
  const texts = [...container.querySelectorAll("text")].map((t) => t.textContent ?? "");
  cleanup();
  return texts;
}

describe("StructureDiagram", () => {
  it("draws the pair's blocks in signal order for each type", () => {
    expect(labels(0)).toEqual(["WG1", "TVF1", "TVA1", "WG2", "TVF2", "TVA2"]);
    expect(labels(1)).toEqual(["WG1", "TVA1", "WG2", "+", "TVF1", "TVF2", "TVA2"]);
    expect(labels(2)).toEqual(["WG1", "TVA1", "WG2", "+", "TVF1", "B", "TVF2", "TVA2"]);
    expect(labels(3)).toContain("B");
    expect(labels(9)).toEqual(["WG1", "TVF1", "TVA1", "WG2", "TVF2", "R", "TVA2"]);
  });

  it("numbers the second pair 3 and 4", () => {
    expect(labels(0, 3)).toEqual(["WG3", "TVF3", "TVA3", "WG4", "TVF4", "TVA4"]);
  });
});
