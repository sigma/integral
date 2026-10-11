/**
 * Loaded state of the Expansions, read from the device into the Rust
 * `WasmDeviceState` and exposed to React, and the loads that change it.
 *
 * The slots are read when a service connects, on window focus, when the
 * pointer comes back into the page (front-panel loads send nothing and
 * don't blur the window), and when a tone picker or the Expansions page
 * opens (via `refresh`); never on a timer. A read with no answer makes the state unknown, and unknown state
 * marks nothing.
 */

import { useCallback, useEffect, useState } from "react";
import { LoadOutcomeKind } from "../pkg/integral_wasm.js";
import type { ExpansionLoadResult, IntegraService } from "./integra";
import { findToneBank } from "./toneBanks";
import { expansionByLoadValue, type Expansion } from "./expansionCatalog";

/** What loading some target slots would change. */
export interface ExpansionLoadPlan {
  /** Slots (0 = A) whose content changes; empty for a no-op. */
  changedSlots: number[];
  /** Parts (1-based) on a loaded Expansion that the load unloads. */
  silencedParts: number[];
}

/** A load in progress. */
export interface RunningExpansionLoad {
  /** Names of the Expansions being loaded (empty when only unloading). */
  names: string[];
  /** When it started (`performance.now()` clock). */
  startedAt: number;
}

export interface UseExpansionsResult {
  /** Re-read the Expansion slots from the device. */
  refresh: () => void;
  /** Slot values A–D loaded now, or null while unknown. */
  slots: readonly number[] | null;
  /** Whether `expansion` is known not to be loaded (false while unknown). */
  isUnloaded: (expansion: Expansion | null) => boolean;
  /**
   * Name of the Expansion providing the bank of (msb, lsb) if it is known
   * not to be loaded; null for built-in banks, loaded Expansions or while
   * the slots are unknown.
   */
  unloadedExpansionOf: (msb: number, lsb: number) => string | null;
  /** Parts (1-based) whose current tone comes from `expansion`. */
  partsUsing: (expansion: Expansion) => number[];
  /**
   * The slot (0 = A) to stage another Expansion into over `slots`: an OFF
   * slot, else one no current Part uses, else A.
   */
  preselectSlot: (slots: readonly number[]) => number;
  /** What loading `target` would change, or null while slots are unknown. */
  plan: (target: readonly number[]) => ExpansionLoadPlan | null;
  /** The load in progress, or null. */
  running: RunningExpansionLoad | null;
  /**
   * Load `target` (slot values A–D). The send queue holds until it ends.
   * Rejects if it can't start (slots unknown, a save or load running).
   */
  load: (target: readonly number[]) => Promise<ExpansionLoadResult>;
}

/** Minimum time between two pointer re-entry reads (ms). */
const REENTRY_READ_INTERVAL_MS = 2000;

export function useExpansions(
  service: IntegraService | null,
): UseExpansionsResult {
  // Slot values A–D (null while unknown), tagged with the service that read
  // them: a read for a replaced service says nothing of the new one.
  const [read, setRead] = useState<{
    service: IntegraService;
    slots: readonly number[] | null;
  } | null>(null);
  const slots = read?.service === service ? read.slots : null;
  const [running, setRunning] = useState<RunningExpansionLoad | null>(null);

  /** Record the slots now held by `service`'s device state. */
  const record = useCallback((svc: IntegraService) => {
    const values = svc.device.expansionSlots();
    const next = values ? Array.from(values) : null;
    // Same slots: keep the state object so nothing re-renders.
    setRead((prev) =>
      prev?.service === svc && prev.slots?.join() === next?.join()
        ? prev
        : { service: svc, slots: next },
    );
  }, []);

  const refresh = useCallback(() => {
    if (!service) return;
    void service.readExpansionSlots().then(() => record(service));
  }, [service, record]);

  useEffect(() => {
    if (!service) return;
    refresh();
    // The pointer moving back in from the device's panel is the cue; a
    // pointer wiggling across the window edge reads at most once per interval.
    let lastReentryRead = -Infinity;
    const onReentry = (e: PointerEvent) => {
      // A null relatedTarget means the pointer came from outside the window.
      if (e.relatedTarget !== null) return;
      const now = performance.now();
      if (now - lastReentryRead < REENTRY_READ_INTERVAL_MS) return;
      lastReentryRead = now;
      refresh();
    };
    window.addEventListener("focus", refresh);
    document.addEventListener("pointerover", onReentry);
    return () => {
      window.removeEventListener("focus", refresh);
      document.removeEventListener("pointerover", onReentry);
    };
  }, [service, refresh]);

  const isUnloaded = useCallback(
    (expansion: Expansion | null) =>
      slots !== null &&
      expansion !== null &&
      service?.device.isExpansionLoaded(expansion.loadValue) === false,
    [service, slots],
  );

  const unloadedExpansionOf = useCallback(
    (msb: number, lsb: number) => {
      const expansion = findToneBank(msb, lsb)?.expansion ?? null;
      return expansion && isUnloaded(expansion) ? expansion.name : null;
    },
    [isUnloaded],
  );

  // Reads the core's current Part tones on every call.
  const partsUsing = useCallback(
    (expansion: Expansion) =>
      service
        ? Array.from(service.device.partsUsingExpansion(expansion.loadValue))
        : [],
    [service],
  );

  const preselectSlot = useCallback(
    (base: readonly number[]) =>
      service?.device.preselectExpansionSlot(Uint8Array.from(base)) ?? 0,
    [service],
  );

  const plan = useCallback(
    (target: readonly number[]): ExpansionLoadPlan | null => {
      if (!service || slots === null) return null;
      let p;
      try {
        p = service.device.planExpansionLoad(Uint8Array.from(target));
      } catch {
        // The device state lost the slots before React did.
        return null;
      }
      const result = {
        changedSlots: Array.from(p.changedSlots),
        silencedParts: Array.from(p.silencedParts),
      };
      p.free();
      return result;
    },
    [service, slots],
  );

  const load = useCallback(
    async (target: readonly number[]): Promise<ExpansionLoadResult> => {
      if (!service) throw new Error("No INTEGRA-7 connected");
      const changed = plan(target)?.changedSlots ?? [];
      const names = changed.flatMap((slot) => {
        const expansion = expansionByLoadValue(target[slot]!);
        return expansion ? [expansion.name] : [];
      });
      const result = service.loadExpansions(target);
      setRunning({ names: [...new Set(names)], startedAt: performance.now() });
      try {
        const outcome = await result;
        // A timed-out load leaves the slots unknown: try reading them back.
        if (outcome.outcome === LoadOutcomeKind.TimedOut) refresh();
        return outcome;
      } finally {
        setRunning(null);
        record(service);
      }
    },
    [service, plan, refresh, record],
  );

  return {
    refresh,
    slots,
    isUnloaded,
    unloadedExpansionOf,
    partsUsing,
    preselectSlot,
    plan,
    running,
    load,
  };
}
