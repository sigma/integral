/**
 * Grid geometry for Controls drawn on whole Cells (#329, #333).
 *
 * A Control rendered with its `grid` variant fills exactly its footprint at
 * scale 1. The footprints here are the one table layouts size Controls from;
 * if one proves wrong, change it here, not per layout.
 */

/** Cell size in px at zoom 1. */
export const CELL = 24;

/** A rectangle of whole Cells. */
export interface Cells {
  /** Width in Cells. */
  w: number;
  /** Height in Cells. */
  h: number;
}

/** Fixed footprints, and the width of the ones whose height a layout picks (N). */
export const FOOTPRINT = {
  /** Small knob, name and readout included. */
  knobSm: { w: 2, h: 3 },
  /** Large knob, name and readout included. */
  knobLg: { w: 3, h: 4 },
  /** Two-state switch. */
  switch: { w: 2, h: 1 },
  /** Fader: 2×N. */
  faderW: 2,
  /** Vertical option list: 2×N, one Cell row per option. */
  optionListW: 2,
} as const satisfies Record<string, Cells | number>;

/** A line of Control text on the Grid: its font size and line box, px. */
export interface TextLine {
  /** Font size, px. */
  font: number;
  /** Line box height, px. */
  line: number;
}

/** A Control's name. */
export const NAME: TextLine = { font: 7, line: 10 };
/** A Control's readout. */
export const READOUT: TextLine = { font: 9, line: 12 };

/** Inline style setting `text`'s size and fixing its line box. */
export function lineStyle(text: TextLine): {
  fontSize: number;
  lineHeight: string;
  height: number;
} {
  return { fontSize: text.font, lineHeight: `${text.line}px`, height: text.line };
}

/**
 * Rows of a vertical option list on the Grid: one per option, plus one for its
 * name when it shows one.
 */
export function optionListRows(options: number, named: boolean): number {
  return options + (named ? 1 : 0);
}

/** `cells` in px at zoom 1. */
export function cellsPx(cells: Cells): { width: number; height: number } {
  return { width: cells.w * CELL, height: cells.h * CELL };
}
