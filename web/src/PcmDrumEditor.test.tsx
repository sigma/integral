import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  ParameterFamily,
  PcmdWmtField,
  WasmDeviceState,
  pcmd_wmt_spec,
} from "../pkg/integral_wasm.js";
import { PcmDrumEditor } from "./PcmDrumEditor";
import { familyVar } from "./synth-ui";
import { cleanup, render } from "./test/dom";
import { fakeToneService } from "./test/toneService";

afterEach(cleanup);

describe("PcmDrumEditor", () => {
  it("colours a knob by its field spec's family, not its section's", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={1} service={service} />);
    await act(async () => {});

    // WMT Pan sits in the WAVE section (Pitch), but its spec says Time.
    const family = pcmd_wmt_spec(PcmdWmtField.Pan).family;
    expect(family).toBe(ParameterFamily.Time);
    const pan = container.querySelector('[title="Wave Pan"] [role="slider"]');
    if (!pan) throw new Error("no Wave Pan knob");
    const fills = [...pan.querySelectorAll("circle")].map((c) => c.style.fill);
    expect(fills).toContain(familyVar(family));
    expect(fills).not.toContain(familyVar(ParameterFamily.Pitch));
  });
});
