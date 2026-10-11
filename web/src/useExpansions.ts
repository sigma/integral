/**
 * Loaded state of the Expansions, read from the device into the Rust
 * `WasmDeviceState` and exposed to React.
 *
 * The slots are read when a service connects, on window focus and when a
 * tone picker opens (via `refresh`); never on a timer. A read with no answer
 * makes the state unknown, and unknown state marks nothing.
 */

import { useCallback, useEffect, useState } from "react";
import type { IntegraService } from "./integra";
import { findToneBank, type BankExpansion } from "./toneBanks";

export interface UseExpansionsResult {
  /** Re-read the Expansion slots from the device. */
  refresh: () => void;
  /** Whether `expansion` is known not to be loaded (false while unknown). */
  isUnloaded: (expansion: BankExpansion | null) => boolean;
  /**
   * Name of the Expansion providing the bank of (msb, lsb) if it is known
   * not to be loaded; null for built-in banks, loaded Expansions or while
   * the slots are unknown.
   */
  unloadedExpansionOf: (msb: number, lsb: number) => string | null;
}

export function useExpansions(
  service: IntegraService | null,
): UseExpansionsResult {
  // Slot values A–D joined (null while unknown), tagged with the service
  // that read them: a read for a replaced service says nothing of the new.
  const [read, setRead] = useState<{
    service: IntegraService;
    slots: string | null;
  } | null>(null);
  const slotsKey = read?.service === service ? read.slots : null;

  const refresh = useCallback(() => {
    if (!service) return;
    void service.readExpansionSlots().then((slots) => {
      const key = slots ? slots.join() : null;
      // Same slots: keep the state object so nothing re-renders.
      setRead((prev) =>
        prev?.service === service && prev.slots === key
          ? prev
          : { service, slots: key },
      );
    });
  }, [service]);

  useEffect(() => {
    if (!service) return;
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [service, refresh]);

  const isUnloaded = useCallback(
    (expansion: BankExpansion | null) =>
      slotsKey !== null &&
      expansion !== null &&
      service?.device.isExpansionLoaded(expansion.loadValue) === false,
    [service, slotsKey],
  );

  const unloadedExpansionOf = useCallback(
    (msb: number, lsb: number) => {
      const expansion = findToneBank(msb, lsb)?.expansion ?? null;
      return expansion && isUnloaded(expansion) ? expansion.name : null;
    },
    [isUnloaded],
  );

  return { refresh, isUnloaded, unloadedExpansionOf };
}
