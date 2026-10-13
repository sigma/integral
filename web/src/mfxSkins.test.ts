import { describe, expect, it } from "vitest";
import { mfx_params, mfx_type_names } from "../pkg/integral_wasm.js";
import { MFX_SKINS, defaultRows, skinParams } from "./mfxSkins";
import { pairParams } from "./ParamControl";

describe("MFX skins", () => {
  it("name only params the spec has", () => {
    for (const [type, skin] of Object.entries(MFX_SKINS)) {
      const names = new Set(mfx_params(Number(type)).map((d) => d.name));
      for (const name of skinParams(skin)) expect(names, `type ${type}`).toContain(name);
    }
  });

  it("are layout only: no colours", () => {
    const layouts = [
      ...Object.values(MFX_SKINS),
      ...mfx_type_names().map((_, type) => ({ rows: defaultRows(pairParams(mfx_params(type))) })),
    ];
    const keys = (o: object): string[] =>
      Object.entries(o).flatMap(([k, v]) => [k, ...(typeof v === "object" && v !== null ? keys(v) : [])]);
    for (const skin of layouts) expect(keys(skin)).not.toContain("color");
  });

  it("can address every param by name", () => {
    mfx_type_names().forEach((_, type) => {
      const names = mfx_params(type).map((d) => d.name);
      expect(new Set(names).size, `type ${type}: ${names.join(", ")}`).toBe(names.length);
    });
  });
});
