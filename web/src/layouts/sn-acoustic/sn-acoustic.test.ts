// The SN Acoustic layouts: every instrument core lists has an INST variant in
// one footprint, the pianos and TW Organ by hand and the rest generated, and
// the whole screen is registered for the validator.
import { describe, expect, it } from "vitest";
import { SnaInstPage, sna_inst_page_kind } from "../../../pkg/integral_wasm.js";
import { snaInsts } from "../../snaInstCatalog";
import { allElements, itemRect, variantLayout } from "../geometry";
import { screenLayouts, SECTION_LAYOUTS, VARIANT_SETS } from "../registry";
import { validateScreen, validateVariantSet } from "../validate";
import { SNA_AMP_LAYOUT } from "./amp";
import { SNA_COMMON_LAYOUT } from "./common";
import { modifyFields } from "./fields";
import { SNA_FILTER_LAYOUT } from "./filter";
import { AC_PIANO_BODY, SNA_INST_SET, TW_ORGAN_BODY, packModifyParams } from "./inst";
import { SNA_SCREEN } from "./screen";
import { SNA_VIBRATO_LAYOUT } from "./vibrato";

describe("SN-A INST variant set", () => {
  it("is keyed by every instrument core lists", () => {
    expect(SNA_INST_SET.keys).toEqual(snaInsts().map((i) => i.number));
    expect(SNA_INST_SET.keys.length).toBeGreaterThan(100);
  });

  it("passes the validator for every instrument", () => {
    expect(validateVariantSet(SNA_INST_SET)).toEqual([]);
  });

  it("lays out the acoustic pianos and TW Organ by hand, and generates the rest", () => {
    for (const inst of snaInsts()) {
      const page = sna_inst_page_kind(inst.variation, inst.instNumber);
      const hand = SNA_INST_SET.variants.get(inst.number);
      if (page === SnaInstPage.AcPiano) expect(hand, inst.name).toBe(AC_PIANO_BODY);
      else if (page === SnaInstPage.TwOrgan) expect(hand, inst.name).toBe(TW_ORGAN_BODY);
      else expect(hand, inst.name).toBeUndefined();
    }
    expect([...SNA_INST_SET.variants.values()]).toContain(AC_PIANO_BODY);
    expect([...SNA_INST_SET.variants.values()]).toContain(TW_ORGAN_BODY);
  });

  it("generates a Control for each Modify Parameter, a switch for each two-state one", () => {
    // Glockenspiel (list entry 10): Mallet Hardness, Roll Speed, Variation.
    const glock = snaInsts().find((i) => i.name === "Glockenspiel")!;
    const fields = modifyFields(glock.number);
    expect(fields.ids).toEqual([1, 2, 10]);
    const items = packModifyParams(glock.number).items;
    expect(items.map((i) => (i.kind === "alternatives" || i.kind === "display" ? null : [i.kind, i.field]))).toEqual([
      ["knob", 1],
      ["knob", 2],
      ["knob", 10],
    ]);
    // Some instrument has a two-state parameter; it gets a switch.
    const twoState = snaInsts().find((i) =>
      modifyFields(i.number).ids.some((id) => modifyFields(i.number).optionCount(id) === 2),
    )!;
    const kinds = packModifyParams(twoState.number).items.map((i) => i.kind);
    expect(kinds).toContain("switch");
  });

  it("keeps one footprint whatever the instrument", () => {
    for (const key of SNA_INST_SET.keys) {
      const layout = variantLayout(SNA_INST_SET, key);
      expect([layout.w, layout.h]).toEqual([SNA_INST_SET.w, SNA_INST_SET.h]);
      for (const el of allElements(layout.items)) {
        const r = itemRect(el);
        expect(r.x + r.w).toBeLessThanOrEqual(SNA_INST_SET.w);
        expect(r.y + r.h).toBeLessThanOrEqual(SNA_INST_SET.h);
      }
    }
  });
});

describe("SN-A screen", () => {
  it("passes the validator", () => {
    expect(validateScreen(SNA_SCREEN)).toEqual([]);
  });

  it("is registered, with every Section and the INST variant set", () => {
    expect(screenLayouts()).toContain(SNA_SCREEN);
    expect(VARIANT_SETS).toContain(SNA_INST_SET);
    for (const layout of [SNA_COMMON_LAYOUT, SNA_VIBRATO_LAYOUT, SNA_FILTER_LAYOUT, SNA_AMP_LAYOUT]) {
      expect(SECTION_LAYOUTS).toContain(layout);
    }
  });
});
