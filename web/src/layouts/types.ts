/**
 * The layout format (ADR 0005, `CONTEXT.md` *Layout*).
 *
 * A layout is plain data. Sections, Controls, Displays, `alternatives` and
 * Lanes sit on whole Cells; Silkscreen sits on the half-Cell lattice. Positions
 * are plain numbers, and `layouts.test.ts` enforces the lattice. A layout names
 * what core defines (fields, Displays) and defines nothing itself; it holds no
 * predicates over parameter values, so relevance and the choice between
 * alternatives are the caller's, from core.
 */

import type { DisplayKind } from "../../pkg/integral_wasm.js";
import type { FieldKey, FieldSpace } from "./fields";

/** A rectangle of whole Cells, at `(x, y)` from its container's origin. */
export interface CellRect {
  /** Left edge, in Cells. */
  x: number;
  /** Top edge, in Cells. */
  y: number;
  /** Width, in Cells. */
  w: number;
  /** Height, in Cells. */
  h: number;
}

/**
 * What an inactive Control does with its Cells: `dim` draws it dimmed and
 * inert, `hide` draws nothing. Either way it keeps its Cells; nothing reflows.
 */
export type Inactive = "dim" | "hide";

/** What every Control shares, whatever its kind. */
interface ControlBase<F extends FieldKey> {
  /** The one parameter the Control binds. */
  field: F;
  /** Left edge, in Cells from the Section's content origin. */
  x: number;
  /** Top edge, in Cells from the Section's content origin. */
  y: number;
  /** Overrides the name core gives the field; `false` shows no name. */
  label?: string | false;
  /** What the Control does when core says its field is inactive. Default `dim`. */
  inactive?: Inactive;
}

/** A knob: small 2×3 or large 3×4 (`FOOTPRINT`). */
export interface KnobLayout<F extends FieldKey> extends ControlBase<F> {
  kind: "knob";
  /** Default `sm`. */
  size?: "sm" | "lg";
}

/** A two-state switch, 2×1. */
export interface SwitchLayout<F extends FieldKey> extends ControlBase<F> {
  kind: "switch";
}

/** A vertical fader, 2×`h`. */
export interface FaderLayout<F extends FieldKey> extends ControlBase<F> {
  kind: "fader";
  /** Height in Cells: the N of 2×N. */
  h: number;
}

/** A vertical option list, 2×`h`, one Cell row per option plus one for its name. */
export interface OptionListLayout<F extends FieldKey> extends ControlBase<F> {
  kind: "list";
  /** Height in Cells: the N of 2×N, at least `optionListRows` for the field. */
  h: number;
}

/** One interactive element bound to one parameter. */
export type ControlLayout<F extends FieldKey> =
  | KnobLayout<F>
  | SwitchLayout<F>
  | FaderLayout<F>
  | OptionListLayout<F>;

/**
 * A Display, named by its core id: core says which fields it reads (#352), and
 * the layout never lists them. It fills its rectangle, at least
 * `DISPLAY_MIN_FOOTPRINT` for its kind.
 */
export interface DisplayLayout extends CellRect {
  kind: "display";
  /** Which Display. */
  display: DisplayKind;
}

/** Anything that covers Cells on its own: a Control or a Display. */
export type ElementLayout<F extends FieldKey> = ControlLayout<F> | DisplayLayout;

/** One of an `alternatives` group's choices: the elements it draws. */
export interface Alternative<F extends FieldKey> {
  /** The name the caller picks it by. */
  name: string;
  /** Its elements, in Section coordinates, inside the group's rectangle. */
  items: readonly ElementLayout<F>[];
}

/**
 * Different elements on the same Cells (PCM wave picker or OSC scope, ambience
 * or sends). One alternative is drawn at a time, picked by the caller from
 * core; the first is the default. Only these may overlap each other.
 */
export interface AlternativesLayout<F extends FieldKey> extends CellRect {
  kind: "alternatives";
  /** The group's name, which the caller picks an alternative for. */
  group: string;
  /** The choices; at least one. */
  options: readonly Alternative<F>[];
}

/** Anything a Section places on its Cells. */
export type ItemLayout<F extends FieldKey> = ElementLayout<F> | AlternativesLayout<F>;

/** A point on the half-Cell lattice, `[x, y]` in Cells. */
export type HalfCellPoint = readonly [number, number];

/** A rule: a polyline through half-Cell points, clipped to the drawn border. */
export interface RuleSilk {
  kind: "rule";
  /** At least two points. */
  points: readonly HalfCellPoint[];
}

/** A group name (HI, MID, LO) centred on a half-Cell point in a free slot. */
export interface GroupNameSilk {
  kind: "label";
  /** The name drawn. */
  text: string;
  /** Centre, in Cells. */
  x: number;
  /** Centre, in Cells. */
  y: number;
}

/** The edge of an element an arrow leaves or enters, at its centre. */
export type Side = "top" | "right" | "bottom" | "left";

/** An end of an arrow inside a Section: a placed Control (by field) or Display. */
export type ItemEndpoint<F extends FieldKey> =
  | { field: F; side: Side }
  | { display: DisplayKind; side: Side };

/** A signal arrow inside a Section. Routed from the layout, not the DOM (#355). */
export interface SectionArrowSilk<F extends FieldKey> {
  kind: "arrow";
  /** Where the signal leaves. */
  from: ItemEndpoint<F>;
  /** Where it enters. */
  to: ItemEndpoint<F>;
}

/** A Section's own Silkscreen, in Section coordinates; covers no Cells. */
export type SectionSilk<F extends FieldKey> = RuleSilk | GroupNameSilk | SectionArrowSilk<F>;

/** Which edge of a Section carries its 1-Cell title. */
export type TitleEdge = "top" | "side";

/**
 * A Section's outline: its title, and its content size in whole Cells. The
 * Section covers one more Cell row (`top`) or column (`side`) for the title;
 * see `sectionCells`. Content coordinates start after the title.
 */
export interface SectionFrame {
  /**
   * Shown in the title. `{name}` is replaced with the Scope's `name` index,
   * 1-based: `"Partial {partial}"` with `{ partial: 0 }` reads "Partial 1".
   */
  title: string;
  /** Which edge carries the title. */
  titleEdge: TitleEdge;
  /** Content width, in Cells. */
  w: number;
  /** Content height, in Cells. */
  h: number;
}

/** What a Section draws: its items and Section Silkscreen. */
export interface SectionBody<F extends FieldKey> {
  /** Controls, Displays and `alternatives`, in any order (focus is row-major). */
  items: readonly ItemLayout<F>[];
  /** Rules, group names and arrows. */
  silkscreen?: readonly SectionSilk<F>[];
}

/** One Section's layout. One module per Section; a Scope binds each instance. */
export interface SectionLayout<F extends FieldKey> extends SectionFrame, SectionBody<F> {
  /** The fields its Controls may name, from core. */
  fields: FieldSpace<F>;
}

/**
 * The indices binding one instance of a Section to its parameters (part,
 * partial, drum key, layer), 0-based. The caller's `bind` resolves fields
 * with it; the renderer only reads it for the title.
 */
export type Scope = Readonly<Record<string, number>>;

/**
 * A variant set: one Section's layouts, one per key (MFX type, structure
 * type, instrument, strip kind), all in one footprint, so changing the key
 * never moves a neighbour. Keys without a hand layout get one from
 * `generate`. Every variant must place every parameter of its key.
 */
export interface VariantSet<K extends FieldKey, F extends FieldKey> extends SectionFrame {
  /** Which set this is, for validator messages. */
  name: string;
  /** Every key, enumerated from core. */
  keys: readonly K[];
  /** The parameters of one key, from core: all of them must be placed. */
  fieldsOf(key: K): FieldSpace<F>;
  /** Hand layouts, by key. */
  variants: ReadonlyMap<K, SectionBody<F>>;
  /** The layout of a key without a hand one, generated from its fields. */
  generate(key: K): SectionBody<F>;
}

/** A Section placed on a screen. */
export interface SectionPlacement {
  /** Unique on the screen; arrows name it. */
  id: string;
  /** The Section (or variant set) placed; only its outline matters here. */
  section: SectionFrame;
  /** Left edge of the whole Section, title included, in Cells. */
  x: number;
  /** Top edge of the whole Section, title included, in Cells. */
  y: number;
}

/** An end of an arrow between Sections. */
export interface SectionEndpoint {
  /** A placement's `id`. */
  section: string;
  /** Which edge. */
  side: Side;
}

/** A signal arrow between Sections, running through Lanes (#355). */
export interface ScreenArrowSilk {
  kind: "arrow";
  /** Where the signal leaves. */
  from: SectionEndpoint;
  /** Where it enters. */
  to: SectionEndpoint;
}

/** A run of Cells nothing covers, so arrows between Sections have room. */
export type Lane = CellRect;

/** A screen's layout: Sections on the Grid, its Lanes, and Screen Silkscreen. */
export interface ScreenLayout {
  /** Which screen, for validator messages. */
  name: string;
  /** Grid width, in Cells. */
  w: number;
  /** Grid height, in Cells. */
  h: number;
  /** The Sections, in any order (focus goes in reading order). */
  sections: readonly SectionPlacement[];
  /** Cells left uncovered for arrows. */
  lanes: readonly Lane[];
  /** Arrows between Sections. */
  silkscreen: readonly ScreenArrowSilk[];
}
