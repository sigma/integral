import { snaInstsJson } from "../pkg/integral_wasm.js";
import type { Wave, WaveSource } from "./waveCatalog";

/** An SN Acoustic instrument as core lists it. */
export interface SnaInst {
  /** Position in Roland's list (1…N), the picker's number. */
  number: number;
  name: string;
  /** Roland's category, the picker's family. */
  category: string;
  /** `INT`, or the `ExSN1`…`ExSN5` expansion it comes from. */
  bank: string;
  /** Inst Variation and Inst Number that select it. */
  variation: number;
  instNumber: number;
}

let insts: SnaInst[] | null = null;
let source: WaveSource | null = null;

/** Every SN Acoustic instrument, in Roland's list order. */
export function snaInsts(): SnaInst[] {
  insts ??= JSON.parse(snaInstsJson()) as SnaInst[];
  return insts;
}

/** The list number of the instrument a tone's Inst Variation and Number select, or 0 if unlisted. */
export function snaInstNumberOf(variation: number, instNumber: number): number {
  return snaInsts().find((i) => i.variation === variation && i.instNumber === instNumber)?.number ?? 0;
}

/** The instruments as a single-list source for the Wave picker, filtered by category. */
export function snaInstSource(): WaveSource {
  if (source) return source;
  const waves: Wave[] = snaInsts().map((i) => ({
    number: i.number,
    // Expansion instruments say where they come from.
    name: i.bank === "INT" ? i.name : `${i.name} (${i.bank})`,
    family: i.category,
  }));
  source = {
    id: "Insts",
    params: null,
    // shortcut: ExSN1-5 instruments aren't marked when their expansion isn't loaded; the picker marks a whole source, not single entries.
    expansion: null,
    waves,
    families: [...new Set(waves.map((w) => w.family))],
    name: (n) => (n === 0 ? "(unlisted)" : (waves[n - 1]?.name ?? null)),
    stereoPartner: () => null,
  };
  return source;
}
