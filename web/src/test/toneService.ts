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
 *  requested size, ignores note previews, and lets the test read back the
 *  DT1s the core sends. */
export function fakeToneService(device: WasmDeviceState) {
  const service = {
    device,
    onDt1: () => () => {},
    sendNoteOn: () => {},
    sendNoteOff: () => {},
    requestData: async (_address: number[], size: number[]) => {
      // RQ1 size: 4 bytes of 7 bits each.
      const bytes = size.reduce((n, b) => (n << 7) | b, 0);
      return new Uint8Array(bytes);
    },
  } as unknown as IntegraService;

  const sentDt1s = dt1Drainer(device);
  return { service, sentDt1s };
}

/** Which reads fail how often: the address of an RQ1, and how many of its
 * first reads reject instead of answering. */
export interface FlakyRead {
  address: number[];
  times: number;
}

/**
 * A fake service like {@link fakeToneService}'s, but whose answer to a data
 * request is withheld, rejecting like an RQ1 timeout until it clears: with a
 * count, the first `rejections` reads of every address reject (`Infinity`:
 * every read); with a list, the first `times` reads of each listed address
 * reject and unlisted addresses always answer. A test can exercise a read
 * that fails and then clears, or one failing block among loading ones.
 */
export function flakyToneService(
  device: WasmDeviceState,
  rejections: number | readonly FlakyRead[],
) {
  const { service, sentDt1s } = fakeToneService(device);
  const every = typeof rejections === "number" ? rejections : null;
  const left = new Map<string, number>(
    (typeof rejections === "number" ? [] : rejections).map((r) => [
      r.address.join(","),
      r.times,
    ]),
  );
  const answered = service.requestData.bind(service);
  service.requestData = async (address: number[], size: number[]) => {
    const key = address.join(",");
    if (every !== null && !left.has(key)) left.set(key, every);
    const remaining = left.get(key) ?? 0;
    if (remaining > 0) {
      left.set(key, remaining === Infinity ? Infinity : remaining - 1);
      throw new Error(`RQ1 timeout for ${address.join(" ")}`);
    }
    return answered(address, size);
  };
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
