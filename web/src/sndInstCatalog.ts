import { snDrumInstsJson } from "../pkg/integral_wasm.js";
import type { Wave, WaveSource } from "./waveCatalog";

/** An SN Drum Inst as core lists it. */
interface SndInst {
  number: number;
  name: string;
  /** Roland's category (Kick, Snare, …), used as the picker's family. */
  family: string;
  /** `INT`, or `ExSN6` when the Inst comes from that expansion. */
  bank: string;
}

let source: WaveSource | null = null;

/**
 * The SN Drum Insts as a single-list source for the Wave picker: picked by
 * name, filtered by Roland's category. Inst Number 0 is OFF.
 */
export function sndInstSource(): WaveSource {
  if (source) return source;
  const insts = JSON.parse(snDrumInstsJson()) as SndInst[];
  const waves: Wave[] = insts.map(({ number, name, family }) => ({ number, name, family }));
  source = {
    id: "Insts",
    params: null,
    // shortcut: ExSN6 Insts (169-294) aren't marked when that expansion isn't loaded; the picker marks a whole source, not single entries.
    expansion: null,
    waves,
    families: [...new Set(waves.map((w) => w.family))],
    name: (n) => (n === 0 ? "OFF" : (waves[n - 1]?.name ?? null)),
    stereoPartner: () => null,
  };
  return source;
}
