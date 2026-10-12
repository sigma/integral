import { describe, expect, it } from "vitest";
import { sndInstSource } from "./sndInstCatalog";

describe("sndInstSource", () => {
  it("names SN Drum Insts by number, grouped by Roland's category", () => {
    const source = sndInstSource();
    expect(source.name(0)).toBe("OFF");
    expect(source.name(1)).toBe("Studio Kick");
    expect(source.name(294)).toBe("Telephone 3");
    expect(source.name(295)).toBeNull();
    expect([...source.families].sort()).toEqual(
      ["Crash", "Hi-Hat", "Kick", "Other", "Percussion", "Ride", "SFX", "Snare", "Tom"],
    );
  });
});
