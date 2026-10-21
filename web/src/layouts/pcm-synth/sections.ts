/**
 * The PCM Synth editor's Sections. BOOST, RING and each partial's switch are
 * drawn from their layouts here. The rest (COMMON, a partial's LFO1 to OUTPUT
 * boxes, MFX) still draw their own CSS Controls, hosted on whole-Cell
 * footprints (PRD #349: Grid and CSS Sections coexist while a screen
 * migrates); their frames are only outlines for the screen.
 *
 * Hosted footprints are each box's natural size at zoom 1, plus the 4px
 * border inset either side, rounded up to whole Cells; every box in a lane
 * row takes the row's height, as the old grid stretched them.
 */

import { PcmsPmtField, pcms_pmt_spec } from "../../../pkg/integral_wasm.js";
import { enumFields } from "../fields";
import type { SectionFrame, SectionLayout } from "../types";

/** The PMT fields, for the Sections that bind the partial mix. */
const PMT_FIELDS = enumFields<PcmsPmtField>("PcmsPmtField", PcmsPmtField, pcms_pmt_spec);

/** Cells of one lane row: the tallest box (FILTER, 650px) and its insets. */
export const ROW_H = 28;

/** A hosted box's outline: `w`×`h` Cells in all, its own header in the top row. */
const hosted = (title: string, w: number, h: number = ROW_H): SectionFrame => ({
  title,
  titleEdge: "top",
  w,
  h: h - 1,
});

/** The hosted Sections' outlines, by the box they host. */
export const HOSTED = {
  /** Tone-wide settings and the partial mix: 147px tall on one line, up to 1590px wide with the widest Structure diagrams. */
  common: hosted("COMMON", 70, 7),
  /** 204px wide; under the partial's switch, so two rows short. */
  lfo1: hosted("LFO1", 9, ROW_H - 2),
  /** 204px. */
  lfo2: hosted("LFO2", 9),
  /** 192px. */
  step: hosted("STEP LFO", 9),
  /** 242px. */
  wave: hosted("WAVE", 11),
  /** 278px. */
  pitch: hosted("PITCH", 12),
  /** 71px. */
  ctrl: hosted("CTRL", 4),
  /** 278×650px: the tallest box sets {@link ROW_H}. */
  tvf: hosted("FILTER", 12),
  /** 278px. */
  tva: hosted("AMP", 12),
  /** 164px. */
  out: hosted("OUTPUT", 8),
} as const satisfies Record<string, SectionFrame>;

/** One partial's on/off switch, over its LFO1; a Partial Scope binds it. */
export const PARTIAL_LAYOUT: SectionLayout<PcmsPmtField> = {
  title: "Partial {partial}",
  titleEdge: "top",
  w: 9,
  h: 1,
  fields: PMT_FIELDS,
  // The title already names the switch.
  items: [{ kind: "switch", field: PcmsPmtField.PartialSwitch, x: 0, y: 0, label: false }],
};

/** A pair's booster, on its signal path (Structure 3 and 4), as tall as a lane. */
function boostLayout(field: PcmsPmtField): SectionLayout<PcmsPmtField> {
  return {
    title: "BOOST",
    titleEdge: "top",
    w: 2,
    h: ROW_H - 1,
    fields: PMT_FIELDS,
    // Centred on the lane, where the arrows meet it.
    items: [{ kind: "knob", field, x: 0, y: 12, label: "Boost" }],
  };
}

/** Partials 1 & 2's booster (Booster 1&2). */
export const BOOST_12_LAYOUT = boostLayout(PcmsPmtField.Booster12);
/** Partials 3 & 4's booster (Booster 3&4). */
export const BOOST_34_LAYOUT = boostLayout(PcmsPmtField.Booster34);

/** A pair's ring modulator (Structure 5-10): no settings, only what it multiplies. */
function ringLayout(product: string): SectionLayout<PcmsPmtField> {
  return {
    title: "RING",
    titleEdge: "top",
    w: 3,
    h: 3,
    fields: PMT_FIELDS,
    items: [],
    silkscreen: [{ kind: "label", text: product, x: 1.5, y: 1.5 }],
  };
}

/** Partials 1 & 2's ring modulator. */
export const RING_12_LAYOUT = ringLayout("1×2");
/** Partials 3 & 4's ring modulator. */
export const RING_34_LAYOUT = ringLayout("3×4");
