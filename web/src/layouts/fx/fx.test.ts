// The MFX, Chorus and Reverb variant sets (#360). `layouts.test.ts` runs the
// validator over every key of each; these pin what the sets are made of.
import { describe, expect, it } from "vitest";
import { chorus_type_names, mfx_type_names, reverb_type_names } from "../../../pkg/integral_wasm.js";
import { fxFields, fxTypes, isTwoState, type FxBlock } from "../fields";
import { variantLayout } from "../geometry";
import { VARIANT_SETS } from "../registry";
import type { ControlLayout } from "../types";
import { MFX_LEVEL, MFX_SET } from "./mfx";
import { CHORUS_SET, REVERB_SET, STRIP_CAPACITY } from "./strip";

const SETS = [
  ["mfx", MFX_SET, mfx_type_names],
  ["chorus", CHORUS_SET, chorus_type_names],
  ["reverb", REVERB_SET, reverb_type_names],
] as const;

/** The Controls a key's layout draws, by field. */
function controls(set: typeof MFX_SET, key: number): Map<string, ControlLayout<string>> {
  const out = new Map<string, ControlLayout<string>>();
  for (const item of variantLayout(set, key).items) {
    if (item.kind !== "display" && item.kind !== "alternatives") out.set(item.field, item);
  }
  return out;
}

describe.each(SETS)("the %s variant set", (block, set, typeNames) => {
  it("is in the registry, so the validator walks it", () => {
    expect(VARIANT_SETS).toContain(set);
  });

  it("is keyed by every type core enumerates", () => {
    expect(set.keys).toEqual(typeNames().map((_, i) => i));
  });

  it("names each parameter of a type once, so a name is an address", () => {
    for (const type of set.keys) {
      const ids = fxFields(block as FxBlock, type).ids;
      expect(new Set(ids).size, `${block} type ${type}: ${ids.join(", ")}`).toBe(ids.length);
    }
  });

  it("draws a two-state parameter as a switch and the rest as knobs, where generated", () => {
    for (const type of set.keys.filter((k) => !set.variants.has(k))) {
      const fields = fxFields(block as FxBlock, type);
      for (const [field, c] of controls(set, type)) {
        if (block === "mfx" && field === MFX_LEVEL) continue;
        expect(c.kind, `${block} type ${type} ${field}`).toBe(isTwoState(fields, field) ? "switch" : "knob");
      }
    }
  });
});

describe("the MFX variant set", () => {
  it("has a hand layout for each of the six hand-laid types", () => {
    expect([...MFX_SET.variants.keys()].sort((a, b) => a - b)).toEqual([1, 22, 28, 29, 31, 34]);
  });

  it("puts the output Level fader on the same Cells for every type", () => {
    const levels = MFX_SET.keys
      .map((type) => controls(MFX_SET, type).get(MFX_LEVEL))
      .filter((c) => c !== undefined);
    // Every type but Thru has a Level.
    expect(levels).toHaveLength(MFX_SET.keys.length - 1);
    for (const c of levels) expect(c).toEqual(levels[0]);
    expect(levels[0]?.kind).toBe("fader");
  });

  it("keeps the hand-laid names and Control kinds", () => {
    const delay = controls(MFX_SET, 34);
    expect(delay.get("Delay Left")).toMatchObject({ kind: "fader", label: "Time L" });
    expect(delay.get("Feedback Mode")).toMatchObject({ kind: "switch", label: "FB" });
    const comp = controls(MFX_SET, 31);
    expect(["Attack", "Threshold", "Post Gain"].map((f) => comp.get(f)?.kind)).toEqual(["fader", "fader", "fader"]);
    expect(controls(MFX_SET, 28).get("Amp Sw")).toMatchObject({ kind: "switch", label: "Amp" });
    expect(controls(MFX_SET, 22).get("Low Gain")?.label).toBe("Lo Gain");
  });

  it("lays the Equalizer out like the mixer EQ: zig-zag, freq and Q left, gain right", () => {
    const eq = controls(MFX_SET, 1);
    const at = (f: string) => {
      const c = eq.get(f);
      return c && [c.kind, c.x, c.y];
    };
    // HI and MID2 zig-zag down the left pair, MID1 and LO down the right, 2 rows apart.
    const left = ["High Freq", "High Gain", "Mid2 Freq", "Mid2 Gain", "Mid2 Q"];
    const right = ["Mid1 Freq", "Mid1 Gain", "Mid1 Q", "Low Gain", "Low Freq"];
    expect(left.map(at)).toEqual(left.map((_, i) => ["knob", (i % 2) * 2, i * 2]));
    expect(right.map(at)).toEqual(right.map((_, i) => ["knob", 6 + (i % 2) * 2, i * 2]));
    for (const [field, c] of eq) {
      if (field === MFX_LEVEL) continue;
      const column = (c.x / 2) % 3;
      expect(column, field).toBe(field.endsWith("Gain") ? 1 : 0);
    }
  });

  it("divides the Equalizer's bands with diagonal rules and names them beside a knob", () => {
    const silk = variantLayout(MFX_SET, 1).silkscreen ?? [];
    const names = silk.flatMap((s) => (s.kind === "label" ? [s.text] : []));
    expect(names).toEqual(["HI", "MID2", "MID1", "LO"]);
    const diagonals = silk.filter(
      (s) => s.kind === "rule" && new Set(s.points.map(([, y]) => y)).size > 1 && new Set(s.points.map(([x]) => x)).size > 1,
    );
    expect(diagonals).toHaveLength(2);
  });
});

describe("the Chorus and Reverb variant sets", () => {
  it("reserve room for exactly the largest type", () => {
    const most = (["chorus", "reverb"] as const).flatMap((b) => fxTypes(b).map((t) => fxFields(b, t).ids.length));
    expect(Math.max(...most)).toBe(STRIP_CAPACITY);
  });

  it("zig-zag one Control per 2-Cell row down two columns, like the mixer EQ", () => {
    const delay = [...controls(CHORUS_SET, 2).values()];
    expect(delay.map((c) => [c.x, c.y])).toEqual(delay.map((_, i) => [(i % 2) * 2, i * 2]));
  });
});
