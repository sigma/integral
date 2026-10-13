/**
 * Parameter family colours. The palette lives in `integral-core`; at startup
 * `installFamilyPalette` copies it into one CSS custom property per family,
 * and components refer to those properties through `familyVar`.
 */

import { ParameterFamily, familyColour } from "../../pkg/integral_wasm.js";

/** Every Parameter family, read from the wasm enum. */
export const FAMILIES: readonly ParameterFamily[] = Object.values(
  ParameterFamily,
).filter((v): v is ParameterFamily => typeof v === "number");

/** The CSS custom property holding `family`'s colour, e.g. `--family-filter`. */
export function familyProperty(family: ParameterFamily): string {
  return `--family-${ParameterFamily[family].toLowerCase()}`;
}

/** A CSS colour value for `family`, e.g. `var(--family-filter)`. */
export function familyVar(family: ParameterFamily): string {
  return `var(${familyProperty(family)})`;
}

/** `family`'s colour when set, else `fallback`. */
export function accentColour(
  family: ParameterFamily | undefined,
  fallback: string,
): string {
  return family === undefined ? fallback : familyVar(family);
}

/** Set each family's custom property on `root` from the core palette. Needs wasm initialised. */
export function installFamilyPalette(root: HTMLElement): void {
  for (const family of FAMILIES) {
    root.style.setProperty(familyProperty(family), familyColour(family));
  }
}
