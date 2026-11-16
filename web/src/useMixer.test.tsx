import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MixerField, MixerScopeKind, WasmDeviceState } from "../pkg/integral_wasm.js";
import { InitProvider, useInit } from "./InitFlow";
import type { IntegraService } from "./integra";
import { StudioSetSelector } from "./StudioSetSelector";
import { LOAD_POLL_MS, STUDIO_SET_SWITCH_DELAY_MS, useMixer, type UseMixerResult } from "./useMixer";
import { button, cleanup, click, render } from "./test/dom";
import { fakeInitService } from "./test/initService";
import { fakeToneService } from "./test/toneService";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

/** Let the mixer's requestAnimationFrame sync run. */
async function frame(): Promise<void> {
  await act(() => new Promise<void>((resolve) => requestAnimationFrame(() => resolve())));
}

/** Wait `ms`, then let the sync that follows run. */
async function waitThenSync(ms: number): Promise<void> {
  await act(() => new Promise<void>((resolve) => setTimeout(resolve, ms)));
  await frame();
}

/** How long a load's end takes to be noticed: one load poll, and some. */
const POLL_SETTLE_MS = LOAD_POLL_MS * 2;
/** Past the wait between a Studio Set change and its mixer read. */
const SWITCH_SETTLE_MS = STUDIO_SET_SWITCH_DELAY_MS + LOAD_POLL_MS;

/**
 * A mixer on a device whose Temporary Studio Set is the test's to set, and
 * whose loads end when the test says, an Init service with it and the
 * Studio Set selector the mixer feeds.
 */
function studioSetMixer() {
  const device = new WasmDeviceState(0x10);
  const { service: toneService } = fakeToneService(device);
  const init = fakeInitService();
  const service = { ...toneService, initialize: init.initialize } as unknown as IntegraService;
  const temporary = { name: "MY SET", pc: 3 };
  let loading = true;
  const readState = device.readState.bind(device);
  vi.spyOn(device, "readState").mockImplementation(() => ({
    ...(readState() as object),
    studioSetName: temporary.name,
    studioSetPC: temporary.pc,
  }));
  vi.spyOn(device, "mixerLoading").mockImplementation(() => loading);
  vi.spyOn(device, "loadMixer").mockImplementation(() => {
    loading = true;
  });

  let mixer: UseMixerResult | undefined;
  function InitTrigger() {
    const { request } = useInit();
    return <button onClick={() => request({ kind: "studio-set" })}>Init…</button>;
  }
  function Probe() {
    mixer = useMixer(service);
    return (
      <InitProvider service={service} mixer={mixer}>
        <InitTrigger />
        <div data-testid="selector">
          <StudioSetSelector
            currentPC={mixer.state.studioSetPC}
            names={mixer.state.studioSetNames}
            onSelect={mixer.switchStudioSet}
            onLoadNames={() => {}}
          />
        </div>
      </InitProvider>
    );
  }
  const { container } = render(<Probe />);
  return {
    container,
    temporary,
    mixer: () => mixer!,
    finishInit: init.finish,
    /** The load in flight ends; its end shows after a poll. */
    endLoad: () => {
      loading = false;
    },
    /** What the selector shows for the current slot. */
    shown: () => container.querySelector('[data-testid="selector"] button')!.textContent,
  };
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

  it("names the slot after the Temporary when the first read ends", async () => {
    const { endLoad, shown, mixer } = studioSetMixer();
    endLoad();
    await waitThenSync(POLL_SETTLE_MS);
    expect(mixer().state.studioSetNames.get(3)).toBe("MY SET");
    expect(shown()).toContain("4: MY SET");
  });

  it("leaves the slot's stored name after a Studio Set Init's re-read", async () => {
    const { container, endLoad, shown, temporary, finishInit, mixer } = studioSetMixer();
    endLoad();
    await waitThenSync(POLL_SETTLE_MS);

    click(button(container, "Init…"));
    click(button(container, "Init"));
    // The Init resets the Temporary, not the stored slot.
    temporary.name = "INIT STUDIO";
    await finishInit();
    expect(mixer().state.loading).toBe(true);
    endLoad();
    await waitThenSync(POLL_SETTLE_MS);
    expect(mixer().state.loading).toBe(false);
    expect(mixer().state.studioSetName).toBe("INIT STUDIO");
    expect(mixer().state.studioSetNames.get(3)).toBe("MY SET");
    expect(shown()).toContain("4: MY SET");
  });

  it("leaves the names alone on any other re-read", async () => {
    const { endLoad, temporary, mixer } = studioSetMixer();
    endLoad();
    await waitThenSync(POLL_SETTLE_MS);

    // An unsaved rename, then a retry or a plugin refresh.
    temporary.name = "EDITED";
    act(() => mixer().reload());
    endLoad();
    await waitThenSync(POLL_SETTLE_MS);
    expect(mixer().state.studioSetNames.get(3)).toBe("MY SET");
  });

  it("records the new slot's name after a Studio Set change", async () => {
    const { endLoad, shown, temporary, mixer } = studioSetMixer();
    endLoad();
    await waitThenSync(POLL_SETTLE_MS);

    act(() => mixer().switchStudioSet(7));
    temporary.name = "OTHER SET";
    temporary.pc = 7;
    await waitThenSync(SWITCH_SETTLE_MS);
    expect(mixer().state.loading).toBe(true);
    endLoad();
    await waitThenSync(POLL_SETTLE_MS);
    expect(mixer().state.studioSetNames.get(7)).toBe("OTHER SET");
    expect(mixer().state.studioSetNames.get(3)).toBe("MY SET");
    expect(shown()).toContain("8: OTHER SET");
  });
});
