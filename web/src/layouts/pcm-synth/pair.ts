/**
 * A PCM Synth partial pair (1 & 2, or 3 & 4): a region variant set keyed by
 * Structure Type. Each lane keeps its heads (the partial's switch, LFOs, STEP
 * LFO, WAVE, PITCH and CTRL) in fixed columns; its path follows. An
 * independent lane runs straight on to its OUTPUT. A merged pair's blocks
 * after the join sit centred on both lanes, ending in the lower partial's
 * OUTPUT, through the RING where the Structure rings. Boxes of one block
 * abut; arrows run through the Lanes between blocks.
 *
 * Core's routing (`pcms_structure_routing`) says which blocks each Structure
 * has and in what order; this file says where they go: fixed head columns, a
 * Lane between blocks, the join centred on both lanes, all in one footprint
 * the validator holds every Structure to.
 */

import { PcmsPmtField, pcms_pmt_spec } from "../../../pkg/integral_wasm.js";
import { routingOf, type Block, type Side } from "../../pcm-synth/structureRouting";
import { sectionCells } from "../geometry";
import { freeLanes } from "../region";
import type { ArrowGlyph, RegionVariantSet, ScreenArrowSilk, ScreenRegion, SectionFrame, SectionPlacement } from "../types";
import { BOOST_12_LAYOUT, HOSTED, PARTIAL_LAYOUT, RING_12_LAYOUT, ROW_H } from "./sections";

/** A partial's own boxes, one Section each. */
export type PartialBlock = "partial" | "lfo1" | "lfo2" | "step" | "wave" | "pitch" | "ctrl" | "tvf" | "tva" | "out";

/** A Section of the pair, by its id in the region: a partial's box, or the pair's BOOST or RING. */
export type PairSection = `${Side}.${PartialBlock}` | "boost" | "ring";

/** The partial's box a pair Section shows, or `undefined` for BOOST and RING. */
export function partialBoxOf(section: PairSection): { side: Side; block: PartialBlock } | undefined {
  if (section === "boost" || section === "ring") return undefined;
  const [side, block] = section.split(".") as [Side, PartialBlock];
  return { side, block };
}

/** Top row of each lane: the lower lane abuts the upper. */
export const LANE_Y: Record<Side, number> = { upper: 0, lower: ROW_H };

/** The pair's height: its two lanes. */
const H = 2 * ROW_H;

/** Top row of a block centred on both lanes, `h` Cells tall. */
const centred = (h: number): number => (H - h) / 2;

/** The heads of each lane, left to right; the partial's switch sits over LFO1. */
const HEADS = ["lfo1", "lfo2", "step", "wave", "pitch", "ctrl"] as const;

/** The outline of each Section; BOOST and RING have one per pair, of one size. */
function frameOf(id: PairSection): SectionFrame {
  if (id === "boost") return BOOST_12_LAYOUT;
  if (id === "ring") return RING_12_LAYOUT;
  const { block } = partialBoxOf(id)!;
  return block === "partial" ? PARTIAL_LAYOUT : HOSTED[block];
}

/** Content plus title Cells: what a Section covers. */
const cellsOf = (f: SectionFrame) => sectionCells(f);

/** `ids` side by side from column `x` on row `y`, each abutting the last. */
function chain(x: number, y: number, ids: readonly PairSection[]): SectionPlacement[] {
  return ids.map((id) => {
    const section = frameOf(id);
    const placed = { id, section, x, y };
    x += cellsOf(section).w;
    return placed;
  });
}

/** Width in Cells of `ids` side by side. */
const widthOf = (ids: readonly PairSection[]): number => ids.reduce((w, id) => w + cellsOf(frameOf(id)).w, 0);

/** Cells of the switch strip that heads each lane. */
const SWITCH_W = cellsOf(PARTIAL_LAYOUT).w;

/** A lane's heads: its switch strip, then LFO1 and the rest abutting. */
function heads(side: Side): SectionPlacement[] {
  const y = LANE_Y[side];
  const sw = { id: `${side}.partial`, section: PARTIAL_LAYOUT, x: 0, y };
  return [sw, ...chain(SWITCH_W, y, HEADS.map((b) => `${side}.${b}` as const))];
}

/** The first column after the heads, where every path starts. */
export const PATH_X = SWITCH_W + widthOf(HEADS.map((b) => `upper.${b}` as const));

/** One arrow between two Sections of the pair, left to right: `[from, to, glyph]`. */
type Arrow = readonly [PairSection, PairSection, ArrowGlyph?];

/** Where a Structure puts its Sections, and the arrows between them. */
interface Path {
  /** Each lane's blocks before the join, or its whole path when independent. */
  upper: readonly PairSection[];
  lower: readonly PairSection[];
  /** The blocks after the join, centred on both lanes; absent when independent. */
  after?: readonly PairSection[];
  /** Whether the lanes meet in the RING, ahead of `after`. */
  ring?: boolean;
  arrows: readonly Arrow[];
}

/** Cells taken by a 1-Cell Lane column between blocks. */
const LANE = 1;

/** Where `path` places the pair's Sections. */
function placements(path: Path): SectionPlacement[] {
  const sections = [...heads("upper"), ...heads("lower")];
  if (!path.after) {
    // Independent: each lane one straight block from WAVE on.
    sections.push(...chain(PATH_X, LANE_Y.upper, path.upper), ...chain(PATH_X, LANE_Y.lower, path.lower));
  } else {
    // A Lane, the lanes' own blocks, a Lane, then the join and what follows it.
    const pre = PATH_X + LANE;
    sections.push(...chain(pre, LANE_Y.upper, path.upper), ...chain(pre, LANE_Y.lower, path.lower));
    let x = pre + Math.max(widthOf(path.upper), widthOf(path.lower)) + LANE;
    if (path.ring) {
      const ring = cellsOf(RING_12_LAYOUT);
      sections.push({ id: "ring", section: RING_12_LAYOUT, x, y: centred(ring.h) });
      x += ring.w + LANE;
    }
    sections.push(...chain(x, centred(ROW_H), path.after));
  }
  return sections;
}

/** The region `path` lays out in a `w`×`H` footprint: its Sections, the Lanes left free, and its arrows. */
function region(path: Path, w: number): ScreenRegion {
  const sections = placements(path);
  const silkscreen: ScreenArrowSilk[] = path.arrows.map(([from, to, glyph]) => ({
    kind: "arrow",
    from: { section: from, side: "right" },
    to: { section: to, side: "left" },
    ...(glyph ? { glyph } : {}),
  }));
  return { sections, lanes: freeLanes(w, H, sections), silkscreen };
}

/** The Section a routing block puts on the path; WAVE heads its lane. */
function blockSection(b: Block): PairSection | null {
  if (b.kind === "booster") return "boost";
  if (b.kind === "wave") return null;
  return `${b.side}.${b.kind === "filter" ? "tvf" : "tva"}`;
}

/** The Sections `blocks` put on the path, in signal order. */
const sectionsOn = (blocks: readonly Block[]): PairSection[] =>
  blocks.flatMap((b) => {
    const id = blockSection(b);
    return id ? [id] : [];
  });

/**
 * Structure `structure`'s path, from core's routing (`pcm_structure.rs`): each
 * lane's blocks, the join and what follows it, and the arrows between blocks.
 * Boxes of one block abut, so arrows only enter a block: from a lane's heads
 * into its own blocks, from each lane into the join, and from the RING on.
 */
function pathOf(structure: number): Path {
  const r = routingOf(structure);
  const join = r.join;
  if (!join) {
    // Independent: each lane one straight block from WAVE to its OUTPUT.
    return { upper: [...sectionsOn(r.upper), "upper.out"], lower: [...sectionsOn(r.lower), "lower.out"], arrows: [] };
  }
  const upper = sectionsOn(r.upper);
  const lower = sectionsOn(r.lower);
  const after: PairSection[] = [...(join.kind === "booster" ? ["boost" as const] : []), ...sectionsOn(join.after), "lower.out"];
  const ring = join.kind === "ring";
  const arrows: Arrow[] = [];
  /** Where each lane leaves for the join: its last block, or its heads. */
  const last = { upper: upper[upper.length - 1] ?? "upper.ctrl", lower: lower[lower.length - 1] ?? "lower.ctrl" } as const;
  if (upper[0]) arrows.push(["upper.ctrl", upper[0]]);
  if (lower[0]) arrows.push(["lower.ctrl", lower[0]]);
  const merged = after[0]!;
  if (ring) {
    // Both lanes into the RING; its product on, summed with partial 2 dry where the Structure says.
    arrows.push([last.upper, "ring"], [last.lower, "ring"], ["ring", merged, join.lowerDry ? "+" : undefined]);
    if (join.lowerDry) arrows.push([last.lower, merged, "+"]);
  } else {
    // Summed where the lanes meet (through the booster, if it opens the merged block).
    arrows.push([last.upper, merged, "+"], [last.lower, merged, "+"]);
  }
  return { upper, lower, after, ring, arrows };
}

/**
 * The Sections Structure `structure` has, from core's routing: every lane's
 * heads, each filter and amp, the BOOST and RING it goes through, and its
 * OUTPUTs: both when independent, else the lower partial's, after the join.
 */
export function pairSectionsOf(structure: number): PairSection[] {
  const r = routingOf(structure);
  const ids = new Set<PairSection>();
  for (const side of ["upper", "lower"] as const) {
    ids.add(`${side}.partial`);
    for (const b of HEADS) ids.add(`${side}.${b}`);
  }
  for (const b of [...r.upper, ...r.lower, ...(r.join?.after ?? [])]) {
    const id = blockSection(b);
    if (id) ids.add(id);
  }
  if (r.join?.kind === "booster") ids.add("boost");
  if (r.join?.kind === "ring") ids.add("ring");
  ids.add("lower.out");
  if (!r.join) ids.add("upper.out");
  return [...ids];
}

/**
 * The pair's width: its heads and its widest path (Structures 5-8: a Lane,
 * partial 1's FILTER and AMP, a Lane, RING, a Lane, then partial 2's FILTER,
 * AMP and OUTPUT). The validator holds every Structure to it.
 */
const W =
  PATH_X +
  LANE +
  widthOf(["upper.tvf", "upper.tva"]) +
  LANE +
  cellsOf(RING_12_LAYOUT).w +
  LANE +
  widthOf(["lower.tvf", "lower.tva", "lower.out"]);

/** Every Structure Type core defines. */
function structureKeys(): number[] {
  const { min, max } = pcms_pmt_spec(PcmsPmtField.StructureType12);
  return Array.from({ length: max - min + 1 }, (_, i) => min + i);
}

/** Each Structure's region, built on first use. */
let variants: ReadonlyMap<number, ScreenRegion> | undefined;

/**
 * A partial pair, keyed by Structure Type (0-based). Its keys and variants are
 * read from core on use, as the app loads this module before the wasm core
 * starts.
 */
export const PCMS_PAIR: RegionVariantSet<number> = {
  name: "PCM Synth partial pair",
  w: W,
  h: H,
  get keys() {
    return structureKeys();
  },
  sectionsOf: pairSectionsOf,
  get variants() {
    variants ??= new Map(structureKeys().map((s) => [s, region(pathOf(s), W)]));
    return variants;
  },
};
