import { describe, expect, it } from "vitest";
import { MixerField, ParameterFamily } from "../pkg/integral_wasm.js";
import { paramFormatter, paramMeta } from "./paramDefaults";

describe("paramMeta", () => {
  it("reads a mixer field's range, default and family from core", () => {
    expect(paramMeta(MixerField.EqLowGain)).toEqual({
      name: "Low Gain", min: 0, max: 30, defaultValue: 15, family: ParameterFamily.Filter,
    });
    expect(paramMeta(MixerField.PartChorusSend).name).toBe("FX1");
  });

  it("covers the EQ switch", () => {
    expect(paramMeta(MixerField.EqSwitch)).toMatchObject({ min: 0, max: 1 });
    expect(paramFormatter(MixerField.EqSwitch)(1)).toBe("ON");
  });

  it("formats values with core's labels", () => {
    expect(paramFormatter(MixerField.EqMidFreq)(7)).toBe("1.0k");
    expect(paramFormatter(MixerField.CompRatio)(paramMeta(MixerField.CompRatio).max)).toBe("inf:1");
  });

  it("takes only core's enum, so a misspelt field fails tsc", () => {
    // Never called: these lines exist for the type checker.
    const typos = () => [
      // @ts-expect-error: a misspelt field is not a MixerField.
      paramMeta(MixerField.EqLowGian),
      // @ts-expect-error: a string id is not a MixerField.
      paramMeta("EqLowGain"),
    ];
    expect(typos).toBeTypeOf("function");
  });
});
