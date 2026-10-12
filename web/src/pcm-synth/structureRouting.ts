import { pcms_structure_routing } from "../../pkg/integral_wasm.js";

/** Which partial of a pair a block belongs to: 1 (or 3) is upper, 2 (or 4) lower. */
export type Side = "upper" | "lower";

/** A block of the pair's signal path. */
export type Block = { kind: "wave" | "filter" | "amp"; side: Side } | { kind: "booster" };

/** Where and how the pair's two paths join. */
export interface Join {
  kind: "mix" | "booster" | "ring";
  /** Whether the lower path is also mixed in dry, beside the join. */
  lowerDry: boolean;
  /** Blocks after the join, in signal order. */
  after: Block[];
}

/** The wiring of a Structure Type, from core's `pcm_structure::routing`. */
export interface Routing {
  upper: Block[];
  lower: Block[];
  /** Absent when the partials stay independent (Type 1). */
  join?: Join | null;
}

/** The routing of Structure Type `structure` (0-9). */
export function routingOf(structure: number): Routing {
  return pcms_structure_routing(structure) as Routing;
}
