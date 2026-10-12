import { describe, expect, it } from "vitest";
import { snaInstNumberOf, snaInstSource, snaInsts } from "./snaInstCatalog";

describe("snaInstSource", () => {
  it("lists every instrument by name, grouped by Roland's category", () => {
    const source = snaInstSource();
    expect(source.name(1)).toBe("Concert Grand");
    expect(source.waves).toHaveLength(127);
    expect(source.families).toContain("Ac.Piano");
    // Expansion instruments say where they come from.
    expect(source.waves.find((w) => w.name.startsWith("Santoor"))?.name).toBe("Santoor (ExSN1)");
  });

  it("maps a tone's Inst Variation and Number to its list entry and back", () => {
    expect(snaInstNumberOf(64, 0)).toBe(1); // Concert Grand
    expect(snaInstNumberOf(127, 127)).toBe(0); // unlisted
    const organ = snaInsts().find((i) => i.name === "TW Organ")!;
    expect([organ.variation, organ.instNumber]).toEqual([65, 0]);
  });
});
