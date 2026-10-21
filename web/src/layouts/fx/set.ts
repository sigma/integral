/** What the MFX, Chorus and Reverb variant sets share. */

import { fxFields, fxTypes, type FxBlock } from "../fields";
import type { SectionBody, SectionFrame, VariantSet } from "../types";

/**
 * A variant set keyed by the types of one FX block. Core names the keys and
 * each type's fields; both are read on first use, since the wasm core loads
 * after this module is evaluated.
 */
export function fxVariantSet(
  block: FxBlock,
  frame: SectionFrame,
  variants: ReadonlyMap<number, SectionBody<string>>,
  generate: (type: number) => SectionBody<string>,
): VariantSet<number, string> {
  let keys: readonly number[] | undefined;
  return {
    ...frame,
    name: block,
    get keys() {
      return (keys ??= fxTypes(block));
    },
    fieldsOf: (type) => fxFields(block, type),
    variants,
    generate,
  };
}
