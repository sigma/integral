/**
 * The layout validator (ADR 0005). `layouts.test.ts` runs it over every
 * layout in `registry.ts` and every variant key, so misalignment fails CI
 * rather than review. Each function returns the problems it finds; none is
 * a valid layout.
 */

import { DisplayKind } from "../../pkg/integral_wasm.js";
import { BLEED, optionListRows, type Cells, type Edge } from "../synth-ui/grid";
import { isTwoState, type FieldKey, type FieldSpace } from "./fields";
import { DISPLAY_MIN_FOOTPRINT, allElements, itemRect, sectionCells, variantLayout } from "./geometry";
import {
  drawnArrows,
  glyphClashes,
  isEndpoint,
  routeScreenArrow,
  routeSectionArrow,
  type DrawnArrow,
} from "./routing";
import type {
  CellRect,
  ElementLayout,
  HalfCellPoint,
  ItemEndpoint,
  ScreenLayout,
  SectionArrowSilk,
  SectionLayout,
  VariantSet,
} from "./types";

/** The kinds of problem the validator reports, one per check. */
export type Check =
  /** Whole Cells for Sections, Controls, Displays, Lanes; half Cells for Silkscreen. */
  | "lattice"
  /** Two things on one Cell, outside one `alternatives` group's choices. */
  | "overlap"
  /** Something outside its Section, its variant footprint, its group or its screen. */
  | "containment"
  /** A Display under its kind's minimum, an option list shorter than its options, or a switch with more than two. */
  | "footprint"
  /**
   * A Control or Display drawing under its Section's border: on an edge Cell
   * row or column with no title between it and the border (`BLEED`).
   */
  | "clearance"
  /** A variant that leaves a parameter of its key out. */
  | "unplaced"
  /** A Section covering a Lane's Cell. */
  | "lane"
  /**
   * An arrow whose endpoint isn't on the Section or screen, that no free Cells
   * or Lanes can carry, or whose glyph shares a Cell with another.
   */
  | "arrow"
  /** A field, Display or variant key core doesn't define. */
  | "field";

/** One thing wrong with a layout. */
export interface Problem {
  /** Which check failed. */
  check: Check;
  /** The layout, and the item in it. */
  where: string;
  /** What is wrong. */
  message: string;
}

/** Records one problem at `at` within the layout being checked. */
type Report = (check: Check, at: string, message: string) => void;

/** A problem list and the `Report` that appends to it, prefixing `where`. */
function collector(where: string): { problems: Problem[]; report: Report } {
  const problems: Problem[] = [];
  return { problems, report: (check, at, message) => problems.push({ check, where: `${where}: ${at}`, message }) };
}

const whole = (n: number): boolean => Number.isInteger(n);
const half = (n: number): boolean => Number.isInteger(n * 2);

/** `inner` lies within `outer`. */
function inside(inner: CellRect, outer: CellRect): boolean {
  return (
    inner.x >= outer.x &&
    inner.y >= outer.y &&
    inner.x + inner.w <= outer.x + outer.w &&
    inner.y + inner.h <= outer.y + outer.h
  );
}

/** Every Cell of `r`, as `"x,y"`. */
function* cellsOf(r: CellRect): Generator<string> {
  for (let y = r.y; y < r.y + r.h; y++) for (let x = r.x; x < r.x + r.w; x++) yield `${x},${y}`;
}

/** Records which owner covers each Cell, reporting the second owner of any Cell. */
class Coverage {
  private readonly owners = new Map<string, string>();

  constructor(private readonly report: Report) {}

  claim(rect: CellRect, owner: string): void {
    const hit = new Set<string>();
    for (const cell of cellsOf(rect)) {
      const other = this.owners.get(cell);
      if (other !== undefined && !hit.has(other)) {
        hit.add(other);
        this.report("overlap", owner, `overlaps ${other}`);
      }
      this.owners.set(cell, owner);
    }
  }

  covered(rect: CellRect): string | undefined {
    for (const cell of cellsOf(rect)) {
      const owner = this.owners.get(cell);
      if (owner !== undefined) return owner;
    }
    return undefined;
  }
}

/** Whether `r` is on whole Cells with a positive size. */
function onLattice(r: CellRect): boolean {
  return [r.x, r.y, r.w, r.h].every(whole) && r.w > 0 && r.h > 0;
}

/** Reports `rect` (named `name`) if it is off whole Cells or outside `bounds`. */
function checkRect(report: Report, name: string, rect: CellRect, bounds: CellRect): void {
  if (!onLattice(rect)) report("lattice", name, "not on whole Cells");
  if (!inside(rect, bounds)) report("containment", name, "outside its bounds");
}

/** A Display's name for messages: its core id, or the bare number core doesn't know. */
function displayName(display: DisplayKind): string {
  return `display ${DisplayKind[display] ?? display}`;
}

/** Names an element for messages. */
function elementName<F extends FieldKey>(el: ElementLayout<F>, fields: FieldSpace<F>): string {
  return el.kind === "display" ? displayName(el.display) : `${el.kind} ${fields.nameOf(el.field)}`;
}

/** Reports each Cell where two different glyphs would be drawn. */
function reportGlyphClashes(report: Report, arrows: readonly DrawnArrow[]): void {
  for (const cell of glyphClashes(arrows)) {
    report("arrow", `glyph at ${cell}`, "two targets or glyphs share one Cell");
  }
}

/** Problems with one Section's layout. */
export function validateSection<F extends FieldKey>(
  layout: SectionLayout<F>,
  where: string = layout.title,
): Problem[] {
  const { problems, report } = collector(where);
  const known = new Set<FieldKey>(layout.fields.ids);
  const section: CellRect = { x: 0, y: 0, w: layout.w, h: layout.h };
  if (!onLattice(section)) report("lattice", "frame", "size is not whole Cells");

  /** Checks one element inside `bounds`, claiming its Cells in `coverage`. */
  const element = (el: ElementLayout<F>, bounds: CellRect, coverage: Coverage): void => {
    const name = elementName(el, layout.fields);
    const rect = itemRect(el);
    checkRect(report, name, rect, bounds);
    coverage.claim(rect, name);
    if (el.kind === "display") {
      const min = DISPLAY_MIN_FOOTPRINT[el.display] as Cells | undefined;
      if (min === undefined) report("field", name, "not a DisplayKind");
      else if (rect.w < min.w || rect.h < min.h) {
        report("footprint", name, `${rect.w}×${rect.h} is under its ${min.w}×${min.h} minimum`);
      }
    } else if (!known.has(el.field)) {
      report("field", name, `not in ${layout.fields.name}`);
    } else if (el.kind === "list") {
      const need = optionListRows(layout.fields.optionCount(el.field), el.label !== false);
      if (el.h < need) report("footprint", name, `${el.h} rows, needs ${need}`);
    } else if (el.kind === "switch" && !isTwoState(layout.fields, el.field)) {
      report("footprint", name, "a 2×1 switch needs a two-state field");
    }
  };

  const coverage = new Coverage(report);
  for (const item of layout.items) {
    if (item.kind !== "alternatives") {
      element(item, section, coverage);
      continue;
    }
    const name = `alternatives ${item.group}`;
    checkRect(report, name, item, section);
    coverage.claim(item, name);
    // Choices may share the group's Cells; elements of one choice may not.
    for (const option of item.options) {
      const own = new Coverage(report);
      for (const el of option.items) element(el, item, own);
    }
  }

  // An endpoint in any alternative exists: the arrow is drawn when it is.
  const placed = allElements(layout.items);
  const arrows: SectionArrowSilk<F>[] = [];
  for (const silk of layout.silkscreen ?? []) {
    if (silk.kind === "arrow") {
      arrows.push(silk);
      continue;
    }
    const points: readonly HalfCellPoint[] = silk.kind === "rule" ? silk.points : [[silk.x, silk.y]];
    const name = silk.kind === "rule" ? `rule ${JSON.stringify(silk.points)}` : `label ${silk.text}`;
    if (!points.every(([x, y]) => half(x) && half(y))) {
      report("lattice", name, "not on the half-Cell lattice");
    }
    if (!points.every(([x, y]) => x >= 0 && y >= 0 && x <= layout.w && y <= layout.h)) {
      report("containment", name, "outside the Section");
    }
    if (silk.kind === "rule" && silk.points.length < 2) report("lattice", name, "needs two points");
  }
  const endName = (end: ItemEndpoint<F>): string =>
    "field" in end ? layout.fields.nameOf(end.field) : displayName(end.display);
  for (const end of arrows.flatMap((a) => [a.from, a.to])) {
    if (!placed.some((el) => isEndpoint(el, end))) report("arrow", `arrow ${endName(end)}`, "endpoint not placed");
  }
  // Every placement of each end (one per alternative naming it) must route.
  for (const arrow of arrows) {
    const froms = placed.filter((el) => isEndpoint(el, arrow.from));
    const tos = placed.filter((el) => isEndpoint(el, arrow.to));
    const stuck = froms.some((f) => tos.some((t) => routeSectionArrow(layout, arrow, [f, t]) === null));
    if (stuck) report("arrow", `arrow ${endName(arrow.from)} → ${endName(arrow.to)}`, "no route through free Cells");
  }
  reportGlyphClashes(report, drawnArrows(arrows, (a) => routeSectionArrow(layout, a)));
  return problems;
}

/**
 * Clearance problems with one Section's layout: each element whose drawn edge
 * (`BLEED`) lies on a Cell the border runs through. The border sits `INSET`
 * inside the Section's outer Cells; a title row (`top`) or column (`side`)
 * stands between it and the content on that edge, the other edges have none.
 */
export function validateClearance<F extends FieldKey>(layout: SectionLayout<F>, where: string = layout.title): Problem[] {
  const { problems, report } = collector(where);
  for (const el of allElements(layout.items)) {
    const r = itemRect(el);
    const under: Record<Edge, boolean> = {
      top: r.y === 0 && layout.titleEdge !== "top",
      left: r.x === 0 && layout.titleEdge !== "side",
      bottom: r.y + r.h === layout.h,
      right: r.x + r.w === layout.w,
    };
    const edges = (BLEED[el.kind] as readonly Edge[]).filter((e) => under[e]);
    if (edges.length > 0) {
      report("clearance", elementName(el, layout.fields), `draws under the border on its ${edges.join(", ")} edge`);
    }
  }
  return problems;
}

/**
 * Problems with a variant set: every key's layout, and every parameter of
 * each key placed. A parameter in any alternative counts as placed.
 */
export function validateVariantSet<K extends FieldKey, F extends FieldKey>(
  set: VariantSet<K, F>,
): Problem[] {
  const { problems, report } = collector(set.name);
  const keys = new Set<FieldKey>(set.keys);
  for (const key of set.variants.keys()) {
    if (!keys.has(key)) report("field", `[${key}]`, "not a key core defines");
  }
  for (const key of set.keys) {
    const layout = variantLayout(set, key);
    problems.push(...validateSection(layout, `${set.name}[${key}]`));
    const placed = new Set<FieldKey>();
    for (const el of allElements(layout.items)) if (el.kind !== "display") placed.add(el.field);
    for (const id of layout.fields.ids) {
      if (!placed.has(id)) report("unplaced", `[${key}] ${layout.fields.nameOf(id)}`, "not placed");
    }
  }
  return problems;
}

/** Problems with a screen's layout: Section placement, Lanes and arrows between Sections. */
export function validateScreen(screen: ScreenLayout): Problem[] {
  const { problems, report } = collector(screen.name);
  const grid: CellRect = { x: 0, y: 0, w: screen.w, h: screen.h };
  if (!onLattice(grid)) report("lattice", "frame", "size is not whole Cells");

  const coverage = new Coverage(report);
  const ids = new Set<string>();
  for (const p of screen.sections) {
    const name = `section ${p.id}`;
    ids.add(p.id);
    if (!onLattice({ x: 0, y: 0, w: p.section.w, h: p.section.h })) {
      report("lattice", name, "content size is not whole Cells");
    }
    const rect: CellRect = { x: p.x, y: p.y, ...sectionCells(p.section) };
    checkRect(report, name, rect, grid);
    coverage.claim(rect, name);
  }
  screen.lanes.forEach((lane, i) => {
    const name = `lane ${i}`;
    checkRect(report, name, lane, grid);
    const by = coverage.covered(lane);
    if (by !== undefined) report("lane", name, `covered by ${by}`);
  });
  for (const arrow of screen.silkscreen) {
    const unknown = [arrow.from, arrow.to].filter((end) => !ids.has(end.section));
    for (const end of unknown) report("arrow", `arrow ${end.section}`, "no such Section");
    if (unknown.length === 0 && routeScreenArrow(screen, arrow) === null) {
      report("arrow", `arrow ${arrow.from.section} → ${arrow.to.section}`, "no route through the Lanes");
    }
  }
  reportGlyphClashes(report, drawnArrows(screen.silkscreen, (a) => routeScreenArrow(screen, a)));
  return problems;
}
