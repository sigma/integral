/**
 * The SN Acoustic INST Section: a variant set keyed by instrument (its
 * picker number, the 1-based position in core's list). The acoustic pianos
 * and TW Organ, which Roland's Editor gives pages of their own, have hand
 * layouts; every other instrument's Modify Parameters are packed by
 * {@link packModifyParams}. All fit one footprint, so picking another
 * instrument never moves a neighbouring Section.
 */

import { SnaInstPage, sna_inst_page, sna_inst_page_kind } from "../../../pkg/integral_wasm.js";
import { snaInsts } from "../../snaInstCatalog";
import { memo } from "../fields";
import type { ControlLayout, SectionBody, VariantSet } from "../types";
import { instOf, isTwoState, modifyDefs, modifyFields } from "./fields";

/** Footprint width in Cells: TW Organ's nine drawbar faders. */
const W = 18;
/** Footprint height in Cells: TW Organ's drawbars, switch row and knob row. */
const H = 10;

/**
 * The acoustic pianos' page: one row of large knobs, slots 1-6 in Parameter
 * Guide order.
 */
export const AC_PIANO_BODY: SectionBody<number> = {
  items: [
    { kind: "knob", size: "lg", field: 1, x: 0, y: 0, label: "String Res" },
    { kind: "knob", size: "lg", field: 2, x: 3, y: 0, label: "Key Off Res" },
    { kind: "knob", size: "lg", field: 3, x: 6, y: 0, label: "Hammer" },
    { kind: "knob", size: "lg", field: 4, x: 9, y: 0, label: "Width" },
    { kind: "knob", size: "lg", field: 5, x: 12, y: 0, label: "Nuance" },
    { kind: "knob", size: "lg", field: 6, x: 15, y: 0, label: "Character" },
  ],
};

/** TW Organ's drawbar footages, slots 1-9 left to right. */
const FOOTAGES = ["16'", "5-1/3'", "8'", "4'", "2-2/3'", "2'", "1-3/5'", "1-1/3'", "1'"] as const;

/** Drawbar fader height in Cells. */
const BAR_H = 6;

/**
 * TW Organ's page: the nine harmonic bars as faders; under them the
 * percussion switches, then the leakage, percussion and click knobs.
 */
export const TW_ORGAN_BODY: SectionBody<number> = {
  items: [
    ...FOOTAGES.map(
      (label, i): ControlLayout<number> => ({ kind: "fader", field: i + 1, x: i * 2, y: 0, h: BAR_H, label }),
    ),
    { kind: "switch", field: 11, x: 0, y: BAR_H, label: "Perc" },
    { kind: "switch", field: 12, x: 2, y: BAR_H, label: "Soft" },
    { kind: "switch", field: 15, x: 4, y: BAR_H, label: "Slow" },
    { kind: "switch", field: 18, x: 6, y: BAR_H, label: "Harm" },
    { kind: "knob", field: 10, x: 0, y: BAR_H + 1, label: "Leakage" },
    { kind: "knob", field: 13, x: 2, y: BAR_H + 1, label: "Soft Lvl" },
    { kind: "knob", field: 14, x: 4, y: BAR_H + 1, label: "Norm Lvl" },
    { kind: "knob", field: 16, x: 6, y: BAR_H + 1, label: "Slow Time" },
    { kind: "knob", field: 17, x: 8, y: BAR_H + 1, label: "Fast Time" },
    { kind: "knob", field: 19, x: 10, y: BAR_H + 1, label: "Recharge" },
    { kind: "knob", field: 20, x: 12, y: BAR_H + 1, label: "Harm Bar" },
    { kind: "knob", field: 21, x: 14, y: BAR_H + 1, label: "On Click" },
    { kind: "knob", field: 22, x: 16, y: BAR_H + 1, label: "Off Click" },
  ],
};

/** Small knobs (or switches) per row: the footprint's width in 2-Cell Controls. */
const PER_ROW = W / 2;

/**
 * The INST layout of an instrument without a hand one: its knobs row-major,
 * then its two-state parameters as switches in rows below, in core's slot
 * order.
 *
 * This is SN-A's own small generator. #360 may promote a shared variant-set
 * generator into `layouts/`; this one is meant to be folded into it then.
 */
export function packModifyParams(key: number): SectionBody<number> {
  const defs = [...modifyDefs(key).values()];
  const knobs = defs.filter((d) => !isTwoState(d));
  const switches = defs.filter(isTwoState);
  const knobRows = Math.ceil(knobs.length / PER_ROW);
  return {
    items: [
      ...knobs.map(
        (d, i): ControlLayout<number> => ({
          kind: "knob",
          field: d.slot,
          x: (i % PER_ROW) * 2,
          y: Math.floor(i / PER_ROW) * 3,
        }),
      ),
      ...switches.map(
        (d, i): ControlLayout<number> => ({
          kind: "switch",
          field: d.slot,
          x: (i % PER_ROW) * 2,
          y: knobRows * 3 + Math.floor(i / PER_ROW),
        }),
      ),
    ],
  };
}

/** The INST page core gives instrument `key`: `"Ac.Piano"`, `"TW Organ"`, or its category. */
export function instPage(key: number): string | undefined {
  const inst = instOf(key);
  return inst ? sna_inst_page(inst.variation, inst.instNumber) : undefined;
}

/** The hand layout of each page that has one. */
const HAND_BODIES: Partial<Record<SnaInstPage, SectionBody<number>>> = {
  [SnaInstPage.AcPiano]: AC_PIANO_BODY,
  [SnaInstPage.TwOrgan]: TW_ORGAN_BODY,
};

/** The hand layout of each instrument whose page has one. */
function handVariants(): Map<number, SectionBody<number>> {
  const out = new Map<number, SectionBody<number>>();
  for (const inst of snaInsts()) {
    const page = sna_inst_page_kind(inst.variation, inst.instNumber);
    const body = page === undefined ? undefined : HAND_BODIES[page];
    if (body) out.set(inst.number, body);
  }
  return out;
}

const instKeys = memo(() => snaInsts().map((i) => i.number));
const instVariants = memo(handVariants);

/**
 * The INST variant set over every instrument core lists. Its title is a
 * stand-in: the editor shows the instrument's page name instead. Its keys and
 * hand variants are read from core on first use, since the wasm core starts
 * after this module loads.
 */
export const SNA_INST_SET: VariantSet<number, number> = {
  name: "SN-A INST",
  title: "INST",
  titleEdge: "top",
  w: W,
  h: H,
  get keys() {
    return instKeys();
  },
  fieldsOf: modifyFields,
  get variants() {
    return instVariants();
  },
  generate: packModifyParams,
};
