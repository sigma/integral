// The mixer strip variant set and screen: every strip kind is laid out by
// hand, places its Scope's fields, and lines up with its neighbours.
import { describe, expect, it } from "vitest";
import { DisplayKind, MixerField, MixerScopeKind } from "../../../pkg/integral_wasm.js";
import { deviceSpec } from "../../deviceSpec";
import { allElements, itemRect, sectionCells, variantLayout } from "../geometry";
import { validateVariantSet } from "../validate";
import { EQ_LAYOUT } from "./eq";
import { mixerScreen } from "./screen";
import { STRIP_KINDS, STRIP_SET, stripFields } from "./strip";

/** The placed fields of a strip kind. */
const placed = (kind: MixerScopeKind) =>
  allElements(variantLayout(STRIP_SET, kind).items).flatMap((el) => (el.kind === "display" ? [] : [el.field]));

describe("mixer strip", () => {
  it("is keyed by every strip kind, each laid out by hand", () => {
    expect([...STRIP_KINDS].sort()).toEqual(
      [MixerScopeKind.Part, MixerScopeKind.Ext, MixerScopeKind.Master, MixerScopeKind.CompEqUnit].sort(),
    );
    for (const kind of STRIP_KINDS) expect(STRIP_SET.variants.has(kind)).toBe(true);
    expect(validateVariantSet(STRIP_SET)).toEqual([]);
  });

  it("leaves the EQ to the EQ Section", () => {
    for (const kind of STRIP_KINDS) {
      expect(placed(kind)).not.toContain(MixerField.EqSwitch);
      expect(stripFields(kind).ids).not.toContain(MixerField.EqLowGain);
    }
  });

  it("places the strip controls core had no field for", () => {
    expect(placed(MixerScopeKind.Part)).toEqual(
      expect.arrayContaining([MixerField.PartAmbienceSend, MixerField.SoloPart, MixerField.DrumCompEqPart]),
    );
    expect(placed(MixerScopeKind.CompEqUnit)).toContain(MixerField.CompOutputAssign);
  });

  it("swaps the FX sends for the ambience send on the same Cells", () => {
    const group = variantLayout(STRIP_SET, MixerScopeKind.Part).items.find((i) => i.kind === "alternatives");
    expect(group?.kind === "alternatives" && group.options.map((o) => o.name)).toEqual(["fx", "surround"]);
  });

  it("draws the compressor's curves as Displays", () => {
    const displays = allElements(variantLayout(STRIP_SET, MixerScopeKind.CompEqUnit).items).flatMap((el) =>
      el.kind === "display" ? [el.display] : [],
    );
    expect(displays.sort()).toEqual([DisplayKind.CompEnvelope, DisplayKind.CompTransferCurve].sort());
  });

  it("ends every fader on the strip's bottom row", () => {
    for (const kind of STRIP_KINDS) {
      const fader = variantLayout(STRIP_SET, kind).items.find((i) => i.kind === "fader");
      expect(fader && itemRect(fader).y + itemRect(fader).h).toBe(STRIP_SET.h);
    }
  });
});

describe("mixer screen", () => {
  const { part_count: parts, comp_eq_unit_count: units } = deviceSpec();

  it("gives each strip a 4-Cell column: parts, Ext, Chorus, Reverb, master", () => {
    expect(mixerScreen(false).layout.w).toBe((parts + 4) * 4);
    expect(mixerScreen(true).layout.w).toBe((parts + units + 4) * 4);
  });

  it("starts every strip on the row below the EQ, Ext included", () => {
    const strips = mixerScreen(true).layout.sections.filter((p) => p.id.startsWith("strip."));
    expect(strips).toHaveLength(parts + 1 + units + 1);
    for (const p of strips) expect(p.y).toBe(sectionCells(EQ_LAYOUT).h);
    expect(mixerScreen(false).slots.get("eq.ext")).toBeUndefined();
  });

  it("names what every placement draws", () => {
    for (const screen of [mixerScreen(false), mixerScreen(true)]) {
      for (const p of screen.layout.sections) expect(screen.slots.has(p.id)).toBe(true);
    }
    expect(mixerScreen(false).slots.get("strip.part.3")).toEqual({ role: "strip", kind: MixerScopeKind.Part, index: 3 });
  });
});
