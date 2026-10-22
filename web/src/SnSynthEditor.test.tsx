import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  DisplayKind,
  ParameterFamily,
  SnsCommonField,
  SnsPartialField,
  WasmDeviceState,
  sns_common_spec,
  sns_partial_spec,
  sns_partial_sync,
} from "../pkg/integral_wasm.js";
import { SnSynthEditor } from "./SnSynthEditor";
import type { IntegraService } from "./integra";
import { familyVar } from "./synth-ui";
import { SNS_BOXES } from "./layouts/sn-synth/partial";
import { SNS_ROW, SNS_SECTION, type SnsRowSection } from "./layouts/sn-synth/screen";
import { button, cleanup, click, render } from "./test/dom";
import { dt1Drainer } from "./test/toneService";

// jsdom has no ResizeObserver, which the page's GridZoom watches its width with.
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      disconnect() {}
    },
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

const PART = 2;

/** A service around a real core, whose RQ1s the test answers. */
function fakeService() {
  const device = new WasmDeviceState(0x10);
  const listeners = new Set<() => void>();
  const service = {
    device,
    onDt1: (cb: () => void) => {
      listeners.add(cb);
      return () => listeners.delete(cb);
    },
    // MFX: never answered.
    requestData: () => new Promise<Uint8Array>(() => {}),
  } as unknown as IntegraService;

  /** Send the queued messages; return the (address, size) of each RQ1. */
  function sentRequests(): { address: Uint8Array; size: number }[] {
    const sent = [];
    for (let now = 1000; ; now += 100) {
      const msg = device.drain(now);
      if (!msg) break;
      // RQ1: F0 41 dev 00 00 64 11 <address: 4> <size: 4> sum F7
      if (msg[6] !== 0x11) continue;
      sent.push({ address: msg.slice(7, 11), size: (msg[13]! << 7) + msg[14]! });
    }
    return sent;
  }

  /** Deliver a DT1 reply, as the service does: to the core, then to listeners. */
  function reply(address: Uint8Array, data: Uint8Array) {
    act(() => {
      device.handleDt1(address, data, 2000);
      for (const cb of listeners) cb();
    });
  }

  return { device, service, sentRequests, reply };
}

/** Render the editor and answer its reads; Common is named "Lead" with Tone Level 100. */
function loadedEditor() {
  const { device, service, sentRequests, reply } = fakeService();
  const { container } = render(<SnSynthEditor partIndex={PART} service={service} />);
  expect(container.textContent).toContain("Loading SN-S tone data");

  // Common and 3 Partials, each read once.
  const requests = sentRequests();
  expect(requests).toHaveLength(4);
  for (const [i, { address, size }] of requests.entries()) {
    const data = new Uint8Array(size);
    if (i === 0) {
      data.set(new TextEncoder().encode("Lead"));
      data[0x0c] = 100; // Tone Level
    }
    expect(container.textContent).toContain("Loading SN-S tone data");
    reply(address, data);
  }
  expect(container.textContent).toContain("Lead");
  return { device, container, requests };
}

describe("SnSynthEditor", () => {
  it("shows the tone the core read, and edits to it", () => {
    const { device, container, requests } = loadedEditor();
    const common = container.querySelector(`[data-section="${SNS_SECTION.common}"]`);
    const level = common?.querySelector('[role="slider"][aria-label="Level"]');
    if (!level) throw new Error("no Tone Level knob");
    expect(level.getAttribute("aria-valuenow")).toBe("100");

    const sentDt1s = dt1Drainer(device, 10_000);
    act(() => {
      level.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    });
    expect(level.getAttribute("aria-valuenow")).toBe("101");
    // Tone Level is Common (the first block read) `00 0C`.
    expect(sentDt1s()).toEqual([
      { address: [...requests[0]!.address.slice(0, 3), 12], data: [101] },
    ]);
  });

  it("lights each knob in its field's family, not its section's", () => {
    const { container } = loadedEditor();
    // Pan sits in AMP, but its spec makes it Time.
    const panFamily = sns_partial_spec(SnsPartialField.AmpPan).family;
    expect(panFamily).toBe(ParameterFamily.Time);
    const pan = container.querySelector('[role="slider"][aria-label="Pan"]');
    if (!pan) throw new Error("no Pan knob");
    const fills = [...pan.querySelectorAll("circle")].map((c) => c.style.fill);
    expect(fills).toContain(familyVar(panFamily));
    expect(fills).not.toContain(familyVar(ParameterFamily.Amp));
  });
});

/** Section `section` of partial `partial`'s row (0-based). */
function rowSection(container: HTMLElement, partial: number, section: SnsRowSection): HTMLElement {
  const el = container.querySelector<HTMLElement>(`[data-section="${SNS_SECTION.partial(partial, section)}"]`);
  if (!el) throw new Error(`no ${section} for partial ${partial}`);
  return el;
}

/** Partial `partial`'s row (0-based): what its Sections hold, as one scope to query. */
function row(container: HTMLElement, partial: number): ParentNode {
  const sections = SNS_ROW.map((s) => rowSection(container, partial, s));
  const fragment = { querySelector: (sel: string) => sections.map((s) => s.querySelector(sel)).find((e) => e) ?? null };
  return fragment as ParentNode;
}

/** The element wrapping the Control bound to `field` in `scope`. */
function control(scope: ParentNode, field: SnsPartialField | SnsCommonField): HTMLElement {
  const el = scope.querySelector<HTMLElement>(`[data-field="${field}"]`);
  if (!el) throw new Error(`no Control for field ${field}`);
  return el;
}

/** Every Control and Display on the page, by Section and binding, with its box. */
function boxes(container: HTMLElement): Map<string, string> {
  const out = new Map<string, string>();
  for (const el of container.querySelectorAll<HTMLElement>("[data-field], [data-display]")) {
    const section = el.closest("[data-section]")?.getAttribute("data-section");
    const what = el.dataset.field ?? `display ${el.dataset.display}`;
    const { left, top, width, height } = el.style;
    out.set(`${section} ${what}`, `${left},${top},${width},${height}`);
  }
  return out;
}

/** Picks OSC Wave `name` on partial `partial`. */
function pickWave(container: HTMLElement, partial: number, name: string) {
  click(button(control(row(container, partial), SnsPartialField.OscWave), name));
}

describe("SnSynthEditor on the Grid", () => {
  it("places COMMON, every partial row of Sections and the MFX on one zoomed screen", () => {
    const { container } = loadedEditor();
    const zoom = container.querySelector("[data-grid-zoom]");
    expect(zoom?.querySelector('[data-grid-screen="SN Synth"]')).toBeTruthy();
    const title = (el: HTMLElement) => el.querySelector("[data-grid-section]")?.getAttribute("data-grid-section");
    expect(title(container.querySelector<HTMLElement>(`[data-section="${SNS_SECTION.common}"]`)!)).toBe("COMMON");
    for (const p of [0, 1, 2]) {
      // Each row: its switch strip, then one Section per box.
      expect(title(rowSection(container, p, "switch"))).toBe(`Partial ${p + 1}`);
      expect(SNS_BOXES.map((b) => title(rowSection(container, p, b.id)))).toEqual([
        "OSC",
        "FILTER",
        "AMP",
        "MOD LFO",
        "LFO",
      ]);
    }
    expect(container.querySelector(`[data-section="${SNS_SECTION.mfx}"]`)).toBeTruthy();
  });

  it("sums every row into the MFX: Screen Silkscreen arrows meeting at one +", () => {
    const { container } = loadedEditor();
    const silk = container.querySelector("[data-screen-silkscreen]");
    if (!silk) throw new Error("no Screen Silkscreen");
    expect([...silk.querySelectorAll("[data-glyph]")].map((g) => g.textContent)).toEqual(["+"]);
    // One body per row, and the shared glyph's lead into the MFX.
    expect(silk.querySelectorAll("path[data-arrow]")).toHaveLength(4);
  });

  it("draws the envelopes and scopes as Displays", () => {
    const { container } = loadedEditor();
    const r = row(container, 0);
    for (const kind of [
      DisplayKind.PitchEnvelope,
      DisplayKind.FilterEnvelope,
      DisplayKind.AmpEnvelope,
      DisplayKind.OscScope,
      DisplayKind.ModLfoScope,
      DisplayKind.LfoScope,
    ]) {
      const display = r.querySelector(`[data-display="${kind}"]`);
      expect(display?.querySelector("path"), DisplayKind[kind]).toBeTruthy();
    }
  });

  it("swaps the OSC scope for the PCM Wave picker without moving a Control", () => {
    const { container } = loadedEditor();
    const original = boxes(container);
    const before = new Map(original);
    const r = row(container, 0);
    expect(r.querySelector(`[data-display="${DisplayKind.OscScope}"]`)).toBeTruthy();

    pickWave(container, 0, "PCM");
    expect(r.querySelector(`[data-display="${DisplayKind.OscScope}"]`)).toBeNull();
    const picker = control(r, SnsPartialField.WaveNumber);
    // The picker names the zeroed partial's Wave as core formats it.
    expect(sns_partial_spec(SnsPartialField.WaveNumber).formatValue(0)).toBe("OFF");
    expect(picker.querySelector("button")?.textContent).toBe("OFF");

    // Only the alternative changed: every other element kept its box, and the picker took the scope's top-left Cells.
    const after = boxes(container);
    const scope = `${SNS_SECTION.partial(0, "osc")} display ${DisplayKind.OscScope}`;
    const pcm = `${SNS_SECTION.partial(0, "osc")} ${SnsPartialField.WaveNumber}`;
    const [scopeLeft, scopeTop] = before.get(scope)!.split(",");
    expect(after.get(pcm)).toBe(`${scopeLeft},${scopeTop},144px,24px`);
    before.delete(scope);
    after.delete(pcm);
    expect(after).toEqual(before);

    // And back to a scope: everything where it started.
    pickWave(container, 0, "SP-SAW");
    expect(boxes(container)).toEqual(original);
  });

  it("hides Var and S-Saw in place, as core holds them inactive", () => {
    const { container } = loadedEditor();
    const r = row(container, 0);
    const state = () =>
      [SnsPartialField.OscWaveVariation, SnsPartialField.SuperSawDetune].map(
        (f) => control(r, f).dataset.inactive ?? "active",
      );
    const at = () =>
      [SnsPartialField.OscWaveVariation, SnsPartialField.SuperSawDetune].map((f) => {
        const { left, top } = control(r, f).style;
        return `${left},${top}`;
      });
    const where = at();
    // SAW: Var applies, S-Saw doesn't.
    expect(state()).toEqual(["active", "hide"]);
    pickWave(container, 0, "SP-SAW");
    expect(state()).toEqual(["hide", "active"]);
    pickWave(container, 0, "PCM");
    expect(state()).toEqual(["hide", "hide"]);
    expect(at()).toEqual(where);
  });

  it("dims each box of a partial's row, inert, while it is off; the switch heading the row stays live", () => {
    const { container } = loadedEditor();
    // The zeroed Common holds every partial off.
    const off = (p: number) =>
      SNS_BOXES.map((b) => rowSection(container, p, b.id).querySelector("[data-scope-inactive]")?.hasAttribute("inert"));
    expect(off(0)).toEqual(SNS_BOXES.map(() => true));
    // Field relevance is apart: Cutoff always applies.
    expect(control(row(container, 0), SnsPartialField.FilterCutoff).dataset.inactive).toBeUndefined();
    // The switches moved out of COMMON, into their rows.
    const common = container.querySelector<HTMLElement>(`[data-section="${SNS_SECTION.common}"]`)!;
    expect(common.querySelector(`[data-field="${SnsCommonField.Partial1Switch}"]`)).toBeNull();
    const sw = rowSection(container, 0, "switch").querySelector("button")!;
    expect(sw.closest("[inert]")).toBeNull();
    click(sw);
    expect(off(0)).toEqual(SNS_BOXES.map(() => undefined));
    expect(off(1)).toEqual(SNS_BOXES.map(() => true));
    // Partial 2's strip binds Partial 2's switch.
    click(rowSection(container, 1, "switch").querySelector("button")!);
    expect(off(1)).toEqual(SNS_BOXES.map(() => undefined));
    expect(off(2)).toEqual(SNS_BOXES.map(() => true));
  });

  it("edits the note with the rate knob while its LFO follows the tempo", () => {
    const { container } = loadedEditor();
    const sync = sns_partial_sync(SnsPartialField.LfoRate)!;
    const knob = () => control(row(container, 0), SnsPartialField.LfoRate).querySelector('[role="slider"]')!;
    expect(knob().getAttribute("aria-valuemax")).toBe(String(sns_partial_spec(SnsPartialField.LfoRate).max));
    // The tempo-sync toggle sits inside the rate knob; no separate Sync switch is placed.
    expect(row(container, 0).querySelector(`[data-field="${sync.switchField}"]`)).toBeNull();
    const unit = control(row(container, 0), SnsPartialField.LfoRate).querySelector("button")!;
    expect(unit.textContent).toBe(sync.labels[0]);
    click(unit);
    expect(knob().getAttribute("aria-valuemax")).toBe(String(sns_partial_spec(sync.noteField).max));
  });

  it("labels Common's Controls from the layout and ranges them from core", () => {
    const { container } = loadedEditor();
    const common = container.querySelector<HTMLElement>(`[data-section="${SNS_SECTION.common}"]`)!;
    const level = control(common, SnsCommonField.ToneLevel).querySelector('[role="slider"]')!;
    expect(level.getAttribute("aria-valuemax")).toBe(String(sns_common_spec(SnsCommonField.ToneLevel).max));
  });
});
