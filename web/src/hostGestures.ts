/**
 * The plugin editor's host gestures: a Control drag the DAW can record.
 *
 * A DAW records automation only for a parameter whose edit is bracketed by a
 * begin and an end. A Control drag is that bracket: it begins on the first
 * `input` (the first write the drag makes) and ends on `change` (the
 * pointer's release). A write with no pointer held (a click on a switch, a
 * key press) is its own bracket. The editor sends begin, perform and end to
 * the plugin over IPC ({@link PageMessage}), keyed by the host parameter the
 * write sets (ADR 0008); the plugin forwards them to the host.
 */

import { useEffect } from "react";
import type { HostWrite, PluginKind } from "../pkg/integral_wasm.js";
import type { IntegraService } from "./integra";
import { encodePageMessage, type PageMessage, type PluginBinding, type PluginBridge } from "./plugin";

/**
 * Turns the writes of Control drags into host gestures: `begin` before a
 * parameter's first write, `perform` for each write, `end` on `change`.
 */
export class HostGestures {
  /** The parameters with a gesture begun, in the order they began. */
  private readonly open = new Set<string>();

  constructor(private readonly post: (message: PageMessage) => void) {}

  /**
   * Writes the page made. `held` is whether a pointer is down, in which case
   * the gesture stays open for {@link change}.
   */
  input(writes: readonly HostWrite[], held: boolean): void {
    for (const { id, value } of writes) {
      if (!this.open.has(id)) {
        this.open.add(id);
        this.post({ type: "beginEdit", id });
      }
      this.post({ type: "performEdit", id, value });
    }
    if (!held) this.change();
  }

  /** The drag ended: end every gesture begun, in the order begun. */
  change(): void {
    for (const id of this.open) this.post({ type: "endEdit", id });
    this.open.clear();
  }
}

/** How often the writes of a drag are collected, ms: about a frame. */
export const GESTURE_POLL_MS = 8;

/**
 * Relay the page's writes to the plugin as host gestures while the page runs
 * in a plugin (`bridge`) of `kind` bound to `binding`. A drag's gesture ends
 * when the pointer is released (or the window loses focus, or the Binding
 * changes), however it was released.
 */
export function useHostGestures(
  service: IntegraService | null,
  bridge: PluginBridge | null,
  kind: PluginKind | undefined,
  binding: PluginBinding | null,
  relay = true,
): void {
  const part = binding?.part;
  const key = binding?.key;
  const bound = binding !== null;
  useEffect(() => {
    if (!service || !bridge || kind === undefined || !relay) return;
    const device = service.device;
    const gestures = new HostGestures((message) => bridge.post(encodePageMessage(message)));
    let held = false;
    device.recordWrites(true);

    // Writes of an unbound instance set nothing; taking them keeps the log short.
    const flush = () => {
      if (!device.hasWrites()) return;
      const writes = device.takeHostWrites(kind, part, key);
      if (bound) gestures.input(writes, held);
    };
    const press = () => {
      held = true;
    };
    const release = () => {
      flush();
      held = false;
      gestures.change();
    };

    const timer = setInterval(flush, GESTURE_POLL_MS);
    window.addEventListener("pointerdown", press, true);
    window.addEventListener("pointerup", release, true);
    window.addEventListener("pointercancel", release, true);
    // Does not bubble: a capture listener hears it.
    window.addEventListener("lostpointercapture", release, true);
    window.addEventListener("blur", release);
    return () => {
      clearInterval(timer);
      window.removeEventListener("pointerdown", press, true);
      window.removeEventListener("pointerup", release, true);
      window.removeEventListener("pointercancel", release, true);
      window.removeEventListener("lostpointercapture", release, true);
      window.removeEventListener("blur", release);
      release();
      device.recordWrites(false);
    };
  }, [service, bridge, kind, bound, part, key, relay]);
}
