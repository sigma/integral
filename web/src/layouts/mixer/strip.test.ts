// The mixer strip variant set and screen: every strip kind is laid out by
// hand, places its Scope's fields, and lines up with its neighbours.
import { describe, expect, it } from "vitest";
import { DisplayKind, MixerField, MixerScopeKind } from "../../../pkg/integral_wasm.js";
import { deviceSpec } from "../../deviceSpec";
import { allElements, itemRect, sectionCells, variantLayout } from "../geometry";
import { validateVariantSet } from "../validate";
import { EQ_LAYOUT } from "./eq";
import { mixerScreen } from "./screen";
import { FX_STRIP_SET, STRIP_KINDS, STRIP_SET, fxStripFields, stripFields } from "./strip";
import { mixerFields } from "../fields";

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
      expect.arrayContaining([MixerField.SurroundAmbienceSend, MixerField.SoloPart, MixerField.DrumCompEqPart]),
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
    expect(displays.sort()).toEqual([DisplayKind.CompEnvelope, DisplayKind.CompTransferCurve, DisplayKind.StripName].sort());
  });

  it("gives every strip kind one level fader, the same length, ending on the strip's bottom row", () => {
    const levels = STRIP_KINDS.map((kind) => {
      const faders = variantLayout(STRIP_SET, kind).items.filter((i) => i.kind === "fader").map(itemRect);
      return faders.reduce((a, b) => (b.h > a.h ? b : a));
    });
    for (const r of levels) expect([r.y + r.h, r.h]).toEqual([STRIP_SET.h, levels[0]!.h]);
    // The old mixer's proportions: a fader about 21 Cells long in a 4-Cell strip (#392).
    expect(STRIP_SET.w).toBe(4);
    expect(levels[0]!.h).toBe(21);
  });

  it("draws the level scale on every level fader, and only there", () => {
    for (const kind of STRIP_KINDS) {
      const faders = variantLayout(STRIP_SET, kind).items.filter((i) => i.kind === "fader");
      const level = faders.reduce((a, b) => (b.h > a.h ? b : a));
      for (const f of faders) expect(f.scale ?? false, `${MixerScopeKind[kind]} ${MixerField[f.field]}`).toBe(f === level);
    }
  });

  it("runs every strip's name up the column left of its level fader, as long as it", () => {
    for (const kind of STRIP_KINDS) {
      const items = allElements(variantLayout(STRIP_SET, kind).items);
      const name = items.find((el) => el.kind === "display" && el.display === DisplayKind.StripName);
      const level = items.filter((el) => el.kind === "fader").map(itemRect).reduce((a, b) => (b.h > a.h ? b : a));
      expect(name && itemRect(name), MixerScopeKind[kind]).toEqual({ x: level.x - 1, y: level.y, w: 1, h: level.h });
    }
  });

  it("puts a part's mute and solo side by side, as the old mixer did", () => {
    const items = allElements(variantLayout(STRIP_SET, MixerScopeKind.Part).items);
    const at = (field: MixerField) => items.find((el) => el.kind !== "display" && el.field === field)!;
    expect(at(MixerField.PartMute).y).toBe(at(MixerField.SoloPart).y);
    expect([at(MixerField.PartMute).x, at(MixerField.SoloPart).x]).toEqual([0, 2]);
  });

  it("draws every output assign as a picker that fits the strip", () => {
    for (const kind of STRIP_SET.keys) {
      for (const el of allElements(variantLayout(STRIP_SET, kind).items)) {
        if (el.kind === "display" || !MixerField[el.field].endsWith("OutputAssign")) continue;
        expect(el.kind, MixerField[el.field]).toBe("picker");
        expect(itemRect(el).w).toBeLessThanOrEqual(STRIP_SET.w);
      }
    }
  });

  it("draws a part's input, its receive channel, as a picker on the strip's first row", () => {
    const input = allElements(variantLayout(STRIP_SET, MixerScopeKind.Part).items).find(
      (el) => el.kind !== "display" && el.field === MixerField.PartReceiveChannel,
    );
    expect(input?.kind).toBe("picker");
    expect(input && itemRect(input)).toEqual({ x: 0, y: 0, w: STRIP_SET.w, h: 1 });
  });

  it("draws the compressor's Displays across the strip", () => {
    for (const el of allElements(variantLayout(STRIP_SET, MixerScopeKind.CompEqUnit).items)) {
      if (el.kind === "display" && el.display !== DisplayKind.StripName) {
        expect(itemRect(el)).toMatchObject({ x: 0, w: STRIP_SET.w });
      }
    }
  });
});

describe("mixer screen", () => {
  const { part_count: parts, comp_eq_unit_count: units } = deviceSpec();

  it("gives each strip a 4-Cell column: parts, Ext, Chorus, Reverb, master", () => {
    expect(mixerScreen(false).layout.w).toBe((parts + 4) * 4);
    expect(mixerScreen(true).layout.w).toBe((parts + units + 4) * 4);
  });

  it("starts every strip on the row below the EQ, Ext and FX included", () => {
    const strips = mixerScreen(true).layout.sections.filter((p) => p.id.startsWith("strip."));
    expect(strips).toHaveLength(parts + 1 + units + 2 + 1);
    for (const p of strips) expect(p.y).toBe(sectionCells(EQ_LAYOUT).h);
    expect(mixerScreen(false).slots.get("eq.ext")).toBeUndefined();
  });

  it("names what every placement draws", () => {
    for (const screen of [mixerScreen(false), mixerScreen(true)]) {
      for (const p of screen.layout.sections) expect(screen.slots.has(p.id)).toBe(true);
    }
    expect(mixerScreen(false).slots.get("strip.part.3")).toEqual({ role: "strip", kind: MixerScopeKind.Part, index: 3 });
    expect(mixerScreen(false).slots.get("strip.reverb")).toEqual({ role: "fx", fx: "reverb", section: "strip" });
  });

  it("gives the FX strips the strips' footprint, their level fader on the same rows", () => {
    expect([FX_STRIP_SET.w, FX_STRIP_SET.h, FX_STRIP_SET.titleEdge]).toEqual([STRIP_SET.w, STRIP_SET.h, STRIP_SET.titleEdge]);
    const level = (items: ReturnType<typeof variantLayout>["items"]) =>
      items.filter((i) => i.kind === "fader").map(itemRect).reduce((a, b) => (b.h > a.h ? b : a));
    const part = level(variantLayout(STRIP_SET, MixerScopeKind.Part).items);
    for (const fx of FX_STRIP_SET.keys) {
      const items = variantLayout(FX_STRIP_SET, fx).items;
      expect(level(items)).toEqual(part);
      expect(items.find((i) => i.kind === "fader" && i.h === part.h)).toMatchObject({ scale: true });
    }
  });

  it("draws each FX field in the role core gives it", () => {
    expect(fxStripFields("chorus")).toEqual({
      switch: MixerField.ChorusSwitch,
      type: MixerField.ChorusType,
      level: MixerField.ChorusLevel,
      outputs: [MixerField.ChorusOutputAssign, MixerField.ChorusOutputSelect],
    });
    expect(fxStripFields("reverb")).toEqual({
      switch: MixerField.ReverbSwitch,
      type: MixerField.ReverbType,
      level: MixerField.ReverbLevel,
      outputs: [MixerField.ReverbOutputAssign],
    });
    for (const fx of FX_STRIP_SET.keys) {
      const { switch: sw, type, level, outputs } = fxStripFields(fx);
      const layout = variantLayout(FX_STRIP_SET, fx);
      expect(layout.titleSwitch).toBe(sw);
      const items = allElements(layout.items);
      expect(items.find((i) => i.kind === "list")).toMatchObject({ field: type });
      expect(items.find((i) => i.kind === "fader")).toMatchObject({ field: level, scale: true });
      expect(items.filter((i) => i.kind === "picker").map((i) => i.kind === "picker" && i.field)).toEqual(outputs);
    }
  });

  it("puts every Common field of the Chorus and Reverb on their FX strip", () => {
    for (const fx of FX_STRIP_SET.keys) {
      const prefix = fx === "chorus" ? "Chorus" : "Reverb";
      const named = mixerFields([MixerScopeKind.Common]).ids.filter((f) => MixerField[f].startsWith(prefix));
      expect([...FX_STRIP_SET.fieldsOf(fx).ids].sort()).toEqual(named.sort());
    }
  });
});
