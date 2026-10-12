import { describe, expect, it } from "vitest";
import { sn_drum_inst_supports_variation } from "../pkg/integral_wasm.js";

describe("SN Drum Variations", () => {
  it("follow the families each Inst lists", () => {
    const supported = (inst: number) => [0, 1, 2, 3, 4, 5, 6, 7].filter((v) => sn_drum_inst_supports_variation(inst, v));
    expect(supported(1)).toEqual([0, 1, 2, 3, 4, 5, 6]); // Studio Kick: Flam/Buzz
    expect(supported(8)).toEqual([0, 1, 2, 3, 4, 5, 6, 7]); // Studio Sn: Flam/Buzz/Roll
    expect(supported(294)).toEqual([0]); // Telephone 3: none
  });
});
