/**
 * Every layout, for the validator. `layouts.test.ts` walks these lists (and
 * every key of each variant set), so a layout not listed here is not checked:
 * add each new Section, variant set and screen when you write it.
 */

import type { FieldKey } from "./fields";
import { EQ_LAYOUT } from "./mixer/eq";
import { mixerScreen } from "./mixer/screen";
import { STRIP_SET } from "./mixer/strip";
import { surroundScreen } from "./surround/screen";
import {
  SURROUND_AMBIENCE_LAYOUT,
  SURROUND_COMMON_LAYOUT,
  SURROUND_EXT_LAYOUT,
  SURROUND_PAD_LAYOUT,
  SURROUND_PART_LAYOUT,
} from "./surround/sections";
import type { ScreenLayout, SectionLayout, VariantSet } from "./types";

/** Section layouts that are not variants. */
export const SECTION_LAYOUTS: readonly SectionLayout<FieldKey>[] = [
  EQ_LAYOUT,
  SURROUND_COMMON_LAYOUT,
  SURROUND_AMBIENCE_LAYOUT,
  SURROUND_PAD_LAYOUT,
  SURROUND_PART_LAYOUT,
  SURROUND_EXT_LAYOUT,
];

/** Variant sets; the validator walks every key core enumerates. */
export const VARIANT_SETS: readonly VariantSet<FieldKey, FieldKey>[] = [STRIP_SET];

/** Screen layouts; a function, as screens read the device spec from core. */
export function screenLayouts(): readonly ScreenLayout[] {
  return [mixerScreen(false).layout, mixerScreen(true).layout, surroundScreen()];
}
