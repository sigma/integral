import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import { MixerField, MixerScopeKind, WasmDeviceState } from "../pkg/integral_wasm.js";
import { useMixer, type UseMixerResult } from "./useMixer";
import { cleanup, render } from "./test/dom";
import { fakeToneService } from "./test/toneService";

afterEach(cleanup);

/** Let the mixer's requestAnimationFrame sync run. */
async function frame(): Promise<void> {
  await act(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));
}

describe("useMixer", () => {
  it("keeps the effective routing in step with the state", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    let mixer: UseMixerResult | undefined;
    function Probe() {
      mixer = useMixer(service);
      return null;
    }
    render(<Probe />);
    expect(mixer!.state.routing).toBeNull();

    act(() => mixer!.setMixerField(MixerScopeKind.Part, 0, MixerField.PartOutputAssign, 3));
    await frame();
    expect(mixer!.state.routing?.routes[0]?.landsOn).toEqual([{ kind: "output", output: 3 }]);
  });
});
