import { act } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  StudioSetSetting,
  WasmDeviceState,
  system_common_address,
  system_common_size,
} from "../pkg/integral_wasm.js";
import { StudioSetDrawer } from "./StudioSetDrawer";
import type { IntegraService } from "./integra";
import type { UseMixerResult } from "./useMixer";
import { defaultMixerState, type StudioSetSettings } from "./types";
import { button, cleanup, click, option, render } from "./test/dom";

afterEach(cleanup);

/** The System Common block's length, from the size its read asks for. */
const SYSTEM_COMMON_LEN = Array.from(system_common_size()).reduce((n, b) => n * 128 + b, 0);

/**
 * A System Common dump (offsets from `docs/midi/04-address-map.md`): System
 * Control 1 Source CC01, System Tempo 120, and the two selectors.
 */
function systemDump({ controlSource, tempoAssign }: { controlSource: number; tempoAssign: number }) {
  const data = new Uint8Array(SYSTEM_COMMON_LEN);
  data[0x20] = 0x01; // System Control 1 Source: CC01
  data[0x24] = controlSource; // Control Source: 0 SYSTEM, 1 STUDIO SET
  data.set([0x07, 0x08], 0x26); // System Tempo: 120, nibblized
  data[0x28] = tempoAssign; // Tempo Assign Source: 0 SYSTEM, 1 STUDIO SET
  return data;
}

/** What the device read: Tone Control 1 on CC74 (74), tempo 96, Phase Lock on channel 10. */
const READ: StudioSetSettings = {
  toneControlSources: [74, 0, 0, 0],
  tempo: 96,
  phaseLock: Array.from({ length: 16 }, (_, i) => i === 9),
};

/** The drawer over a mixer that read `READ`, its System Common answering `dump`. */
async function drawer(dump: Uint8Array | null = systemDump({ controlSource: 0, tempoAssign: 1 })) {
  const setStudioSetField = vi.fn();
  const onClose = vi.fn();
  const onOpenSystem = vi.fn();
  const requestData = vi.fn(async (address: number[], size: number[]) => {
    expect(address).toEqual(Array.from(system_common_address()));
    expect(size).toEqual(Array.from(system_common_size()));
    if (!dump) throw new Error("RQ1 timeout");
    return dump;
  });
  const mixer = {
    state: { ...defaultMixerState(), loading: false, studioSetName: "Jazz Trio", studioSetSettings: READ },
    setStudioSetField,
  } as unknown as UseMixerResult;
  const service = { requestData } as unknown as IntegraService;
  const { container } = render(
    <StudioSetDrawer mixer={mixer} service={service} onClose={onClose} onOpenSystem={onOpenSystem} />,
  );
  // Let the System Common read answer.
  await act(async () => {});
  return { container, setStudioSetField, onClose, onOpenSystem, requestData };
}

function group(scope: ParentNode, name: string): HTMLElement {
  const found = scope.querySelector<HTMLElement>(`section[aria-label="${name}"]`);
  if (!found) throw new Error(`no "${name}" group`);
  return found;
}

/** Whether each side of `scope`'s pairs is dimmed, by side. */
function dimming(scope: ParentNode) {
  return [...scope.querySelectorAll<HTMLElement>("[data-side]")].map(
    (s) => `${s.dataset.side}:${s.dataset.dimmed ? "dim" : "lit"}`,
  );
}

describe("the Studio Set settings drawer", () => {
  it("opens on the current Studio Set, its groups stacked, showing what was read", async () => {
    const { container, requestData } = await drawer();
    const aside = container.querySelector('aside[aria-label="Studio Set settings"]')!;
    expect(aside.getAttribute("aria-label")).toBe("Studio Set settings");
    expect(aside.textContent).toContain("Jazz Trio");
    expect([...aside.querySelectorAll("section")].map((s) => s.getAttribute("aria-label"))).toEqual([
      "CONTROL",
      "TEMPO",
      "PHASE LOCK",
    ]);
    expect(requestData).toHaveBeenCalledTimes(1);

    const control = group(container, "CONTROL");
    expect(button(control, "Tone Control 1 Source: CC74")).toBeTruthy();
    expect(control.querySelector('[aria-label="Control Source"]')!.textContent).toBe("SYSTEM");
    expect(control.querySelector('[aria-label="System Control 1 Source"]')!.textContent).toBe("CC01");

    const tempo = group(container, "TEMPO");
    expect(tempo.querySelector('[role="slider"]')!.getAttribute("aria-valuenow")).toBe("96");
    expect(tempo.querySelector('[aria-label="Tempo Assign Source"]')!.textContent).toBe("STUDIO SET");
    expect(tempo.querySelector('[aria-label="System Tempo"]')!.textContent).toBe("120");

    const phaseLock = group(container, "PHASE LOCK");
    expect(phaseLock.textContent).toContain("latency");
    expect(phaseLock.textContent).toContain("SN-A organ");
    const switches = [...phaseLock.querySelectorAll("button")].map((b) => b.textContent);
    expect(switches).toHaveLength(16);
    expect(switches[9]).toBe("ON");
    expect(switches[0]).toBe("OFF");
  });

  it("dims the side of each pair System's selectors put out of force", async () => {
    const studio = await drawer(systemDump({ controlSource: 0, tempoAssign: 1 }));
    // Control Source SYSTEM: the Tone Controls are out of force.
    expect(dimming(group(studio.container, "CONTROL"))).toEqual(
      Array.from({ length: 4 }, () => ["studioSet:dim", "system:lit"]).flat(),
    );
    // Tempo Assign STUDIO SET: the System Tempo is.
    expect(dimming(group(studio.container, "TEMPO"))).toEqual(["studioSet:lit", "system:dim"]);
    cleanup();

    const system = await drawer(systemDump({ controlSource: 1, tempoAssign: 0 }));
    expect(dimming(group(system.container, "CONTROL"))).toEqual(
      Array.from({ length: 4 }, () => ["studioSet:lit", "system:dim"]).flat(),
    );
    expect(dimming(group(system.container, "TEMPO"))).toEqual(["studioSet:dim", "system:lit"]);
  });

  it("dims nothing, and offers a retry, when System Common doesn't answer", async () => {
    const { container, requestData } = await drawer(null);
    expect(dimming(container).every((s) => s.endsWith(":lit"))).toBe(true);
    expect(container.querySelector('[aria-label="Control Source"]')!.textContent).toBe("—");
    click(button(container, "Retry"));
    await act(async () => {});
    expect(requestData).toHaveBeenCalledTimes(2);
  });

  it("writes a Tone Control source picked", async () => {
    const { container, setStudioSetField } = await drawer();
    click(button(group(container, "CONTROL"), "Tone Control 2 Source: OFF"));
    click(option(document, "BEND"));
    expect(setStudioSetField).toHaveBeenCalledWith(StudioSetSetting.ToneControlSource, 1, 96);
  });

  it("writes the tempo turned", async () => {
    const { container, setStudioSetField } = await drawer();
    const knob = group(container, "TEMPO").querySelector('[role="slider"]')!;
    act(() => {
      knob.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });
    expect(setStudioSetField).toHaveBeenCalledWith(StudioSetSetting.Tempo, 0, 97);
  });

  it("writes a channel's Phase Lock switched", async () => {
    const { container, setStudioSetField } = await drawer();
    const switches = [...group(container, "PHASE LOCK").querySelectorAll("button")];
    click(switches[2]!);
    expect(setStudioSetField).toHaveBeenCalledWith(StudioSetSetting.PhaseLock, 2, 1);
    click(switches[9]!);
    expect(setStudioSetField).toHaveBeenCalledWith(StudioSetSetting.PhaseLock, 9, 0);
  });

  it("closes on ✕ and on Escape, and links to the System page", async () => {
    const { container, onClose, onOpenSystem } = await drawer();
    click(button(container, "Close Studio Set settings"));
    expect(onClose).toHaveBeenCalledTimes(1);
    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    });
    expect(onClose).toHaveBeenCalledTimes(2);
    // Escape in an open picker closes the picker only.
    click(button(group(container, "CONTROL"), "Tone Control 2 Source: OFF"));
    act(() => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(document.querySelector('[role="listbox"]')).toBeNull();
    expect(onClose).toHaveBeenCalledTimes(2);
    click(button(group(container, "CONTROL"), "System page"));
    expect(onOpenSystem).toHaveBeenCalledTimes(1);
  });
});

describe("the core's Studio Set settings", () => {
  it("read into the shape the drawer shows, and take an edit", () => {
    const device = new WasmDeviceState(0x10);
    device.setStudioSetField(StudioSetSetting.Tempo, 0, 96);
    device.setStudioSetField(StudioSetSetting.PhaseLock, 9, 1);
    const read = (device.readState() as { studioSetSettings: StudioSetSettings }).studioSetSettings;
    expect(read.tempo).toBe(96);
    expect(read.toneControlSources).toHaveLength(4);
    expect(read.phaseLock).toHaveLength(16);
    expect(read.phaseLock[9]).toBe(true);
    device.free();
  });
});
