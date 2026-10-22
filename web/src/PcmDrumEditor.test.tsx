import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  DisplayKind,
  ParameterFamily,
  PcmdPartialField,
  PcmdWmtField,
  WasmDeviceState,
  pcmd_wmt_spec,
} from "../pkg/integral_wasm.js";
import { PcmDrumEditor } from "./PcmDrumEditor";
import { familyVar } from "./synth-ui";
import { cleanup, click, render } from "./test/dom";
import { controlsOffGrid, drumKey, kitPlacement } from "./test/drum";
import { arrowUp, dt1Drainer, fakeToneService, slider } from "./test/toneService";

afterEach(cleanup);

const PART = 1;

/** The DT1s a fresh core sends for `write`: what the editor's edit must match. */
function expected(write: (device: WasmDeviceState) => void) {
  const device = new WasmDeviceState(0x10);
  const sent = dt1Drainer(device);
  write(device);
  return sent();
}

/** The knob or fader whose tooltip is `title`. */
function titled(scope: ParentNode, title: string): Element {
  const found = scope.querySelector(`[title="${title}"] [role="slider"]`);
  if (!found) throw new Error(`no "${title}" slider`);
  return found;
}

/** The WMT layer tab reading `name`. */
function tab(scope: ParentNode, name: string): Element {
  const found = [...scope.querySelectorAll('[role="tab"]')].find((t) => t.textContent === name);
  if (!found) throw new Error(`no ${name} tab`);
  return found;
}

describe("PcmDrumEditor", () => {
  it("colours a knob by its field spec's family, not its section's", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // WMT Pan sits in the WMT section (Pitch), but its spec says Time.
    const family = pcmd_wmt_spec(PcmdWmtField.Pan).family;
    expect(family).toBe(ParameterFamily.Time);
    const fills = [...titled(container, "Wave Pan").querySelectorAll("circle")].map((c) => c.style.fill);
    expect(fills).toContain(familyVar(family));
    expect(fills).not.toContain(familyVar(ParameterFamily.Pitch));
  });

  it("places the KIT Section beside the key matrix, out of the key Sections", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    expect(kitPlacement(container, 36)).toEqual({ besideKeys: true, inKeyScreen: false });
  });

  it("lays every key Control out on the Grid", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    expect(container.querySelector('[data-grid-screen="PCM Drum"]')).not.toBeNull();
    expect(controlsOffGrid(container)).toEqual([]);
  });

  it("draws every Display from the key's fields", async () => {
    const { service } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    const drawn = (kind: DisplayKind) => container.querySelector(`[data-display="${kind}"]`);
    for (const kind of [DisplayKind.PitchEnvelope, DisplayKind.FilterEnvelope, DisplayKind.AmpEnvelope]) {
      expect(drawn(kind)?.querySelector("path")?.getAttribute("d")).toMatch(/^M /);
    }
    // The zeroed key's filter type is OFF: a flat response.
    expect(drawn(DisplayKind.FilterScope)?.querySelector("path")).not.toBeNull();
    expect(drawn(DisplayKind.WavePicker)?.textContent).toContain("INT");
  });

  it("writes the kit-wide Kit Level", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    arrowUp(slider(container, "Kit Lv"));
    expect(sentDt1s()).toEqual(expected((d) => d.setPcmdKitLevel(PART, 1)));
  });

  it("writes an envelope fader to its value of the envelope field", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});

    // The first T2 is the pitch envelope's: value 1 of Pitch Env Time.
    arrowUp(slider(container, "T2"));
    expect(sentDt1s()).toEqual(
      expected((d) => d.setPcmdPartialField(PART, 36, PcmdPartialField.PitchEnvTime, 1, 1)),
    );
  });

  it("rebinds the WMT Section to another layer without remounting it", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});
    const pan = titled(container, "Wave Pan");
    expect(pan.closest("[data-grid-section]")?.getAttribute("data-grid-section")).toBe("WMT 1");

    click(tab(container, "WMT2"));

    expect(titled(container, "Wave Pan")).toBe(pan);
    expect(pan.closest("[data-grid-section]")?.getAttribute("data-grid-section")).toBe("WMT 2");
    arrowUp(pan);
    expect(sentDt1s()).toEqual(expected((d) => d.setPcmdWmtParam(PART, 36, 1, PcmdWmtField.Pan, 1)));
  });

  it("rebinds the Partial Sections to another key without remounting them", async () => {
    const { service, sentDt1s } = fakeToneService(new WasmDeviceState(0x10));
    const { container } = render(<PcmDrumEditor partIndex={PART} service={service} />);
    await act(async () => {});
    const level = titled(container, "Partial Level");
    const pan = titled(container, "Wave Pan");

    click(drumKey(container, 38));
    await act(async () => {});

    expect(titled(container, "Partial Level")).toBe(level);
    expect(titled(container, "Wave Pan")).toBe(pan);
    // Zeroed, the Partial Level reads 0: one step up writes 1 to key 38's.
    arrowUp(level);
    expect(sentDt1s()).toEqual(
      expected((d) => d.setPcmdPartialField(PART, 38, PcmdPartialField.Level, 0, 1)),
    );
  });
});
