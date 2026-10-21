// The PCM Synth screen: every Structure Type fits the pair's one footprint,
// places the Sections core's routing gives it, and moves no neighbour.
import { describe, expect, it } from "vitest";
import { PcmsPmtField, pcms_pmt_spec } from "../../../pkg/integral_wasm.js";
import { routingOf } from "../../pcm-synth/structureRouting";
import { sectionCells } from "../geometry";
import { REGION_VARIANT_SETS, screenLayouts } from "../registry";
import type { ScreenLayout, SectionPlacement } from "../types";
import { validateRegionSet, validateScreen } from "../validate";
import { LANE_Y, PATH_X, PCMS_PAIR, pairSectionsOf, type PairSection } from "./pair";
import { PAIR_Y, PCMS_IDS, pairSectionId, parsePairSectionId, pcmsScreen } from "./screen";

const { min, max } = pcms_pmt_spec(PcmsPmtField.StructureType12);
const STRUCTURES = Array.from({ length: max - min + 1 }, (_, i) => min + i);

/** Where each Section of `screen` sits, by id. */
const where = (screen: ScreenLayout) => new Map(screen.sections.map((p) => [p.id, p]));

/** A pair variant's Section placement, by id. */
const placed = (structure: number, id: PairSection): SectionPlacement | undefined =>
  PCMS_PAIR.variants.get(structure)!.sections.find((p) => p.id === id);

describe("PCM Synth pair", () => {
  it("has a variant for every Structure Type core defines", () => {
    expect(PCMS_PAIR.keys).toEqual(STRUCTURES);
    expect([...PCMS_PAIR.variants.keys()]).toEqual(STRUCTURES);
    expect(STRUCTURES).toHaveLength(10);
  });

  it("is listed for the validator, and every Structure Type passes", () => {
    expect(REGION_VARIANT_SETS).toContain(PCMS_PAIR);
    expect(validateRegionSet(PCMS_PAIR)).toEqual([]);
  });

  it("takes BOOST and RING from core's routing", () => {
    const has = (s: number, id: PairSection) => pairSectionsOf(s).includes(id);
    expect(STRUCTURES.filter((s) => has(s, "boost"))).toEqual([2, 3]);
    expect(STRUCTURES.filter((s) => has(s, "ring"))).toEqual([4, 5, 6, 7, 8, 9]);
    // Independent partials keep both OUTPUTs; a merged pair has partial 2's only.
    expect(STRUCTURES.filter((s) => has(s, "upper.out"))).toEqual([0]);
    expect(STRUCTURES.every((s) => has(s, "lower.out"))).toBe(true);
  });

  it("keeps every lane's heads in fixed columns", () => {
    for (const s of STRUCTURES) {
      for (const id of ["upper.wave", "lower.wave", "upper.ctrl", "lower.lfo1", "lower.partial"] as const) {
        expect(placed(s, id), `type ${s + 1} ${id}`).toEqual(placed(0, id));
      }
    }
    expect(placed(0, "lower.wave")!.x).toBe(placed(0, "upper.wave")!.x);
  });

  it("keeps independent lanes in their own rows, with no arrows", () => {
    const variant = PCMS_PAIR.variants.get(0)!;
    expect(variant.silkscreen).toEqual([]);
    for (const side of ["upper", "lower"] as const) {
      const ids = [`${side}.tvf`, `${side}.tva`, `${side}.out`] as const;
      expect(ids.map((id) => placed(0, id)!.y)).toEqual([LANE_Y[side], LANE_Y[side], LANE_Y[side]]);
      // One straight block from the heads on: each box abuts the last.
      expect(placed(0, ids[0])!.x).toBe(PATH_X);
    }
  });

  it("centres a merged pair's blocks after the join on both lanes, in core's signal order", () => {
    for (const s of STRUCTURES.slice(1)) {
      const join = routingOf(s).join!;
      const after = join.after.map((b) =>
        b.kind === "booster" ? "boost" : `${b.side}.${b.kind === "filter" ? "tvf" : "tva"}`,
      ) as PairSection[];
      const ids: PairSection[] = [...(join.kind === "booster" ? ["boost" as const] : []), ...after, "lower.out"];
      const rows = ids.map((id) => placed(s, id)!);
      const centre = (p: SectionPlacement) => 2 * p.y + sectionCells(p.section).h;
      // All on the pair's middle, left to right.
      expect(new Set(rows.map(centre)), `type ${s + 1}`).toEqual(new Set([PCMS_PAIR.h]));
      expect(rows.map((p) => p.x)).toEqual([...rows.map((p) => p.x)].sort((a, b) => a - b));
    }
  });

  it("draws partial 2's dry signal into the mix in types 6, 8 and 10", () => {
    const arrows = (s: number) => PCMS_PAIR.variants.get(s)!.silkscreen;
    for (const s of [5, 7, 9]) {
      expect(arrows(s).length - arrows(s - 1).length, `type ${s + 1}`).toBe(1);
      expect(arrows(s).filter((a) => a.glyph === "+")).toHaveLength(2);
      expect(arrows(s - 1).filter((a) => a.glyph === "+")).toHaveLength(0);
    }
  });
});

describe("PCM Synth screen", () => {
  it("lists one screen per Structure Type, each valid", () => {
    for (const s of STRUCTURES) {
      const screen = screenLayouts().find((l) => l.name === pcmsScreen([s, s]).name);
      expect(screen, `type ${s + 1}`).toBeDefined();
      expect(validateScreen(screen!)).toEqual([]);
    }
  });

  it("passes the validator with the pairs on different Structures", () => {
    for (const s of STRUCTURES) {
      expect(validateScreen(pcmsScreen([s, 9 - s])), `types ${s + 1}, ${10 - s}`).toEqual([]);
    }
  });

  it("never moves a Section outside the pair whose Structure changes", () => {
    for (const pair of [0, 1]) {
      const screens = STRUCTURES.map((s) => pcmsScreen(pair ? [0, s] : [s, 0]));
      const base = where(screens[0]!);
      const footprint = { x: 0, y: PAIR_Y[pair]!, w: PCMS_PAIR.w, h: PCMS_PAIR.h };
      for (const screen of screens) {
        for (const p of screen.sections) {
          const own = parsePairSectionId(p.id)?.pair === pair;
          if (!own) {
            expect(p, `${screen.name} ${p.id}`).toEqual(base.get(p.id));
            continue;
          }
          // The pair's own Sections stay inside its footprint.
          const { w, h } = sectionCells(p.section);
          expect(p.x >= footprint.x && p.y >= footprint.y, p.id).toBe(true);
          expect(p.x + w <= footprint.x + footprint.w && p.y + h <= footprint.y + footprint.h, p.id).toBe(true);
        }
      }
    }
  });

  it("sums every partial's OUTPUT into MFX", () => {
    const into = (screen: ScreenLayout) =>
      screen.silkscreen.filter((a) => a.to.section === PCMS_IDS.mfx).map((a) => [a.from.section, a.glyph]);
    expect(into(pcmsScreen([0, 1]))).toEqual([
      [pairSectionId(0, "upper.out"), "+"],
      [pairSectionId(0, "lower.out"), "+"],
      [pairSectionId(1, "lower.out"), "+"],
    ]);
  });
});
