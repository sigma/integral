/** A fake service for tone editor tests, around a real WASM core. */

import { act } from "react";
import type { WasmDeviceState } from "../../pkg/integral_wasm.js";
import type { IntegraService } from "../integra";

/** A DT1 the core sent: its address and data bytes. */
export interface SentDt1 {
  address: number[];
  data: number[];
}

/** A service that answers every data request with a zeroed block of the
 *  requested size, and lets the test read back the DT1s the core sends. */
export function fakeToneService(device: WasmDeviceState) {
  const service = {
    device,
    onDt1: () => () => {},
    requestData: async (_address: number[], size: number[]) => {
      // RQ1 size: 4 bytes of 7 bits each.
      const bytes = size.reduce((n, b) => (n << 7) | b, 0);
      return new Uint8Array(bytes);
    },
  } as unknown as IntegraService;

  const sentDt1s = dt1Drainer(device);
  return { service, sentDt1s };
}

/** A function that drains `device`'s queue, from time `start` (ms) on, and
 *  returns every DT1 sent since its last call. */
export function dt1Drainer(device: WasmDeviceState, start = 0): () => SentDt1[] {
  let now = start;
  return () => {
    const sent: SentDt1[] = [];
    for (;;) {
      now += 1000;
      const msg = device.drain(now);
      if (!msg) break;
      // DT1: F0 41 dev 00 00 64 12 <address: 4> <data...> sum F7
      if (msg[6] !== 0x12) continue;
      sent.push({
        address: [...msg.slice(7, 11)],
        data: [...msg.slice(11, msg.length - 2)],
      });
    }
    return sent;
  };
}

/** The slider control labelled `label` (the first, if several). */
export function slider(scope: ParentNode, label: string): Element {
  const found = scope.querySelector(`[role="slider"][aria-label="${label}"]`);
  if (!found) throw new Error(`no "${label}" slider`);
  return found;
}

/** Step `control` (a slider) up once, as the Up arrow key does. */
export function arrowUp(control: Element): void {
  act(() => {
    control.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
  });
}
