import { describe, expect, it } from "vitest";
import { ParameterFamily, familyColour } from "../../pkg/integral_wasm.js";
import { FAMILIES, familyProperty, installFamilyPalette } from "./family";

describe("installFamilyPalette", () => {
  it("sets one custom property per family from the core palette", () => {
    const root = document.createElement("div");
    installFamilyPalette(root);
    expect(FAMILIES).toHaveLength(6);
    expect(root.style.getPropertyValue("--family-time")).toBe(
      familyColour(ParameterFamily.Time),
    );
    for (const family of FAMILIES) {
      expect(root.style.getPropertyValue(familyProperty(family))).toBe(
        familyColour(family),
      );
    }
  });
});
