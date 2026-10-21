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
  /**
   * Picker: its name, then the current option's, opening a searchable list.
   * N×1: a layout may pick another width (a mixer strip's output, 2 or 4).
   */
  picker: { w: 6, h: 1 },
} as const satisfies Record<string, Cells | number>;

/** An edge of a footprint. */
export type Edge = "top" | "right" | "bottom" | "left";

const ALL_EDGES = ["top", "right", "bottom", "left"] as const satisfies readonly Edge[];

/**
 * The edges of its footprint each kind of Control or Display draws right up
 * to. A Section's border runs through the Cells along its edges (`INSET`
 * inside them), so where one of these edges lies on a border Cell the
 * renderer draws the element clear of the border on that edge (`edgePadding`).
 * Every Control's name sits in its top line. A knob's LED ring spans its
 * box's width, so it reaches the sides too, and only its readout keeps a
 * margin at the bottom; a fader keeps its own margin on the sides and
 * bottom; a switch, an option list, a picker or a Display fills its Cells.
 */
export const BLEED = {
  knob: ["top", "left", "right"],
  fader: ["top"],
  switch: ALL_EDGES,
  list: ALL_EDGES,
  picker: ALL_EDGES,
  display: ALL_EDGES,
} as const satisfies Record<string, readonly Edge[]>;

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
/** Height of a Control's addon line under its readout (the tempo-sync toggle), px. */
export const ADDON_LINE = 12;

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

/** A box in px at zoom 1. */
export interface Px {
  /** Width, px. */
  width: number;
  /** Height, px. */
  height: number;
}

/** `cells` in px at zoom 1. */
export function cellsPx(cells: Cells): Px {
  return { width: cells.w * CELL, height: cells.h * CELL };
}

/**
 * The zoom `el` is drawn at: its on-screen width over its layout width, or 1
 * when it isn't laid out. Under a page's CSS `zoom` (`GridZoom`),
 * `getBoundingClientRect` is on screen and `offsetWidth` is in layout px;
 * divide rect distances by this to draw in layout px.
 */
export function zoomOf(el: HTMLElement): number {
  const layout = el.offsetWidth;
  return layout > 0 ? el.getBoundingClientRect().width / layout : 1;
}
