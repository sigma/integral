// The layout validator (ADR 0005): every layout and every variant key is
// valid, and each check fails on the kind of error it guards against.
import { describe, expect, it } from "vitest";
import {
  DisplayKind,
  MixerField,
  MixerScopeKind,
  PcmdPartialField,
  PcmsPartialField,
  mfx_params,
  mfx_type_names,
  pcms_partial_count,
  pcms_partial_spec,
} from "../../pkg/integral_wasm.js";
import {
  PCMD_AMP_ENV_LAYOUT,
  PCMD_FILTER_ENV_LAYOUT,
  PCMD_FILTER_LAYOUT,
  PCMD_MISC_LAYOUT,
  PCMD_PITCH_ENV_LAYOUT,
  PCMD_SCREEN,
  PCMD_WMT_LAYOUT,
} from "./drum/pcmDrum";
import { SND_SCREEN, SND_TONE_LAYOUT } from "./drum/snDrum";
import { elementCount, enumFields, fxFields, mixerFields, type FieldKey } from "./fields";
import { MFX_PANEL_FRAME, MFX_SET } from "./fx/mfx";
import { variantLayout } from "./geometry";
import { EQ_LAYOUT } from "./mixer/eq";
import { mixerScreen } from "./mixer/screen";
import { STRIP_SET } from "./mixer/strip";
import { REGION_VARIANT_SETS, SECTION_LAYOUTS, VARIANT_SETS, screenLayouts } from "./registry";
import { SNA_AMP_LAYOUT } from "./sn-acoustic/amp";
import { SNA_COMMON_LAYOUT } from "./sn-acoustic/common";
import { SNA_FILTER_LAYOUT } from "./sn-acoustic/filter";
import { SNA_INST_SET } from "./sn-acoustic/inst";
import { surroundScreen } from "./surround/screen";
import { SURROUND_COMMON_LAYOUT, SURROUND_PAD_LAYOUT } from "./surround/sections";
import type {
  ItemLayout,
  RegionVariantSet,
  ScreenLayout,
  ScreenRegion,
  SectionBody,
  SectionLayout,
  SectionSilk,
  VariantSet,
} from "./types";
import {
  validateClearance,
  validateRegionSet,
  validateScreen,
  validateSection,
  validateVariantSet,
  type Check,
} from "./validate";

/**
 * Layouts from before `validateClearance`, still drawing some Control or
 * Display under their border. Clear one, then drop it from here.
 */
const UNDER_BORDER = new Set<object>([
  SURROUND_COMMON_LAYOUT,
  SURROUND_PAD_LAYOUT,
  SNA_COMMON_LAYOUT,
  SNA_FILTER_LAYOUT,
  SNA_AMP_LAYOUT,
  SND_TONE_LAYOUT,
  PCMD_WMT_LAYOUT,
  PCMD_PITCH_ENV_LAYOUT,
  PCMD_FILTER_ENV_LAYOUT,
  PCMD_AMP_ENV_LAYOUT,
  PCMD_FILTER_LAYOUT,
  PCMD_MISC_LAYOUT,
  STRIP_SET,
  MFX_SET,
  SNA_INST_SET,
]);

describe("every layout", () => {
  it.each(SECTION_LAYOUTS.map((l) => [l.title, l] as const))("Section %s is valid", (_, layout) => {
    expect(validateSection(layout)).toEqual([]);
  });
  it.each(VARIANT_SETS.map((s) => [s.name, s] as const))("variant set %s is valid", (_, set) => {
    expect(validateVariantSet(set)).toEqual([]);
  });
  it.each(SECTION_LAYOUTS.filter((l) => !UNDER_BORDER.has(l)).map((l) => [l.title, l] as const))(
    "Section %s keeps clear of its border",
    (_, layout) => {
      expect(validateClearance(layout)).toEqual([]);
    },
  );
  it.each(VARIANT_SETS.filter((s) => !UNDER_BORDER.has(s)).map((s) => [s.name, s] as const))(
    "variant set %s keeps clear of its border",
    (_, set) => {
      for (const key of set.keys) expect(validateClearance(variantLayout(set, key), `${set.name}[${key}]`)).toEqual([]);
    },
  );
  it("lists only layouts still drawn under their border as such", () => {
    const sets = VARIANT_SETS.filter((s) => UNDER_BORDER.has(s));
    const under = [
      ...SECTION_LAYOUTS.filter((l) => UNDER_BORDER.has(l) && validateClearance(l).length > 0),
      ...sets.filter((s) => s.keys.some((k) => validateClearance(variantLayout(s, k)).length > 0)),
    ];
    expect(under).toHaveLength(UNDER_BORDER.size);
  });
  it.each(screenLayouts().map((s) => [s.name, s] as const))("screen %s is valid", (_, screen) => {
    expect(validateScreen(screen)).toEqual([]);
  });
  it.each(REGION_VARIANT_SETS.map((s) => [s.name, s] as const))("region variant set %s is valid", (_, set) => {
    expect(validateRegionSet(set)).toEqual([]);
  });
  it("lists the mixer EQ, strips and screens", () => {
    expect(SECTION_LAYOUTS).toContain(EQ_LAYOUT);
    expect(VARIANT_SETS).toContain(STRIP_SET);
    expect(screenLayouts()).toEqual(expect.arrayContaining([mixerScreen(false).layout, mixerScreen(true).layout]));
  });
  it("lists the Surround screen and every Section it places", () => {
    expect(screenLayouts()).toContain(surroundScreen());
    for (const { section } of surroundScreen().sections) expect(SECTION_LAYOUTS).toContain(section);
  });
  it("lists the drum editors and every Section they place", () => {
    expect(screenLayouts()).toEqual(expect.arrayContaining([SND_SCREEN, PCMD_SCREEN]));
    for (const screen of [SND_SCREEN, PCMD_SCREEN]) {
      // The MFX placement hosts `MfxPanel`, a CSS Section around the MFX variant set.
      for (const p of screen.sections) if (p.section !== MFX_PANEL_FRAME) expect(SECTION_LAYOUTS).toContain(p.section);
    }
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
    const ok = withItems([], [{ kind: "arrow", from: { field: MixerField.EqSwitch, side: "bottom" }, to: { field: MixerField.EqMidGain, side: "bottom" } }]);
    expect(validateSection(ok)).toEqual([]);
    const bad = withItems([], [{ kind: "arrow", from: { field: MixerField.EqSwitch, side: "bottom" }, to: { display: DisplayKind.AmpEnvelope, side: "left" } }]);
    expect(checks(validateSection(bad))).toEqual(["arrow"]);
  });

  it("arrow: no free Cells out of an endpoint's side", () => {
    // The knob's right side abuts the other knob.
    const bad = withItems([], [{ kind: "arrow", from: { field: MixerField.EqLowGain, side: "right" }, to: { field: MixerField.EqSwitch, side: "top" } }]);
    const problems = validateSection(bad);
    expect(checks(problems)).toEqual(["arrow"]);
    expect(problems[0]?.message).toBe("no route through free Cells");
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

describe("validateClearance", () => {
  /** A side-titled Section, 6×6, holding `items`. */
  const side = (items: ItemLayout<MixerField>[]): SectionLayout<MixerField> => ({
    ...BASE,
    titleEdge: "side",
    w: 6,
    h: 6,
    items,
    silkscreen: [],
  });
  const top = (items: ItemLayout<MixerField>[]): SectionLayout<MixerField> => ({ ...side(items), titleEdge: "top" });

  it("passes Controls clear of the border", () => {
    expect(validateClearance(top([{ kind: "switch", field: MixerField.EqSwitch, x: 2, y: 0 }]))).toEqual([]);
    expect(validateClearance(side([{ kind: "switch", field: MixerField.EqSwitch, x: 0, y: 1 }]))).toEqual([]);
  });

  it("clearance: any name in the first row, with no title over it", () => {
    // The border's accent runs through the top Cell row, where names sit.
    const knob = side([{ kind: "knob", field: MixerField.EqLowGain, x: 2, y: 0 }]);
    expect(checks(validateClearance(knob))).toEqual(["clearance"]);
    expect(validateClearance({ ...knob, titleEdge: "top" })).toEqual([]);
  });

  it("clearance: a switch, list or Display on the bottom, right or untitled left edge", () => {
    // A switch fills its Cell; the border runs through the last row and column.
    expect(checks(validateClearance(top([{ kind: "switch", field: MixerField.EqSwitch, x: 2, y: 5 }])))).toEqual(["clearance"]);
    expect(checks(validateClearance(top([{ kind: "switch", field: MixerField.EqSwitch, x: 4, y: 2 }])))).toEqual(["clearance"]);
    expect(checks(validateClearance(top([{ kind: "switch", field: MixerField.EqSwitch, x: 0, y: 2 }])))).toEqual(["clearance"]);
    const scope: ItemLayout<MixerField> = { kind: "display", display: DisplayKind.AmpEnvelope, x: 1, y: 4, w: 4, h: 2 };
    expect(checks(validateClearance(top([scope])))).toEqual(["clearance"]);
    // Knobs and faders keep their own margin there.
    expect(validateClearance(top([{ kind: "fader", field: MixerField.PartLevel, x: 4, y: 2, h: 4 }]))).toEqual([]);
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
    // Without the arrow, which can't route to a Section off the lattice either.
    const bad = { ...SCREEN, silkscreen: [], sections: [...SCREEN.sections.slice(0, 1), { id: "b", section: frame(3, 4), x: 5.5, y: 0 }] };
    expect(checks(validateScreen(bad))).toEqual(["lattice"]);
  });

  it("lattice: a placed Section with no content Cells", () => {
    const bad = { ...SCREEN, sections: [...SCREEN.sections.slice(0, 1), { id: "b", section: frame(4, 0), x: 5, y: 0 }] };
    expect(checks(validateScreen(bad))).toEqual(["lattice"]);
  });

  it("overlap: two Sections on one Cell", () => {
    const bad = { ...SCREEN, lanes: [], silkscreen: [], sections: [...SCREEN.sections, { id: "c", section: frame(1, 1), x: 3, y: 3 }] };
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

  it("arrow: glyphs into two targets in one Lane Cell", () => {
    const into = (from: string, to: string) => ({ kind: "arrow", from: { section: from, side: from === "a" ? "right" : "left" }, to: { section: to, side: to === "a" ? "right" : "left" }, glyph: "+" }) as const;
    expect(validateScreen({ ...SCREEN, silkscreen: [into("a", "b")] })).toEqual([]);
    const problems = validateScreen({ ...SCREEN, silkscreen: [into("a", "b"), into("b", "a")] });
    expect(checks(problems)).toEqual(["arrow"]);
    expect(problems[0]?.where).toBe("screen: glyph at 4.5,2.5");
  });

  it("arrow: no Lane out of an endpoint's side", () => {
    const bad: ScreenLayout = { ...SCREEN, silkscreen: [{ kind: "arrow", from: { section: "a", side: "top" }, to: { section: "b", side: "left" } }] };
    const problems = validateScreen(bad);
    expect(checks(problems)).toEqual(["arrow"]);
    expect(problems[0]?.message).toBe("no route through the Lanes");
  });
});

/** The PCM Synth Partial fields, with core's element counts. */
const PCMS_PARTIAL = enumFields("PcmsPartialField", PcmsPartialField, pcms_partial_spec, { count: pcms_partial_count });

describe("array fields", () => {
  /** A Section holding just `item`, over the PCM Synth Partial fields. */
  const holding = (item: ItemLayout<PcmsPartialField>): SectionLayout<PcmsPartialField> => ({
    title: "AMP",
    titleEdge: "top",
    w: 4,
    h: 6,
    fields: PCMS_PARTIAL,
    items: [item],
  });
  const fader = (field: PcmsPartialField, index?: number): ItemLayout<PcmsPartialField> => ({
    kind: "fader",
    field,
    index,
    x: 1,
    y: 0,
    h: 4,
  });

  it("takes each field's element count from core", () => {
    expect(elementCount(PCMS_PARTIAL, PcmsPartialField.Level)).toBe(1);
    expect(elementCount(PCMS_PARTIAL, PcmsPartialField.TvaEnvLevel)).toBe(3);
    expect(elementCount(PCMS_PARTIAL, PcmsPartialField.LfoStep)).toBe(16);
    // A space without counts holds single-valued fields only.
    expect(elementCount(mixerFields([MixerScopeKind.Part]), MixerField.EqLowGain)).toBe(1);
  });

  it("passes a Control on one element of an array field", () => {
    expect(validateSection(holding(fader(PcmsPartialField.TvaEnvLevel, 2)))).toEqual([]);
    expect(validateSection(holding(fader(PcmsPartialField.LfoStep, 15)))).toEqual([]);
  });

  it.each([
    ["past the field's count", PcmsPartialField.TvaEnvLevel, 3, "element 3 outside 0..2"],
    ["negative", PcmsPartialField.TvaEnvLevel, -1, "element -1 outside 0..2"],
    ["not whole", PcmsPartialField.TvaEnvLevel, 0.5, "element 0.5 outside 0..2"],
    ["missing on an array field", PcmsPartialField.TvaEnvLevel, undefined, "an array field of 3: name one element"],
    ["on a single-valued field", PcmsPartialField.Level, 0, "not an array field: names no element"],
  ])("field: an element index %s", (_, field, index, message) => {
    const problems = validateSection(holding(fader(field, index)));
    expect(checks(problems)).toEqual(["field"]);
    expect(problems[0]?.message).toBe(message);
  });

  it("field: a PCM Drum envelope fader past its field's count", () => {
    const past: ItemLayout<PcmdPartialField> = { kind: "fader", field: PcmdPartialField.PitchEnvTime, index: 4, x: 0, y: 5, h: 4 };
    const layout: SectionLayout<PcmdPartialField> = {
      ...PCMD_PITCH_ENV_LAYOUT,
      items: [...PCMD_PITCH_ENV_LAYOUT.items.filter((i) => i.kind !== "fader" || i.label !== "L0"), past],
    };
    const problems = validateSection(layout);
    expect(checks(problems)).toEqual(["field"]);
    expect(problems[0]?.where).toBe("PITCH ENV: fader PitchEnvTime[4]");
  });

  /** A one-key variant set over the TVA envelope levels, placing `indices`. */
  const levelsSet = (indices: readonly number[]): VariantSet<number, PcmsPartialField> => ({
    title: "AMP",
    titleEdge: "top",
    w: 6,
    h: 4,
    name: "amp",
    keys: [0],
    fieldsOf: () =>
      enumFields("PcmsPartialField", PcmsPartialField, pcms_partial_spec, {
        keep: (f) => f === PcmsPartialField.TvaEnvLevel,
        count: pcms_partial_count,
      }),
    variants: new Map([[0, { items: indices.map((i) => ({ ...fader(PcmsPartialField.TvaEnvLevel, i), x: 2 * i, y: 0 })) }]]),
    generate: () => ({ items: [] }),
  });

  it("counts each element of an array field as placed", () => {
    expect(validateVariantSet(levelsSet([0, 1, 2]))).toEqual([]);
  });

  it("unplaced: a variant that leaves one element of an array field out", () => {
    const problems = validateVariantSet(levelsSet([0, 2]));
    expect(checks(problems)).toEqual(["unplaced"]);
    expect(problems.map((p) => p.where)).toEqual(["amp: [0] TvaEnvLevel[1]"]);
  });
});

describe("PCM Drum envelopes", () => {
  const faders = (layout: SectionLayout<FieldKey>) =>
    layout.items.flatMap((i) => (i.kind === "fader" ? [i.label] : []));

  it("take their faders from core's binding: a fader per stored time and level", () => {
    expect(faders(PCMD_PITCH_ENV_LAYOUT)).toEqual(["L0", "L1", "L2", "L3", "L4", "T1", "T2", "T3", "T4"]);
    // The device fixes the TVA's L0 and L4: only its curve draws them.
    expect(faders(PCMD_AMP_ENV_LAYOUT)).toEqual(["L1", "L2", "L3", "T1", "T2", "T3", "T4"]);
  });
});

/** {@link SCREEN}'s Sections and arrow, as a region. */
const REGION: ScreenRegion = { sections: SCREEN.sections, lanes: SCREEN.lanes, silkscreen: SCREEN.silkscreen };

/** A region set over keys 0 and 1, both {@link REGION} unless `variants` says otherwise. */
function regionSet(
  variants: ReadonlyMap<number, ScreenRegion> = new Map([[0, REGION], [1, REGION]]),
  sectionsOf: (key: number) => readonly string[] = () => ["a", "b"],
): RegionVariantSet<number> {
  return { name: "region", w: SCREEN.w, h: SCREEN.h, keys: [0, 1], sectionsOf, variants };
}

describe("validateRegionSet", () => {
  it("passes the base fixture", () => {
    expect(validateRegionSet(regionSet())).toEqual([]);
  });

  it("unplaced: a variant that leaves a Section of its key out", () => {
    const problems = validateRegionSet(regionSet(undefined, (key) => (key ? ["a", "b", "c"] : ["a", "b"])));
    expect(checks(problems)).toEqual(["unplaced"]);
    expect(problems[0]?.where).toBe("region: [1] c");
  });

  it("unplaced: a key with no variant", () => {
    expect(checks(validateRegionSet(regionSet(new Map([[0, REGION]]))))).toEqual(["unplaced"]);
  });

  it("field: a variant placing a Section its key doesn't have", () => {
    expect(checks(validateRegionSet(regionSet(undefined, () => ["a"])))).toEqual(["field"]);
  });

  it("field: a variant for a key core doesn't define", () => {
    const set = regionSet(new Map([[0, REGION], [1, REGION], [2, REGION]]));
    expect(checks(validateRegionSet(set))).toEqual(["field"]);
  });

  it("containment: a variant outside the set's footprint", () => {
    const wide: ScreenRegion = { ...REGION, sections: [...REGION.sections, { id: "c", section: frame(1, 1), x: 9, y: 0 }] };
    const problems = validateRegionSet(regionSet(new Map([[0, REGION], [1, wide]]), (key) => (key ? ["a", "b", "c"] : ["a", "b"])));
    expect(checks(problems)).toEqual(["containment"]);
  });
});
