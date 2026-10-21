// The layout validator (ADR 0005): every layout and every variant key is
// valid, and each check fails on the kind of error it guards against.
import { describe, expect, it } from "vitest";
import { DisplayKind, MixerField, MixerScopeKind, mfx_params, mfx_type_names } from "../../pkg/integral_wasm.js";
import { fxFields, mixerFields } from "./fields";
import { EQ_LAYOUT } from "./mixer/eq";
import { SCREEN_LAYOUTS, SECTION_LAYOUTS, VARIANT_SETS } from "./registry";
import type { ItemLayout, ScreenLayout, SectionBody, SectionLayout, SectionSilk, VariantSet } from "./types";
import { validateScreen, validateSection, validateVariantSet, type Check } from "./validate";

describe("every layout", () => {
  it.each(SECTION_LAYOUTS.map((l) => [l.title, l] as const))("Section %s is valid", (_, layout) => {
    expect(validateSection(layout)).toEqual([]);
  });
  it.each(VARIANT_SETS.map((s) => [s.name, s] as const))("variant set %s is valid", (_, set) => {
    expect(validateVariantSet(set)).toEqual([]);
  });
  it.each(SCREEN_LAYOUTS.map((s) => [s.name, s] as const))("screen %s is valid", (_, screen) => {
    expect(validateScreen(screen)).toEqual([]);
  });
  it("lists the mixer EQ", () => {
    expect(SECTION_LAYOUTS).toContain(EQ_LAYOUT);
  });
});

/** The checks `problems` failed. */
const checks = (problems: { check: Check }[]): Check[] => [...new Set(problems.map((p) => p.check))];

/** A small valid Section over the mixer fields, to break one way at a time. */
const BASE: SectionLayout<MixerField> = {
  title: "T",
  titleEdge: "top",
  w: 4,
  h: 6,
  fields: mixerFields([MixerScopeKind.Part]),
  items: [
    { kind: "knob", field: MixerField.EqLowGain, x: 0, y: 0 },
    { kind: "knob", field: MixerField.EqMidGain, x: 2, y: 0 },
    { kind: "switch", field: MixerField.EqSwitch, x: 0, y: 3 },
  ],
  silkscreen: [
    { kind: "rule", points: [[0, 3], [4, 3]] },
    { kind: "label", text: "LO", x: 3, y: 4.5 },
  ],
};

/** `BASE` with `items` added and `silkscreen` added. */
function withItems(
  items: ItemLayout<MixerField>[],
  silkscreen: SectionSilk<MixerField>[] = [],
): SectionLayout<MixerField> {
  return { ...BASE, items: [...BASE.items, ...items], silkscreen: [...(BASE.silkscreen ?? []), ...silkscreen] };
}

describe("validateSection", () => {
  it("passes the base fixture", () => {
    expect(validateSection(BASE)).toEqual([]);
  });

  it("lattice: a Control off whole Cells", () => {
    const bad = withItems([{ kind: "switch", field: MixerField.PartMute, x: 1.5, y: 4 }]);
    expect(checks(validateSection(bad))).toEqual(["lattice"]);
  });

  it("lattice: Silkscreen off the half-Cell lattice", () => {
    const bad = withItems([], [{ kind: "label", text: "X", x: 1.25, y: 5 }]);
    expect(checks(validateSection(bad))).toEqual(["lattice"]);
  });

  it("overlap: two Controls on one Cell", () => {
    const bad = withItems([{ kind: "switch", field: MixerField.PartMute, x: 1, y: 2 }]);
    const problems = validateSection(bad);
    expect(checks(problems)).toEqual(["overlap"]);
    expect(problems[0]?.where).toContain("switch PartMute");
  });

  it("overlap: allowed between the choices of one alternatives group, not with its neighbours", () => {
    const group: ItemLayout<MixerField> = {
      kind: "alternatives",
      group: "g",
      x: 2,
      y: 3,
      w: 2,
      h: 3,
      options: [
        { name: "a", items: [{ kind: "knob", field: MixerField.EqMidQ, x: 2, y: 3 }] },
        { name: "b", items: [{ kind: "switch", field: MixerField.PartMute, x: 2, y: 4 }] },
      ],
    };
    expect(validateSection(withItems([group]))).toEqual([]);
    const crowded = withItems([group, { kind: "switch", field: MixerField.EqLowFreq, x: 2, y: 5 }]);
    expect(checks(validateSection(crowded))).toEqual(["overlap"]);
  });

  it("containment: a Control past the Section's edge", () => {
    const bad = withItems([{ kind: "knob", field: MixerField.EqMidQ, x: 3, y: 3 }]);
    expect(checks(validateSection(bad))).toEqual(["containment"]);
  });

  it("containment: an alternative outside its group", () => {
    const bad = withItems([
      {
        kind: "alternatives",
        group: "g",
        x: 2,
        y: 3,
        w: 2,
        h: 1,
        options: [{ name: "a", items: [{ kind: "switch", field: MixerField.PartMute, x: 2, y: 4 }] }],
      },
    ]);
    expect(checks(validateSection(bad))).toEqual(["containment"]);
  });

  it("footprint: a Display under its kind's minimum", () => {
    const tall = { ...BASE, h: 8 };
    const ok = { ...tall, items: [...BASE.items, { kind: "display", display: DisplayKind.AmpEnvelope, x: 0, y: 4, w: 4, h: 2 } as const] };
    expect(validateSection(ok)).toEqual([]);
    const small = { ...tall, items: [...BASE.items, { kind: "display", display: DisplayKind.AmpEnvelope, x: 0, y: 4, w: 3, h: 2 } as const] };
    expect(checks(validateSection(small))).toEqual(["footprint"]);
  });

  it("footprint: an option list shorter than its options", () => {
    // One row per option of Chorus output select, plus one for its name.
    const fields = mixerFields([MixerScopeKind.Common]);
    const layout = (h: number): SectionLayout<MixerField> => ({
      ...BASE,
      fields,
      items: [{ kind: "list", field: MixerField.ChorusOutputSelect, x: 0, y: 0, h }],
      silkscreen: [],
    });
    const need = fields.optionCount(MixerField.ChorusOutputSelect) + 1;
    expect(validateSection(layout(need))).toEqual([]);
    expect(checks(validateSection(layout(need - 1)))).toEqual(["footprint"]);
  });

  it("footprint: a switch bound to a field with more than two values", () => {
    const bad = withItems([{ kind: "switch", field: MixerField.EqMidQ, x: 2, y: 3 }]);
    expect(checks(validateSection(bad))).toEqual(["footprint"]);
  });

  it("field: a Display id core doesn't define", () => {
    const tall = { ...BASE, h: 8 };
    const bad = { ...tall, items: [...BASE.items, { kind: "display", display: 99 as DisplayKind, x: 0, y: 4, w: 4, h: 2 } as const] };
    expect(checks(validateSection(bad))).toEqual(["field"]);
  });

  it("arrow: an endpoint not placed in the Section", () => {
    const ok = withItems([], [{ kind: "arrow", from: { field: MixerField.EqLowGain, side: "right" }, to: { field: MixerField.EqSwitch, side: "top" } }]);
    expect(validateSection(ok)).toEqual([]);
    const bad = withItems([], [{ kind: "arrow", from: { field: MixerField.EqLowGain, side: "right" }, to: { display: DisplayKind.AmpEnvelope, side: "left" } }]);
    expect(checks(validateSection(bad))).toEqual(["arrow"]);
  });

  it("field: an id outside core's list for the Section's Scopes", () => {
    // Not a MixerField at all.
    const unknown = withItems([{ kind: "switch", field: 999 as MixerField, x: 2, y: 3 }]);
    expect(checks(validateSection(unknown))).toEqual(["field"]);
    // A real MixerField, but not one a part strip has.
    const outOfScope = withItems([{ kind: "switch", field: MixerField.MasterLevel, x: 2, y: 3 }]);
    expect(checks(validateSection(outOfScope))).toEqual(["field"]);
  });

  it("field: an FX name core doesn't list for the type", () => {
    const layout: SectionLayout<string> = {
      title: "MFX",
      titleEdge: "side",
      w: 2,
      h: 3,
      fields: fxFields("mfx", 1),
      items: [{ kind: "knob", field: "Low Gain", x: 0, y: 0 }],
    };
    expect(validateSection(layout)).toEqual([]);
    const bad = { ...layout, items: [{ kind: "knob", field: "Lo Gain", x: 0, y: 0 } as const] };
    expect(checks(validateSection(bad))).toEqual(["field"]);
  });
});

/** MFX types, enumerated from core. */
const MFX_TYPES = mfx_type_names().map((_, i) => i);
/** The most params any MFX type has. */
const MOST = Math.max(...MFX_TYPES.map((t) => mfx_params(t).length));
/** Knobs per row in the fixture's footprint. */
const PER_ROW = 8;

/** Packs every param of an MFX type as small knobs, row-major: a stand-in generator. */
function packKnobs(type: number): SectionBody<string> {
  return {
    items: fxFields("mfx", type).ids.map((field, i) => ({
      kind: "knob",
      field,
      x: (i % PER_ROW) * 2,
      y: Math.floor(i / PER_ROW) * 3,
    })),
  };
}

/** An MFX variant set over every type core has, with `variants` by hand. */
function mfxSet(variants: Map<number, SectionBody<string>>): VariantSet<number, string> {
  return {
    name: "mfx",
    title: "MFX",
    titleEdge: "side",
    w: PER_ROW * 2,
    h: Math.ceil(MOST / PER_ROW) * 3,
    keys: MFX_TYPES,
    fieldsOf: (type) => fxFields("mfx", type),
    variants,
    generate: packKnobs,
  };
}

describe("validateVariantSet", () => {
  it("walks every key core enumerates, generated ones included", () => {
    expect(MFX_TYPES.length).toBeGreaterThan(60);
    expect(validateVariantSet(mfxSet(new Map()))).toEqual([]);
  });

  it("unplaced: a hand variant that leaves a parameter of its key out", () => {
    const equalizer = packKnobs(1);
    const missing = { items: equalizer.items.filter((i) => i.kind !== "knob" || i.field !== "Level") };
    const problems = validateVariantSet(mfxSet(new Map([[1, missing]])));
    expect(checks(problems)).toEqual(["unplaced"]);
    expect(problems[0]?.where).toBe("mfx: [1] Level");
  });

  it("containment: a variant outside the set's footprint", () => {
    const wide = { items: [...packKnobs(0).items, { kind: "knob", field: "Level", x: PER_ROW * 2, y: 0 } as const] };
    // Thru has no params, so the stray knob is also an unknown field.
    expect(checks(validateVariantSet(mfxSet(new Map([[0, wide]]))))).toEqual(["containment", "field"]);
  });

  it("field: a hand variant for a key core doesn't define", () => {
    const set = mfxSet(new Map([[MFX_TYPES.length, { items: [] }]]));
    expect(checks(validateVariantSet(set))).toEqual(["field"]);
  });
});

/** A section outline of `w`×`h` content Cells, title on top. */
const frame = (w: number, h: number) => ({ title: "S", titleEdge: "top" as const, w, h });

/** Two Sections with a 1-Cell Lane between them, and an arrow across it. */
const SCREEN: ScreenLayout = {
  name: "screen",
  w: 9,
  h: 5,
  sections: [
    { id: "a", section: frame(4, 4), x: 0, y: 0 },
    { id: "b", section: frame(4, 4), x: 5, y: 0 },
  ],
  lanes: [{ x: 4, y: 0, w: 1, h: 5 }],
  silkscreen: [{ kind: "arrow", from: { section: "a", side: "right" }, to: { section: "b", side: "left" } }],
};

describe("validateScreen", () => {
  it("passes the base fixture", () => {
    expect(validateScreen(SCREEN)).toEqual([]);
  });

  it("lattice: a Section off whole Cells", () => {
    const bad = { ...SCREEN, sections: [...SCREEN.sections.slice(0, 1), { id: "b", section: frame(3, 4), x: 5.5, y: 0 }] };
    expect(checks(validateScreen(bad))).toEqual(["lattice"]);
  });

  it("lattice: a placed Section with no content Cells", () => {
    const bad = { ...SCREEN, sections: [...SCREEN.sections.slice(0, 1), { id: "b", section: frame(4, 0), x: 5, y: 0 }] };
    expect(checks(validateScreen(bad))).toEqual(["lattice"]);
  });

  it("overlap: two Sections on one Cell", () => {
    const bad = { ...SCREEN, lanes: [], sections: [...SCREEN.sections, { id: "c", section: frame(1, 1), x: 3, y: 3 }] };
    expect(checks(validateScreen(bad))).toEqual(["overlap"]);
  });

  it("containment: a Section past the screen's edge", () => {
    const bad = { ...SCREEN, sections: [...SCREEN.sections, { id: "c", section: frame(9, 1), x: 0, y: 5 }] };
    expect(checks(validateScreen(bad))).toEqual(["containment"]);
  });

  it("lane: a Section covering a Lane", () => {
    const bad = { ...SCREEN, sections: [...SCREEN.sections, { id: "c", section: frame(1, 1), x: 4, y: 3 }] };
    expect(checks(validateScreen(bad))).toEqual(["lane"]);
  });

  it("arrow: an endpoint naming no Section", () => {
    const bad: ScreenLayout = { ...SCREEN, silkscreen: [{ kind: "arrow", from: { section: "a", side: "right" }, to: { section: "z", side: "left" } }] };
    expect(checks(validateScreen(bad))).toEqual(["arrow"]);
  });
});
