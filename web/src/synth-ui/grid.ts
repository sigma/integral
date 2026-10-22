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
  /** Power toggle (`PowerButton`): a two-state switch drawn as the power symbol, lit while on. */
  power: { w: 1, h: 1 },
  /** Fader: 2×N. */
  faderW: 2,
  /** Level fader (a fader with its scale, as the mixer's): 3×N, the scale beside the wide cap. */
  levelFaderW: 3,
  /** Vertical option list: 2×N, one Cell row per option; a layout may pick a wider `w` for long names. */
  optionListW: 2,
  /**
   * Picker: its name, then the current option's, opening its list (searchable when long).
   * N×1: a layout may pick another width (a mixer strip's output, 2 or 4).
   */
  picker: { w: 6, h: 1 },
  /** A synth box's dropdown (a picker over a short list): its name and the longest option's, N of N×1. */
  dropdownW: 4,
} as const satisfies Record<string, Cells | number>;

/**
 * The type and spacing of an option's name on the Grid, px at zoom 1: a list
 * row's (beside its LED) and a switch state's (beside its LED, inside the
 * toggle's margin). `SynthSwitch` draws its Grid variant from these, and
 * {@link visibleOption} measures from them, so the two can't drift.
 */
export const OPTION_TEXT = {
  list: { fontSize: 8, padding: 3, led: 6, gap: 3, margin: 0 },
  switch: { fontSize: 9, padding: 2, led: 6, gap: 3, margin: 2 },
} as const;

/**
 * The average width of an uppercase option name's glyph, per px of font
 * size, in the UI's bold system font (measured in Chrome: `KEY-OFF-NORMAL`
 * is 77px at 8px, `NO-SUS` 38px at 9px). Wide glyphs (`M`, `W`) take more.
 */
export const GLYPH_EM = 0.72;

/** What draws an option's name: a row of an option list, or a two-state switch. */
export type OptionKind = keyof typeof OPTION_TEXT;

/**
 * Characters of an option's name a `kind` Control `w` Cells wide shows at
 * zoom 1. A list row's name runs on into its right padding before the row
 * clips it; a switch's state stops at its padding.
 */
export function optionChars(kind: OptionKind, w: number): number {
  const t = OPTION_TEXT[kind];
  const pads = kind === "list" ? t.padding : 2 * t.padding;
  const px = w * CELL - 2 * t.margin - pads - t.led - t.gap;
  return Math.floor(px / (t.fontSize * GLYPH_EM));
}

/**
 * `label` as a `kind` Control `w` Cells wide shows it at zoom 1: whole, or
 * cut. A list row clips it; a switch cuts it to one fewer than fits and an
 * ellipsis.
 */
export function visibleOption(label: string, kind: OptionKind, w: number): string {
  const n = optionChars(kind, w);
  if (label.length <= n) return label;
  return kind === "list" ? label.slice(0, n) : `${label.slice(0, n - 1)}…`;
}

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
 * bottom, but a level fader's panel fills its Cells, as a switch, a power
 * toggle, an option list, a picker or a Display does.
 */
export const BLEED = {
  knob: ["top", "left", "right"],
  fader: ["top"],
  levelFader: ALL_EDGES,
  switch: ALL_EDGES,
  power: ALL_EDGES,
  list: ALL_EDGES,
  picker: ALL_EDGES,
  display: ALL_EDGES,
} as const satisfies Record<string, readonly Edge[]>;

/**
 * The direction a vertical name reads: bottom to top, as the mixer's strip
 * names and Section side titles do. Inline style, so the two can't drift.
 */
export const BOTTOM_TO_TOP = { writingMode: "vertical-rl", transform: "rotate(180deg)" } as const;

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

/** A box in px at zoom 1. */
export interface Px {
  /** Width, px. */
  width: number;
  /** Height, px. */
  height: number;
}

/** A fader's footprint, N rows tall: a level fader's (`scale`) is wider, for its scale. */
export function faderCells(h: number, scale = false): Cells {
  return { w: scale ? FOOTPRINT.levelFaderW : FOOTPRINT.faderW, h };
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
