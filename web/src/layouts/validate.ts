/**
 * The layout validator (ADR 0005). `layouts.test.ts` runs it over every
 * layout in `registry.ts` and every variant key, so misalignment fails CI
 * rather than review. Each function returns the problems it finds; none is
 * a valid layout.
 */

import { DisplayKind } from "../../pkg/integral_wasm.js";
import { optionListRows, visibleOption, type Cells } from "../synth-ui/grid";
import { elementCount, isTwoState, type FieldKey, type FieldSpace } from "./fields";
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
  RegionVariantSet,
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
  /** Two options of a switch or list that read the same once cut to its width (`visibleOption`). */
  | "label"
  /** A variant that leaves a parameter (or, in a region variant set, a Section) of its key out. */
  | "unplaced"
  /** A Section covering a Lane's Cell. */
  | "lane"
  /**
   * An arrow whose endpoint isn't on the Section or screen, that no free Cells
   * or Lanes can carry, or whose glyph shares a Cell with another.
   */
  | "arrow"
  /**
   * A field, Display or variant key core doesn't define, a Section a region
   * variant's key doesn't have, or an element index outside its field's
   * count (or missing on an array field, or given on a single-valued one).
   */
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

/** A field's name for messages, with its element index when it has one: `TvaEnvLevel[1]`. */
function fieldName<F extends FieldKey>(fields: FieldSpace<F>, field: F, index?: number): string {
  return index === undefined ? fields.nameOf(field) : `${fields.nameOf(field)}[${index}]`;
}

/** Names an element for messages. */
function elementName<F extends FieldKey>(el: ElementLayout<F>, fields: FieldSpace<F>): string {
  return el.kind === "display" ? displayName(el.display) : `${el.kind} ${fieldName(fields, el.field, el.index)}`;
}

/** What is wrong with a Control's element index for its field, if anything. */
function indexProblem<F extends FieldKey>(fields: FieldSpace<F>, field: F, index: number | undefined): string | undefined {
  const count = elementCount(fields, field);
  if (index === undefined) return count > 1 ? `an array field of ${count}: name one element` : undefined;
  if (count === 1) return "not an array field: names no element";
  if (!Number.isInteger(index) || index < 0 || index >= count) return `element ${index} outside 0..${count - 1}`;
  return undefined;
}

/** A key for one element of a field, for counting placements. */
const placementKey = (field: FieldKey, index: number): string => `${typeof field}:${String(field)}#${index}`;

/** Reports each Cell where two different glyphs would be drawn. */
function reportGlyphClashes(report: Report, arrows: readonly DrawnArrow[]): void {
  for (const cell of glyphClashes(arrows)) {
    report("arrow", `glyph at ${cell}`, "two targets or glyphs share one Cell");
  }
}

/** Two of `field`'s option names that a `kind` Control `w` Cells wide shows as the same text, and that text; or none. */
function sameVisibleOptions<F extends FieldKey>(
  fields: FieldSpace<F>,
  field: F,
  kind: "list" | "switch",
  w: number,
): { first: string; second: string; shown: string } | undefined {
  const seen = new Map<string, string>();
  for (const label of fields.optionLabels?.(field) ?? []) {
    const shown = visibleOption(label, kind, w);
    const first = seen.get(shown);
    if (first !== undefined && first !== label) return { first, second: label, shown };
    seen.set(shown, label);
  }
  return undefined;
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
    } else {
      const badIndex = indexProblem(layout.fields, el.field, el.index);
      if (badIndex !== undefined) {
        report("field", name, badIndex);
      } else if (el.kind === "list") {
        const need = optionListRows(layout.fields.optionCount(el.field), el.label !== false);
        if (el.h < need) report("footprint", name, `${el.h} rows, needs ${need}`);
      } else if (el.kind === "switch" && !isTwoState(layout.fields, el.field)) {
        report("footprint", name, "a 2×1 switch needs a two-state field");
      }
      if (el.kind === "list" || el.kind === "switch") {
        const clash = sameVisibleOptions(layout.fields, el.field, el.kind, rect.w);
        if (clash) report("label", name, `${clash.first} and ${clash.second} both read "${clash.shown}"`);
      }
    }
  };

  const sw = layout.titleSwitch;
  if (sw !== undefined) {
    const name = `title switch ${fieldName(layout.fields, sw)}`;
    if (!known.has(sw)) report("field", name, `not in ${layout.fields.name}`);
    else if (!isTwoState(layout.fields, sw)) report("footprint", name, "a title switch needs a two-state field");
  }

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
    "field" in end ? fieldName(layout.fields, end.field, end.index) : displayName(end.display);
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
 * Problems with a variant set: every key's layout, and every parameter of
 * each key placed, each element of an array field on its own. A parameter
 * in any alternative, or as the title switch, counts as placed.
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
    const placed = new Set<string>();
    if (layout.titleSwitch !== undefined) placed.add(placementKey(layout.titleSwitch, 0));
    for (const el of allElements(layout.items)) {
      if (el.kind !== "display") placed.add(placementKey(el.field, el.index ?? 0));
    }
    for (const id of layout.fields.ids) {
      const count = elementCount(layout.fields, id);
      for (let i = 0; i < count; i++) {
        if (placed.has(placementKey(id, i))) continue;
        report("unplaced", `[${key}] ${fieldName(layout.fields, id, count > 1 ? i : undefined)}`, "not placed");
      }
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

/**
 * Problems with a region variant set: every key's region as a screen of the
 * set's footprint, and each placing exactly the Sections core gives its key.
 */
export function validateRegionSet<K extends FieldKey>(set: RegionVariantSet<K>): Problem[] {
  const { problems, report } = collector(set.name);
  const keys = new Set<FieldKey>(set.keys);
  for (const key of set.variants.keys()) {
    if (!keys.has(key)) report("field", `[${key}]`, "not a key core defines");
  }
  for (const key of set.keys) {
    const region = set.variants.get(key);
    if (region === undefined) {
      report("unplaced", `[${key}]`, "no variant");
      continue;
    }
    problems.push(...validateScreen({ name: `${set.name}[${key}]`, w: set.w, h: set.h, ...region }));
    const wanted = new Set(set.sectionsOf(key));
    const placed = new Set(region.sections.map((p) => p.id));
    for (const id of wanted) if (!placed.has(id)) report("unplaced", `[${key}] ${id}`, "not placed");
    for (const id of placed) if (!wanted.has(id)) report("field", `[${key}] ${id}`, "not a Section of its key");
  }
  return problems;
}
