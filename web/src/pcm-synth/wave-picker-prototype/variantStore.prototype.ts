// PROTOTYPE ONLY (throwaway). ?variant= in the URL, shared by every WaveSection.
import { useSyncExternalStore } from "react";

export const VARIANTS = [
  { id: "0", label: "0 — Current UI" },
  { id: "A", label: "A — Combobox" },
  { id: "B", label: "B — Browser modal" },
  { id: "C", label: "C — Stepper + families" },
] as const;

export type VariantId = (typeof VARIANTS)[number]["id"];

const EVENT = "wave-picker-prototype-variant";

function read(): VariantId {
  const v = new URLSearchParams(location.search).get("variant");
  return VARIANTS.find((x) => x.id === v)?.id ?? "0";
}

export function setVariant(id: VariantId) {
  const params = new URLSearchParams(location.search);
  params.set("variant", id);
  history.replaceState(null, "", `${location.pathname}?${params}${location.hash}`);
  window.dispatchEvent(new Event(EVENT));
}

function subscribe(cb: () => void) {
  window.addEventListener(EVENT, cb);
  return () => window.removeEventListener(EVENT, cb);
}

export function useVariant(): VariantId {
  return useSyncExternalStore(subscribe, read);
}
