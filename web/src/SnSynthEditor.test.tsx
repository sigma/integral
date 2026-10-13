import { act } from "react";
import { afterEach, describe, expect, it } from "vitest";
import {
  ParameterFamily,
  SnsPartialField,
  WasmDeviceState,
  sns_partial_spec,
} from "../pkg/integral_wasm.js";
import { SnSynthEditor } from "./SnSynthEditor";
import type { IntegraService } from "./integra";
import { familyVar } from "./synth-ui";
import { cleanup, render } from "./test/dom";
import { dt1Drainer } from "./test/toneService";

afterEach(cleanup);

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
    const level = container.querySelector('[role="slider"][aria-label="Tone Level"]');
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
