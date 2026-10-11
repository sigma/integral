/**
 * The INTEGRA-7's Expansions and Expansion slots, read from the core device
 * spec via wasm.
 *
 * Slots travel as slot values A–D: `expansionSlotOff()` for OFF, else the
 * Expansion's load value (ExPCM is its load value in all four).
 */

import {
  expansionCatalog as wasmExpansionCatalog,
  expansionSlotNames as wasmExpansionSlotNames,
} from "../pkg/integral_wasm.js";

export { expansionSlotOff } from "../pkg/integral_wasm.js";

/** One Expansion of the catalog. */
export interface Expansion {
  /** Display name (e.g. "SRX-05"). */
  name: string;
  /** Product title (e.g. "Supreme Dance"). */
  title: string;
  /** Short label of its family ("SRX", "ExSN", "ExPCM"). */
  kindLabel: string;
  /** Value selecting it in an Expansion slot read or load. */
  loadValue: number;
  /** Number of Expansion slots it takes when loaded. */
  slotCost: number;
}

let cachedCatalog: Expansion[] | null = null;
let cachedSlotNames: string[] | null = null;

/** Every Expansion, in catalog order. */
export function expansionCatalog(): Expansion[] {
  if (!cachedCatalog) {
    cachedCatalog = wasmExpansionCatalog().map((e) => {
      const result = {
        name: e.name,
        title: e.title,
        kindLabel: e.kindLabel,
        loadValue: e.loadValue,
        slotCost: e.slotCost,
      };
      e.free();
      return result;
    });
  }
  return cachedCatalog;
}

/** The Expansion selected by a slot value, or null for OFF or unknown. */
export function expansionByLoadValue(value: number): Expansion | null {
  return expansionCatalog().find((e) => e.loadValue === value) ?? null;
}

/** Names of the Expansion slots, in slot order ("A" to "D"). */
export function expansionSlotNames(): string[] {
  cachedSlotNames ??= wasmExpansionSlotNames();
  return cachedSlotNames;
}
